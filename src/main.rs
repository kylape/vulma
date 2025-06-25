use clap::Parser;
use env_logger::Env;
use vulma::VulmaConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().filter_or("VULMA_LOGLEVEL", "info")).init();
    let args = VulmaConfig::parse();
    vulma::run(args).await
}
