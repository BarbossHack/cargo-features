mod cli;
mod export_info;
mod print;
mod resolver;

use crate::{
    cli::{Cargo, PackageVer},
    print::pretty_print,
    resolver::{build_export_info, build_ws_resolve},
};
use anyhow::{anyhow, Result};
use cargo::Config;
use clap::Parser;

fn main() -> Result<()> {
    let Cargo::Features(cli) = Cargo::parse();

    let manifest_path = match cli.manifest_path {
        Some(manifest_path) => manifest_path,
        None => format!(
            "{}/Cargo.toml",
            std::env::current_dir()?
                .to_str()
                .expect("Can't get current dir")
        ),
    };

    let (package_name, package_version) = parse_manifest(&manifest_path)?;

    // Get the root package to display
    let package_ver = match cli.root_package {
        Some(package_ver) => package_ver,
        None => PackageVer {
            name: package_name.clone(),
            version: None,
        },
    };

    let config = Config::default()?;
    let ws_resolve = build_ws_resolve(
        &config,
        manifest_path,
        cli.features.unwrap_or_default(),
        cli.all_features,
        !cli.no_default_features,
        &package_name,
        &package_version,
    )?;
    let export_info = build_export_info(ws_resolve, package_ver, cli.deps)?;
    pretty_print(export_info);

    Ok(())
}

fn parse_manifest(manifest_path: &str) -> Result<(String, String)> {
    let manifest = std::fs::read_to_string(manifest_path)
        .map_err(|_| anyhow!("Could not find a valid Cargo.toml"))?
        .parse::<toml::Value>()
        .map_err(|_| anyhow!("Could not find a valid Cargo.toml"))?;
    let manifest_name = manifest["package"]["name"]
        .as_str()
        .ok_or_else(|| anyhow!("Toml package should contains name"))?
        .to_string();
    let manifest_version = manifest["package"]["version"]
        .as_str()
        .ok_or_else(|| anyhow!("Toml package should contains version"))?
        .to_string();
    Ok((manifest_name, manifest_version))
}
