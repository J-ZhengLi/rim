use std::{
    fmt::Display,
    sync::{mpsc::Receiver, Arc, Mutex, MutexGuard},
    time::Duration,
};

use crate::consts::{LOADING_FINISHED, LOADING_TEXT, MANAGER_WINDOW_LABEL, TOOLKIT_UPDATE_EVENT};
use crate::{
    common::{self, FrontendFunctionPayload},
    error::Result,
    notification::{self, Notification, NotificationAction},
};
use anyhow::Context;
use rim::{
    cli::ExecutableCommand,
    components::Component,
    toolkit::{self, Toolkit},
    update::{self, UpdateCheckBlocker, UpdateOpt},
};
use rim::{
    configuration::{Configuration, UpdateTarget, DEFAULT_UPDATE_CHECK_DURATION},
    get_toolkit_manifest, AppInfo,
};
use rim_common::{types::ToolkitManifest, utils};
use tauri::{
    async_runtime,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow, WindowEvent,
};
use url::Url;

static SELECTED_TOOLSET: Mutex<Option<ToolkitManifest>> = Mutex::new(None);
// If adding more notification windows, make sure their label start with 'notification:'
const MANAGER_UPD_POPUP_LABEL: &str = "notification:manager";
const TOOLKIT_UPD_POPUP_LABEL: &str = "notification:toolkit";

fn selected_toolset<'a>() -> MutexGuard<'a, Option<ToolkitManifest>> {
    SELECTED_TOOLSET
        .lock()
        .expect("unable to lock global mutex")
}

pub(super) fn main(
    msg_recv: Receiver<String>,
    maybe_args: anyhow::Result<rim::cli::Manager>,
) -> Result<()> {
    // store the cli args for future use
    if let Ok(args) = &maybe_args {
        common::update_shared_configs(args);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cmd| {
            let cli = match rim::cli::Manager::try_from(argv) {
                Ok(a) => {
                    if !a.no_gui() {
                        show_manager_window_if_possible(app);
                    }
                    a
                }
                Err(err) => {
                    error!("unable to parse commandline arguments: {err}");
                    return;
                }
            };
            common::update_shared_configs(&cli);
            common::handle_manager_args(app.clone(), cli);
        }))
        .invoke_handler(tauri::generate_handler![
            close_window,
            get_installed_kit,
            get_available_kits,
            get_install_dir,
            uninstall_toolkit,
            install_toolkit,
            check_updates_in_background,
            get_toolkit_from_url,
            common::supported_languages,
            common::set_locale,
            common::app_info,
            common::get_label,
            self_update_now,
            toolkit_update_now,
            skip_version,
            notification::close,
            notification::notification_content,
            common::get_build_cfg_locale_str,
        ])
        .setup(|app| {
            let window = common::setup_manager_window(app, msg_recv, maybe_args)?;
            handle_window_event(window);
            setup_system_tray(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .context("unknown error occurs while running tauri application")?;
    Ok(())
}

// In manager mode, we normally don't want to close the window completely,
// instead we should just "hide" it (unless it fails),
// so that we can later show it after user clicks the tray icon.
//
// Unless this function was called with an exit code, which indicates that
// we should exit the program completely.
#[tauri::command]
fn close_window(window: tauri::WebviewWindow, code: Option<i32>) {
    if let Some(code) = code {
        window.app_handle().exit(code);
        return;
    }
    if let Err(e) = window.hide() {
        log::error!(
            "unable to hide the main window '{MANAGER_WINDOW_LABEL}', \
            forcing it to close instead: {e}"
        );
        common::close_window(window);
    }
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
            p.to_path_buf(),
            manifest.to_owned(),
            true,
        );
        Ok(())
    })?;
    Ok(())
}

