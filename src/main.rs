use clap::Parser;
use vulma::Vulma;

fn main() -> anyhow::Result<()> {
    let args = Vulma::parse();
    vulma::run(args)
}
