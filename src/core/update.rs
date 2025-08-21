use std::env;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use rim_common::types::TomlParser;
use rim_common::utils::{HiddenProgress, LinkKind, ProgressHandler};
use rim_common::{build_config, utils};
use semver::Version;
use serde::Serialize;
use url::Url;

use super::directories::RimDir;
use super::parser::release_info::Releases;
use super::{AppInfo, GlobalOpts};
use crate::toolkit;

/// Caching the latest manager release info, reduce the number of time accessing the server.
static LATEST_RELEASE: OnceLock<Releases> = OnceLock::new();

#[derive(Default)]
pub struct UpdateOpt<T> {
    insecure: bool,
    progress_handler: T,
}

impl<T> RimDir for UpdateOpt<T> {
    fn install_dir(&self) -> &Path {
        AppInfo::get_installed_dir()
    }
}

impl<T: ProgressHandler + Clone + 'static> UpdateOpt<T> {
    pub fn new(handler: T) -> Self {
        Self {
            insecure: false,
            progress_handler: handler,
        }
    }

    setter!(insecure(self.insecure, bool));

    /// Update self when applicable.
    ///
    /// Note: After the update, this binary of this application will be scheduled to replace,
    /// so make sure to terminate the application after success ASAP.
    ///
    /// Latest version check can be disabled by passing `skip_check` as `false`.
    /// Otherwise, this function will check whether if the current version is older
    /// than the latest one, if not, return `Ok(false)` indicates no update has been done.
    pub async fn self_update(&self, skip_check: bool) -> Result<bool> {
        if !skip_check && !self.check_self_update().await.update_needed() {
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

        let app_name = &build_config().app_name();
        let src_name = exe!(format!("{app_name}{cli}"));
        let latest_version = self.latest_manager_release().await?.version();
        let download_url = parse_download_url(&format!(
            "manager/archive/{latest_version}/{}/{src_name}",
            env!("TARGET"),
        ))?;

        // creates another directory under `temp` folder, it will be used to hold a
        // newer version of the manager binary, which will then replacing the current running one.
        let temp_root = tempfile::Builder::new()
            .prefix("manager-download_")
            .tempdir_in(self.temp_dir())?;
        // dest file don't need the `-cli` suffix to confuse users
        let dest_name = exe!(app_name);
        let newer_manager = temp_root.path().join(dest_name);
        let opt = if GlobalOpts::get().quiet {
            utils::DownloadOpt::new("latest manager", Box::new(HiddenProgress))
        } else {
            utils::DownloadOpt::new("latest manager", Box::new(self.progress_handler.clone()))
        };
        opt.download(&download_url, &newer_manager).await?;

        // replace the current executable
        self.self_replace_including_links(&newer_manager)?;
        info!("{}", t!("self_update_finished"));
        Ok(true)
    }

    fn self_replace_including_links(&self, replace_by: &Path) -> Result<()> {
        // before self-replacing, we need to check if `self` is a hard-link or not
        let app_name = exe!(build_config().app_name());

        let current_exe = env::current_exe()?;
        let master_bin = self.install_dir().join(&app_name);
        let shortname_link = self.cargo_bin().join(exe!("rim"));
        let fullname_link = self.cargo_bin().join(&app_name);

        // handle links
        match utils::is_link_of(&current_exe, &master_bin)? {
            // User is running one of the symlinks,
            // do nothing since `self_replace` will handle it.
            LinkKind::Symbolic => {}
            // User might be running the actual RIM, then we need to replace
            // the links if they are not symbolic links.
            // (This will also create new links if those are missing)
            LinkKind::Unlinked => {
                if !(shortname_link.is_symlink() && fullname_link.is_symlink()) {
                    // replace both of the proxies to the new one
                    utils::copy_file(replace_by, &shortname_link)?;
                    utils::copy_file(replace_by, &fullname_link)?;
                }
            }
            // User is running a hard-link of the actual RIM (typically on Windows),
            // then we should replace the other link, then the actual RIM before `self_replace`.
            LinkKind::Hard => {
                if current_exe == shortname_link {
                    utils::copy_file(replace_by, &fullname_link)?;
                } else if current_exe == fullname_link {
                    utils::copy_file(replace_by, &shortname_link)?;
                }
                utils::copy_file(replace_by, &master_bin)?;
            }
        }

        self_replace::self_replace(replace_by)?;
        Ok(())
    }

    /// Try to get the manager's latest release information.
    ///
    /// This will try to access the internet upon first call in order to
    /// read the `release.toml` file from the server, and the result will be "cached" after.
    async fn latest_manager_release(&self) -> Result<&'static Releases> {
        if let Some(release_info) = LATEST_RELEASE.get() {
            return Ok(release_info);
        }

        let download_url = parse_download_url(&format!("manager/{}", Releases::FILENAME))?;
        let opt = if GlobalOpts::get().quiet {
            utils::DownloadOpt::new("manager release info", Box::new(HiddenProgress))
        } else {
            utils::DownloadOpt::new(
                "manager release info",
                Box::new(self.progress_handler.clone()),
            )
        };
        let raw = opt.insecure(self.insecure).read(&download_url).await?;
        let release_info = Releases::from_str(&raw)?;

        Ok(LATEST_RELEASE.get_or_init(|| release_info))
    }

    /// Check self(manager) updates.
    pub async fn check_self_update(&self) -> UpdateKind<UpdatePayload> {
        info!("{}", t!("checking_manager_updates"));

        let latest_version = match self
            .latest_manager_release()
            .await
            .map(|release| release.version())
        {
            Ok(version) => version.clone(),
            Err(e) => {
                warn!("{}: {e}", t!("fetch_latest_manager_version_failed"));
                return UpdateKind::Uncertain;
            }
        };

        // safe to unwrap, otherwise cargo would fails the build
        let cur_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

        if cur_version < latest_version {
            UpdateKind::Newer {
                current: UpdatePayload::new(cur_version),
                latest: UpdatePayload::new(latest_version),
            }
        } else {
            UpdateKind::UnNeeded
        }
    }
}

