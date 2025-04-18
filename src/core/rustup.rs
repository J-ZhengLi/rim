use std::path::Path;
use std::path::PathBuf;
use std::thread;

use anyhow::{Context, Result};
use rim_common::types::Proxy;
use rim_common::types::ToolkitManifest;
use rim_common::utils;
use url::Url;

use super::components::ToolchainComponent;
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
}

impl ToolchainInstaller {
    pub(crate) fn init() -> Self {
        std::env::remove_var("RUSTUP_TOOLCHAIN");
        Self { insecure: false }
    }

    setter!(insecure(self.insecure, bool));

    fn install_toolchain_via_rustup(
        &self,
        rustup: &Path,
        manifest: &ToolkitManifest,
        components: Vec<&str>,
    ) -> Result<()> {
        // TODO: check local manifest.
        let version = &manifest.rust.channel;
        let mut args = vec!["toolchain", "install", version, "--no-self-update"];
        let comps_arg = components.join(",");
        if let Some(profile) = manifest.rust.profile() {
            args.extend(["--profile", profile]);
        }
        if !components.is_empty() {
            args.push("--component");
            args.push(&comps_arg);
        }
        let mut cmd = if let Some(local_server) = manifest.offline_dist_server()? {
            cmd!([RUSTUP_DIST_SERVER=local_server.as_str()] rustup)
        } else if let Ok(dist_server) = std::env::var(RUSTUP_DIST_SERVER) {
            let mut server: Url = dist_server.parse()?;
            if server.scheme() == "https" && self.insecure {
                warn!("{}", t!("insecure_http_override"));
                // the old scheme is `https` and new scheme is `http`, meaning that this
                // is guaranteed to be `Ok`.
                server.set_scheme("http").unwrap();
            }
            cmd!([RUSTUP_DIST_SERVER=server.as_str()] rustup)
        } else {
            cmd!(rustup)
        };
        cmd.args(args);
        utils::execute(cmd)
    }

    /// Install rust toolchain & components via rustup.
    pub(crate) fn install(
        &self,
        config: &InstallConfiguration,
        manifest: &ToolkitManifest,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        let rustup = ensure_rustup(config, manifest, self.insecure)?;

        let extra_comps = components
            .iter()
            .filter_map(|c| (!c.is_profile).then_some(&c.name));
        let all_components = manifest
            .rust
            .components
            .iter()
            .chain(extra_comps)
            .map(|s| s.as_str())
            .collect();
        self.install_toolchain_via_rustup(&rustup, manifest, all_components)?;

        // Remove the `rustup` uninstall entry on windows, because we don't want users to
        // accidentally uninstall `rustup` thus removing the tools installed by this program.
        #[cfg(windows)]
        super::os::windows::do_remove_from_programs(
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Rustup",
        )?;

        Ok(())
    }

    /// Update rust toolchain by invoking `rustup toolchain add`, then `rustup default`
    pub(crate) fn update(
        &self,
        config: &InstallConfiguration,
        manifest: &ToolkitManifest,
    ) -> Result<()> {
        let rustup = ensure_rustup(config, manifest, self.insecure)?;
        let tc_ver = &manifest.rust.channel;
        run!(&rustup, "toolchain", "add", tc_ver, "--no-self-update")
    }

    // Rustup self uninstall all the components and toolchains.
    pub(crate) fn remove_self(&self, config: &UninstallConfiguration) -> Result<()> {
        let progress = utils::CliProgress::new(GlobalOpts::get().quiet);
        let spinner = (progress.start)(
            t!("uninstalling_rust_toolchain").to_string(),
            utils::Style::Spinner {
                auto_tick_duration: None,
            },
        )?;

        let rustup = config.cargo_bin().join(RUSTUP);
        let cargo_home = config.cargo_home().to_path_buf();
        let rustup_home = config.rustup_home().to_path_buf();
        let handle = thread::spawn(
            move || run!([CARGO_HOME=cargo_home, RUSTUP_HOME=rustup_home] rustup, "self", "uninstall", "-y"),
        );
        while !handle.is_finished() {
            (progress.update)(&spinner, None);
        }

        handle.join().unwrap()?;
        (progress.stop)(&spinner, t!("rust_toolchain_uninstalled").to_string());
        Ok(())
    }
}

fn ensure_rustup(
    config: &InstallConfiguration,
    manifest: &ToolkitManifest,
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
        if let Some(bundled_rustup) = &manifest.rustup_bin()?.filter(|p| p.is_file()) {
            (bundled_rustup.to_path_buf(), None)
        } else {
            // We are putting the binary here so that it will be deleted automatically after done.
            let temp_dir = config.create_temp_dir("rustup-init")?;
            let rustup_init = temp_dir.path().join(RUSTUP_INIT);
            // Download rustup-init.
            download_rustup_init(
                &rustup_init,
                &config.rustup_update_root,
                manifest.proxy.as_ref(),
                insecure,
            )?;
            (rustup_init, Some(temp_dir))
        };

    install_rustup(&rustup_init)?;
    // We don't need the rustup-init anymore, drop the whole temp dir containing it.
    drop(maybe_temp_dir);

    Ok(rustup_bin)
}

fn download_rustup_init(
    dest: &Path,
    server: &Url,
    proxy: Option<&Proxy>,
    insecure: bool,
) -> Result<()> {
    info!("{}", t!("downloading_rustup_init"));

    let download_url = utils::url_join(server, format!("dist/{}/{RUSTUP_INIT}", env!("TARGET")))
        .context("Failed to init rustup download url.")?;
    utils::DownloadOpt::new(RUSTUP_INIT, GlobalOpts::get().quiet)
        .insecure(insecure)
        .with_proxy(proxy.cloned())
        .blocking_download(&download_url, dest)
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
    utils::execute(cmd)
}
