use clap::{Args, Parser};
use semver::Version;

#[derive(Parser)]
#[command(
    name = "cargo",
    bin_name = "cargo",
    about = "Display all the active/available features for the specified package"
)]
pub enum Command {
    Features(Cli),
}

#[derive(Args, Clone)]
#[command(
    version,
    about = "Display all the active/available features for the specified package"
)]
pub struct Cli {
    #[arg(
        short = 'F',
        long,
        value_delimiter = ',',
        help = "Comma separated list of features to activate"
    )]
    pub features: Option<Vec<String>>,

    #[arg(long, help = "Activate all available features")]
    pub all_features: bool,

    #[arg(long, help = "Do not activate the `default` feature")]
    pub no_default_features: bool,

    #[arg(
        value_name = "CRATE",
        help = "Package to be used as the root of the tree"
    )]
    pub root_package: Option<PackageVer>,

    #[arg(long, help = "Path to Cargo.toml")]
    pub manifest_path: Option<String>,

    #[arg(long, help = "Output informations about crate dependencies")]
    pub deps: bool,
}

#[derive(Clone, Debug)]
pub struct PackageVer {
    pub name: String,
    pub version: Option<Version>,
}

impl From<String> for PackageVer {
    fn from(package_ver: String) -> Self {
        let mut split = package_ver.split('@');
        PackageVer {
            name: split
                .next()
                .expect("There should be at least a package name")
                .to_string(),
            version: split
                .next()
                .map(|v| Version::parse(v).expect("This is not a valid version.")),
        }
    }
}
