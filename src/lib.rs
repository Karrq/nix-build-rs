use std::path::PathBuf;

mod config;
pub use config::{Config, Derivation};

/// Collection of Nix expressions useful for package configuration
pub mod exprs;

#[derive(Debug)]
pub enum Error {
    NixNotAvailable,
    BuildFailed,
    UnknownOutput,
}

type Result<T> = std::result::Result<T, Error>;

const NIX_BIN_NAME: &str = "nix";

/// Returns the path to the found `nix` program
///
/// Will prioritize the `NIX` environment variable if set
pub fn is_nix_available() -> Option<PathBuf> {
    std::env::var_os("NIX")
        .map(PathBuf::from)
        .or_else(|| which::which(NIX_BIN_NAME).ok())
        .and_then(|nix| {
            if nix.try_exists().ok().unwrap_or_default() {
                Some(nix)
            } else {
                None
            }
        })
}

/// Builds the derivation found in `default.nix` with default options
///
/// Returns the resulting derivations
///
/// # Examples
/// ```no_run
/// use nix_build as nix;
///
/// # fn main() -> Result<(), nix::Error> {
/// let derivations = nix::build()?; // will build ./default.nix
/// let libfoo = derivations[0].output().expect("to have an 'out' derivation");
///
/// println!("cargo:rustc-link-search=native={}", libfoo.display());
/// println!("cargo:rustc-link-lib=static=foo");
/// # Ok(()) }
/// ```
pub fn build() -> Result<Vec<Derivation>> {
    Config::new().build()
}
