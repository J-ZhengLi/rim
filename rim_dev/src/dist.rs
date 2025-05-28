use env::consts::EXE_SUFFIX;
use rim_common::build_config;
use rim_common::utils::{copy_as, copy_file, ensure_dir};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Result};

use crate::common::*;
use crate::toolkits_parser::{ReleaseMode, Toolkit, Toolkits, PACKAGE_DIR};

pub const DIST_HELP: &str = r#"
Generate release binaries

Usage: cargo dev dist [OPTIONS]

Options:
        --cli       Generate release binary for CLI mode only
        --gui       Generate release binary for GUI mode only
        --for       Distribute for a target triple, defaulting to current build target
    -t, --target    Build for the target triple
    -n, --name      Specify another name of toolkit to distribute
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
    /// Target triple for building the installer binaries
    build_target: &'a str,
    /// Target triples to distribute
    dist_target: &'a str,
    edition: &'a str,
}

impl<'a> DistWorker<'a> {
    fn new_(
        toolkit: &'a Toolkit,
        build_target: &'a str,
        is_cli: bool,
        edition: &'a str,
        dist_target: &'a str,
    ) -> Self {
        Self {
            toolkit,
            build_target,
            is_cli,
            edition,
            dist_target,
        }
    }

    fn cli(
        toolkit: &'a Toolkit,
        edition: &'a str,
        build_target: &'a str,
        dist_target: &'a str,
    ) -> Self {
        Self::new_(toolkit, build_target, true, edition, dist_target)
    }

    fn gui(
        toolkit: &'a Toolkit,
        edition: &'a str,
        build_target: &'a str,
        dist_target: &'a str,
    ) -> Self {
        Self::new_(toolkit, build_target, false, edition, dist_target)
    }

    /// The compiled binary name
    fn source_binary_name(&self) -> String {
        if self.is_cli {
            format!("rim-cli{EXE_SUFFIX}")
        } else {
            format!("rim-gui{EXE_SUFFIX}")
        }
    }

    fn release_name(&self) -> String {
        format!(
            "{}-{}-{}",
            &build_config().identifier,
            self.toolkit
                .version()
                .unwrap_or(self.toolkit.rust_version()),
            self.dist_target
        )
        .replace(' ', "-")
    }

    /// The binary name that user see.
    ///
    /// `simple` - the simple version of binary name, just `installer`.
    fn dest_binary_name(&self, simple: bool, is_cli: bool) -> String {
        format!(
            "{}installer{}{EXE_SUFFIX}",
            (!simple)
                .then_some(format!("{}-", self.release_name()))
                .unwrap_or_default(),
            is_cli.then_some("-cli").unwrap_or_default(),
        )
    }

    fn command(&self, noweb: bool) -> Command {
        if self.is_cli {
            let mut cmd = Command::new("cargo");
            cmd.args([
                "build",
                "--release",
                "--locked",
                "--target",
                self.build_target,
            ]);
            if noweb {
                cmd.args(["--features", "no-web"]);
            }
            cmd
        } else {
            let mut cmd = pnpm_cmd();
            cmd.args(["run", "tauri", "build", "--target", self.build_target]);
            if noweb {
                cmd.args(["--features", "no-web"]);
            }
            cmd.args(["--", "--locked"]);
            cmd
        }
    }

    /// Run build command, move the built binary into a specific location,
    /// then return the path to that location.
    fn build_binary(&self, noweb: bool) -> Result<PathBuf> {
        let mut dest_dir = dist_dir(self.dist_target)?;
        if noweb {
            dest_dir.push(self.release_name());
            ensure_dir(&dest_dir)?;

            // when packing the offline build, is better to include both CLI and GUI binary,
            // so, here we check if there's an alternative binary and copy them into the folder
            // if exists.
            let alt_bin_name = self.dest_binary_name(noweb, !self.is_cli);
            let possible_alt_bin = dist_dir(self.dist_target)?.join(&alt_bin_name);
            if possible_alt_bin.is_file() {
                copy_file(&possible_alt_bin, dest_dir.join(&alt_bin_name))?;
            }
        }

        let mut cmd = self.command(noweb);
        cmd.env("HOST_TRIPLE", self.dist_target);
        cmd.env("EDITION", self.edition);

        let status = cmd.status()?;
        if status.success() {
            // when not using cross compilation, we are not running `cargo build` with
            // `--target` option, therefore the release dir's path will not have a target in it.
            let src = release_dir(self.build_target).join(self.source_binary_name());
            // copy and rename the binary with vendor name
            let to = dest_dir.join(self.dest_binary_name(noweb, self.is_cli));
            copy_file(src, to)?;
        } else {
            bail!("build failed with code: {}", status.code().unwrap_or(-1));
        }
        Ok(dest_dir)
    }

