use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rim_common::types::ToolkitManifest;
use rim_common::utils;
use rim_common::utils::HiddenProgress;
use rim_common::utils::ProgressHandler;
use url::Url;

use super::check::RUNNER_TOOLCHAIN_NAME;
use super::components::ToolchainComponent;
use super::default_rustup_dist_server;
use super::directories::RimDir;
use super::install::InstallConfiguration;
use super::uninstall::UninstallConfiguration;
use super::GlobalOpts;
use super::ToolkitManifestExt;
use super::CARGO_HOME;
use super::RUSTUP_DIST_SERVER;
use super::RUSTUP_HOME;

#[cfg(windows)]
pub(crate) const RUSTUP_INIT: &str = "rustup-init.exe";
#[cfg(not(windows))]
pub(crate) const RUSTUP_INIT: &str = "rustup-init";

#[cfg(windows)]
const RUSTUP: &str = "rustup.exe";
#[cfg(not(windows))]
const RUSTUP: &str = "rustup";

pub struct ToolchainInstaller {
    insecure: bool,
    rustup_dist_server: Option<Url>,
}

impl ToolchainInstaller {
    pub(crate) fn init<T: RimDir>(config: T) -> Self {
        let cargo_home = config.cargo_home().to_path_buf();
        let rustup_home = config.rustup_home().to_path_buf();

        std::env::set_var(CARGO_HOME, cargo_home);
        std::env::set_var(RUSTUP_HOME, rustup_home);

        // this env var interfering our installation, may causing incorrect version being installed
        std::env::remove_var("RUSTUP_TOOLCHAIN");
        // skip path check, as it shows an `error: cannot install while Rust is installed`.
        // Although it's not a big deal since we use `-y` when executing `rustup-init`,
        // some user find this error message a bit concerning.
        std::env::set_var("RUSTUP_INIT_SKIP_PATH_CHECK", "yes");

        Self {
            insecure: false,
            rustup_dist_server: None,
        }
    }

    setter!(insecure(self.insecure, bool));
    setter!(rustup_dist_server(self.rustup_dist_server, Option<Url>));

