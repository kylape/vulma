use core::time;
use std::{env, fs::read_to_string, path::PathBuf, process::Command, str::FromStr, sync::LazyLock};

use anyhow::Context;
use certs::Certs;
use clap::Parser;
use client::{
    sensor::{
        virtual_machine_service_client::VirtualMachineServiceClient, UpsertVirtualMachineRequest,
    },
    storage::{EmbeddedImageScanComponent, VirtualMachine, VirtualMachineScan},
};
use crossbeam::{
    channel::{bounded, tick},
    select,
};
use log::{debug, info, warn};
use tonic::{
    metadata::MetadataValue,
    service::{interceptor::InterceptedService, Interceptor},
    transport::{Channel, ClientTlsConfig},
};
use vsock::VsockClient;

mod certs;
mod client;
mod vsock;

static HOST_MOUNT: LazyLock<PathBuf> =
    LazyLock::new(|| env::var("VULMA_HOST_MOUNT").unwrap_or_default().into());

static HOSTNAME: LazyLock<String> = LazyLock::new(|| {
    let hostname_paths = ["/etc/hostname", "/proc/sys/kernel/hostname"];
    for p in hostname_paths {
        let p = HOST_MOUNT.join(p);
        if p.exists() {
            return read_to_string(p).unwrap().trim().to_string();
        }
    }
    "no-hostname".to_string()
});

#[derive(Debug, Clone)]
struct UserAgentInterceptor {}

impl Interceptor for UserAgentInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        request.metadata_mut().insert(
            "user-agent",
            MetadataValue::from_str("Rox Admission Controller").unwrap(),
        );
        Ok(request)
    }
}

#[derive(Debug, Parser)]
pub struct VulmaConfig {
    /// URL to forward the packages to
    #[arg(env = "VULMA_URL", default_value = "http://localhost:8080")]
    url: String,

    /// Skip sending packages over HTTP
    #[arg(short, long)]
    skip_http: bool,

    /// Use VSOCK instead of HTTP/gRPC for communication
    #[arg(long, env = "VULMA_USE_VSOCK")]
    use_vsock: bool,

    /// Path to the rpmdb
    #[arg(short, long, env = "VULMA_RPMDB", default_value = "/var/lib/rpm")]
    rpmdb: String,

    /// Interval between package scanning in seconds
    #[arg(short, long, env = "VULMA_INTERVAL", default_value_t = 3600)]
    interval: u64,

    /// Directory holding the mTLS certificates and keys
    #[arg(short, long, env = "VULMA_CERTS")]
    certs: Option<PathBuf>,
}

struct Vulma {
    url: Option<String>,
    cmd: Command,
    certs: Option<Certs>,
    user_agent: UserAgentInterceptor,
    use_vsock: bool,
}

impl Vulma {
    async fn run(&mut self) -> anyhow::Result<()> {
        info!("Collecting package information...");
        let pkgs = self.cmd.output().context("Failed to run rpm command")?;
        let stdout =
            std::str::from_utf8(pkgs.stdout.as_slice()).context("Failed to parse rpm output")?;

        info!("Parsing...");
        let pkgs = stdout
            .lines()
            .map(str::parse::<EmbeddedImageScanComponent>)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse package information")?;
        debug!("{pkgs:?}");

        info!("Sending updates...");

        if self.use_vsock {
            self.send_vsock(pkgs).await?;
        } else if let Some(url) = &self.url {
            self.send_grpc(url.to_string(), pkgs).await?;
        }
        Ok(())
    }

    async fn create_client(
        &self,
        url: String,
    ) -> anyhow::Result<
        VirtualMachineServiceClient<InterceptedService<Channel, UserAgentInterceptor>>,
    > {
        let mut channel = Channel::from_shared(url)?;

        if let Some(certs) = &self.certs {
            let tls = ClientTlsConfig::new()
                .domain_name("sensor.stackrox.svc")
                .ca_certificate(certs.ca.clone())
                .identity(certs.identity.clone());
            channel = channel.tls_config(tls)?;
        }

        let channel = channel.connect().await?;
        let client =
            VirtualMachineServiceClient::with_interceptor(channel, self.user_agent.clone());
        Ok(client)
    }

    async fn send_grpc(&self, url: String, pkgs: Vec<EmbeddedImageScanComponent>) -> anyhow::Result<()> {
        let mut client = self.create_client(url).await?;
        let scan = VirtualMachineScan {
            components: pkgs,
            ..Default::default()
        };
        let vm = VirtualMachine {
            id: HOSTNAME.to_string(),
            scan: Some(scan),
            ..Default::default()
        };
        let request = UpsertVirtualMachineRequest {
            virtual_machine: Some(vm),
        };

        client.upsert_virtual_machine(request).await?;
        Ok(())
    }

    async fn send_vsock(&self, pkgs: Vec<EmbeddedImageScanComponent>) -> anyhow::Result<()> {
        if !VsockClient::is_available() {
            return Err(anyhow::anyhow!("VSOCK is not available on this system"));
        }

        let mut client = VsockClient::connect()
            .context("Failed to connect to VSOCK endpoint")?;

        // Create package data message
        let scan = VirtualMachineScan {
            components: pkgs,
            ..Default::default()
        };
        let vm = VirtualMachine {
            id: HOSTNAME.to_string(),
            scan: Some(scan),
            ..Default::default()
        };

        // Serialize the VM data to bytes
        // For now, we'll use a simple JSON serialization
        // In production, you'd want to use protobuf
        let data = serde_json::to_vec(&vm)
            .context("Failed to serialize VM data")?;

        // Send with message type 1 (package data)
        client.send_data(1, &data)
            .context("Failed to send VM data via VSOCK")?;

        info!("Successfully sent {} packages via VSOCK", vm.scan.as_ref().map(|s| s.components.len()).unwrap_or(0));
        Ok(())
    }
}

impl TryFrom<VulmaConfig> for Vulma {
    type Error = anyhow::Error;

    fn try_from(cfg: VulmaConfig) -> Result<Self, Self::Error> {
        let url = if !cfg.skip_http { Some(cfg.url) } else { None };
        let mut cmd = Command::new("rpm");
        cmd.args([
            "--dbpath",
            &cfg.rpmdb,
            "-qa",
            "--qf",
            "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}\n",
        ]);
        let certs = if let Some(path) = cfg.certs {
            Some(path.try_into()?)
        } else {
            None
        };

        Ok(Vulma {
            url,
            cmd,
            certs,
            user_agent: UserAgentInterceptor {},
            use_vsock: cfg.use_vsock,
        })
    }
}

pub async fn run(args: VulmaConfig) -> anyhow::Result<()> {
    let (tx, rx) = bounded(0);
    let ticks = tick(time::Duration::from_secs(args.interval));
    let mut vulma: Vulma = args.try_into()?;

    // Check VSOCK availability if requested
    if vulma.use_vsock {
        if !VsockClient::is_available() {
            return Err(anyhow::anyhow!(
                "VSOCK support requested but not available on this system. \
                Ensure the VM has autoattachVSOCK: true configured."
            ));
        }
        info!("Using VSOCK communication mode");
    } else if vulma.url.is_some() {
        info!("Using gRPC communication mode");
    } else {
        info!("No communication method configured (dry run mode)");
    }

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .context("Failed setting signal handler")?;

    // Run once before going into the loop
    vulma.run().await?;

    loop {
        select! {
            recv(ticks) -> _ => vulma.run().await?,
            recv(rx) -> _ => {
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}
