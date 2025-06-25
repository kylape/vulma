use core::time;
use std::{env, fs::read_to_string, path::PathBuf, process::Command, sync::LazyLock};

use anyhow::Context;
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
use log::{debug, info};

mod client;

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

        if let Some(url) = &self.url {
            let mut client = VirtualMachineServiceClient::connect(url.clone()).await?;
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
            "%{NAME}|%{VERSION}|%{RELEASE}|%{ARCH}\n",
        ]);

        Vulma { url, cmd }
    }
}

pub async fn run(args: VulmaConfig) -> anyhow::Result<()> {
    let (tx, rx) = bounded(0);
    let ticks = tick(time::Duration::from_secs(args.interval));
    let mut vulma: Vulma = args.into();

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
