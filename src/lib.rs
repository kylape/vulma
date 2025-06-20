use std::process::Command;

mod rpm;
use anyhow::Context;
use clap::Parser;
use rpm::Rpm;

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

pub fn run(args: Vulma) -> anyhow::Result<()> {
    let mut rpm = Command::new("rpm");
    rpm.args([
        "--dbpath",
        &args.rpmdb,
        "-qa",
        "--qf",
        "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{SHA256HEADER}\n",
    ]);

    let pkgs = rpm.output().context("Failed to run rpm command")?;
    let stdout =
        std::str::from_utf8(pkgs.stdout.as_slice()).context("Failed to parse rpm output")?;

    let pkgs = stdout
        .lines()
        .map(str::parse::<Rpm>)
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse package information")?;

    println!("Sending updates:");
    pkgs.iter().for_each(|p| println!("{p}"));

    if !args.skip_http {
        reqwest::blocking::Client::new()
            .post(args.url)
            .json(&pkgs)
            .send()
            .context("Failed to post package information")?;
    }
    Ok(())
}
