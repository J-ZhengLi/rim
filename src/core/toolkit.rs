use crate::components;
use crate::core::{parser::dist_manifest::DistManifest, GlobalOpts};
use crate::fingerprint::InstallationRecord;
use anyhow::Result;
use rim_common::types::ToolkitManifest;
use rim_common::{types::TomlParser, utils};
use semver::Version;
use serde::Serialize;
use tokio::sync::{Mutex, OnceCell};

use super::parser::dist_manifest::DistPackage;
use super::ToolkitManifestExt;

/// A cached installed [`Toolkit`] struct to prevent the program doing
/// excessive IO operations as in [`installed`](Toolkit::installed).
static INSTALLED_KIT: OnceCell<Mutex<Toolkit>> = OnceCell::const_new();

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toolkit {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
    desc: Option<String>,
    #[serde(alias = "notes")]
    info: Option<String>,
    #[serde(rename = "manifestURL")]
    pub manifest_url: Option<String>,
    pub components: Vec<components::Component>,
}

impl PartialEq for Toolkit {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version
    }
}

impl Toolkit {
    /// Try getting the toolkit from installation record and the original manifest for installed toolset.
    ///
    /// The installed kit will be cached to reduce the number of IO operations.
    /// However, if `reload_cache` is `true`, the cache will be ignored, and will be
    /// updated once installed kit is being reloaded.
    pub async fn installed(reload_cache: bool) -> Result<Option<&'static Mutex<Self>>> {
        if INSTALLED_KIT.get().is_some() && !reload_cache {
            return Ok(INSTALLED_KIT.get());
        }

        if !InstallationRecord::exists() {
            // No toolkit installed, return None
            return Ok(None);
        }

        let fp = InstallationRecord::load_from_config_dir()?;
        let Some(name) = fp.name.clone() else {
            return Ok(None);
        };
        let components = components::all_components_from_installation(&fp)?;

        let tk = Self {
            name,
            version: fp.version.as_deref().unwrap_or("N/A").to_string(),
            edition: fp.edition.clone(),
            desc: None,
            info: None,
            manifest_url: None,
            components,
        };

        if let Some(existing) = INSTALLED_KIT.get() {
            // If we already have a cache, update the inner value of it.
            let mut guard = existing.lock().await;
            *guard = tk;
            drop(guard);
            Ok(Some(existing))
        } else {
            // If we are creating a fresh cache, just return the inner mutex guard.
            let mutex = INSTALLED_KIT.get_or_init(|| async { Mutex::new(tk) }).await;
            Ok(Some(mutex))
        }
    }
}

impl From<DistPackage> for Toolkit {
    fn from(value: DistPackage) -> Self {
        Self {
            name: value.name,
            version: value.version,
            edition: value.edition,
            desc: value.desc,
            info: value.info,
            manifest_url: Some(value.manifest_url.to_string()),
            components: vec![],
        }
    }
}

impl TryFrom<&ToolkitManifest> for Toolkit {
    type Error = anyhow::Error;
    fn try_from(value: &ToolkitManifest) -> Result<Self> {
        Ok(Self {
            name: value
                .name
                .clone()
                .unwrap_or_else(|| "Unknown Toolkit".into()),
            version: value.version.clone().unwrap_or_else(|| "N/A".to_string()),
            edition: value.edition.clone(),
            desc: None,
            info: None,
            manifest_url: None,
            components: value.current_target_components(false)?,
        })
    }
}

/// Download the dist manifest from server to get the list of all provided toolkits.
///
/// The result will be sorted by the version.
///
/// The collection will always be cached to reduce the number of server requests.
// TODO: track how many times this function was called, are all server requests necessary?
// if not, cached them locally.
pub(crate) async fn toolkits_from_server(insecure: bool) -> Result<Vec<Toolkit>> {
    let dist_server = super::rim_dist_server();

    // download dist manifest from server
    let dist_m_filename = DistManifest::FILENAME;
    info!("{} {dist_m_filename}", t!("fetching"));
    let dist_m_url = utils::url_join(&dist_server, format!("dist/{dist_m_filename}"))?;
    let dist_m_file = utils::make_temp_file("dist-manifest-", None)?;
    utils::DownloadOpt::new("distribution manifest", GlobalOpts::get().quiet)
        .insecure(insecure)
        .download(&dist_m_url, dist_m_file.path())
        .await?;
    debug!("distribution manifest file successfully downloaded!");

    // load dist "pacakges" then convert them into `toolkit`s
    let packages = DistManifest::load(dist_m_file.path())?.packages;
    let mut toolkits: Vec<Toolkit> = packages.into_iter().map(Toolkit::from).collect();
    toolkits.sort_by(|a, b| trim_version(&b.version).cmp(trim_version(&a.version)));
    debug!(
        "detected {} available toolkits by accessing server:\n{}",
        toolkits.len(),
        toolkits
            .iter()
            .map(|tk| format!("\t{} ({})", &tk.name, &tk.version))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok(toolkits)
}

/// Return a list of all toolkits that are not currently installed.
pub async fn installable_toolkits(reload_cache: bool, insecure: bool) -> Result<Vec<Toolkit>> {
    info!("{}", t!("checking_toolkit_updates"));

    let all_toolkits = toolkits_from_server(insecure).await?;
    let installable = if let Some(installed) = Toolkit::installed(reload_cache).await? {
        let installed: tokio::sync::MutexGuard<'_, Toolkit> = installed.lock().await;
        // filter the installed toolkit
        let all = all_toolkits.into_iter().filter(|tk| tk != &*installed);
        // partition the list so that the ones with same edition are always on top
        let (mut same, diff): (Vec<_>, Vec<_>) =
            all.partition(|tk| tk.edition == installed.edition);
        same.extend_from_slice(&diff);
        same
    } else {
        all_toolkits
    };
    Ok(installable)
}

/// Get available toolkits from server, then return the latest one if it has
/// not been installed yet.
pub async fn latest_installable_toolkit(
    installed: &Toolkit,
    insecure: bool,
) -> Result<Option<Toolkit>> {
    info!("{}", t!("checking_toolkit_updates"));

    let Some(maybe_latest) = toolkits_from_server(insecure)
        .await?
        .into_iter()
        // make sure they are the same **product**
        .find(|tk| tk.edition == installed.edition)
    else {
        info!("{}", t!("no_available_updates", toolkit = &installed.name));
        return Ok(None);
    };
    let cur_ver = trim_version(&installed.version);
    let target_ver = trim_version(&maybe_latest.version);
    let cur_version: Version = cur_ver.parse()?;
    let target_version: Version = target_ver.parse()?;

    if target_version > cur_version {
        Ok(Some(maybe_latest))
    } else {
        info!(
            "{}",
            t!(
                "latest_toolkit_installed",
                name = installed.name,
                version = cur_version
            )
        );
        Ok(None)
    }
}

// For some reason, the version might contains prefixes such as "stable 1.80.1",
// therefore we need to trim them so that `semver` can be used to parse the actual
// version string.
// NB (J-ZhengLi): We might need another version field... one for display,
// one for the actual version.
fn trim_version(raw: &str) -> &str {
    raw.trim_start_matches(|c| !char::is_ascii_digit(&c))
}
