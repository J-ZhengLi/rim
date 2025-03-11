use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        LazyLock, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use super::consts::*;
use crate::error::Result;
use rim::{
    components::Component,
    setter,
    toolset_manifest::ToolsetManifest,
    update::UpdateCheckBlocker,
    utils::{self, Progress},
    AppInfo, InstallConfiguration, UninstallConfiguration,
};
use serde::Serialize;
use tauri::api::cli::{Matches, SubcommandMatches};
use tauri::{App, AppHandle, Manager, Window};

#[allow(clippy::type_complexity)]
static THREAD_POOL: LazyLock<Mutex<Vec<JoinHandle<anyhow::Result<()>>>>> =
    LazyLock::new(|| Mutex::new(vec![]));
static CLI_OPT: OnceLock<CliOpt> = OnceLock::new();

#[derive(Clone, serde::Serialize)]
pub(crate) struct SingleInstancePayload {
    pub(crate) argv: Vec<String>,
    pub(crate) cmd: String,
}

/// Configure the logger to use a communication channel ([`mpsc`]),
/// allowing us to send logs accrossing threads.
///
/// This will return a log message's receiver which can be used to emitting
/// messages onto [`tauri::Window`]
pub(crate) fn setup_logger() -> Receiver<String> {
    let (msg_sendr, msg_recvr) = mpsc::channel::<String>();
    if let Err(e) = utils::Logger::new().sender(msg_sendr).setup() {
        // TODO: make this error more obvious
        eprintln!(
            "Unable to setup logger, cause: {e}\n\
            The program will continues to run, but it might not functioning correctly."
        );
    }
    msg_recvr
}

pub(crate) fn spawn_gui_update_thread(window: tauri::Window, msg_recv: Receiver<String>) {
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
                // resume update check when all tasks are finished
                if pool.is_empty() {
                    UpdateCheckBlocker::unblock();
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
            if msg.starts_with("error:") {
                emit(&window, ON_FAILED_EVENT, msg);
                break;
            } else {
                emit(&window, MESSAGE_UPDATE_EVENT, msg);
            }
        }
    });
}

fn emit(window: &tauri::Window, event: &str, msg: String) {
    window.emit(event, msg).unwrap_or_else(|e| {
        log::error!(
            "unexpected error occurred \
            while emiting tauri event: {e}"
        )
    });
}

