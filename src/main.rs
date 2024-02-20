mod cargo_features;
mod cli;
mod export_info;
mod print;
mod resolver;

use crate::{cargo_features::CargoFeatures, cli::Cargo};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let Cargo::Features(cli) = Cargo::parse();

    let cargo_features = CargoFeatures::init(
        cli.manifest_path,
        cli.root_package,
        cli.features,
        cli.all_features,
        cli.no_default_features,
        cli.deps,
    )?;
    cargo_features.print();

    Ok(())
}
