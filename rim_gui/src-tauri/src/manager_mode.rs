use std::{
    fmt::Display,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use crate::{
    common::{
        self, BLOCK_EXIT_EVENT, LOADING_FINISHED, LOADING_TEXT, ON_COMPLETE_EVENT,
        PROGRESS_UPDATE_EVENT,
    },
    error::Result,
    notification::{self, Notification, NotificationAction},
};
use anyhow::Context;
use rim::{
    components::Component,
    toolkit::{self, Toolkit},
    toolset_manifest::{get_toolset_manifest, ToolsetManifest},
    update::{self, UpdateOpt},
    utils::{self, Progress},
    AppInfo,
};
use rim::{
    updates::{UpdateCheckerOpt, UpdateTarget, DEFAULT_UPDATE_CHECK_DURATION},
    UninstallConfiguration,
};
use tauri::{
    async_runtime, AppHandle, CustomMenuItem, GlobalWindowEvent, Manager, SystemTray,
    SystemTrayEvent, SystemTrayMenu, Window, WindowEvent,
};

static SELECTED_TOOLSET: Mutex<Option<ToolsetManifest>> = Mutex::new(None);
const MANAGER_WINDOW_LABEL: &str = "manager_window";

fn selected_toolset<'a>() -> MutexGuard<'a, Option<ToolsetManifest>> {
    SELECTED_TOOLSET
        .lock()
        .expect("unable to lock global mutex")
}

pub(super) fn main() -> Result<()> {
    let msg_recv = common::setup_logger()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .system_tray(system_tray())
        .on_system_tray_event(system_tray_event_handler)
        .on_window_event(window_event_handler)
        .invoke_handler(tauri::generate_handler![
            close_window,
            get_installed_kit,
            get_available_kits,
            get_install_dir,
            uninstall_toolkit,
            install_toolkit,
            check_updates_in_background,
            handle_toolkit_install_click,
            common::supported_languages,
            common::set_locale,
            common::app_info,
            self_update_now,
            toolkit_update_now,
            skip_version,
            notification::close,
            notification::notification_content,
        ])
        .setup(|app| {
            let window = tauri::WindowBuilder::new(
                app,
                MANAGER_WINDOW_LABEL,
                tauri::WindowUrl::App("index.html/#/manager".into()),
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

// In manager mode, we don't want to close the window completely,
// instead we should just "hide" it, so that we can later show it after click
// on the tray icon.
#[tauri::command]
async fn close_window(window: tauri::Window) {
    if let Err(e) = window.hide() {
        log::error!(
            "unable to hide the main window '{MANAGER_WINDOW_LABEL}', \
            forcing it to close instead: {e}"
        );
        common::close_window(&window).await;
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
    Ok(toolkit::installable_toolkits(reload, false)
        .await?
        .into_iter()
        .cloned()
        .collect())
}

#[tauri::command]
fn get_install_dir() -> String {
    rim::get_installed_dir().to_string_lossy().to_string()
}

#[tauri::command(rename_all = "snake_case")]
fn uninstall_toolkit(window: tauri::Window, remove_self: bool) -> Result<()> {
    let window = Arc::new(window);

    thread::spawn(move || -> anyhow::Result<()> {
        // FIXME: this is needed to make sure the other thread could recieve the first couple messages
        // we sent in this thread. But it feels very wrong, there has to be better way.
        thread::sleep(Duration::from_millis(500));

        window.emit(BLOCK_EXIT_EVENT, true)?;

        let pos_cb =
            |pos: f32| -> anyhow::Result<()> { Ok(window.emit(PROGRESS_UPDATE_EVENT, pos)?) };
        let progress = Progress::new(&pos_cb);

        let config = UninstallConfiguration::init(Some(progress))?;
        config.uninstall(remove_self)?;

        window.emit(ON_COMPLETE_EVENT, ())?;
        window.emit(BLOCK_EXIT_EVENT, false)?;
        Ok(())
    });

    Ok(())
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
                show_update_notification_popup(
                    &app,
                    UpdateTarget::Manager,
                    &current,
                    &latest,
                    None,
                )
                .await?;
            }
            UpdateCheckerOpt::load_from_install_dir().duration_until_next_run(UpdateTarget::Manager)
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
                    &app,
                    UpdateTarget::Toolkit,
                    current.version,
                    latest.version,
                    latest.payload,
                )
                .await?;
            }
            UpdateCheckerOpt::load_from_install_dir().duration_until_next_run(UpdateTarget::Toolkit)
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
            let timeout_for_manager = check_manager_update(&app_clone).await?;
            let timeout_for_toolkit = check_toolkit_update(&app_clone).await?;

            let timeout = timeout_for_manager.min(timeout_for_toolkit);
            println!("time for next check: {timeout:?}");
            utils::async_sleep(timeout).await;
        }
    })
    .await?
}

