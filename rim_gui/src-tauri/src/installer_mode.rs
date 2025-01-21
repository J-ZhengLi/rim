use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Context;
use tauri::api::dialog::FileDialogBuilder;
use tauri::Manager;

use super::{common, INSTALL_DIR};
use crate::error::Result;
use rim::components::Component;
use rim::toolset_manifest::{get_toolset_manifest, ToolsetManifest};
use rim::{try_it, utils, AppInfo};

static TOOLSET_MANIFEST: OnceLock<ToolsetManifest> = OnceLock::new();
const INSTALLER_WINDOW_LABEL: &str = "installer_window";

pub(super) fn main() -> Result<()> {
    let msg_recv = common::setup_logger()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cmd| {
            _ = app.emit_all(
                "single-instance",
                common::SingleInstancePayload { argv, cmd },
            );
        }))
        .invoke_handler(tauri::generate_handler![
            close_window,
            default_install_dir,
            select_folder,
            check_install_path,
            get_component_list,
            install_toolchain,
            run_app,
            welcome_label,
            load_manifest_and_ret_version,
            common::supported_languages,
            common::set_locale,
            common::app_info,
        ])
        .setup(|app| {
            let window = tauri::WindowBuilder::new(
                app,
                INSTALLER_WINDOW_LABEL,
                tauri::WindowUrl::App("index.html/#/installer".into()),
            )
            .inner_size(800.0, 600.0)
            .min_inner_size(640.0, 480.0)
            .decorations(false)
            .transparent(true)
            .title(AppInfo::name())
            .build()?;

            common::set_window_shadow(&window);
            common::spawn_gui_update_thread(window, msg_recv);

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

#[tauri::command]
fn select_folder(window: tauri::Window) {
    FileDialogBuilder::new().pick_folder(move |path| {
        // 处理用户选择的路径
        let folder = path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        // 通过窗口发送事件给前端
        window.emit("folder-selected", folder).unwrap();
    });
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
fn get_component_list() -> Result<Vec<Component>> {
    Ok(cached_manifest().current_target_components(true)?)
}

#[tauri::command]
fn welcome_label() -> String {
    t!("welcome", product = t!("product")).into()
}

// Make sure this function is called first after launch.
#[tauri::command]
async fn load_manifest_and_ret_version() -> Result<String> {
    // TODO: Give an option for user to specify another manifest.
    // note that passing command args currently does not work due to `windows_subsystem = "windows"` attr
    let mut manifest = get_toolset_manifest(None, false).await?;
    manifest.adjust_paths()?;

    let m = TOOLSET_MANIFEST.get_or_init(|| manifest);
    Ok(m.version.clone().unwrap_or_default())
}

#[tauri::command(rename_all = "snake_case")]
fn install_toolchain(window: tauri::Window, components_list: Vec<Component>, install_dir: String) {
    let install_dir = PathBuf::from(install_dir);
    common::install_toolkit_in_new_thread(
        window,
        components_list,
        install_dir,
        cached_manifest().to_owned(),
        false,
    );
}

/// Retrieve cached toolset manifest.
///
/// # Panic
/// Will panic if the manifest is not cached.
fn cached_manifest() -> &'static ToolsetManifest {
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
