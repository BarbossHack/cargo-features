# cargo-features

Word in progress

## Usage

```bash
Display all the active/available features for the specified package

Usage: cargo features [OPTIONS] [CRATE]

Arguments:
  [CRATE]  Package to be used as the root of the tree

Options:
  -F, --features <FEATURES>            Comma separated list of features to activate
      --all-features                   Activate all available features
      --no-default-features            Do not activate the `default` feature
      --manifest-path <MANIFEST_PATH>  Path to Cargo.toml
      --deps                           Output informations about crate dependencies
  -h, --help                           Print help information
  -V, --version                        Print version information
```
