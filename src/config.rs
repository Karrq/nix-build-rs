use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{Error, Result};

enum NixTarget {
    Function(OsString),
    Flake(String),
    Expr(String),
}

impl Default for NixTarget {
    fn default() -> Self {
        Self::Function(OsString::from("default.nix"))
    }
}

/// Build style configration for a pending build.
pub struct Config {
    target: NixTarget,
    arg_exprs: Vec<(String, String)>,
    arg_strs: Vec<(String, String)>,
    impure: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a nix build output derivation
#[derive(Debug, serde::Deserialize)]
pub struct Derivation {
    #[serde(alias = "drvPath")]
    /// Derivation path
    pub drv_path: PathBuf,
    /// List of outputs for this derivation
    ///
    /// Example outputs: `out`, `dev`
    pub outputs: HashMap<String, PathBuf>,
}

impl Derivation {
    pub fn out(&self) -> Option<&PathBuf> {
        self.outputs.get("out")
    }
}

impl Config {
    /// Create a new nix build [`Config`]
    ///
    /// Target is defaulted to `default.nix`
    pub fn new() -> Self {
        Self {
            target: NixTarget::default(),
            arg_exprs: vec![],
            arg_strs: vec![],
            impure: false,
        }
    }

    /// Add an expression argument to the invoked nix expression
    ///
    /// # Example
    /// ```
    /// Config::default().arg_expr("{pkgs}:pkgs.hello").arg_expr("pkgs", "import <nixpkgs> {}")
    /// ```
    pub fn arg_expr(&mut self, name: &str, value: &str) -> &mut Self {
        self.arg_exprs.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Add a string argument to the invoked nix expression
    ///
    /// # Example
    /// ```
    /// Config::default().arg_str("{pkgs,name}:pkgs.hello.overrideAttrs (_: {inherit name;})").arg_str("name", "not-hello")
    /// ```
    pub fn arg_str(&mut self, name: &str, value: &str) -> &mut Self {
        self.arg_strs.push((name.to_owned(), value.to_owned()));
        self
    }

    /// Build the derivation described by the given .nix file
    ///
    /// # Example
    /// ```
    /// Config::default().target_file("hello.nix")
    /// ```
    pub fn target_file(&mut self, filename: impl AsRef<OsStr>) -> &mut Self {
        self.target = NixTarget::Function(filename.as_ref().to_owned());
        self
    }

    /// Build the derivation described by the given flake output
    ///
    /// # Example
    /// ```
    /// Config::default().target_flake("nixpkgs#hello")
    /// ```
    pub fn target_flake(&mut self, flake: &str) -> &mut Self {
        self.target = NixTarget::Flake(flake.to_owned());
        self
    }

    /// Build the derivation described by the given expression
    ///
    /// # Example
    /// ```
    /// Config::default().target_expr("{pkgs}: pkgs.hello").build()
    /// ```
    pub fn target_expr(&mut self, expr: &str) -> &mut Self {
        self.target = NixTarget::Expr(expr.to_owned());
        self
    }

    /// Set to enable impure evaluation mode
    ///
    /// Will pass the `--impure` flag to the invocation if set
    pub fn impure(&mut self, impure: bool) -> &mut Self {
        self.impure = impure;
        self
    }

    /// Invoke `nix build` with the given configuration
    #[must_use]
    pub fn build(&self) -> Result<Vec<Derivation>> {
        let nix = crate::is_nix_available().ok_or(Error::NixNotAvailable)?;

        let cwd = std::env::current_dir().unwrap();
        let mut cmd = Command::new(nix);
        cmd.current_dir(&cwd);
        cmd.arg("build");

        cmd.args(&["--no-link", "--json"]);

        match &self.target {
            NixTarget::Function(file) => {
                cmd.args(&[OsStr::new("-f"), &file]);

                // make sure the build script is rerun if the file changes
                println!(
                    "cargo:rerun-if-changed={}",
                    AsRef::<Path>::as_ref(file).display()
                );
            }
            NixTarget::Flake(installable) => {
                cmd.arg(installable);

                // try to detect if the flake is local
                if let Some(Ok(local_flake)) = cwd
                    .to_string_lossy()
                    .split_once('#')
                    .map(|(path, _)| path)
                    .map(std::fs::canonicalize)
                {
                    // and if so, rerun if it changes
                    println!(
                        "cargo:rerun-if-changed={}",
                        local_flake.join("flake.lock").display()
                    );
                }
            }
            NixTarget::Expr(expr) => {
                cmd.args(["--expr", expr.as_str()]);
            }
        }

        for (key, val) in &self.arg_exprs {
            cmd.args(&["--arg", &key, &val]);
        }

        for (key, val) in &self.arg_strs {
            cmd.args(&["--argstr", &key, &val]);
        }

        if self.impure {
            cmd.arg("--impure");
        }

        //show build logs
        cmd.arg("-L");

        // enable split commands and flakes
        cmd.args(&["--experimental-features", "nix-command flakes"]);

        let output = cmd.output().map_err(|_| Error::BuildFailed)?;

        if !output.status.success() {
            return Err(Error::BuildFailed);
        }

        serde_json::from_slice(&output.stdout).map_err(|_| Error::UnknownOutput)
    }
}
