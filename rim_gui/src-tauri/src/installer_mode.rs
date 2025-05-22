use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::OnceLock;

use anyhow::{anyhow, Context};
use rim_common::build_config;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::{common, INSTALL_DIR};
use crate::error::Result;
use rim::components::Component;
use rim::{get_toolkit_manifest, try_it, ToolkitManifestExt};
use rim_common::types::{ToolInfo, ToolSource, ToolkitManifest};
use rim_common::utils;

static TOOLSET_MANIFEST: OnceLock<Mutex<ToolkitManifest>> = OnceLock::new();

pub(super) fn main(
    msg_recv: Receiver<String>,
    maybe_args: anyhow::Result<Box<rim::cli::Installer>>,
) -> Result<()> {
    if let Ok(args) = &maybe_args {
        common::update_shared_configs(args.as_ref());
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cmd| {}))
        .invoke_handler(tauri::generate_handler![
            close_window,
            default_install_dir,
            check_install_path,
            get_component_list,
            get_restricted_components,
            updated_package_sources,
            install_toolchain,
            run_app,
            welcome_label,
            load_manifest_and_ret_version,
            common::supported_languages,
            common::set_locale,
            common::app_info,
            common::get_label,
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
fn close_window(window: tauri::Window) {
    common::close_window(window);
}

#[tauri::command]
fn default_install_dir() -> String {
    INSTALL_DIR
        .get()
        .cloned()
        .unwrap_or_else(rim::default_install_dir)
        .to_string_lossy()
        .to_string()
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
fn welcome_label() -> String {
    let product = utils::build_cfg_locale("product");
    t!("welcome", product = product).into()
}

// Make sure this function is called first after launch.
#[tauri::command]
async fn load_manifest_and_ret_version() -> Result<String> {
    // TODO: Give an option for user to specify another manifest.
    // note that passing command args currently does not work due to `windows_subsystem = "windows"` attr
    let mut manifest = get_toolkit_manifest(None, false).await?;
    manifest.adjust_paths()?;
    let version = manifest.version.clone().unwrap_or_default();

    if TOOLSET_MANIFEST.set(Mutex::new(manifest)).is_err() {
        error!(
            "unable to set initialize manifest to desired one \
            as it was already initialized somewhere else, \
            returning the cached version instead"
        );
        Ok(cached_manifest()
            .lock()
            .await
            .version
            .clone()
            .unwrap_or_default())
    } else {
        Ok(version)
    }
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
    manifest.fill_missing_package_source(&mut selected, |name| {
        raw.iter()
            .find(|rc| rc.name == name)
            .and_then(|rc| rc.source.clone())
            .with_context(|| format!("tool '{name}' still have no package source filled yet"))
    })?;
    Ok(selected)
}

#[tauri::command(rename_all = "snake_case")]
async fn install_toolchain(
    window: tauri::Window,
    components_list: Vec<Component>,
    install_dir: String,
) {
    let install_dir = PathBuf::from(install_dir);
    common::install_toolkit_in_new_thread(
        window,
        components_list,
        install_dir,
        cached_manifest().lock().await.to_owned(),
        false,
    );
}

/// Retrieve cached toolset manifest.
///
/// # Panic
/// Will panic if the manifest is not cached.
fn cached_manifest() -> &'static Mutex<ToolkitManifest> {
    TOOLSET_MANIFEST
        .get()
        .expect("toolset manifest should be loaded by now")
}

#[tauri::command(rename_all = "snake_case")]
fn run_app(install_dir: String) -> Result<()> {
    let dir: PathBuf = install_dir.into();
    try_it(Some(&dir))?;
    Ok(())
}

#[tauri::command]
fn get_home_page_url() -> String {
    build_config().home_page_url.as_str().into()
}
