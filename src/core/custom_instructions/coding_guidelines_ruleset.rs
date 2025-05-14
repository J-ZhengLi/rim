use std::path::{Path, PathBuf};
use anyhow::{bail, Result};
use rim_common::utils;
use crate::{core::{check::RUNNER_TOOLCHAIN_NAME, directories::RimDir}, InstallConfiguration};

// In this preview version, we are using a toolchain with customized clippy to
// provide linting base on coding guidelines, in the future, we'll change to
// using `cargo-dylint`, so the `install` and `uninstall` functions need to
// change respectively.

pub(super) fn install(path: &Path, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
    if !path.is_dir() {
        bail!("incorrect rule set package format, it should be an existing directory, got: {}", path.display());
    }

    // we're basically installing a separated toolchain contains
    // our customized clippy and "hides" it.
    // Step 1: Make a `ruleset` dir under `tools`
    let ruleset_dir = config.tools_dir().join("ruleset");
    utils::ensure_dir(&ruleset_dir)?;

    // Step 2: Copy the folder `path` as `ruleset/runner`
    // (Because we are using clippy, which is in the runner toolchain, therefore
    // we don't need additional rule set files. If we use dylint, make sure to
    // create another folder called `lints` to store custom lints)
    let runner_dir = ruleset_dir.join("runner");
    utils::copy_as(path, &runner_dir)?;

    Ok(vec![runner_dir])
}

pub(super) fn post_install(paths: &[PathBuf], _config: &InstallConfiguration) -> Result<()> {
    let Some(runner_path) = paths.first() else {
        bail!("missing a path to the runner toolchain");
    };

    // use rustup to link the runner toolchain
    run!(exe!("rustup"), "toolchain", "link", RUNNER_TOOLCHAIN_NAME, runner_path)?;
    Ok(())
}

pub(super) fn uninstall<T: RimDir>(config: T) -> Result<()> {
    // remove the `ruleset` dir
    let ruleset_dir = config.tools_dir().join("ruleset");
    utils::remove(ruleset_dir)?;

    Ok(())
}

pub(super) fn is_installed() -> bool {
    // there's no need to check if the rule set is installed or not,
    // assuming not installed.
    false
}