    /// Install toolchain including optional set of components.
    ///
    /// If `first_install` flag was set to `false`, meaning this is likely an
    /// update operation, thus will not try to use offline dist server and
    /// will not try to remove `rustup`'s uninstallation entry on Windows.
    async fn install_toolchain_with_components<T: ProgressHandler + Clone + 'static>(
        &self,
        config: &InstallConfiguration<'_, T>,
        components: &[ToolchainComponent],
        first_install: bool,
    ) -> Result<()> {
        self.ensure_rustup_dist_server_env(config.manifest, first_install)?;

        let rustup = &ensure_rustup(config, self.insecure).await?;
        let components_arg = components
            .iter()
            .filter_map(|c| (!c.is_profile).then_some(c.name.as_str()))
            .collect::<Vec<_>>()
            .join(",");

        let version = &config.manifest.toolchain.channel;
        let mut cmd = cmd!(rustup, "toolchain", "install", version, "--no-self-update",);
        if !components_arg.is_empty() {
            cmd.args(["-c", &components_arg]);
        }
        if let Some(profile) = config.manifest.toolchain.profile() {
            cmd.args(["--profile", profile]);
        }

        // install the toolchain
        utils::execute(cmd)?;
        // set it as default
        run!(rustup, "-q", "default", version)?;

        // Remove the `rustup` uninstall entry on windows, because we don't want users to
        // accidentally uninstall `rustup` thus removing the tools installed by this program.
        #[cfg(windows)]
        if first_install {
            _ = super::os::windows::do_remove_from_programs(
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Rustup",
            );
        }

        Ok(())
    }

    /// Install rust toolchain & components via rustup.
    pub(crate) async fn install<T: ProgressHandler + Clone + 'static>(
        &self,
        config: &InstallConfiguration<'_, T>,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        self.install_toolchain_with_components(config, components, true)
            .await
    }

    /// Update rust toolchain by invoking `rustup toolchain add`, then `rustup default`.
    pub(crate) async fn update<T: ProgressHandler + Clone + 'static>(
        &self,
        config: &InstallConfiguration<'_, T>,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        self.install_toolchain_with_components(config, components, false)
            .await
    }

    /// Install components via rustup.
    pub(crate) async fn add_components<T: ProgressHandler + Clone + 'static>(
        &self,
        config: &InstallConfiguration<'_, T>,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        if components.is_empty() || components.iter().all(|c| c.is_profile) {
            return Ok(());
        }

        self.ensure_rustup_dist_server_env(config.manifest, false)?;
        let rustup = &ensure_rustup(config, self.insecure).await?;

        // check if toolchain is installed
        let version = &config.manifest.toolchain.channel;
        let toolchain_list_cmd = cmd!(rustup, "toolchain", "list");
        let toolchain_list_output = utils::command_output(toolchain_list_cmd)?;
        if toolchain_list_output
            .split('\n')
            .any(|line| line.starts_with(version))
        {
            // if toolchain is installed, add the component directly
            let mut cmd = cmd!(rustup, "component", "add");
            let comp_args = components
                .iter()
                .filter_map(|c| (!c.is_profile).then_some(&c.name));
            info!(
                "{}",
                t!(
                    "install_toolchain_components",
                    list = comp_args
                        .clone()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            );

            cmd.args(comp_args);
            utils::execute(cmd)
        } else {
            // otherwise install the toolchain with the components
            self.install_toolchain_with_components(config, components, false)
                .await
        }
    }

    pub(crate) fn remove_components<T>(
        &self,
        config: &UninstallConfiguration<T>,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        if components.is_empty() || components.iter().all(|c| c.is_profile) {
            return Ok(());
        }

        let rustup_bin = config.cargo_bin().join(RUSTUP);
        if !rustup_bin.is_file() {
            // rustup not installed, perhaps user already remove it manually?
            // Therefore nothing needs to be done
            return Ok(());
        }

        let comp_args = components
            .iter()
            .filter_map(|c| (!c.is_profile).then_some(&c.name));

        info!(
            "{}",
            t!(
                "uninstall_toolchain_components",
                list = comp_args
                    .clone()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        );

        let mut cmd = cmd!(rustup_bin, "component", "remove");
        cmd.args(comp_args);
        utils::execute(cmd)?;
        Ok(())
    }

    /// Uninstall rust toolchain.
    ///
    /// This will try to uninstall everything that `rustup` installed,
    /// meaning that anything else will be kept, such as the third-party tools with-in cargo/bin,
    /// or the links of this current binary.
    ///
    /// Note: We cannot use `rustup self install` anymore because it removes everything in
    /// `cargo/bin` as well, which is not acceptable because it may contains other binaries/link
    /// that we stored that are not part of the rust toolchain.
    pub(crate) fn uninstall<T: ProgressHandler>(
        &self,
        config: &mut UninstallConfiguration<T>,
    ) -> Result<()> {
        config
            .progress_handler
            .set_style(utils::ProgressStyle::Spinner {
                auto_tick_duration: Some(std::time::Duration::from_millis(100)),
            })?;
        config
            .progress_handler
            .start(t!("uninstalling_rust_toolchain").to_string())?;

        // remove rustup home:
        // We want to keep the linked check runner toolchain folder presented,
        // until user choose to remove the ruleset themselves.
        let rustup_dir = config.rustup_home();
        let mut remove_empty_rustup_home = true;
        for entry in utils::walk_dir(rustup_dir, false)? {
            if entry.ends_with("toolchains") {
                // we want to delete all rust toolchain except the one we installed
                // for ruleset check runner
                for sub_entry in utils::walk_dir(&entry, false)? {
                    if sub_entry.ends_with(RUNNER_TOOLCHAIN_NAME) {
                        remove_empty_rustup_home = false;
                        continue;
                    }
                    utils::remove(sub_entry)?;
                }
            } else {
                utils::remove(entry)?;
            }
        }
        if remove_empty_rustup_home {
            utils::remove(rustup_dir)?;
        }

        // remove cargo home:
        // walk cargo home and do some special treatment for the bin dir,
        // while deleting everything else
        let cargo_dir = config.cargo_home();
        let rustup_bin = config.cargo_bin().join(RUSTUP);
        for entry in utils::walk_dir(cargo_dir, false)? {
            if entry.ends_with("bin") {
                // in this bin dir, remove rustup and its proxies only
                // remove proxies first, then remove rustup itself.
                let proxies_to_rm = utils::walk_dir(&entry, false)?
                    .into_iter()
                    .filter(|p| utils::is_link_of(p, &rustup_bin).unwrap_or_default());
                for link in proxies_to_rm {
                    utils::remove(link)?;
                }
                utils::remove(&rustup_bin)?;
                continue;
            }

            utils::remove(entry)?;
        }

        config
            .progress_handler
            .finish(t!("rust_toolchain_uninstalled").to_string())?;
        Ok(())
    }

    fn ensure_rustup_dist_server_env(
        &self,
        manifest: &ToolkitManifest,
        use_offline_server: bool,
    ) -> Result<()> {
        if use_offline_server && manifest.toolchain.offline_dist_server.is_some() {
            let local_server = manifest
                .offline_dist_server()?
                .unwrap_or_else(|| unreachable!("already checked in if condition"));
            info!(
                "{}",
                t!("use_offline_dist_server", url = local_server.as_str())
            );
            std::env::set_var(RUSTUP_DIST_SERVER, local_server.as_str());
        } else {
            let mut server = self
                .rustup_dist_server
                .as_ref()
                .unwrap_or_else(|| default_rustup_dist_server())
                .clone();
            if server.scheme() == "https" && self.insecure {
                warn!("{}", t!("insecure_http_override"));
                // the old scheme is `https` and new scheme is `http`, meaning that this
                // is guaranteed to be `Ok`.
                server.set_scheme("http").unwrap();
            }
            std::env::set_var(RUSTUP_DIST_SERVER, server.as_str());
        }

        Ok(())
    }
}

