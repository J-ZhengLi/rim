//! Module to create a fake installation root, useful to test the `manager` utilities.

use crate::common::{self, pnpm_cmd};

use super::TOOLKIT_NAME;
use anyhow::{bail, Result};
use std::{env::consts::EXE_SUFFIX, fs, path::PathBuf, process::Command};

struct FakeInstallation {
    manager_bin: Option<PathBuf>,
}

impl FakeInstallation {
    fn new() -> Self {
        Self { manager_bin: None }
    }

    fn fingerprint_content(&self, ver: &str) -> String {
        format!(
            "
name = '{TOOLKIT_NAME}'
version = 'stable-{ver}'
root = '{0}'

[rust]
version = '{ver}'
components = [\"llvm-tools\", \"rustc-dev\"]

[tools.mingw64]
kind = 'dir-with-bin'
version = '13.0.0'
paths = ['{0}/tools/mingw64']
",
            super::install_dir().display()
        )
    }

    fn generate_manager_bin(&mut self, no_gui: bool) -> Result<()> {
        let (mut cmd, src_bin, dest_bin) = if no_gui {
            // use cargo build
            let mut cmd = Command::new("cargo");
            cmd.arg("build");

            (
                cmd,
                format!("rim-cli{EXE_SUFFIX}"),
                format!("manager-cli{EXE_SUFFIX}"),
            )
        } else {
            common::install_gui_deps();

            // use tauri-cli under rim_gui dir
            let mut cmd = pnpm_cmd();
            cmd.args(["run", "tauri", "build", "-d", "--no-bundle"]);

            (
                cmd,
                format!("rim-gui{EXE_SUFFIX}"),
                format!("manager{EXE_SUFFIX}"),
            )
        };

        // build rim
        let build_status = cmd.status()?;
        if !build_status.success() {
            bail!("failed to build manager binary");
        }

        let build_artifact = super::debug_dir().join(src_bin);
        let dest_path = super::install_dir().join(dest_bin);
        // make a copy of rim as manager binary to the fake installation root
        fs::copy(build_artifact, &dest_path)?;

        self.manager_bin = Some(dest_path);

        Ok(())
    }

    fn generate_meta_files(&self) -> Result<()> {
        let fingerprint_path = super::install_dir().join(".fingerprint.toml");
        let manifest_path = super::install_dir().join("toolset-manifest.toml");

        // don't write if the path already exists
        if !fingerprint_path.exists() {
            fs::write(fingerprint_path, self.fingerprint_content("1.0.0"))?;
        }
        if !manifest_path.exists() {
            let manifest =
                include_str!("../../../resources/toolkit-manifest/online/community.toml");
            fs::write(manifest_path, manifest)?;
        }

        Ok(())
    }
}

pub(crate) fn generate_and_run_manager(no_gui: bool, args: &[String]) -> Result<()> {
    let mut fake = FakeInstallation::new();
    fake.generate_meta_files()?;
    fake.generate_manager_bin(no_gui)?;

    // `fake.manager_bin` cannot be none if the previous `generate_manager_bin`
    // succeeded, so it's safe to unwrap
    let manager = &fake.manager_bin.unwrap();

    let mocked_dist_server = common::path_to_url(super::rim_server_dir());
    let mocked_rustup_server_path = super::rustup_server_dir();
    let mut command = Command::new(manager);

    if !no_gui {
        command.args([
            "--rustup-dist-server",
            common::path_to_url(mocked_rustup_server_path).as_str(),
        ]);
    }

    // run the manager copy
    let status = command
        .args(args)
        .env("RIM_DIST_SERVER", mocked_dist_server.as_str())
        .status()?;
    println!(
        "\nmanager exited with status code: {}",
        status.code().unwrap_or(-1)
    );
    Ok(())
}
