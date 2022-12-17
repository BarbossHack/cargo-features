use crate::{cli::PackageVer, export_info};
use anyhow::{anyhow, bail, Result};
use cargo::{
    core::{
        compiler::{CompileKind, RustcTargetData},
        dependency::DepKind,
        resolver::{features::FeaturesFor, CliFeatures, ForceAllTargets, HasDevUnits},
        PackageId, PackageIdSpec, Workspace,
    },
    ops::WorkspaceResolve,
};
use semver::VersionReq;
use std::{collections::BTreeMap, path::Path, process::exit};

pub fn build_ws_resolve<'cfg>(
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

pub fn build_export_info(
    ws_resolve: WorkspaceResolve,
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
            // TODO: maybe get some info from Resolve Graph here
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
                // FIXME: bug
                // cargo run -- features clap@4.0.29 --deps : unicode-width not active (but features)
                // cargo tree -i unicode-width : active by cargo, but not by clap, so maybe it's normal that it's not active in clap
                // what choice to make ?
                // check if global active with ws_resolves.packages.any(dep) == true
                // and maybe add a var "activated by another crate"

                // well... pas d'autres choix que de parcourir tout l'arbre du Resolve et de voir s'il est bien de DepKin::Normal
                ws_resolve.resolved_features.is_dep_activated(
                    root_package.package_id(),
                    FeaturesFor::NormalOrDev,
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
                    dep.matches_id(p.package_id())
                    // p.name() == dep.package_name() && dep.version_req().matches(p.version())
                }) {
                    Some(dependency) => {
                        build_package(&ws_resolve, dependency, dep.is_optional(), active)?
                    }
                    // FIX: dirty, isn't it ?
                    // maybe use an enum Dep::Active(package) and Dep::NotActive(Dependency)
                    None => export_info::Package {
                        name: dep.package_name().to_string(),
                        version: dep.version_req().to_string(),
                        optional: dep.is_optional(),
                        active: false,
                        globally_active: false,
                        features: vec![],
                    },
                };
                export_dependencies.push(export_dependency);
            }
        }
    }

    Ok(export_info::ExportInfo {
        root_package: build_package(
            &ws_resolve,
            root_package,
            false,
            is_globally_active(&ws_resolve, &root_package.package_id())?,
        )?,
        dependencies: export_dependencies,
    })
}

pub fn build_package(
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

    let active_features: Vec<String> = match ws_resolve
        .resolved_features
        .activated_features_unverified(package.package_id(), FeaturesFor::NormalOrDev)
    {
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
        globally_active: is_globally_active(ws_resolve, &package.package_id())?,
        // TODO: when adding "optionals" to "features", check if they are active
        features: export_features,
    })
}

pub fn is_globally_active(ws_resolve: &WorkspaceResolve, package_id: &PackageId) -> Result<bool> {
    let mut normal_count = 0;
    let mut build_count = 0;
    let mut dev_count = 0;

    ws_resolve
        .workspace_resolve
        .as_ref()
        .ok_or_else(|| anyhow!("Resolve is empty but it should not happen here"))?
        .path_to_top(package_id)
        .iter()
        .for_each(|(_package_id, opt_deps)| match opt_deps {
            Some(deps) => deps.iter().for_each(|dep| match dep.kind() {
                DepKind::Normal => normal_count += 1,
                DepKind::Development => dev_count += 1,
                DepKind::Build => build_count += 1,
            }),
            None => {}
        });

    if build_count > 0 || dev_count > 0 {
        Ok(normal_count > 0)
    } else {
        Ok(true)
    }
}
