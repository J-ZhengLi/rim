use crate::common;
use crate::error::Result;
use rim::components::Component;
use rim::{AppInfo, GlobalOpts};
use rim_common::{build_config, utils};
use tauri::{async_runtime, Window};

/// Generate tauri command handler with shared commands.
macro_rules! with_shared_commands {
    ($($additional_cmd:expr),* $(,)?) => {
        tauri::generate_handler![
            $crate::command::close_window,
            $crate::command::set_locale,
            $crate::command::get_locale,
            $crate::command::app_info,
            $crate::command::get_home_page_url,
            $crate::command::get_build_cfg_locale_str,
            $crate::command::install_toolkit,
            $($additional_cmd),*
        ]
    };
}
pub(crate) use with_shared_commands;

/// Close the given window in a separated thread.
///
/// [`Window::close`] may panic on main thread, thus spawning an async task
/// to close it separatedly to avoid panics.
#[tauri::command]
pub(crate) async fn close_window(win: Window) {
    let label = win.label().to_owned();
    async_runtime::spawn(async move { win.close() })
        .await
        .unwrap_or_else(|_| panic!("thread join failed when attempt to close window '{label}'"))
        .unwrap_or_else(|e| log::error!("failed when closing window '{label}': {e}"))
}

/// Setting current locale to the desired value.
#[tauri::command]
pub(crate) fn set_locale(language: String) -> Result<()> {
    utils::set_locale(language.parse()?);
    Ok(())
}

/// Getting the current locale string.
#[tauri::command]
pub(crate) fn get_locale() -> String {
    utils::get_locale().locale_str().into()
}

#[tauri::command]
pub(crate) fn app_info() -> AppInfo {
    AppInfo::get().to_owned()
}

/// Return the URL for official website, used in the `about` page.
#[tauri::command]
pub(crate) fn get_home_page_url() -> String {
    build_config().home_page_url.as_str().into()
}

#[tauri::command]
pub(crate) fn get_build_cfg_locale_str(key: &str) -> &str {
    utils::build_cfg_locale(key)
}

#[tauri::command]
pub(crate) async fn install_toolkit(
    window: tauri::Window,
    components_list: Vec<Component>,
    config: Option<common::BaseConfiguration>,
) -> Result<()> {
    let manifest_guard = common::cached_manifest().read().await;
    let cfg = if let Some(cfg) = config {
        cfg
    } else {
        common::BaseConfiguration::new(AppInfo::get_installed_dir(), &manifest_guard)
    };

    GlobalOpts::set(false, false, false, false, !cfg.add_to_path);
    common::install_toolkit_(window, components_list, cfg, manifest_guard.clone(), false).await?;
    Ok(())
}
