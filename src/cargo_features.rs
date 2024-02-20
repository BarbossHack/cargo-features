use crate::{
    cli::PackageVer,
    export_info::ExportInfo,
    print::pretty_print,
    resolver::{build_export_info, build_ws_resolve},
};
use anyhow::{anyhow, Result};
use cargo::Config;

pub struct CargoFeatures {
    pub export_info: ExportInfo,
}

impl CargoFeatures {
    pub fn init(
        manifest_path: Option<String>,
        root_package: Option<PackageVer>,
        features: Vec<String>,
        all_features: bool,
        no_default_features: bool,
        deps: bool,
    ) -> Result<CargoFeatures, anyhow::Error> {
        let manifest_path = match manifest_path {
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
        let package_ver = match root_package {
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
            features,
            all_features,
            !no_default_features,
            &package_name,
            &package_version,
        )?;

        let export_info = build_export_info(ws_resolve, package_ver, deps)?;
        Ok(CargoFeatures { export_info })
    }

    pub fn print(&self) {
        pretty_print(self.export_info.clone());
    }
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
