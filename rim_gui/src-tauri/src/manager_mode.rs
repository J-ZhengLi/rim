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
};
use rim::{
    version_skip::{SkipFor, VersionSkip},
    UninstallConfiguration,
};
use tauri::{api::dialog, AppHandle, Manager};

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
        .invoke_handler(tauri::generate_handler![
            super::close_window,
            get_installed_kit,
            get_available_kits,
            get_install_dir,
            uninstall_toolkit,
            install_toolkit,
            maybe_self_update,
            handle_toolkit_install_click,
            get_name_and_version,
            common::supported_languages,
            common::set_locale,
            self_update_now,
            skip_self_version,
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
fn get_name_and_version() -> (String, String) {
    (
        t!("manager_title", product = t!("product")).to_string(),
        format!("v{}", env!("CARGO_PKG_VERSION")),
    )
}

#[tauri::command]
fn get_installed_kit(reload: bool) -> Result<Option<Toolkit>> {
    Ok(Toolkit::installed(reload)?.map(|mutex| mutex.lock().unwrap().clone()))
}

#[tauri::command]
fn get_available_kits(reload: bool) -> Result<Vec<Toolkit>> {
    Ok(toolkit::installable_toolkits(reload, false)?
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

#[tauri::command]
async fn maybe_self_update(app: AppHandle) -> Result<()> {
    let update_kind = update::check_self_update(false);
    let Some(new_ver) = update_kind.newer_version() else {
        return Ok(());
    };

    // TODO: if is_on_background {}
    show_self_update_notification_popup(&app, new_ver).await?;
    // TODO: else
    show_self_update_dialog(app, new_ver)?;

    Ok(())
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

fn do_self_update(app: &AppHandle) -> Result<()> {
    let window = app.get_window(MANAGER_WINDOW_LABEL);
    // block UI interaction, and show loading toast
    if let Some(win) = &window {
        win.emit(LOADING_TEXT, t!("self_update_in_progress"))?;
    }

    // do self update, skip version check because it should already
    // been checked using `update::check_self_update`
    if let Err(e) = UpdateOpt::new().self_update(true) {
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

fn show_self_update_dialog<S: Display>(app: AppHandle, new_ver: S) -> Result<()> {
    dialog::ask(
        app.get_window(MANAGER_WINDOW_LABEL).as_ref(),
        t!("self_update_available"),
        t!(
            "ask_self_update",
            latest = new_ver,
            current = env!("CARGO_PKG_VERSION")
        ),
        move |yes| {
            if yes {
                if let Err(e) = do_self_update(&app) {
                    log::error!("failed when perform self update: {e}");
                }
            }
        },
    );
    Ok(())
}

async fn show_self_update_notification_popup<S: Display>(
    app_handle: &AppHandle,
    new_ver: S,
) -> Result<()> {
    Notification::new(
        t!("self_update_available").into(),
        t!(
            "ask_self_update",
            current = env!("CARGO_PKG_VERSION"),
            latest = new_ver
        )
        .into(),
        vec![
            NotificationAction {
                label: t!("update").into(),
                icon: Some("/update-icon.svg".into()),
                command: ("self_update_now".into(), None),
            },
            NotificationAction {
                label: t!("skip_version").into(),
                icon: Some("/stop-icon.svg".into()),
                command: (
                    "skip_self_version".into(),
                    Some(format!("{{ \"version\": \"{new_ver}\" }}")),
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
    tauri::async_runtime::spawn(async move { do_self_update(&app) }).await?
}

#[tauri::command]
async fn skip_self_version(app: AppHandle, version: String) -> Result<()> {
    notification::close(app.clone()).await;
    tauri::async_runtime::spawn(async move {
        log::info!("skipping manager version: '{version}'");
        VersionSkip::load_from_install_dir()
            .skip(SkipFor::Manager, version)
            .write_to_install_dir()
    })
    .await??;
    Ok(())
}