async fn ensure_rustup<T: ProgressHandler + Clone + 'static>(
    config: &InstallConfiguration<'_, T>,
    insecure: bool,
) -> Result<PathBuf> {
    let rustup_bin = config.cargo_bin().join(RUSTUP);
    if rustup_bin.exists() {
        return Ok(rustup_bin);
    }

    // Run the bundled rustup-init or download it from server.
    // NOTE: When running updates, the manifest we cached might states that it has a bundled
    // rustup-init, but in reality it might not exists, therefore we need to check if that file
    // exists and download it otherwise.
    let (rustup_init, maybe_temp_dir) =
        if let Some(bundled_rustup) = &config.manifest.rustup_bin()?.filter(|p| p.is_file()) {
            (bundled_rustup.to_path_buf(), None)
        } else {
            // We are putting the binary here so that it will be deleted automatically after done.
            let temp_dir = config.create_temp_dir("rustup-init")?;
            let rustup_init = temp_dir.path().join(RUSTUP_INIT);
            // Download rustup-init.
            download_rustup_init(&rustup_init, config, insecure).await?;
            (rustup_init, Some(temp_dir))
        };

    install_rustup(&rustup_init)?;
    // We don't need the rustup-init anymore, drop the whole temp dir containing it.
    drop(maybe_temp_dir);

    Ok(rustup_bin)
}

async fn download_rustup_init<T: ProgressHandler + Clone + 'static>(
    dest: &Path,
    config: &InstallConfiguration<'_, T>,
    insecure: bool,
) -> Result<()> {
    info!("{}", t!("downloading_rustup_init"));

    let download_url = utils::url_join(
        config.rustup_update_root(),
        format!("dist/{}/{RUSTUP_INIT}", env!("TARGET")),
    )
    .context("Failed to init rustup download url.")?;
    let download_opt = if GlobalOpts::get().quiet {
        utils::DownloadOpt::new(RUSTUP_INIT, Box::new(HiddenProgress))
    } else {
        utils::DownloadOpt::new(RUSTUP_INIT, Box::new(config.progress_handler.clone()))
    };

    download_opt
        .insecure(insecure)
        .with_proxy(config.manifest.proxy_config().cloned())
        .download(&download_url, dest)
        .await
        .context("Failed to download rustup.")
}

fn install_rustup(rustup_init: &PathBuf) -> Result<()> {
    // make sure it can be executed
    utils::set_exec_permission(rustup_init)?;

    let mut args = vec![
        // tell rustup not to add `. $HOME/.cargo/env` because we already wrote one for them.
        "--no-modify-path",
        "--default-toolchain",
        "none",
        "--default-host",
        env!("TARGET"),
        "-y",
    ];
    if GlobalOpts::get().verbose {
        args.push("-v");
    } else if GlobalOpts::get().quiet {
        args.push("-q");
    }
    let mut cmd = cmd!(rustup_init);
    cmd.args(args);
    utils::execute(cmd)?;
    Ok(())
}
