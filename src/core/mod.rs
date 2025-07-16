//! Core functionalities of this program
//!
//! Including configuration, toolchain, toolset management.

pub(crate) mod check;
pub mod components;
mod custom_instructions;
mod dependency_handler;
pub(crate) mod directories;
pub mod install;
pub(crate) mod os;
pub(crate) mod parser;
mod path_ext;
pub(crate) mod rustup;
pub mod toolkit;
mod toolkit_manifest_ext;
pub(crate) mod tools;
pub mod try_it;
pub(crate) mod uninstall;
pub mod update;

// re-exports
pub(crate) use path_ext::PathExt;
pub use toolkit_manifest_ext::*;

use crate::{cli, fingerprint::InstallationRecord};
use anyhow::{bail, Result};
use rim_common::{
    build_config,
    types::{Configuration, TomlParser},
    utils,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use url::Url;

macro_rules! declare_env_vars {
    ($($key:ident),+) => {
        $(pub(crate) const $key: &str = stringify!($key);)*
        #[cfg(windows)]
        pub(crate) static ALL_VARS: &[&str] = &[$($key),+];
    };
}

declare_env_vars!(
    CARGO_HOME,
    RUSTUP_HOME,
    RUSTUP_DIST_SERVER,
    RUSTUP_UPDATE_ROOT
);

/// Globally cached values
static GLOBAL_OPTS: Mutex<Option<GlobalOpts>> = Mutex::new(None);
static APP_INFO: OnceLock<AppInfo> = OnceLock::new();
static INSTALL_DIR_ONCE: OnceLock<PathBuf> = OnceLock::new();

/// Get the default rustup dist server url.
pub fn default_rustup_dist_server() -> &'static Url {
    build_config().rustup_dist_server(env!("EDITION"))
}

/// Get the default rustup update root url.
pub fn default_rustup_update_root() -> &'static Url {
    build_config().rustup_update_root(env!("EDITION"))
}

pub(crate) fn rim_dist_server() -> Url {
    if let Ok(env_server) = env::var("RIM_DIST_SERVER") {
        if let Ok(url) = env_server.parse::<Url>() {
            return url;
        }
    }

    build_config().rim_dist_server(env!("EDITION")).clone()
}

/// Get the default name and value of replaced cargo registry.
/// (i.e.: ("mirror", "sparse+http://replaced-crates.io"))
pub fn default_cargo_registry() -> (&'static str, &'static str) {
    let cfg = build_config();

    (&cfg.cargo.registry_name, &cfg.cargo.registry_url)
}

/// Representing the options that user pass to the program, such as
/// `--yes`, `--no-modify-path`, etc.
///
/// This struct will be stored globally for easy access, also make
/// sure the [`set`](GlobalOpts::set) function is called exactly once
/// to initialize the global singleton.
#[derive(Debug, Default, Clone, Copy)]
pub struct GlobalOpts {
    pub verbose: bool,
    pub quiet: bool,
    pub yes_to_all: bool,
    no_modify_env: bool,
    no_modify_path: bool,
}

impl GlobalOpts {
    /// Initialize a new object and store it globally
    pub fn set(verbose: bool, quiet: bool, yes: bool, no_modify_env: bool, no_modify_path: bool) {
        let opts = Self {
            verbose,
            quiet,
            yes_to_all: yes,
            no_modify_env,
            no_modify_path,
        };

        *GLOBAL_OPTS.lock().unwrap() = Some(opts);
    }

    /// Get the stored global options.
    ///
    /// Fallback to default value if is not set.
    pub fn get() -> Self {
        GLOBAL_OPTS.lock().unwrap().unwrap_or_default()
    }

    /// Return `true` if either one of `no-modify-path` or `no-modify-env` was set to `true`
    pub fn no_modify_path(&self) -> bool {
        self.no_modify_path || self.no_modify_env
    }

    /// Return `true` if `no-modify-env` was set to `true`
    pub fn no_modify_env(&self) -> bool {
        self.no_modify_env
    }
}

/// Representing the execution mode of this program.
///
/// Each variant contains a parsed CLI arg matches, which can fail
/// if the user pass some invalid args to the program.
///
/// # Example
/// - In [`Installer`](Mode::Installer) (a.k.a `setup` mode), this program
///   does initial setup and install rust toolkit for the user.
/// - In [`Manager`](Mode::Manager) mode, this program can be used for
///   updating, uninstalling toolkit etc.
pub enum Mode {
    Manager(Result<Box<cli::Manager>>),
    Installer(Result<Box<cli::Installer>>),
}

impl Mode {
    fn manager(manager_callback: Option<Box<dyn FnOnce(&cli::Manager)>>) -> Self {
        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: utils::build_cfg_locale("app_name").into(),
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            is_manager: true,
        });

        let maybe_args = cli::parse_manager_cli();
        // execute callback function on cli args
        if let Ok(args) = &maybe_args {
            if let Some(cb) = manager_callback {
                cb(args);
            }
        }

        if let Err(e) = handle_migration() {
            error!("migration failed, the program might mot be able to work as expected: {e}");
        }

        Self::Manager(maybe_args)
    }
    fn installer(installer_callback: Option<Box<dyn FnOnce(&cli::Installer)>>) -> Self {
        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: utils::build_cfg_locale("app_name").into(),
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            is_manager: false,
        });

        let maybe_args = cli::parse_installer_cli();
        if let Ok(args) = &maybe_args {
            if let Some(cb) = installer_callback {
                cb(args);
            }
        }

        Self::Installer(maybe_args)
    }

    /// Automatically determine which mode that this program is running as.
    ///
    /// Optional callback functions can be passed,
    /// which will be run after a mode has been determined.
    pub fn detect(
        installer_callback: Option<Box<dyn FnOnce(&cli::Installer)>>,
        manager_callback: Option<Box<dyn FnOnce(&cli::Manager)>>,
    ) -> Self {
        match env::var("MODE").as_deref() {
            Ok("manager") => Self::manager(manager_callback),
            // fallback to installer mode
            Ok(_) => Self::installer(installer_callback),
            Err(_) => {
                if InstallationRecord::load_from_config_dir().is_ok() {
                    Self::manager(manager_callback)
                } else {
                    // fallback to installer mode
                    Self::installer(installer_callback)
                }
            }
        }
    }
}