    fn dist_net_installer(&self) -> Result<()> {
        self.build_binary(false)?;
        Ok(())
    }

    /// Build binary and copy the vendored packages into a specify location,
    /// then return the path that contains binary and packages.
    fn dist_noweb_installer(&self) -> Result<PathBuf> {
        let dist_pkg_dir = self.build_binary(true)?;

        // Copy packages to dest dir as well
        let src_pkg_dir = resources_dir()
            .join(PACKAGE_DIR)
            .join(self.toolkit.full_name())
            .join(self.dist_target);

        // copy the vendored packages to dist folder
        if !src_pkg_dir.exists() {
            bail!(
                "missing vendored packages in '{}', \
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
    name: Option<String>,
    build_target: String,
    mut dist_targets: Vec<String>,
) -> Result<()> {
    let edition = name.as_deref().unwrap_or(env!("EDITION"));
    let toolkits = Toolkits::load()?;
    let toolkit = toolkits
        .toolkit
        .get(edition)
        .ok_or_else(|| anyhow::anyhow!("toolkit '{edition}' does not exists in `toolkits.toml`"))?;

    if !matches!(mode, ReleaseMode::Cli) {
        install_gui_deps();
    }

    if dist_targets.is_empty() {
        dist_targets.push(build_target.clone());
    }

    for dist_target in &dist_targets {
        let Some(supported_target) = toolkits
            .config
            .targets
            .iter()
            .find(|t| t.triple() == dist_target)
        else {
            println!("skipping unsupported target '{dist_target}'");
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
            ReleaseMode::Cli => workers.push(DistWorker::cli(
                toolkit,
                edition,
                &build_target,
                dist_target,
            )),
            ReleaseMode::Gui => workers.push(DistWorker::gui(
                toolkit,
                edition,
                &build_target,
                dist_target,
            )),
            ReleaseMode::Both => {
                workers.push(DistWorker::cli(
                    toolkit,
                    edition,
                    &build_target,
                    dist_target,
                ));
                workers.push(DistWorker::gui(
                    toolkit,
                    edition,
                    &build_target,
                    dist_target,
                ));
            }
        }

        let mut offline_dist_dir = None;
        for worker in workers {
            worker.dist_net_installer()?;
            if !binary_only {
                offline_dist_dir = Some(worker.dist_noweb_installer()?);
            }
        }

        if let Some(dir) = offline_dist_dir {
            include_readme(&dir)?;
            // compress the dir in to tarball or zip.
            // the reason why we compress it here after `dist_noweb_installer` in the previous
            // loop is because there's no need to pack it multiple times for `cli` and `gui`,
            // if the only difference is the installer binary, this could save tons of time.
            compress_offline_package(&dir, dist_target)?;
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

fn compress_offline_package(dir: &Path, target: &str) -> Result<()> {
    let filename = dir.file_name().and_then(|n| n.to_str()).with_context(|| {
        format!(
            "directory to compress does not have valid name: {}",
            dir.display()
        )
    })?;

    if target.contains("windows") {
        let dest = dist_dir(target)?.join(format!("{filename}.zip"));
        compress_zip(dir, dest)?;
    } else {
        let dest = dist_dir(target)?.join(format!("{filename}.tar.xz"));
        compress_xz(dir, dest)?;
    }
    Ok(())
}

/// Path to target release directory
fn release_dir(target: &str) -> PathBuf {
    let mut res = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("target"));
    res.push(target);
    res.push("release");
    res
}

/// Path to the directory to store dist artifacts for given target
fn dist_dir(target: &str) -> Result<PathBuf> {
    let res = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .with_file_name("dist")
        .join(target);
    ensure_dir(&res)?;
    Ok(res)
}
