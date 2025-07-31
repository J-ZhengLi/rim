//! Module to create a fake installation root, useful to test the `manager` utilities.

use crate::common::{self, pnpm_cmd};

use super::TOOLKIT_NAME;
use anyhow::Result;
use rim_common::{build_config, exe};
use std::{fs, process::Command};

struct FakeInstallation;

impl FakeInstallation {
    fn new() -> Self {
        Self
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

    /// Fake a `rim` binary and place it into the fake installation directory,
    /// which is required to "trick" the sanity check of `rim`'s manager mode.
    fn generate_manager_bin(&self) -> Result<()> {
        let dest_bin = exe!(build_config().app_name());
        let dest_path = super::install_dir().join(dest_bin);

        // create a blank file to fake the installation binary since we only
        // need it to trick manager mode's `get_installed_dir`.
        fs::write(dest_path, "")?;

        Ok(())
    }

    fn command(&self, no_gui: bool) -> Command {
        if no_gui {
            // use cargo build
            let mut cmd = Command::new("cargo");
            cmd.env("MODE", "manager").arg("run");

            cmd
        } else {
            common::install_gui_deps();

            // use tauri-cli under rim_gui dir
            let mut cmd = pnpm_cmd();
            cmd.env("MODE", "manager")
                .args(["run", "tauri", "dev", "--"]);

            cmd
        }
    }

    fn generate_meta_files(&self) -> Result<()> {
        let fingerprint_path = rim_common::dirs::rim_config_dir().join("install-record.toml");
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
    let fake = FakeInstallation::new();
    fake.generate_meta_files()?;
    fake.generate_manager_bin()?;

    let mocked_dist_server = common::path_to_url(super::rim_server_dir());
    let mut command = fake.command(no_gui);

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