/// Some settings are different
fn handle_migration() -> Result<()> {
    fn migrate_config_file(old_name: &str, new_name: &str) -> Result<()> {
        // the configs were stored under install_dir, the only way to find it without reading the
        // install-record is to check for `<current_exe_dir>/.fingerprint.toml`, or
        // `<current_exe_dir>/../../.fingerprint.toml` since this could be a link under cargo/bin
        let mut old_file = utils::parent_dir_of_cur_exe()?.join(old_name);
        if !old_file.exists() {
            old_file.pop();
            old_file.pop();
            old_file.pop();
            old_file.push(old_name);
        };
        let new_file = rim_common::dirs::rim_config_dir().join(new_name);
        match (old_file.is_file(), new_file.is_file()) {
            (true, false) => utils::move_to(&old_file, &new_file, true),
            (true, true) => {
                warn!(
                    "{}",
                    t!(
                        "duplicated_config_files",
                        first = new_file.display(),
                        second = old_file.display()
                    )
                );
                Ok(())
            }
            (false, _) => Ok(()),
        }
    }

    // Migrate the old config files (rim <= 0.8.0) to the new config_dir
    migrate_config_file(".fingerprint.toml", InstallationRecord::FILENAME)?;
    migrate_config_file("config.toml", Configuration::FILENAME)?;

    Ok(())
}

/// The meta information about this program.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppInfo {
    name: String,
    version: String,
    is_manager: bool,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: rim_common::get_version_info!(),
            is_manager: false,
        }
    }
}

impl AppInfo {
    pub fn get() -> &'static Self {
        APP_INFO.get_or_init(Self::default)
    }
    pub fn name() -> &'static str {
        &Self::get().name
    }
    pub fn version() -> &'static str {
        &Self::get().version
    }
    /// Return `true` if this app is currently running in manager mode.
    pub fn is_manager() -> bool {
        Self::get().is_manager
    }

    /// Return the path of directory where `rim` was installed on.
    ///
    /// Note: In early builds, we stored current executable directly in `install_dir`,
    /// which contains configuration files that help us determine this dir.
    /// However, it's not suitable anymore as we have linked binaries now.
    ///
    /// So, in order to determine the `install_dir` after installation, we have to
    /// store the configuration files under a system's well known `config_dir`,
    /// therefore we can use those to determine which dir we have installed.
    ///
    /// After we get the `install_dir` path, these factors need to be verified:
    /// 1. The directory is not root. (It won't, unless get changed by third-party)
    /// 2. The directory is not any well-known path of the OS.
    /// 3. The directory actually contains the `rim` executable.
    ///
    /// # Panic
    /// If the program is not currently running in **manager** mode
    /// or any of the above check fails.
    pub fn get_installed_dir() -> &'static Path {
        if !Self::is_manager() {
            panic!("`get_installed_dir` should only be used in `manager` mode");
        }

        fn inner_() -> Result<PathBuf> {
            let record = InstallationRecord::load_from_config_dir()?;

            // root check
            if record.install_dir.parent().is_none() {
                bail!(t!("install_dir_is_root"));
            }
            // well known dir check, because we are removing the entire install_dir when uninstall,
            // so you can never be too careful.
            let well_known_dirs = [
                dirs::audio_dir(),
                dirs::cache_dir(),
                dirs::config_dir(),
                dirs::config_local_dir(),
                dirs::data_dir(),
                dirs::data_local_dir(),
                dirs::desktop_dir(),
                dirs::document_dir(),
                dirs::download_dir(),
                dirs::executable_dir(),
                dirs::font_dir(),
                dirs::home_dir(),
                dirs::picture_dir(),
                dirs::preference_dir(),
                dirs::public_dir(),
                dirs::runtime_dir(),
                dirs::state_dir(),
                dirs::template_dir(),
                dirs::video_dir(),
            ];
            for maybe_dir in well_known_dirs {
                let Some(dir) = maybe_dir else {
                    continue;
                };
                if dir == record.install_dir {
                    bail!(t!("install_dir_is_os_dir", path = dir.display()));
                }
            }
            // rim existence check
            if !record
                .install_dir
                .join(exe!(build_config().app_name()))
                .exists()
            {
                bail!("installation directory is incorrect");
            }

            Ok(record.install_dir.clone())
        }

        INSTALL_DIR_ONCE.get_or_init(|| inner_().expect("unable to determine install dir"))
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalOpts;

    #[test]
    fn global_opts_set_and_get() {
        GlobalOpts::set(true, false, true, true, false);

        let opts = GlobalOpts::get();
        assert_eq!(opts.verbose, true);
        assert_eq!(opts.quiet, false);
        assert_eq!(opts.yes_to_all, true);
        assert_eq!(opts.no_modify_env(), true);
        // no-modify-path is dictated by no-modify-env, because PATH is part of env var
        assert_eq!(opts.no_modify_path(), true);
    }
}
