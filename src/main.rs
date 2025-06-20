use clap::Parser;
use env_logger::Env;
use vulma::Vulma;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().filter_or("VULMA_LOGLEVEL", "info")).init();
    let args = Vulma::parse();
    vulma::run(args)
}
