use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use rim_common::{build_config, exe};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tokio::sync::Mutex;
use url::Url;

use super::{common, INSTALL_DIR};
use crate::common::BaseConfiguration;
use crate::error::Result;
use rim::components::Component;
use rim::{get_toolkit_manifest, GlobalOpts, ToolkitManifestExt};
use rim_common::types::{ToolInfo, ToolSource, ToolkitManifest};
use rim_common::utils;

static TOOLKIT_MANIFEST: OnceLock<Mutex<ToolkitManifest>> = OnceLock::new();

pub(super) fn main(msg_recv: Receiver<String>) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cmd| {}))
        .invoke_handler(tauri::generate_handler![
            common::close_window,
            default_configuration,
            check_install_path,
            get_component_list,
            get_restricted_components,
            updated_package_sources,
            install_toolchain,
            post_installation_opts,
            toolkit_name,
            toolkit_version,
            common::supported_languages,
            common::set_locale,
            common::get_locale,
            common::app_info,
            get_home_page_url,
            common::get_build_cfg_locale_str,
        ])
        .setup(|app| {
            common::setup_installer_window(app, msg_recv)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .context("unknown error occurs while running tauri application")?;
    Ok(())
}

#[tauri::command]
async fn default_configuration() -> BaseConfiguration {
    let manifest = TOOLKIT_MANIFEST.get().unwrap_or_else(|| {
        unreachable!(
            "toolkit manifest was loaded before reaching config page, \
            which is where this function should only be called."
        )
    });

    // FIXME: fix support of GUI commandline args, then we can override
    // some of this config using commandline args.
    let path = INSTALL_DIR
        .get()
        .cloned()
        .unwrap_or_else(rim::default_install_dir);
    BaseConfiguration::new(path, &*manifest.lock().await)
}

/// Check if the given path could be used for installation, and return the reason if not.
#[tauri::command]
fn check_install_path(path: String) -> Option<String> {
    if path.is_empty() {
        Some(t!("notify_empty_path").to_string())
    } else if Path::new(&path).is_relative() {
        // We won't accept relative path because the result might gets a little bit unpredictable
        Some(t!("notify_relative_path").to_string())
    } else if utils::is_root_dir(path) {
        Some(t!("notify_root_dir").to_string())
    } else {
        None
    }
}

/// Get full list of supported components
#[tauri::command]
async fn get_component_list() -> Result<Vec<Component>> {
    let components = cached_manifest()
        .lock()
        .await
        .current_target_components(true)?;
    Ok(components)
}

#[tauri::command]
fn toolkit_name() -> String {
    utils::build_cfg_locale("product").into()
}

async fn load_toolkit(path: Option<&Path>) -> Result<&'static Mutex<ToolkitManifest>> {
    async fn load_toolkit_(path: Option<&Path>) -> Result<ToolkitManifest> {
        let path_url = path
            .as_ref()
            .map(Url::from_file_path)
            .transpose()
            .map_err(|_| anyhow!("unable to convert path '{path:?}' to URL"))?;
        let mut manifest = get_toolkit_manifest(path_url, false).await?;
        manifest.adjust_paths()?;
        Ok(manifest)
    }

    // There are there scenario of loading a toolkit manifest.
    // 1. The manifest was cached and the path matched, meaning a same one is being loaded:
    //    Return the cached one.
    // 2. The manifest was cached and the path does not match:
    //    Update the cached one.
    // 3. No manifest was cached:
    //    Load one and cache it.
    if let Some(existing) = TOOLKIT_MANIFEST.get() {
        let mut guard = existing.lock().await;
        if guard.path.as_deref() != path {
            println!(
                "cached manifest path: {:?}, loading from: {:?}",
                guard.path.as_deref(),
                path
            );
            *guard = load_toolkit_(path).await?;
            debug!("manifest updated");
        }
        Ok(existing)
    } else {
        let manifest = load_toolkit_(path).await?;
        let mutex = TOOLKIT_MANIFEST.get_or_init(|| Mutex::new(manifest));
        debug!("manifest initialized");
        Ok(mutex)
    }
}

