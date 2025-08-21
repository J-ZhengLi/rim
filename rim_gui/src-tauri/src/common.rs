use std::{
    ops::Deref,
    path::PathBuf,
    sync::{mpsc::Receiver, OnceLock},
    thread,
    time::Duration,
};

use super::consts::*;
use crate::{error::Result, progress::GuiProgress};
use rim::{
    cli::{ExecutableCommand, ManagerSubcommands},
    components::Component,
    AppInfo, InstallConfiguration, UninstallConfiguration,
};
use rim_common::types::ToolkitManifest;
use serde::{Deserialize, Serialize};
use tauri::{App, AppHandle, Manager, Window, WindowUrl};
use tokio::sync::RwLock as AsyncRwLock;
use url::Url;

/// A cached toolkit manifest.
///
/// This is a read-write lock wrapped under `OnceLock`, which means it can be changed after initialization.
pub(crate) static TOOLKIT_MANIFEST: OnceLock<AsyncRwLock<ToolkitManifest>> = OnceLock::new();

/// Retrieve cached toolkit manifest when it was certainly cached.
///
/// # Panic
/// Will panic if the manifest is not cached.
pub(crate) fn expected_manifest() -> &'static AsyncRwLock<ToolkitManifest> {
    TOOLKIT_MANIFEST
        .get()
        .expect("toolset manifest must be loaded by now, this is a bug.")
}

fn spawn_gui_update_thread(window: Window, msg_recv: Receiver<String>) {
    thread::spawn(move || loop {
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

pub(crate) async fn install_toolkit_(
    window: tauri::Window,
    components_list: Vec<Component>,
    config: BaseConfiguration,
    manifest: &ToolkitManifest,
    is_update: bool,
) -> anyhow::Result<()> {
    window.emit(BLOCK_EXIT_EVENT, true)?;

    let install_dir = PathBuf::from(&config.path);
    // TODO: Use continuous progress
    let i_config = InstallConfiguration::new(
        &install_dir,
        manifest,
        GuiProgress::new(window.app_handle()),
    )?
    .with_rustup_dist_server(config.rustup_dist_server.as_deref().cloned())
    .with_rustup_update_root(config.rustup_update_root.as_deref().cloned())
    .with_cargo_registry(config.cargo_registry())
    .insecure(config.insecure);

    if is_update {
        i_config.update(components_list).await?;
    } else {
        i_config.install(components_list).await?;
    }

    // 安装完成后，发送安装完成事件
    window.emit(ON_COMPLETE_EVENT, ())?;

    Ok(())
}

pub(crate) async fn uninstall_toolkit_(window: tauri::Window, remove_self: bool) -> Result<()> {
    window.emit(BLOCK_EXIT_EVENT, true)?;

    let config = UninstallConfiguration::init(GuiProgress::new(window.app_handle()))?;
    config.uninstall(remove_self)?;

    window.emit(ON_COMPLETE_EVENT, ())?;
    Ok(())
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
        .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .min_inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .decorations(false)
        .title(AppInfo::name())
        .visible(visible)
        .build()?;

    // when opening the application, there's a chance that everything appear
    // to be un-arranged after loaded due to WebView not being fully initialized,
    // therefore we add 1 second delay to hide it after the content was loaded.
    // FIXME: maybe it's better to have a simple splash screen
    window.eval(
        "window.addEventListener('DOMContentLoaded', () => {
    document.body.style.visibility = 'hidden';
    setTimeout(() => { document.body.style.visibility = 'visible' }, 1000);
});",
    )?;

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

/// Contains an extra boolean flag to indicate
/// whether an option was enforced by toolkit or not.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EnforceableOption<T>(T, bool);

impl<T> Deref for EnforceableOption<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for EnforceableOption<T> {
    fn from(value: T) -> Self {
        Self(value, false)
    }
}

impl From<&str> for EnforceableOption<String> {
    fn from(value: &str) -> Self {
        Self(value.to_string(), false)
    }
}

/// The configuration options to install a toolkit.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BaseConfiguration {
    pub(crate) path: PathBuf,
    pub(crate) add_to_path: bool,
    pub(crate) insecure: bool,
    pub(crate) rustup_dist_server: Option<EnforceableOption<Url>>,
    pub(crate) rustup_update_root: Option<EnforceableOption<Url>>,
    cargo_registry_name: Option<EnforceableOption<String>>,
    cargo_registry_value: Option<EnforceableOption<String>>,
}

impl BaseConfiguration {
    /// Create a new configuration set base on toolkit manifest.
    ///
    /// Some options might be enforced by the toolkit manifest,
    /// this why we need to access it when returning the base configuration.
    pub(crate) fn new<P: Into<PathBuf>>(path: P, manifest: Option<&ToolkitManifest>) -> Self {
        let rustup_dist_server = manifest
            .and_then(|m| m.config.rustup_dist_server.clone())
            .map(|u| EnforceableOption(u, true))
            .unwrap_or_else(|| rim::default_rustup_dist_server().clone().into());
        let rustup_update_root = manifest
            .and_then(|m| m.config.rustup_update_root.clone())
            .map(|u| EnforceableOption(u, true))
            .unwrap_or_else(|| rim::default_rustup_update_root().clone().into());

        let registry = manifest.and_then(|m| m.config.cargo_registry.clone());
        let (cargo_registry_name, cargo_registry_value) = registry
            .map(|r| {
                (
                    EnforceableOption(r.name, true),
                    EnforceableOption(r.index, true),
                )
            })
            .unwrap_or_else(|| {
                (
                    rim::default_cargo_registry().0.into(),
                    rim::default_cargo_registry().1.into(),
                )
            });

        BaseConfiguration {
            path: path.into(),
            add_to_path: true,
            insecure: false,
            rustup_dist_server: Some(rustup_dist_server),
            rustup_update_root: Some(rustup_update_root),
            cargo_registry_name: Some(cargo_registry_name),
            cargo_registry_value: Some(cargo_registry_value),
        }
    }

    /// Combine `cargo_registry_name` and `cargo_registry_value` from user input.
    ///
    /// If either `self.cargo_registry_value` or `self.cargo_registry_name` is `None`,
    /// this will return `None`.
    pub(crate) fn cargo_registry(&self) -> Option<(&str, &str)> {
        Some((
            self.cargo_registry_name.as_deref()?,
            self.cargo_registry_value.as_deref()?,
        ))
    }
}
