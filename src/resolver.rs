use crate::{
    cli::PackageVer,
    export_info::{self, Child, Feature, Optional},
};
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
use std::{collections::BTreeMap, path::Path};

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
    // TODO: check that CompileKind::Host is the right thing (try to add a dep in cfg(target anroid))
    let requested_kinds = &[CompileKind::Host];
    let mut target_data = RustcTargetData::new(&ws, requested_kinds)?;
    let cli_features =
        CliFeatures::from_command_line(&features, all_features, uses_default_features)?;
    let ws_resolve = cargo::ops::resolve_ws_with_opts(
        &ws,
        &mut target_data,
        requested_kinds,
        &cli_features,
        &[PackageIdSpec::parse(
            format!("{}@{}", manifest_name, manifest_version,).as_str(),
        )?],
        HasDevUnits::No,
        ForceAllTargets::No,
        None,
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
            "Package `{}` does not exists, or is not part of the tree (optional and not active)",
            root_package.name
        ),
        1 => {}
        _ => {
            eprintln!("There are multiple `{}` packages in your project, and the specification `{}` is ambiguous.", root_package.name, root_package.name);
            eprintln!("Please re-run this command with `-p <spec>` where `<spec>` is one of the following:");
            possible_packages
                .iter()
                .for_each(|p| eprintln!("  {}@{}", p.name(), p.version()));
            std::process::exit(1);
        }
    }

    let root_package = possible_packages
        .first()
        .expect("Can't fail with before check");

    let mut export_dependencies = Vec::new();
    if deps {
        for dep in root_package.dependencies() {
            let active = match (
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
                let export_dependency = match ws_resolve
                    .pkg_set
                    .packages()
                    .find(|p| dep.matches_id(p.package_id()))
                {
                    Some(dependency) => {
                        build_package(&ws_resolve, dependency, dep.is_optional(), active)?
                    }
                    None => export_info::Package {
                        name: dep.package_name().to_string(),
                        version: dep.version_req().to_string(),
                        optional: dep.is_optional(),
                        active: false,
                        globally_active: false,
                        features: Vec::new(),
                        optionals: Vec::new(),
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

    let active_features: Vec<String> = match ws_resolve
        .resolved_features
        .activated_features_unverified(package.package_id(), FeaturesFor::NormalOrDev)
    {
        Some(activated_features) => activated_features.iter().map(|i| i.to_string()).collect(),
        None => Vec::new(),
    };

    let export_features: Vec<Feature> = available_features
        .iter()
        .map(|(name, childs)| export_info::Feature {
            name: name.to_owned(),
            optional: childs.len().eq(&1)
                && childs
                    .first()
                    .expect("we already checked that len is 1")
                    .eq(format!("dep:{}", name).as_str()),
            active: active_features.contains(name),
            childs: childs
                .iter()
                .map(|child_name| Child {
                    name: child_name.to_owned(),
                    optional: child_name.starts_with("dep:")
                        || package.dependencies().iter().any(|d| {
                            d.is_optional()
                                && clean_feature_name(&d.package_name())
                                    .eq(&clean_feature_name(child_name))
                        }),
                })
                .collect(),
        })
        .collect();

    let globally_active = is_globally_active(ws_resolve, &package.package_id())?;
    if active && !globally_active {
        bail!("Package cannot be active by parent, and not globally active");
    }

    // do not include in optionals if another feature has this optional in child, without "dep:" ??
    // see `toml` feature in `cargo run -- features --deps`
    //
    // well, no, but search for childs where name are in Optionals, and print them in cyan.
    // Get info of when "dep:" is required (when there is at least one "dep:" ? or never ?)
    //
    // if we include (in toml) a feature that active an optional, without "dep:", the optional will be seen as a feature
    // else if the feature include the optional with the "dep:", this optional won't be included as a feature

    // FIXME: When use dep: syntax, can’t enable dep without dep: (see screenshot)
    // https://github.com/Riey/cargo-feature/issues/28
    let optionals = package
        .dependencies()
        .iter()
        .filter(|d| d.is_optional())
        .map(|d| Optional {
            name: d.package_name().to_string(),
            // only search for feature that enable this optional
            // we don't want to check here if this optional dep is globally active
            active: export_features.iter().any(|f| {
                f.active
                    && f.childs.iter().any(|c| {
                        clean_feature_name(&c.name).eq(&clean_feature_name(&d.package_name()))
                    })
            }),
        })
        .collect();

    Ok(export_info::Package {
        name: package.name().to_string(),
        version: package.version().to_string(),
        optional,
        active,
        globally_active: is_globally_active(ws_resolve, &package.package_id())?,
        features: export_features,
        optionals,
    })
}

fn clean_feature_name(name: &str) -> String {
    name.replace("dep:", "")
        .split(|c| c == '?' || c == '/')
        .collect::<Vec<&str>>()
        .first()
        .expect("feature name should not have been empty")
        .to_string()
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
