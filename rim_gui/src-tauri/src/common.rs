use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        LazyLock, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::consts::*;
use crate::error::Result;
use rim::{
    cli::{ComponentCommand, ExecutableCommand, ManagerSubcommands},
    components::Component,
    update::UpdateCheckBlocker,
    AppInfo, InstallConfiguration, UninstallConfiguration,
};
use rim_common::{types::ToolkitManifest, utils};
use serde::Serialize;
use tauri::{App, AppHandle, Manager, Window, WindowUrl};
use url::Url;

#[allow(clippy::type_complexity)]
static THREAD_POOL: LazyLock<Mutex<Vec<JoinHandle<anyhow::Result<()>>>>> =
    LazyLock::new(|| Mutex::new(vec![]));

static SHARED_CONFIGS: Mutex<SharedConfigs> = Mutex::new(SharedConfigs::new());

/// Configure the logger to use a communication channel ([`mpsc`]),
/// allowing us to send logs across threads.
///
/// This will return a log message's receiver which can be used to emitting
/// messages onto [`tauri::Window`]
pub(crate) fn setup_logger() -> Receiver<String> {
    let (msg_sender, msg_recvr) = mpsc::channel::<String>();
    if let Err(e) = utils::Logger::new().sender(msg_sender).setup() {
        // TODO: make this error more obvious
        eprintln!(
            "Unable to setup logger, cause: {e}\n\
            The program will continues to run, but it might not functioning correctly."
        );
    }
    msg_recvr
}

pub(crate) fn spawn_gui_update_thread(window: Window, msg_recv: Receiver<String>) {
    thread::spawn(move || loop {
        // wait for all other thread to finish and report errors
        let mut pool = THREAD_POOL
            .lock()
            .expect("failed when accessing thread pool");
        let mut idx = 0;
        while let Some(thread) = pool.get(idx) {
            if thread.is_finished() {
                let handle = pool.swap_remove(idx);
                if let Err(e) = handle.join().unwrap() {
                    log::error!("GUI runtime error: {e}");
                    emit(&window, ON_FAILED_EVENT, e.to_string());
                }
                if pool.is_empty() {
                    // resume update check when all tasks are finished
                    UpdateCheckBlocker::unblock();
                    // make sure to show the exit button
                    emit(&window, BLOCK_EXIT_EVENT, false);
                }
            } else {
                // if a thread is finished, it will be removed,
                // so here we only increase the index otherwise.
                idx += 1;
            }
        }
        // drop before `recv()` blocking the thread, otherwise there'll be deadlock.
        drop(pool);

        // Note: `recv()` will block, therefore it's important to check thread execution at first
        if let Ok(msg) = msg_recv.recv() {
            emit(&window, MESSAGE_UPDATE_EVENT, msg);
        }
    });
}

fn emit<S: Serialize + Clone>(window: &Window, event: &str, msg: S) {
    window.emit(event, msg).unwrap_or_else(|e| {
        log::error!(
            "unexpected error occurred \
            while emitting tauri event: {e}"
        )
    });
}

pub(crate) fn install_toolkit_in_new_thread(
    window: tauri::Window,
    components_list: Vec<Component>,
    install_dir: PathBuf,
    manifest: ToolkitManifest,
    is_update: bool,
) {
    UpdateCheckBlocker::block();

    let rustup_dist_server = SHARED_CONFIGS.lock().unwrap().rustup_dist_server.clone();
    let handle = thread::spawn(move || -> anyhow::Result<()> {
        // FIXME: this is needed to make sure the other thread could receive the first couple messages
        // we sent in this thread. But it feels very wrong, there has to be better way.
        thread::sleep(Duration::from_millis(500));

        window.emit(BLOCK_EXIT_EVENT, true)?;

        // Initialize a progress sender.
        let pos_cb =
            |pos: f32| -> anyhow::Result<()> { Ok(window.emit(PROGRESS_UPDATE_EVENT, pos)?) };
        let progress = utils::Progress::new(&pos_cb);

        // TODO: Use continuous progress
        let config = InstallConfiguration::new(&install_dir, &manifest)?
            .with_progress_indicator(Some(progress))
            .with_rustup_dist_server(rustup_dist_server);
        if is_update {
            config.update(components_list)?;
        } else {
            config.install(components_list)?;
        }

        // 安装完成后，发送安装完成事件
        window.emit(ON_COMPLETE_EVENT, ())?;

        Ok(())
    });

    THREAD_POOL
        .lock()
        .expect("failed pushing installation thread handle into thread pool")
        .push(handle);
}

pub(crate) fn uninstall_toolkit_in_new_thread(window: tauri::Window, remove_self: bool) {
    // block update checker, we don't want to show update notification here.
    UpdateCheckBlocker::block();

    let handle = thread::spawn(move || -> anyhow::Result<()> {
        // FIXME: this is needed to make sure the other thread could receive the first couple messages
        // we sent in this thread. But it feels very wrong, there has to be better way.
        thread::sleep(Duration::from_millis(500));

        window.emit(BLOCK_EXIT_EVENT, true)?;

        let pos_cb =
            |pos: f32| -> anyhow::Result<()> { Ok(window.emit(PROGRESS_UPDATE_EVENT, pos)?) };
        let progress = utils::Progress::new(&pos_cb);

        let config = UninstallConfiguration::init(Some(progress))?;
        config.uninstall(remove_self)?;

        window.emit(ON_COMPLETE_EVENT, ())?;
        Ok(())
    });

    THREAD_POOL
        .lock()
        .expect("failed pushing uninstallation thread handle into thread pool")
        .push(handle);
}

