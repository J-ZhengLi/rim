use std::{
    sync::{mpsc::Receiver, Arc, Mutex, MutexGuard},
    time::Duration,
};

use crate::{common, error::Result};
use crate::{
    common::BaseConfiguration,
    consts::{LOADING_FINISHED, LOADING_TEXT, MANAGER_WINDOW_LABEL, TOOLKIT_UPDATE_EVENT},
};
use anyhow::Context;
use rim::{
    components::Component,
    toolkit::{self, Toolkit},
    update::{self, UpdateCheckBlocker, UpdateOpt},
};
use rim::{get_toolkit_manifest, AppInfo};
use rim_common::types::{
    Configuration, ToolkitManifest, UpdateTarget, DEFAULT_UPDATE_CHECK_DURATION,
};
use rim_common::utils;
use tauri::{async_runtime, AppHandle, Manager};
use url::Url;

static SELECTED_TOOLSET: Mutex<Option<ToolkitManifest>> = Mutex::new(None);

fn selected_toolset<'a>() -> MutexGuard<'a, Option<ToolkitManifest>> {
    SELECTED_TOOLSET
        .lock()
        .expect("unable to lock global mutex")
}

pub(super) fn main(
    msg_recv: Receiver<String>,
    maybe_args: anyhow::Result<Box<rim::cli::Manager>>,
) -> Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cmd| {
            let cli = match rim::cli::Manager::try_from(argv) {
                Ok(a) => a,
                Err(err) => {
                    error!("unable to parse commandline arguments: {err}");
                    return;
                }
            };
            common::handle_manager_args(app.clone(), cli);
        }))
        .invoke_handler(tauri::generate_handler![
            common::close_window,
            get_installed_kit,
            get_available_kits,
            get_install_dir,
            uninstall_toolkit,
            install_toolkit,
            check_updates_in_background,
            get_toolkit_from_url,
            common::supported_languages,
            common::get_locale,
            common::set_locale,
            common::app_info,
            self_update_now,
            toolkit_update_now,
            common::get_build_cfg_locale_str,
        ])
        .setup(|app| {
            common::setup_manager_window(app, msg_recv, maybe_args)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .context("unknown error occurs while running tauri application")?;
    Ok(())
}

#[tauri::command]
async fn get_installed_kit(reload: bool) -> Result<Option<Toolkit>> {
    let Some(mutex) = Toolkit::installed(reload).await? else {
        return Ok(None);
    };
    let installed = mutex.lock().await.clone();
    Ok(Some(installed))
}

#[tauri::command]
async fn get_available_kits(reload: bool) -> Result<Vec<Toolkit>> {
    Ok(toolkit::installable_toolkits(reload, false).await?)
}

#[tauri::command]
fn get_install_dir() -> String {
    AppInfo::get_installed_dir().to_string_lossy().to_string()
}

#[tauri::command(rename_all = "snake_case")]
fn uninstall_toolkit(window: tauri::Window, remove_self: bool) {
    common::uninstall_toolkit_in_new_thread(window, remove_self);
}

#[tauri::command(rename_all = "snake_case")]
fn install_toolkit(window: tauri::Window, components_list: Vec<Component>) -> Result<()> {
    UpdateOpt::new().update_toolkit(|p| {
        let guard = selected_toolset();
        let manifest = guard
            .as_ref()
            .expect("internal error: a toolkit must be selected to install");
        common::install_toolkit_in_new_thread(
            window,
            components_list,
            BaseConfiguration::new(p, manifest),
            manifest.to_owned(),
            true,
        );
        Ok(())
    })?;
    Ok(())
}

/// Check self update and return the timeout duration until the next check.
async fn check_manager_update(_app: &AppHandle) -> Result<Duration> {
    let timeout = match update::check_self_update(false).await {
        Ok(update_kind) => {
            if let update::UpdateKind::Newer {
                current: _,
                latest: _,
            } = update_kind
            {
                // TODO: show update hint
            }
            Configuration::load_from_config_dir()
                .update
                .duration_until_next_run(UpdateTarget::Manager)
        }
        Err(e) => {
            log::error!("manager update check failed: {e}");
            DEFAULT_UPDATE_CHECK_DURATION
        }
    };
    Ok(timeout)
}

/// Check toolkit update and return the timeout duration until the next check.
async fn check_toolkit_update(_app: &AppHandle) -> Result<Duration> {
    let timeout = match update::check_toolkit_update(false).await {
        Ok(update_kind) => {
            if let update::UpdateKind::Newer {
                current: _,
                latest: _,
            } = update_kind
            {
                // TODO: show update hint
            }
            Configuration::load_from_config_dir()
                .update
                .duration_until_next_run(UpdateTarget::Toolkit)
        }
        Err(e) => {
            log::error!("toolkit update check failed: {e}");
            DEFAULT_UPDATE_CHECK_DURATION
        }
    };
    Ok(timeout)
}

#[tauri::command]
async fn check_updates_in_background(app: AppHandle) -> Result<()> {
    let app_arc = Arc::new(app);
    let app_clone = app_arc.clone();

    async_runtime::spawn(async move {
        loop {
            UpdateCheckBlocker::pause_if_blocked().await;

            let timeout_for_manager = check_manager_update(&app_clone).await?;
            let timeout_for_toolkit = check_toolkit_update(&app_clone).await?;

            let timeout = timeout_for_manager.min(timeout_for_toolkit);
            utils::async_sleep(timeout).await;
        }
    })
    .await?
}

/// When the `install` button in a toolkit's card was clicked,
/// the URL of that toolkit was pass to this function, which we can download and
/// deserialized the downloaded toolset-manifest and convert it to an installable toolkit format.
#[tauri::command]
fn get_toolkit_from_url(url: String) -> Result<Toolkit> {
    // the `url` input was converted from `Url`, so it will definitely be convert back without issue,
    // thus the below line should never panic
    let url_ = Url::parse(&url)?;

    // load the manifest for components information
    let manifest = async_runtime::block_on(get_toolkit_manifest(Some(url_), false))?;
    // convert it to toolkit
    let toolkit = Toolkit::try_from(&manifest)?;

    // cache the selected toolset manifest
    let mut guard = selected_toolset();
    *guard = Some(manifest);

    Ok(toolkit)
}

async fn do_self_update(app: &AppHandle) -> Result<()> {
    // block update checker without unblocking (app will restart)
    UpdateCheckBlocker::block();

    let window = app.get_window(MANAGER_WINDOW_LABEL);
    // block UI interaction, and show loading toast
    if let Some(win) = &window {
        win.emit(LOADING_TEXT, t!("self_update_in_progress"))?;
    }

    // do self update, skip version check because it should already
    // been checked using `update::check_self_update`
    if let Err(e) = UpdateOpt::new().self_update(true).await {
        return Err(anyhow::anyhow!("failed when performing self update: {e}").into());
    }

    if let Some(win) = &window {
        // schedule restart with 3 seconds timeout
        win.emit(LOADING_FINISHED, true)?;
        for eta in (1..=3).rev() {
            win.emit(LOADING_TEXT, t!("self_update_finished", eta = eta))?;
            utils::async_sleep(Duration::from_secs(1)).await;
        }
        win.emit(LOADING_TEXT, "")?;
    }

    // restart app
    app.restart();

    Ok(())
}

#[tauri::command]
async fn self_update_now(app: AppHandle) -> Result<()> {
    do_self_update(&app).await
}

#[tauri::command]
fn toolkit_update_now(app: AppHandle, url: String) -> Result<()> {
    // fetch the latest toolkit from the given url, and send it to the frontend.
    // the frontend should know what to do with it.
    let toolkit = get_toolkit_from_url(url)?;
    app.emit_to(MANAGER_WINDOW_LABEL, TOOLKIT_UPDATE_EVENT, toolkit)?;

    Ok(())
}