#[derive(Debug)]
pub enum UpdateKind<T: Sized> {
    Newer { current: T, latest: T },
    Uncertain,
    UnNeeded,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatePayload {
    /// The version string of this update.
    pub version: String,
    /// Optional data to pass to the front-end.
    pub data: Option<String>,
}

impl UpdatePayload {
    pub fn new<S: ToString>(version: S) -> Self {
        Self {
            version: version.to_string(),
            data: None,
        }
    }

    setter!(with_data(self.data, Option<String>));
}

impl<T> UpdateKind<T> {
    pub fn update_needed(&self) -> bool {
        matches!(self, Self::Newer { .. })
    }
}

/// Check toolkit updates.
pub async fn check_toolkit_update(insecure: bool) -> UpdateKind<UpdatePayload> {
    info!("{}", t!("checking_toolkit_updates"));

    let mutex = match toolkit::Toolkit::installed(false).await {
        Ok(Some(installed)) => installed,
        Ok(None) => {
            info!("{}", t!("no_toolkit_installed"));
            return UpdateKind::UnNeeded;
        }
        Err(e) => {
            warn!("{}: {e}", t!("fetch_latest_toolkit_version_failed"));
            return UpdateKind::Uncertain;
        }
    };
    let installed = &*mutex.lock().await;

    // get possible update
    let latest_toolkit = match toolkit::latest_installable_toolkit(installed, insecure).await {
        Ok(Some(tk)) => tk,
        Ok(None) => {
            info!("{}", t!("no_available_updates", toolkit = &installed.name));
            return UpdateKind::UnNeeded;
        }
        Err(e) => {
            warn!("{}: {e}", t!("fetch_latest_toolkit_version_failed"));
            return UpdateKind::Uncertain;
        }
    };

    UpdateKind::Newer {
        current: UpdatePayload::new(&installed.version),
        latest: UpdatePayload::new(&latest_toolkit.version)
            .with_data(latest_toolkit.manifest_url.clone()),
    }
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
