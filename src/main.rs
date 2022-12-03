mod cli;
mod export_info;
mod print;

use crate::{cli::PackageVer, print::pretty_print};
use anyhow::{bail, Result};
use cargo::{
    core::{
        compiler::{CompileKind, RustcTargetData},
        dependency::DepKind,
        resolver::{features::FeaturesFor, CliFeatures, ForceAllTargets, HasDevUnits},
        PackageIdSpec, Workspace,
    },
    ops::WorkspaceResolve,
    Config,
};
use clap::Parser;
use cli::Command;
use semver::VersionReq;
use std::{collections::BTreeMap, fs, path::Path, process::exit};

fn main() -> Result<()> {
    let Command::Features(cli) = Command::parse();

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
    let export_info = build_export_info(&ws_resolve, package_ver, cli.deps)?;
    pretty_print(export_info);

    Ok(())
}

fn parse_manifest(manifest_path: &str) -> Result<(String, String)> {
    let manifest = fs::read_to_string(manifest_path)?.parse::<toml::Value>()?;
    let manifest_name = manifest["package"]["name"]
        .as_str()
        .expect("Toml package should contains name")
        .to_string();
    let manifest_version = manifest["package"]["version"]
        .as_str()
        .expect("Toml package should contains version")
        .to_string();
    Ok((manifest_name, manifest_version))
}

fn build_ws_resolve<'cfg>(
    config: &'cfg cargo::util::Config,
    manifest_path: String,
    features: Vec<String>,
    all_features: bool,
    uses_default_features: bool,
    manifest_name: &str,
    manifest_version: &str,
) -> Result<WorkspaceResolve<'cfg>> {
    let ws = Workspace::new(Path::new(&manifest_path), config)?;
    let requested_kinds = &vec![CompileKind::Host];
    let target_data = RustcTargetData::new(&ws, requested_kinds)?;
    let cli_features =
        CliFeatures::from_command_line(&features, all_features, uses_default_features)?;
    let ws_resolve = cargo::ops::resolve_ws_with_opts(
        &ws,
        &target_data,
        requested_kinds,
        &cli_features,
        &[PackageIdSpec::parse(
            format!("{}@{}", manifest_name, manifest_version,).as_str(),
        )?],
        HasDevUnits::No,
        ForceAllTargets::No,
    )?;
    Ok(ws_resolve)
}

fn build_export_info(
    ws_resolve: &WorkspaceResolve,
    root_package: PackageVer,
    deps: bool,
) -> Result<export_info::ExportInfo> {
    let possible_packages: Vec<&cargo::core::Package> = ws_resolve
        .pkg_set
        .packages()
        .filter(|p| match &root_package.version {
            Some(version) => {
                let vereq = VersionReq::parse(&version.to_string())
                    .expect("Semver should be able to parse Semver");
                p.name().to_string() == root_package.name && vereq.matches(p.version())
            }
            None => p.name().to_string() == root_package.name,
        })
        .collect();

    match possible_packages.len() {
        0 => bail!(
            "Package '{}' does not exists, or is optional and not active",
            root_package.name
        ),
        1 => {}
        _ => {
            println!("There are multiple `{}` packages in your project, and the specification `{}` is ambiguous.", root_package.name, root_package.name);
            println!("Please re-run this command with `-p <spec>` where `<spec>` is one of the following:");
            possible_packages
                .iter()
                .for_each(|p| println!("  {}@{}", p.name(), p.version()));
            exit(101);
        }
    }
    // FIXME: is this root package "active" ?
    // we have to search for the parent->dependencies, if any...
    let root_package = possible_packages
        .first()
        .expect("Can't fail with before check");

    let mut export_dependencies = Vec::new();
    if deps {
        for dep in root_package.dependencies() {
            let active = match (
                ws_resolve.resolved_features.is_dep_activated(
                    root_package.package_id(),
                    FeaturesFor::NormalOrDevOrArtifactTarget(None),
                    dep.package_name(),
                ),
                dep.is_optional(),
            ) {
                (true, true) => true,
                (true, false) => true,
                (false, true) => false,
                (false, false) => true,
            };

            if dep.kind() == DepKind::Normal {
                let export_dependency = match ws_resolve.pkg_set.packages().find(|p| {
                    p.name() == dep.package_name() && dep.version_req().matches(p.version())
                }) {
                    Some(dependency) => {
                        build_package(ws_resolve, dependency, dep.is_optional(), active)?
                    }
                    // TODO: dirty, isn't it ?
                    // maybe use an enum Dep::Active(package) and Dep::NotActive(Dependency)
                    None => export_info::Package {
                        name: dep.package_name().to_string(),
                        version: dep.version_req().to_string(),
                        optional: dep.is_optional(),
                        active,
                        features: vec![],
                    },
                };
                export_dependencies.push(export_dependency);
            }
        }
    }

    Ok(export_info::ExportInfo {
        root_package: build_package(ws_resolve, root_package, false, true)?,
        dependencies: export_dependencies,
    })
}

fn build_package(
    ws_resolve: &WorkspaceResolve,
    package: &cargo::core::Package,
    optional: bool,
    active: bool,
) -> Result<export_info::Package> {
    let available_features: BTreeMap<String, Vec<String>> = package
        .summary()
        .features()
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|fv| fv.to_string()).collect()))
        .collect();
    // TODO: add optionnals ? because not displayed by default
    // if we include (in toml) a feature that active an optional, without "dep:", the optional will be seen as a feature
    // else if the feature include the optional with the "dep:", this optional won't be included as a feature

    let active_features: Vec<String> =
        match ws_resolve.resolved_features.activated_features_unverified(
            package.package_id(),
            FeaturesFor::NormalOrDevOrArtifactTarget(None),
        ) {
            Some(activated_features) => activated_features.iter().map(|i| i.to_string()).collect(),
            None => vec![],
        };

    let export_features = available_features
        .into_iter()
        .map(|f| export_info::Feature {
            name: f.0.clone(),
            active: active_features.contains(&f.0),
            childs: f.1,
        })
        .collect();

    Ok(export_info::Package {
        name: package.name().to_string(),
        version: package.version().to_string(),
        optional,
        active,
        features: export_features,
    })
}
