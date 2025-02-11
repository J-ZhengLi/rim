use env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Result};

use crate::common::*;
use crate::toolkits_parser::{ReleaseMode, Toolkit, Toolkits, PACKAGE_DIR};

pub const DIST_HELP: &str = r#"
Generate release binaries

Usage: cargo dev dist [OPTIONS]

Options:
        --cli       Generate release binary for CLI mode only
        --gui       Generate release binary for GUI mode only
    -n, --name      Choose a toolkit name to distribute, defaulting to `basic`
    -t, --target    Specify a target to distribute, defaulting to current target
    -b, --binary-only
                    Build binary only (net-installer), skip offline package generation
    -h, -help       Print this help message
"#;

/// A dist worker has two basic jobs:
///
/// 1. Run build command to create binaries.
/// 2. Collect built binaries and move them into specific folder.
#[derive(Debug)]
struct DistWorker<'a> {
    is_cli: bool,
    toolkit: &'a Toolkit,
}

impl<'a> DistWorker<'a> {
    fn new_(toolkit: &'a Toolkit, is_cli: bool) -> Self {
        Self { toolkit, is_cli }
    }

    fn cli(toolkit: &'a Toolkit) -> Self {
        Self::new_(toolkit, true)
    }

    fn gui(toolkit: &'a Toolkit) -> Self {
        Self::new_(toolkit, false)
    }

    /// The compiled binary name
    fn source_binary_name(&self) -> String {
        if self.is_cli {
            format!("rim-cli{EXE_SUFFIX}")
        } else {
            format!("rim-gui{EXE_SUFFIX}")
        }
    }

    /// The binary name that user see.
    ///
    /// `simple` - the simple version of binary name, no toolkit info.
    fn dest_binary_name(&self, simple: bool) -> String {
        format!(
            "{}installer{}{EXE_SUFFIX}",
            (!simple)
                .then_some(format!("{}-", self.toolkit.full_name().replace(' ', "-")))
                .unwrap_or_default(),
            self.is_cli.then_some("-cli").unwrap_or_default(),
        )
    }

    fn build_args(&self, target: &'a str, noweb: bool) -> Vec<&'a str> {
        if self.is_cli {
            let mut base = vec!["build", "--release", "--locked", "--target", target];
            if noweb {
                base.extend(["--features", "no-web"]);
            }
            base
        } else {
            let mut base = vec!["tauri", "build", "-b", "none", "--target", target];
            if noweb {
                base.extend(["--features", "no-web"]);
            }
            base.extend(["--", "--locked"]);
            base
        }
    }

    /// Run build command, move the built binary into a spefic location,
    /// then return the path to that location.
    fn build_binary(&self, target: &str, noweb: bool) -> Result<PathBuf> {
        let dest_dir = if noweb {
            dist_dir()?.join(format!("{}-{target}", self.toolkit.full_name()))
        } else {
            dist_dir()?
        };
        ensure_dir(&dest_dir)?;

        let mut cmd = Command::new("cargo");
        cmd.env("HOST_TRIPPLE", target);
        cmd.args(self.build_args(target, noweb));

        let status = cmd.status()?;
        if status.success() {
            // copy and rename the binary with vendor name
            let src = release_dir(target).join(self.source_binary_name());
            let to = dest_dir.join(self.dest_binary_name(noweb));
            copy(src, to)?;
        } else {
            bail!("build failed with code: {}", status.code().unwrap_or(-1));
        }
        Ok(dest_dir)
    }

    fn dist_net_installer(&self, target: &str) -> Result<()> {
        self.build_binary(target, false)?;
        Ok(())
    }

    /// Build binary and copy the vendored packages into a specify location,
    /// then return the path that contains binary and packages.
    fn dist_noweb_installer(&self, target: &str) -> Result<PathBuf> {
        let dist_pkg_dir = self.build_binary(target, true)?;

        // Copy packages to dest dir as well
        let src_pkg_dir = resources_dir()
            .join(PACKAGE_DIR)
            .join(self.toolkit.full_name())
            .join(target);

        // copy the vendored packages to dist folder
        if !src_pkg_dir.exists() {
            bail!(
                "missing vendered packages in '{}', \
            perhaps you forgot to run `cargo dev vendor` first?",
                src_pkg_dir.display()
            );
        }
        copy_as(&src_pkg_dir, &dist_pkg_dir)?;

        Ok(dist_pkg_dir)
    }
}

pub fn dist(
    mode: ReleaseMode,
    binary_only: bool,
    mut targets: Vec<String>,
    name: Option<String>,
) -> Result<()> {
    let name = name.as_deref().unwrap_or("basic");
    let toolkits = Toolkits::load()?;
    let toolkit = toolkits
        .toolkit
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("toolkit '{name}' does not exists in `toolkits.toml`"))?;

    if !matches!(mode, ReleaseMode::Cli) {
        install_gui_deps();
    }

    if targets.is_empty() {
        targets.push(env!("TARGET").into());
    }

    for target in &targets {
        let Some(supported_target) = toolkits
            .config
            .targets
            .iter()
            .find(|t| t.triple() == target)
        else {
            println!("skipping unsupported target '{target}'");
            continue;
        };

        let mut workers = vec![];

        let mode = if let Some(mode_override) = supported_target.release_mode() {
            println!("overriding dist mode to '{mode_override:?}'");
            mode_override
        } else {
            mode
        };

        match mode {
            ReleaseMode::Cli => workers.push(DistWorker::cli(toolkit)),
            ReleaseMode::Gui => workers.push(DistWorker::gui(toolkit)),
            ReleaseMode::Both => {
                workers.push(DistWorker::cli(toolkit));
                workers.push(DistWorker::gui(toolkit));
            }
        }

        let mut offline_dist_dir = None;
        for worker in workers {
            worker.dist_net_installer(target)?;
            if !binary_only {
                offline_dist_dir = Some(worker.dist_noweb_installer(target)?);
            }
        }

        if let Some(dir) = offline_dist_dir {
            include_readme(&dir)?;
            // compress the dir in to tarball or zip.
            // the reason why we compress it here after `dist_noweb_installer` in the previous
            // loop is because there's no need to pack it multiple times for `cli` and `gui`,
            // if the only difference is the installer binary, this could save tons of time.
            compress_offline_package(&dir, &toolkit.full_name(), target)?;
            fs::remove_dir_all(&dir)?;
        }
    }

    Ok(())
}

fn include_readme(dir: &Path) -> Result<()> {
    let readme = include_str!("dist_readme");
    let dest = dir.join("README.md");
    fs::write(dest, readme)?;
    Ok(())
}

fn compress_offline_package(dir: &Path, name: &str, target: &str) -> Result<()> {
    if target.contains("windows") {
        let dest = dist_dir()?.join(format!("{name}-{target}.zip"));
        compress_zip(dir, dest)?;
    } else {
        let dest = dist_dir()?.join(format!("{name}-{target}.tar.xz"));
        compress_xz(dir, dest)?;
    }
    Ok(())
}

/// Path to target release directory
fn release_dir(target: &str) -> PathBuf {
    env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("target"))
        .join(target)
        .join("release")
}

fn dist_dir() -> Result<PathBuf> {
    let res = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("dist");
    ensure_dir(&res)?;
    Ok(res)
}
