use crate::command::with_shared_commands;
use crate::common::TOOLKIT_MANIFEST;
use crate::consts::{MANAGER_UPDATE_NOTICE, MANAGER_WINDOW_LABEL, TOOLKIT_UPDATE_NOTICE};
use crate::progress::GuiProgress;
use crate::{common, error::Result};
use anyhow::Context;
use rim::{get_toolkit_manifest, AppInfo};
use rim::{
    toolkit::{self, Toolkit},
    update::{self, UpdateOpt},
};
use rim_common::types::Configuration;
use std::sync::mpsc::Receiver;
use tauri::{AppHandle, Builder, Manager};
use tokio::sync::RwLock;
use url::Url;

pub(super) fn main(
    msg_recv: Receiver<String>,
    maybe_args: anyhow::Result<Box<rim::cli::Manager>>,
) -> Result<()> {
    Builder::new()
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
        .invoke_handler(with_shared_commands![
            get_installed_kit,
            get_available_kits,
            get_install_dir,
            uninstall_toolkit,
            check_updates_on_startup,
            get_toolkit_from_url,
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
async fn uninstall_toolkit(window: tauri::Window, remove_self: bool) -> Result<()> {
    common::uninstall_toolkit_(window, remove_self).await
}

#[tauri::command]
/// Check self update and show update confirmation dialog if needed.
async fn check_manager_update(app: &AppHandle) -> Result<()> {
    let update_opt = UpdateOpt::new(GuiProgress::new(app.clone()));

    if let update::UpdateKind::Newer { current, latest } = update_opt.check_self_update().await {
        app.emit_to(
            MANAGER_WINDOW_LABEL,
            MANAGER_UPDATE_NOTICE,
            (current, latest),
        )?;
    }
    Ok(())
}

#[tauri::command]
/// Check toolkit update and show update confirmation dialog if needed.
async fn check_toolkit_update(app: &AppHandle) -> Result<()> {
    if let update::UpdateKind::Newer { current, latest } = update::check_toolkit_update(false).await
    {
        app.emit_to(
            MANAGER_WINDOW_LABEL,
            TOOLKIT_UPDATE_NOTICE,
            (current, latest),
        )?;
    }
    Ok(())
}

#[tauri::command]
async fn check_updates_on_startup(app: AppHandle) -> Result<()> {
    let conf = Configuration::load_from_config_dir();

    if conf.update.auto_check_manager_updates {
        check_manager_update(&app).await?;
    }
    if conf.update.auto_check_toolkit_updates {
        check_toolkit_update(&app).await?;
    }

    Ok(())
}

/// When the `install` button in a toolkit's card was clicked,
/// the URL of that toolkit was pass to this function, which we can download and
/// deserialized the downloaded toolset-manifest and convert it to an installable toolkit format.
#[tauri::command]
async fn get_toolkit_from_url(url: String) -> Result<Toolkit> {
    // the `url` input was converted from `Url`, so it will definitely be convert back without issue,
    // thus the below line should never panic
    let url_ = Url::parse(&url)?;

    // load the manifest for components information
    let manifest = get_toolkit_manifest(Some(url_), false).await?;
    // convert it to toolkit
    let toolkit = Toolkit::try_from(&manifest)?;

    // cache the selected toolset manifest
    if let Some(existing) = TOOLKIT_MANIFEST.get() {
        *existing.write().await = manifest;
    } else {
        TOOLKIT_MANIFEST.get_or_init(|| RwLock::new(manifest));
    }

    Ok(toolkit)
}

// async fn do_self_update(app: &AppHandle) -> Result<()> {
//     let window = app.get_window(MANAGER_WINDOW_LABEL);
//     // block UI interaction, and show loading toast
//     if let Some(win) = &window {
//         win.emit(LOADING_TEXT, t!("self_update_in_progress"))?;
//     }

//     // do self update, skip version check because it should already
//     // been checked using `update::check_self_update`
//     if let Err(e) = UpdateOpt::new(GuiProgress::new(app.clone()))
//         .self_update(true)
//         .await
//     {
//         return Err(anyhow::anyhow!("failed when performing self update: {e}").into());
//     }

//     if let Some(win) = &window {
//         // schedule restart with 3 seconds timeout
//         win.emit(LOADING_FINISHED, true)?;
//         for eta in (1..=3).rev() {
//             win.emit(LOADING_TEXT, t!("self_update_finished", eta = eta))?;
//             utils::async_sleep(Duration::from_secs(1)).await;
//         }
//         win.emit(LOADING_TEXT, "")?;
//     }

//     // restart app
//     app.restart();

//     Ok(())
// }
