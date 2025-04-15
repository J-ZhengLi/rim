use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::{env, sync::atomic::AtomicBool};

use anyhow::{Context, Result};
use rim_common::types::TomlParser;
use rim_common::{build_config, utils};
use semver::Version;
use tokio::sync::Notify;
use url::Url;

use super::directories::RimDir;
use super::parser::release_info::ReleaseInfo;
use super::{AppInfo, GlobalOpts};
use crate::configuration::{Configuration, UpdateTarget};
use crate::toolkit;

/// Caching the latest manager release info, reduce the number of time accessing the server.
static LATEST_RELEASE: OnceLock<ReleaseInfo> = OnceLock::new();

#[derive(Default)]
pub struct UpdateOpt {
    insecure: bool,
}

impl RimDir for UpdateOpt {
    fn install_dir(&self) -> &Path {
        AppInfo::get_installed_dir()
    }
}

impl UpdateOpt {
    pub fn new() -> Self {
        Self { insecure: false }
    }

    setter!(insecure(self.insecure, bool));

    /// Calls a function to update toolkit.
    ///
    /// This is just a callback wrapper (for now), you still have to provide a function to do the
    /// internal work.
    // TODO: find a way to generalize this, so we can write a shared logic here instead of
    // creating update functions for both CLI and GUI.
    pub fn update_toolkit<F>(&self, callback: F) -> Result<()>
    where
        F: FnOnce(&Path) -> Result<()>,
    {
        let dir = self.install_dir();
        callback(dir).context("unable to update toolkit")
    }

    /// Update self when applicable.
    ///
    /// Latest version check can be disabled by passing `skip_check` as `false`.
    /// Otherwise, this function will check whether if the current version is older
    /// than the latest one, if not, return `Ok(false)` indicates no update has been done.
    pub async fn self_update(&self, skip_check: bool) -> Result<bool> {
        if !skip_check && !check_self_update(self.insecure).await?.update_needed() {
            info!(
                "{}",
                t!(
                    "latest_manager_installed",
                    version = env!("CARGO_PKG_VERSION")
                )
            );
            return Ok(false);
        }

        #[cfg(not(feature = "gui"))]
        let cli = "-cli";
        #[cfg(feature = "gui")]
        let cli = "";

        let id = &build_config().identifier;
        let src_name = exe!(format!("{id}-manager{cli}"));
        let latest_version = &latest_manager_release(self.insecure).await?.version;
        let download_url = parse_download_url(&format!(
            "manager/archive/{latest_version}/{}/{src_name}",
            env!("TARGET"),
        ))?;

        info!(
            "{}",
            t!("downloading_latest_manager", version = latest_version)
        );
        // creates another directory under `temp` folder, it will be used to hold a
        // newer version of the manager binary, which will then replacing the current running one.
        let temp_root = tempfile::Builder::new()
            .prefix("manager-download_")
            .tempdir_in(self.temp_dir())?;
        // dest file don't need the `-cli` suffix to confuse users
        let dest_name = exe!(format!("{}-manager", &build_config().identifier));
        let newer_manager = temp_root.path().join(dest_name);
        utils::DownloadOpt::new("latest manager", GlobalOpts::get().quiet)
            .download(&download_url, &newer_manager)
            .await?;

        // replace the current executable
        self_replace::self_replace(newer_manager)?;

        info!("{}", t!("self_update_complete"));
        Ok(true)
    }
}

/// Try to get the manager's latest release information.
///
/// This will try to access the internet upon first call in order to
/// read the `release.toml` file from the server, and the result will be "cached" after.
async fn latest_manager_release(insecure: bool) -> Result<&'static ReleaseInfo> {
    if let Some(release_info) = LATEST_RELEASE.get() {
        return Ok(release_info);
    }

    let download_url = parse_download_url(&format!("manager/{}", ReleaseInfo::FILENAME))?;
    let raw = utils::DownloadOpt::new("manager release info", GlobalOpts::get().quiet)
        .insecure(insecure)
        .read(&download_url)
        .await?;
    let release_info = ReleaseInfo::from_str(&raw)?;

    Ok(LATEST_RELEASE.get_or_init(|| release_info))
}

#[derive(Debug)]
pub enum UpdateKind<T: Sized> {
    Newer { current: T, latest: T },
    Uncertain,
    UnNeeded,
}

#[derive(Debug)]
pub struct UpdatePayload {
    pub version: String,
    pub url: Option<String>,
}

impl UpdatePayload {
    pub fn new<S: Into<String>>(version: S) -> Self {
        Self {
            version: version.into(),
            url: None,
        }
    }

    setter!(with_payload(self.url, Option<String>));
}

impl<T> UpdateKind<T> {
    pub fn update_needed(&self) -> bool {
        matches!(self, Self::Newer { .. })
    }
}

static UPDATE_CHECKER_PAUSED: AtomicBool = AtomicBool::new(false);
static UPDATE_CHECKER_PAUSE_NOTIFIER: OnceLock<Arc<Notify>> = OnceLock::new();
/// An abstract struct that temporarily blocks the update checker task.
///
/// Since update check is running inside an infinite loop with an async task.
/// In order to pause it, we need a [`Notify`] struct to help blocks the future,
/// and can be resumed per demand.
pub struct UpdateCheckBlocker;

