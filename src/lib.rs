use std::{env, process::Command};

mod rpm;
use rpm::{Rpm, RpmError};

#[derive(Debug)]
pub enum VulmaError {
    IOError(String),
    Utf8Error(String),
    RpmError(RpmError),
}

impl From<std::io::Error> for VulmaError {
    fn from(value: std::io::Error) -> Self {
        VulmaError::IOError(format!("rpm command failed: {}", value))
    }
}

impl From<std::str::Utf8Error> for VulmaError {
    fn from(value: std::str::Utf8Error) -> Self {
        VulmaError::Utf8Error(format!("Failed to read stdout: {}", value))
    }
}

impl From<RpmError> for VulmaError {
    fn from(value: RpmError) -> Self {
        VulmaError::RpmError(value)
    }
}

pub fn run() -> Result<(), VulmaError> {
    let mut rpm = Command::new("rpm");
    if let Ok(dbpath) = env::var("VULMA_RPMDB") {
        rpm.args(["--dbpath", &dbpath]);
    }
    rpm.args([
        "-qa",
        "--qf",
        "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{SHA256HEADER}\n",
    ]);

    let pkgs = rpm.output()?;
    let stdout = std::str::from_utf8(pkgs.stdout.as_slice())?;

    stdout
        .lines()
        .map(str::parse::<Rpm>)
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .for_each(|p| println!("{p}"));

    Ok(())
}