/// Check self update and return the timeout duration until the next check.
async fn check_manager_update(app: &AppHandle) -> Result<Duration> {
    let timeout = match update::check_self_update(false).await {
        Ok(update_kind) => {
            if let update::UpdateKind::Newer { current, latest } = update_kind {
                show_update_notification_popup(app, UpdateTarget::Manager, &current, &latest, None)
                    .await?;
            }
            Configuration::load_from_install_dir()
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
async fn check_toolkit_update(app: &AppHandle) -> Result<Duration> {
    let timeout = match update::check_toolkit_update(false).await {
        Ok(update_kind) => {
            if let update::UpdateKind::Newer { current, latest } = update_kind {
                show_update_notification_popup(
                    app,
                    UpdateTarget::Toolkit,
                    current.version,
                    latest.version,
                    latest.url,
                )
                .await?;
            }
            Configuration::load_from_install_dir()
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

    // try show the window first, make sure it does not fails the process,
    // as we can still do self update without a window.
    show_manager_window_if_possible(app);

    let window = app.get_webview_window(MANAGER_WINDOW_LABEL);
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
}

async fn show_update_notification_popup<C: Display, S: Display>(
    app_handle: &AppHandle,
    target: UpdateTarget,
    current_ver: C,
    new_ver: S,
    url: Option<String>,
) -> Result<()> {
    let (title, content, update_cmd, label) = match target {
        UpdateTarget::Manager => (
            t!("self_update_available"),
            t!("ask_self_update", current = current_ver, latest = new_ver),
            FrontendFunctionPayload::new("self_update_now"),
            MANAGER_UPD_POPUP_LABEL,
        ),
        UpdateTarget::Toolkit => (
            t!("toolkit_update_available"),
            t!(
                "ask_toolkit_update",
                current = current_ver,
                latest = new_ver,
            ),
            FrontendFunctionPayload::new("toolkit_update_now")
                .with_args(url.into_iter().map(|p| ("url", p)).collect()),
            TOOLKIT_UPD_POPUP_LABEL,
        ),
    };

    Notification::new(
        title,
        content,
        vec![
            NotificationAction {
                label: t!("update").into(),
                icon: Some("/update-icon.svg".into()),
                command: update_cmd,
            },
            NotificationAction {
                label: t!("skip_version").into(),
                icon: Some("/stop-icon.svg".into()),
                command: FrontendFunctionPayload::new("skip_version").with_args(vec![
                    ("version", new_ver.to_string()),
                    ("target", target.to_string()),
                ]),
            },
            NotificationAction {
                label: t!("close").into(),
                icon: Some("/close-icon.svg".into()),
                command: FrontendFunctionPayload::new("close")
                    .with_args(vec![("label", label.to_string())]),
            },
        ],
    )
    .with_window_label(label)
    .show(app_handle)?;

    Ok(())
}

#[tauri::command]
async fn self_update_now(app: AppHandle) -> Result<()> {
    notification::close_all_notification(app.clone());
    do_self_update(&app).await
}

#[tauri::command]
fn toolkit_update_now(app: AppHandle, url: String) -> Result<()> {
    notification::close_all_notification(app.clone());

    // toolkit update requires the main window
    WindowState::detect(&app)?.show()?;

    // fetch the latest toolkit from the given url, and send it to the frontend.
    // the frontend should know what to do with it.
    let toolkit = get_toolkit_from_url(url)?;
    app.emit_to(MANAGER_WINDOW_LABEL, TOOLKIT_UPDATE_EVENT, toolkit)?;

    Ok(())
}

#[tauri::command]
fn skip_version(app: AppHandle, target: UpdateTarget, version: String) -> Result<()> {
    let label = match target {
        UpdateTarget::Manager => MANAGER_UPD_POPUP_LABEL,
        UpdateTarget::Toolkit => TOOLKIT_UPD_POPUP_LABEL,
    };
    notification::close(app, label.into());

    log::info!("skipping version: '{version}' for '{target}'");
    Configuration::load_from_install_dir()
        .skip_update(target, version)
        .write_to_install_dir()?;
    Ok(())
}

enum WindowState {
    Normal(WebviewWindow),
    Hidden(WebviewWindow),
    Minimized(WebviewWindow),
    Closed,
}

impl WindowState {
    /// Detects the state of main manager window.
    fn detect(app: &AppHandle) -> Result<Self> {
        let Some(win) = app.get_webview_window(MANAGER_WINDOW_LABEL) else {
            return Ok(Self::Closed);
        };
        let state = if win.is_visible()? {
            Self::Normal(win)
        } else if win.is_minimized()? {
            Self::Minimized(win)
        } else {
            Self::Hidden(win)
        };
        Ok(state)
    }

    fn show(&self) -> Result<()> {
        let win = match self {
            Self::Normal(win) => win,
            Self::Closed => {
                // TODO(?): maybe it is possible to revive a dead window, find a way.
                log::error!("Attempt to re-open manager window which has already been shutdown.");
                return Ok(());
            }
            Self::Minimized(win) => {
                win.unminimize()?;
                win
            }
            Self::Hidden(win) => {
                win.show()?;
                win
            }
        };
        win.set_focus()?;
        Ok(())
    }
}

fn setup_system_tray(app: &tauri::App) -> Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "show", t!("show_ui"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", t!("quit"), true, None::<&str>)?,
        ],
    )?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_tray_icon_event(|icon, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_manager_window_if_possible(icon.app_handle())
            }
        })
        .on_menu_event(|handle, event| match event.id.as_ref() {
            "show" => show_manager_window_if_possible(handle),
            "quit" => handle.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn handle_window_event(window: WebviewWindow) {
    window.clone().on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close_window(window.clone(), None);
        }
    });
}

fn show_manager_window_if_possible(app: &AppHandle) {
    let Ok(state) = WindowState::detect(app) else {
        return;
    };
    _ = state.show();
}
