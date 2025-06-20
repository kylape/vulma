use clap::Parser;
use vulma::{Vulma, VulmaError};

fn main() -> Result<(), VulmaError> {
    let args = Vulma::parse();
    vulma::run(args)
}
