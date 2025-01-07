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
};
use anyhow::Context;
use rim::UninstallConfiguration;
use rim::{
    components::Component,
    toolkit::{self, Toolkit},
    toolset_manifest::{get_toolset_manifest, ToolsetManifest},
    update::{self, UpdateOpt},
    utils::{self, Progress},
};
use tauri::{api::dialog, AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

static SELECTED_TOOLSET: Mutex<Option<ToolsetManifest>> = Mutex::new(None);

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
            window_title,
            common::supported_languages,
            common::set_locale,
        ])
        .setup(|app| {
            let window = tauri::WindowBuilder::new(
                app,
                "manager_window",
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
fn window_title() -> String {
    format!(
        "{} v{}",
        t!("installer_title", product = t!("product")),
        env!("CARGO_PKG_VERSION")
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
    show_self_update_notification_popup(&app, None, None).await?;
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

fn show_self_update_dialog<S: Display>(app: AppHandle, new_ver: S) -> Result<()> {
    let window = Arc::new(app.get_window("manager_window"));
    dialog::ask(
        window.clone().as_ref().as_ref(),
        t!("update_available"),
        t!(
            "ask_self_update",
            latest = new_ver,
            current = env!("CARGO_PKG_VERSION")
        ),
        move |yes| {
            if !yes {
                return;
            }
            let Some(win) = window.as_ref() else {
                return;
            };

            // block UI interaction, and show loading toast
            _ = win.emit(LOADING_TEXT, t!("self_update_in_progress"));
            // do self update
            if let Ok(true) = UpdateOpt::new().self_update() {
                app.restart();
            }
            _ = win.emit(LOADING_FINISHED, true);
            for eta in (1..=3).rev() {
                _ = win.emit(LOADING_TEXT, t!("self_update_finished", eta = eta));
                thread::sleep(Duration::from_secs(1));
            }
            _ = win.emit(LOADING_TEXT, "");
            // restart app
            app.restart();
        },
    );
    Ok(())
}

async fn show_self_update_notification_popup(
    app_handle: &AppHandle,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<()> {
    let popup = tauri::WindowBuilder::new(
        app_handle,
        "notification_popup", /* the unique window label */
        tauri::WindowUrl::App("notification.html".into()),
    )
    .always_on_top(true)
    .decorations(false)
    .resizable(false)
    .title("notification")
    .inner_size(width.unwrap_or(360.0), height.unwrap_or(220.0))
    .build()?;

    popup.move_window(Position::BottomRight)?;
    Ok(())
}