impl UpdateCheckBlocker {
    fn inner_notify() -> &'static Arc<Notify> {
        UPDATE_CHECKER_PAUSE_NOTIFIER.get_or_init(|| Arc::new(Notify::new()))
    }
    pub fn block() {
        UPDATE_CHECKER_PAUSED.store(true, Ordering::Relaxed);
        Self::inner_notify().notify_waiters();
    }
    pub fn unblock() {
        UPDATE_CHECKER_PAUSED.store(false, Ordering::Relaxed);
    }
    pub fn is_blocked() -> bool {
        UPDATE_CHECKER_PAUSED.load(Ordering::Relaxed)
    }
    pub async fn pause_if_blocked() {
        if Self::is_blocked() {
            Self::inner_notify().notified().await;
        }
    }
}

/// Check self(manager) updates.
///
/// This will also read an [`Updates`] configuration to see whether
/// the update should be checked.
///
/// # Error
/// Return `Err` if we can't change the [`last-run`](crate::updates::UpdateConf::last_run)
/// status of updates checker.
pub async fn check_self_update(insecure: bool) -> Result<UpdateKind<Version>> {
    info!("{}", t!("checking_manager_updates"));

    let mut updates_checker = Configuration::load_from_install_dir();
    // we mark it first then check, it sure seems pretty weird, but it sure preventing
    // infinite loop running in a background thread.
    updates_checker.update.mark_checked(UpdateTarget::Manager);
    updates_checker.write_to_install_dir()?;

    let latest_version = match latest_manager_release(insecure).await {
        Ok(release) => release.version.clone(),
        Err(e) => {
            warn!("{}: {e}", t!("fetch_latest_manager_version_failed"));
            return Ok(UpdateKind::Uncertain);
        }
    };
    if updates_checker.update_skipped(UpdateTarget::Manager, latest_version.to_string()) {
        return Ok(UpdateKind::UnNeeded);
    }

    // safe to unwrap, otherwise cargo would fails the build
    let cur_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

    let res = if cur_version < latest_version {
        UpdateKind::Newer {
            current: cur_version,
            latest: latest_version,
        }
    } else {
        UpdateKind::UnNeeded
    };
    Ok(res)
}

/// Check toolkit updates.
///
/// This will also read an [`Updates`] configuration to see whether
/// the update should be checked.
///
/// # Error
/// Return `Err` if we can't change the [`last-run`](crate::updates::UpdateConf::last_run)
/// status of updates checker.
pub async fn check_toolkit_update(insecure: bool) -> Result<UpdateKind<UpdatePayload>> {
    let mut update_checker = Configuration::load_from_install_dir();
    // we mark it first then check, it sure seems pretty weird, but it sure preventing
    // infinite loop running in a background thread.
    update_checker.update.mark_checked(UpdateTarget::Toolkit);
    update_checker.write_to_install_dir()?;

    let mutex = match toolkit::Toolkit::installed(false).await {
        Ok(Some(installed)) => installed,
        Ok(None) => {
            info!("{}", t!("no_toolkit_installed"));
            return Ok(UpdateKind::UnNeeded);
        }
        Err(e) => {
            warn!("{}: {e}", t!("fetch_latest_toolkit_version_failed"));
            return Ok(UpdateKind::Uncertain);
        }
    };
    let installed = &*mutex.lock().await;

    // get possible update
    let latest_toolkit = match toolkit::latest_installable_toolkit(installed, insecure).await {
        Ok(Some(tk)) => tk,
        Ok(None) => {
            info!("{}", t!("no_available_updates", toolkit = &installed.name));
            return Ok(UpdateKind::UnNeeded);
        }
        Err(e) => {
            warn!("{}: {e}", t!("fetch_latest_toolkit_version_failed"));
            return Ok(UpdateKind::Uncertain);
        }
    };

    if update_checker.update_skipped(UpdateTarget::Toolkit, &latest_toolkit.version) {
        return Ok(UpdateKind::UnNeeded);
    }

    let res = UpdateKind::Newer {
        current: UpdatePayload::new(&installed.version),
        latest: UpdatePayload::new(&latest_toolkit.version)
            .with_payload(latest_toolkit.manifest_url.clone()),
    };
    Ok(res)
}

fn parse_download_url(source_path: &str) -> Result<Url> {
    let base_obs_server = super::rim_dist_server();

    debug!("parsing download url for '{source_path}' from server '{base_obs_server}'");
    utils::url_join(&base_obs_server, source_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_comparison() {
        macro_rules! compare {
            ($lhs:literal $op:tt $rhs:literal) => {
                assert!(
                    semver::Version::parse($lhs).unwrap() $op semver::Version::parse($rhs).unwrap()
                );
            };
        }

        compare!("0.1.0" < "0.2.0");
        compare!("0.1.0" < "0.2.0-alpha");
        compare!("0.1.0" > "0.1.0-alpha");
        compare!("0.1.0-alpha" < "0.1.0-beta");
        compare!("0.1.0-alpha" < "0.1.0-alpha.1");
        compare!("0.1.0-alpha.1" < "0.1.0-alpha.2");
        compare!("1.0.0" == "1.0.0");
    }
}
