use std::os::unix::io::{AsRawFd, RawFd};
use std::io::{self, Write, Read};
use anyhow::{Context, Result};
use log::{debug, info, warn};
use nix::sys::socket::{
    bind, connect, socket, AddressFamily, SockAddr, SockFlag, SockType, VsockAddr,
};

const VSOCK_PORT: u32 = 514; // Same port as used by vsock-listener
const VMADDR_CID_HOST: u32 = 2; // Host context ID

pub struct VsockClient {
    fd: RawFd,
}

impl VsockClient {
    /// Create a new VSOCK client connection to the host
    pub fn connect() -> Result<Self> {
        info!("Connecting to host via VSOCK on port {}", VSOCK_PORT);
        
        // Create VSOCK socket
        let fd = socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .context("Failed to create VSOCK socket")?;
        
        // Connect to host
        let addr = SockAddr::Vsock(VsockAddr::new(VMADDR_CID_HOST, VSOCK_PORT));
        connect(fd, &addr).context("Failed to connect to VSOCK host")?;
        
        info!("Successfully connected to host via VSOCK");
        Ok(VsockClient { fd })
    }
    
    /// Send data with protocol header
    pub fn send_data(&mut self, message_type: u32, data: &[u8]) -> Result<()> {
        debug!("Sending VSOCK message: type={}, len={}", message_type, data.len());
        
        // Create message header (8 bytes: 4 bytes length + 4 bytes type)
        let mut header = [0u8; 8];
        header[0..4].copy_from_slice(&(data.len() as u32).to_le_bytes());
        header[4..8].copy_from_slice(&message_type.to_le_bytes());
        
        // Send header
        self.write_all(&header)
            .context("Failed to send message header")?;
        
        // Send data
        self.write_all(data)
            .context("Failed to send message data")?;
        
        // Read acknowledgment (4 bytes)
        let mut ack = [0u8; 4];
        self.read_exact(&mut ack)
            .context("Failed to read acknowledgment")?;
        
        let ack_code = u32::from_le_bytes(ack);
        if ack_code != 0 {
            return Err(anyhow::anyhow!("Server returned error code: {}", ack_code));
        }
        
        debug!("Message sent successfully and acknowledged");
        Ok(())
    }
    
    /// Write all bytes to the socket
    fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()> {
        while !buf.is_empty() {
            match nix::unistd::write(self.fd, buf) {
                Ok(0) => return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write whole buffer"
                )),
                Ok(n) => buf = &buf[n..],
                Err(nix::errno::Errno::EINTR) => {}
                Err(e) => return Err(io::Error::from(e)),
            }
        }
        Ok(())
    }
    
    /// Read exact number of bytes from the socket
    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        while !buf.is_empty() {
            match nix::unistd::read(self.fd, buf) {
                Ok(0) => return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer"
                )),
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(nix::errno::Errno::EINTR) => {}
                Err(e) => return Err(io::Error::from(e)),
            }
        }
        Ok(())
    }
    
    /// Check if VSOCK is available on this system
    pub fn is_available() -> bool {
        // Try to create a VSOCK socket to test availability
        match socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        ) {
            Ok(fd) => {
                let _ = nix::unistd::close(fd);
                true
            }
            Err(_) => false,
        }
    }
}

impl Drop for VsockClient {
    fn drop(&mut self) {
        if let Err(e) = nix::unistd::close(self.fd) {
            warn!("Failed to close VSOCK socket: {}", e);
        }
    }
}

impl AsRawFd for VsockClient {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}