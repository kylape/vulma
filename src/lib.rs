use std::process::Command;

mod rpm;
use clap::Parser;
use rpm::{Rpm, RpmError};

#[derive(Debug)]
pub enum VulmaError {
    IOError(String),
    Utf8Error(String),
    RpmError(RpmError),
    HttpError(String),
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

impl From<reqwest::Error> for VulmaError {
    fn from(value: reqwest::Error) -> Self {
        VulmaError::HttpError(format!("HTTP request failed: {value}"))
    }
}

#[derive(Debug, Parser)]
pub struct Vulma {
    /// URL to forward the packages to
    #[arg(env = "VULMA_URL", default_value = "http://localhost:8080")]
    url: String,

    /// Skip sending packages over HTTP
    #[arg(short, long)]
    skip_http: bool,

    /// Path to the rpmdb
    #[arg(short, long, env = "VULMA_RPMDB", default_value = "/var/lib/rpm")]
    rpmdb: String,
}

pub fn run(args: Vulma) -> Result<(), VulmaError> {
    let mut rpm = Command::new("rpm");
    rpm.args([
        "--dbpath",
        &args.rpmdb,
        "-qa",
        "--qf",
        "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{SHA256HEADER}\n",
    ]);

    let pkgs = rpm.output()?;
    let stdout = std::str::from_utf8(pkgs.stdout.as_slice())?;

    let pkgs = stdout
        .lines()
        .map(str::parse::<Rpm>)
        .collect::<Result<Vec<_>, _>>()?;

    println!("Sending updates:");
    pkgs.iter().for_each(|p| println!("{p}"));

    if !args.skip_http {
        reqwest::blocking::Client::new()
            .post(args.url)
            .json(&pkgs)
            .send()?;
    }
    Ok(())
}