/// When the `install` button in a toolkit's card was clicked,
/// the URL of that toolkit was pass to this function. Which will be used to
/// download its manifest from the server, and we can then return a list of
/// components that are loaded from it.
#[tauri::command]
fn handle_toolkit_install_click(url: String) -> Result<Vec<Component>> {
    // the `url` input was converted from `Url`, so it will definitely be convert back without issue,
    // thus the below line should never panic
    let url_ = utils::force_parse_url(&url);

    // load the manifest for components information
    let manifest = get_toolset_manifest(Some(url_), false)?;
    let components = manifest.current_target_components(false)?;

    // cache the selected toolset manifest
    let mut guard = selected_toolset();
    *guard = Some(manifest);

    Ok(components)
}

async fn do_self_update(app: &AppHandle) -> Result<()> {
    // try show the window first, make sure it does not fails the process,
    // as we can still do self update without a window.
    show_manager_window_if_possible(app);

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
        // schedual restart with 3 seconds timeout
        win.emit(LOADING_FINISHED, true)?;
        for eta in (1..=3).rev() {
            win.emit(LOADING_TEXT, t!("self_update_finished", eta = eta))?;
            thread::sleep(Duration::from_secs(1));
        }
        win.emit(LOADING_TEXT, "")?;
    }

    // restart app
    app.restart();

    Ok(())
}

async fn do_toolkit_update(app: &AppHandle, url: String) -> Result<Vec<Component>> {
    // toolkit update requires the main window
    WindowState::detect(app)?.show()?;
    // show user the component selection page
    handle_toolkit_install_click(url)
}

async fn show_update_notification_popup<C: Display, S: Display>(
    app_handle: &AppHandle,
    target: UpdateTarget,
    current_ver: C,
    new_ver: S,
    payload: Option<String>,
) -> Result<()> {
    let (title, content, update_cmd) = match target {
        UpdateTarget::Manager => (
            t!("self_update_available"),
            t!("ask_self_update", current = current_ver, latest = new_ver),
            ("self_update_now".into(), None),
        ),
        UpdateTarget::Toolkit => (
            t!("toolkit_update_available"),
            t!(
                "ask_toolkit_update",
                current = current_ver,
                latest = new_ver,
            ),
            (
                "toolkit_update_now".into(),
                payload.map(|p| format!("{{ \"url\": \"{p}\" }}")),
            ),
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
                command: (
                    "skip_version".into(),
                    Some(format!(
                        "{{ \"version\": \"{new_ver}\", \"target\": \"{target}\" }}"
                    )),
                ),
            },
            NotificationAction {
                label: t!("close").into(),
                icon: Some("/close-icon.svg".into()),
                command: ("close".into(), None),
            },
        ],
    )
    .show(app_handle)
    .await?;

    Ok(())
}

#[tauri::command]
async fn self_update_now(app: AppHandle) -> Result<()> {
    notification::close(app.clone()).await;
    tauri::async_runtime::spawn(async move { do_self_update(&app).await }).await?
}

#[tauri::command]
async fn toolkit_update_now(app: AppHandle, url: String) -> Result<Vec<Component>> {
    notification::close(app.clone()).await;
    tauri::async_runtime::spawn(async move { do_toolkit_update(&app, url).await }).await?
}

#[tauri::command]
async fn skip_version(app: AppHandle, target: UpdateTarget, version: String) -> Result<()> {
    notification::close(app.clone()).await;
    tauri::async_runtime::spawn(async move {
        log::info!("skipping version: '{version}' for '{target}'");
        UpdateCheckerOpt::load_from_install_dir()
            .skip(target, version)
            .write_to_install_dir()
    })
    .await??;
    Ok(())
}

enum WindowState {
    Normal(Window),
    Hidden(Window),
    Minimized(Window),
    Closed,
}

impl WindowState {
    /// Detects the state of main manager window.
    fn detect(app: &AppHandle) -> Result<Self> {
        let Some(win) = app.get_window(MANAGER_WINDOW_LABEL) else {
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
                // TODO(?): maybe it is posible to revive a dead window, find a way.
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

fn system_tray() -> SystemTray {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", t!("show_ui")))
        .add_native_item(tauri::SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", t!("quit")));
    SystemTray::new().with_menu(tray_menu)
}

fn system_tray_event_handler(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => show_manager_window_if_possible(app),
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "show" => show_manager_window_if_possible(app),
            "quit" => app.exit(0),
            _ => {}
        },
        _ => {}
    }
}

fn window_event_handler(event: GlobalWindowEvent) {
    match event.event() {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            tauri::async_runtime::block_on(close_window(event.window().clone()));
        }
        _ => {}
    }
}

fn show_manager_window_if_possible(app: &AppHandle) {
    let Ok(state) = WindowState::detect(app) else {
        return;
    };
    _ = state.show();
}