pub(crate) fn install_toolkit_in_new_thread(
    window: tauri::Window,
    components_list: Vec<Component>,
    install_dir: PathBuf,
    manifest: ToolsetManifest,
    is_update: bool,
) {
    UpdateCheckBlocker::block();

    let handle = thread::spawn(move || -> anyhow::Result<()> {
        // FIXME: this is needed to make sure the other thread could recieve the first couple messages
        // we sent in this thread. But it feels very wrong, there has to be better way.
        thread::sleep(Duration::from_millis(500));

        window.emit(BLOCK_EXIT_EVENT, true)?;

        // Initialize a progress sender.
        let pos_cb =
            |pos: f32| -> anyhow::Result<()> { Ok(window.emit(PROGRESS_UPDATE_EVENT, pos)?) };
        let progress = Progress::new(&pos_cb);

        // TODO: Use continuous progress
        let mut config = InstallConfiguration::new(&install_dir, &manifest)?
            .with_progress_indicator(Some(progress));
        if let Some(rustup_dist_server) = get_cli().rustup_dist_server.as_deref() {
            config = config.with_rustup_dist_server(rustup_dist_server.parse()?);
        }
        if is_update {
            config.update(components_list)?;
        } else {
            config.install(components_list)?;
        }

        // 安装完成后，发送安装完成事件
        window.emit(ON_COMPLETE_EVENT, ())?;
        window.emit(BLOCK_EXIT_EVENT, false)?;

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
        .unwrap_or_else(|_| panic!("thread join failed when attemp to close window '{label}'"))
        .unwrap_or_else(|e| log::error!("failed when closing window '{label}': {e}"))
}

/// Simple representation of a Rust's function signature, typically got sent
/// to the frontend, therefore the frontend knows which and how to invoke a
/// certain Rust function.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct FrontendFunctionPayload {
    pub(crate) name: String,
    pub(crate) args: Vec<(&'static str, String)>,
    /// The **identifier** of function return, not the actual return value,
    /// because the frontend can retrieve the return value itself, but it
    /// need to known how to deal with it base on an unique identifier.
    pub(crate) ret_id: Option<&'static str>,
}

impl FrontendFunctionPayload {
    pub(crate) fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            args: vec![],
            ret_id: None,
        }
    }

    setter!(with_args(self.args, Vec<(&'static str, String)>));
    setter!(with_ret_id(self.ret_id, identifier: &'static str) { Some(identifier) });
}

/// Build the main window with shared configuration.
pub(crate) fn setup_main_window(manager: &mut App, log_receiver: Receiver<String>) -> Result<()> {
    let mut visible = true;
    let (label, url) = if AppInfo::is_manager() {
        let opt = handle_cli_args(manager);
        if opt.silent {
            info!("manager launched in silent mode");
            visible = false;
        }
        (MANAGER_WINDOW_LABEL, "index.html/#/manager".into())
    } else {
        (INSTALLER_WINDOW_LABEL, "index.html/#/installer".into())
    };

    let window = tauri::WindowBuilder::new(manager, label, tauri::WindowUrl::App(url))
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

    spawn_gui_update_thread(window.clone(), log_receiver);
    get_cli().execute(manager.handle())?;
    Ok(())
}

macro_rules! cli_value {
    ($m:ident[$key:literal].$f:ident) => {{
        $m.args
            .get($key)
            .unwrap_or_else(|| panic!("argument '{}' does not exists", $key))
            .value
            .$f()
    }};
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CliOpt {
    /// Launching the app without showing the main window.
    silent: bool,
    rustup_dist_server: Option<String>,
    subcommand: Option<CliSubcommand>,
}

impl CliOpt {
    pub(crate) fn execute(&self, app: AppHandle) -> Result<()> {
        let Some(subcmd) = &self.subcommand else {
            return Ok(());
        };

        match subcmd {
            CliSubcommand::Uninstall => {
                if !AppInfo::is_manager() {
                    return Ok(());
                }
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(1500));
                    _ = app.emit_all(
                        "change-view",
                        CliPayload {
                            path: "/manager/uninstall".into(),
                            command: CliSubcommand::Uninstall,
                        },
                    );
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct CliPayload {
    path: String,
    command: CliSubcommand,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
enum CliSubcommand {
    Uninstall,
}

impl TryFrom<Box<SubcommandMatches>> for CliSubcommand {
    type Error = anyhow::Error;
    fn try_from(value: Box<SubcommandMatches>) -> anyhow::Result<Self> {
        let res = match value.name.as_str() {
            "uninstall" => CliSubcommand::Uninstall,
            s => anyhow::bail!("unknown subcommand '{s}'"),
        };
        Ok(res)
    }
}

fn get_cli() -> &'static CliOpt {
    CLI_OPT.get_or_init(CliOpt::default)
}

impl From<Matches> for CliOpt {
    fn from(value: Matches) -> Self {
        Self {
            silent: cli_value!(value["silent"].as_bool).unwrap_or_default(),
            rustup_dist_server: cli_value!(value["rustup-dist-server"].as_str)
                .map(ToOwned::to_owned),
            subcommand: value
                .subcommand
                .and_then(|m| CliSubcommand::try_from(m).ok()),
        }
    }
}

impl TryFrom<&[String]> for CliOpt {
    type Error = anyhow::Error;
    fn try_from(value: &[String]) -> anyhow::Result<Self> {
        let mut silent = false;
        let mut rustup_dist_server = None;
        let mut subcommand = None;

        let mut iter = value.iter();
        while let Some(v) = iter.next() {
            match v.as_str() {
                "-s" | "--silent" => silent = true,
                "--rust-dist-server" => rustup_dist_server = iter.next().map(ToString::to_string),
                "uninstall" => subcommand = Some(CliSubcommand::Uninstall),
                s => anyhow::bail!("unknown argument '{s}'"),
            }
        }

        Ok(Self {
            silent,
            rustup_dist_server,
            subcommand,
        })
    }
}

fn handle_cli_args(app: &mut App) -> &'static CliOpt {
    let Ok(matches) = app.get_cli_matches() else {
        return get_cli();
    };
    // log raw args
    info!("application started with args: {:?}", &matches.args);

    CLI_OPT
        .set(CliOpt::from(matches))
        .expect("unable to to set CLI options");
    get_cli()
}
