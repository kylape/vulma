use core::time;
use std::process::Command;

use crossbeam::{
    channel::{bounded, tick},
    select,
};
use log::{debug, info};

mod rpm;
use anyhow::Context;
use clap::Parser;
use rpm::Rpm;

#[derive(Debug, Parser)]
pub struct VulmaConfig {
    /// URL to forward the packages to
    #[arg(env = "VULMA_URL", default_value = "http://localhost:8080")]
    url: String,

    /// Skip sending packages over HTTP
    #[arg(short, long)]
    skip_http: bool,

    /// Path to the rpmdb
    #[arg(short, long, env = "VULMA_RPMDB", default_value = "/var/lib/rpm")]
    rpmdb: String,

    /// Interval between package scanning in seconds
    #[arg(short, long, env = "VULMA_INTERVAL", default_value_t = 3600)]
    interval: u64,
}

struct Vulma {
    url: Option<String>,
    cmd: Command,
}

impl Vulma {
    fn run(&mut self) -> anyhow::Result<()> {
        info!("Collecting package information...");
        let pkgs = self.cmd.output().context("Failed to run rpm command")?;
        let stdout =
            std::str::from_utf8(pkgs.stdout.as_slice()).context("Failed to parse rpm output")?;

        info!("Parsing...");
        let pkgs = stdout
            .lines()
            .map(str::parse::<Rpm>)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse package information")?;
        debug!("{pkgs:?}");

        info!("Sending updates...");

        if let Some(url) = &self.url {
            reqwest::blocking::Client::new()
                .post(url)
                .json(&pkgs)
                .send()
                .context("Failed to post package information")?;
        }
        Ok(())
    }
}

impl From<VulmaConfig> for Vulma {
    fn from(cfg: VulmaConfig) -> Self {
        let url = if !cfg.skip_http { Some(cfg.url) } else { None };
        let mut cmd = Command::new("rpm");
        cmd.args([
            "--dbpath",
            &cfg.rpmdb,
            "-qa",
            "--qf",
            "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}|%{SHA256HEADER}\n",
        ]);

        Vulma { url, cmd }
    }
}

pub fn run(args: VulmaConfig) -> anyhow::Result<()> {
    let (tx, rx) = bounded(0);
    let ticks = tick(time::Duration::from_secs(args.interval));
    let mut vulma: Vulma = args.into();

    ctrlc::set_handler(move || {
        tx.send(()).unwrap();
    })
    .context("Failed setting signal handler")?;

    // Run once before going into the loop
    vulma.run()?;

    loop {
        select! {
            recv(ticks) -> _ => vulma.run()?,
            recv(rx) -> _ => {
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}
