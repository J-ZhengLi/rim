use std::sync::OnceLock;

use crate::common;
use crate::consts::{MANAGER_UPDATE_NOTICE, MANAGER_WINDOW_LABEL};
use crate::error::Result;
use crate::progress::GuiProgress;
use rim::components::Component;
use rim::update::{UpdateKind, UpdateOpt};
use rim::{AppInfo, GlobalOpts};
use rim_common::types::{Configuration, ReleaseChannel};
use rim_common::{build_config, utils};
use tauri::{async_runtime, AppHandle, Manager, Window};
use tokio::sync::Mutex;

static SAVED_RIM_CONF: OnceLock<Mutex<Configuration>> = OnceLock::new();

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
            $crate::command::get_rim_configuration,
            $crate::command::set_auto_check_manager_updates,
            $crate::command::set_auto_check_toolkit_updates,
            $crate::command::set_manager_update_channel,
            $crate::command::check_manager_update,
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
    let manifest_guard = common::expected_manifest().read().await;
    let cfg = if let Some(cfg) = config {
        cfg
    } else {
        common::BaseConfiguration::new(AppInfo::get_installed_dir(), Some(&manifest_guard))
    };

    GlobalOpts::set(false, false, false, false, !cfg.add_to_path);
    common::install_toolkit_(window, components_list, cfg, &manifest_guard, false).await?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_rim_configuration() -> Configuration {
    let guard = &*saved_rim_conf().lock().await;
    guard.clone()
}

// saved configuration for update purpose
fn saved_rim_conf() -> &'static Mutex<Configuration> {
    SAVED_RIM_CONF.get_or_init(|| Mutex::new(Configuration::load_from_config_dir()))
}

#[tauri::command]
pub(crate) async fn set_auto_check_manager_updates(yes: bool) -> Result<()> {
    let mut rim_conf = saved_rim_conf().lock().await;
    rim_conf.update.auto_check_manager_updates = yes;
    rim_conf.write()?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn set_auto_check_toolkit_updates(yes: bool) -> Result<()> {
    let mut rim_conf = saved_rim_conf().lock().await;
    rim_conf.update.auto_check_toolkit_updates = yes;
    rim_conf.write()?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn set_manager_update_channel(channel: ReleaseChannel) -> Result<()> {
    let mut rim_conf = saved_rim_conf().lock().await;
    rim_conf.update.manager_update_channel = channel;
    rim_conf.write()?;
    Ok(())
}

#[tauri::command]
/// Check self update and show update confirmation dialog if needed.
pub(crate) async fn check_manager_update(app: AppHandle) -> Result<()> {
    let update_opt = UpdateOpt::new(GuiProgress::new(app.clone()));

    if let UpdateKind::Newer { current, latest } = update_opt.check_self_update().await {
        app.emit_to(
            MANAGER_WINDOW_LABEL,
            MANAGER_UPDATE_NOTICE,
            (current, latest),
        )?;
    }
    Ok(())
}