// Make sure this function is called first after launch.
#[tauri::command]
async fn toolkit_version(path: Option<PathBuf>) -> Result<String> {
    let version = load_toolkit(path.as_deref())
        .await?
        .lock()
        .await
        .version
        .clone()
        .unwrap_or_default();
    Ok(version)
}

#[derive(Debug, Serialize, Deserialize)]
struct RestrictedComponent {
    name: String,
    label: String,
    source: Option<String>,
    default: Option<String>,
}

impl TryFrom<(&str, &ToolInfo)> for RestrictedComponent {
    type Error = crate::error::InstallerError;
    fn try_from(value: (&str, &ToolInfo)) -> Result<Self> {
        if let Some(ToolSource::Restricted {
            default, source, ..
        }) = value.1.details().and_then(|d| d.source.as_ref())
        {
            let display_name = value.1.display_name().unwrap_or(value.0);
            return Ok(Self {
                name: display_name.to_string(),
                label: t!("question_package_source", tool = display_name).to_string(),
                source: source.clone(),
                default: default.clone(),
            });
        }
        Err(anyhow!("tool '{}' does not have a restricted source", value.0).into())
    }
}

#[tauri::command]
fn get_restricted_components(components: Vec<Component>) -> Vec<RestrictedComponent> {
    components
        .iter()
        .filter_map(|c| {
            if let Some(info) = &c.tool_installer {
                RestrictedComponent::try_from((c.name.as_str(), info)).ok()
            } else {
                None
            }
        })
        .collect()
}

#[tauri::command]
async fn updated_package_sources(
    raw: Vec<RestrictedComponent>,
    mut selected: Vec<Component>,
) -> Result<Vec<Component>> {
    let mut manifest = cached_manifest().lock().await;
    manifest.fill_missing_package_source(&mut selected, |name, _| {
        raw.iter()
            .find(|rc| rc.name == name)
            .and_then(|rc| rc.source.clone())
            .with_context(|| format!("tool '{name}' still have no package source filled yet"))
    })?;
    Ok(selected)
}

#[tauri::command]
async fn install_toolchain(
    window: tauri::Window,
    components_list: Vec<Component>,
    config: BaseConfiguration,
) {
    GlobalOpts::set(false, false, false, false, !config.add_to_path);
    common::install_toolkit_in_new_thread(
        window,
        components_list,
        config,
        cached_manifest().lock().await.to_owned(),
        false,
    );
}

/// Retrieve cached toolset manifest.
///
/// # Panic
/// Will panic if the manifest is not cached.
fn cached_manifest() -> &'static Mutex<ToolkitManifest> {
    TOOLKIT_MANIFEST
        .get()
        .expect("toolset manifest should be loaded by now")
}

#[tauri::command]
fn post_installation_opts(
    app: AppHandle,
    install_dir: String,
    open: bool,
    shortcut: bool,
) -> Result<()> {
    let install_dir = PathBuf::from(install_dir);
    if shortcut {
        utils::ApplicationShortcut {
            name: utils::build_cfg_locale("app_name"),
            path: install_dir.join(exe!(build_config().app_name())),
            icon: Some(install_dir.join(format!("{}.ico", build_config().app_name()))),
            comment: Some(env!("CARGO_PKG_DESCRIPTION")),
            startup_notify: true,
            startup_wm_class: Some(env!("CARGO_PKG_NAME")),
            categories: &["Development"],
            keywords: &["rust", "rim", "xuanwu"],
            ..Default::default()
        }
        .create()?;
    }

    if open {
        std::env::set_var("MODE", "manager");
        app.restart();
    } else {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
fn get_home_page_url() -> String {
    build_config().home_page_url.as_str().into()
}