#[derive(serde::Serialize)]
pub struct Language {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub(crate) fn get_label(key: &str) -> String {
    t!(key).into()
}

#[tauri::command]
pub(crate) fn supported_languages() -> Vec<Language> {
    rim::Language::possible_values()
        .iter()
        .map(|lang| {
            let id = lang.as_str();
            match lang {
                rim::Language::EN => Language {
                    id: id.to_string(),
                    name: "English".to_string(),
                },
                rim::Language::CN => Language {
                    id: id.to_string(),
                    name: "简体中文".to_string(),
                },
                _ => Language {
                    id: id.to_string(),
                    name: id.to_string(),
                },
            }
        })
        .collect()
}

#[tauri::command]
pub(crate) fn set_locale(language: String) -> Result<()> {
    let lang: rim::Language = language.parse()?;
    utils::set_locale(lang.locale_str());
    Ok(())
}

#[tauri::command]
pub(crate) fn app_info() -> AppInfo {
    AppInfo::get().to_owned()
}

/// Close the given window in a separated thread.
#[tauri::command]
pub(crate) fn close_window(win: Window) {
    let label = win.label().to_owned();
    thread::spawn(move || win.close())
        .join()
        .unwrap_or_else(|_| panic!("thread join failed when attempt to close window '{label}'"))
        .unwrap_or_else(|e| log::error!("failed when closing window '{label}': {e}"))
}

#[tauri::command]
pub(crate) fn get_build_cfg_locale_str(key: &str) -> &str {
    utils::build_cfg_locale(key)
}

/// Build the installer window with shared configuration.
pub(crate) fn setup_installer_window(
    manager: &mut App,
    log_receiver: Receiver<String>,
) -> Result<Window> {
    let window = setup_window_(
        manager,
        INSTALLER_WINDOW_LABEL,
        WindowUrl::App("index.html/#/installer".into()),
        true,
    )?;
    spawn_gui_update_thread(window.clone(), log_receiver);
    Ok(window)
}

/// Build the manager window with shared configuration.
pub(crate) fn setup_manager_window(
    manager: &mut App,
    log_receiver: Receiver<String>,
    maybe_args: anyhow::Result<Box<rim::cli::Manager>>,
) -> Result<Window> {
    let mut visible = true;

    let args = match maybe_args {
        Ok(args) => {
            if args.silent_mode() {
                visible = false;
            }
            Some(args)
        }
        Err(err) => {
            error!(
                "tried to start the program with cli arguments \
            but the arguments cannot be parsed. {err}"
            );

            None
        }
    };

    let window = setup_window_(
        manager,
        MANAGER_WINDOW_LABEL,
        WindowUrl::App("index.html/#/manager".into()),
        visible,
    )?;

    spawn_gui_update_thread(window.clone(), log_receiver);
    if let Some(a) = args {
        handle_manager_args(manager.handle().clone(), *a);
    }
    Ok(window)
}

fn setup_window_(app: &mut App, label: &str, url: WindowUrl, visible: bool) -> Result<Window> {
    let window = tauri::WindowBuilder::new(app, label, url)
        .inner_size(800.0, 600.0)
        .min_inner_size(640.0, 480.0)
        .decorations(false)
        .transparent(true)
        .title(AppInfo::name())
        .visible(visible)
        .build()?;

    #[cfg(not(target_os = "linux"))]
    if let Err(e) = window_shadows::set_shadow(&window, true) {
        log::error!("unable to apply window effects: {e}");
    }

    // enable dev console only on debug mode
    #[cfg(debug_assertions)]
    window.open_devtools();

    Ok(window)
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CliPayload {
    pub(crate) path: String,
    pub(crate) command_id: String,
}

pub(crate) fn handle_manager_args(app: AppHandle, cli: rim::cli::Manager) {
    if let Some(ManagerSubcommands::Uninstall { keep_self }) = cli.command {
        if !AppInfo::is_manager() {
            return;
        }
        let command_id = if keep_self {
            "uninstall-toolkit"
        } else {
            "uninstall"
        }
        .to_string();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1500));
            _ = app.emit_all(
                "change-view",
                CliPayload {
                    path: "/manager/uninstall".into(),
                    command_id,
                },
            );
        });
    }
}

pub(crate) struct SharedConfigs {
    pub(crate) rustup_dist_server: Option<Url>,
}

impl SharedConfigs {
    pub(crate) const fn new() -> Self {
        Self {
            rustup_dist_server: None,
        }
    }
}

impl From<&rim::cli::Installer> for SharedConfigs {
    fn from(value: &rim::cli::Installer) -> Self {
        Self {
            rustup_dist_server: value.rustup_dist_server.clone(),
        }
    }
}

impl From<&rim::cli::Manager> for SharedConfigs {
    fn from(value: &rim::cli::Manager) -> Self {
        let rustup_dist_server = value.command.as_ref().and_then(|cmd| match cmd {
            ManagerSubcommands::Component {
                command:
                    ComponentCommand::Install {
                        rustup_dist_server, ..
                    },
            }
            | ManagerSubcommands::Install {
                rustup_dist_server, ..
            }
            | ManagerSubcommands::Update {
                rustup_dist_server, ..
            } => rustup_dist_server.clone(),
            _ => None,
        });
        Self { rustup_dist_server }
    }
}

pub(crate) fn update_shared_configs<T: Into<SharedConfigs>>(value: T) {
    let mut guard = SHARED_CONFIGS.lock().unwrap();
    *guard = value.into();
}
