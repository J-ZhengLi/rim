use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use anyhow::{anyhow, Context};
use rim_common::{build_config, exe};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Builder};
use url::Url;

use crate::command::with_shared_commands;
use crate::common::{self, cached_manifest, BaseConfiguration, TOOLKIT_MANIFEST};
use crate::error::Result;
use rim::components::Component;
use rim::{get_toolkit_manifest, ToolkitManifestExt};
use rim_common::types::{ToolInfo, ToolSource, ToolkitManifest};
use rim_common::utils;
use tokio::sync::RwLock as AsyncRwLock;

pub(super) fn main(msg_recv: Receiver<String>) -> Result<()> {
    Builder::new()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cmd| {}))
        .invoke_handler(with_shared_commands![
            default_configuration,
            check_install_path,
            get_component_list,
            get_restricted_components,
            updated_package_sources,
            post_installation_opts,
            toolkit_name,
            toolkit_version,
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
    BaseConfiguration::new(rim::default_install_dir(), &*cached_manifest().read().await)
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
        .read()
        .await
        .current_target_components(true)?;
    Ok(components)
}

#[tauri::command]
fn toolkit_name() -> String {
    utils::build_cfg_locale("product").into()
}

/// Load the toolkit and return the version of it.
async fn load_toolkit(path: Option<&Path>) -> Result<Option<String>> {
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
        let read_guard = existing.read().await;
        if read_guard.path.as_deref() != path {
            drop(read_guard); // dropping read guard to avoid dead lock.
            *existing.write().await = load_toolkit_(path).await?;
            debug!("manifest updated");
        }
    } else {
        let manifest = load_toolkit_(path).await?;
        TOOLKIT_MANIFEST.get_or_init(|| AsyncRwLock::new(manifest));
        debug!("manifest initialized");
    }

    Ok(cached_manifest().read().await.version.clone())
}

// Make sure this function is called first after launch.
#[tauri::command]
async fn toolkit_version(path: Option<PathBuf>) -> Result<String> {
    let version = load_toolkit(path.as_deref()).await?.unwrap_or_default();
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
    let mut manifest = cached_manifest().write().await;
    manifest.fill_missing_package_source(&mut selected, |name, _| {
        raw.iter()
            .find(|rc| rc.name == name)
            .and_then(|rc| rc.source.clone())
            .with_context(|| format!("tool '{name}' still have no package source filled yet"))
    })?;
    Ok(selected)
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
