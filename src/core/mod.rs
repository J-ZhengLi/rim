//! Core functionalities of this program
//!
//! Including configuration, toolchain, toolset management.

pub mod components;
mod custom_instructions;
pub(crate) mod directories;
pub mod install;
mod locales;
pub(crate) mod os;
pub(crate) mod parser;
mod path_ext;
pub(crate) mod rustup;
pub mod toolkit;
pub(crate) mod tools;
pub mod try_it;
pub(crate) mod uninstall;
pub mod update;

// re-exports
pub use locales::Language;
pub(crate) use path_ext::PathExt;
use serde::{Deserialize, Serialize};

use crate::{cli, utils};
use std::{env, sync::OnceLock};

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

pub(crate) const RIM_DIST_SERVER: &str = "https://rust-mirror.obs.cn-north-4.myhuaweicloud.com";

/// Globally cached values
static GLOBAL_OPTS: OnceLock<GlobalOpts> = OnceLock::new();
static APP_INFO: OnceLock<AppInfo> = OnceLock::new();

/// Representing the options that user pass to the program, such as
/// `--yes`, `--no-modify-path`, etc.
///
/// This struct will be stored globally for easy access, also make
/// sure the [`set`](GlobalOpts::set) function is called exactly once
/// to initialize the global singleton.
// TODO: add verbose and quiest options
#[derive(Debug, Default)]
pub(crate) struct GlobalOpts {
    pub(crate) verbose: bool,
    pub(crate) quiet: bool,
    pub(crate) yes_to_all: bool,
    no_modify_env: bool,
    no_modify_path: bool,
}

impl GlobalOpts {
    /// Initialize a new object and store it globally, will also return a
    /// static reference to the global stored value.
    ///
    /// Note that the value cannot be updated once initialized.
    pub(crate) fn set(
        verbose: bool,
        quiet: bool,
        yes: bool,
        no_modify_env: bool,
        no_modify_path: bool,
    ) -> &'static Self {
        GLOBAL_OPTS.get_or_init(|| Self {
            verbose,
            quiet,
            yes_to_all: yes,
            no_modify_env,
            no_modify_path,
        })
    }

    /// Get the stored global options.
    ///
    /// # Panic
    /// Will panic if `Self` has not been initialized, make sure [`GlobalOpts::new`] is called
    /// prior to this call.
    pub(crate) fn get() -> &'static Self {
        if let Some(opts) = GLOBAL_OPTS.get() {
            opts
        } else {
            GLOBAL_OPTS.get_or_init(|| {
                warn!("no running options set, fallback to using default options");
                GlobalOpts::default()
            })
        }
    }

    /// Return `true` if either one of `no-modify-path` or `no-modify-env` was set to `true`
    pub(crate) fn no_modify_path(&self) -> bool {
        self.no_modify_path || self.no_modify_env
    }

    /// Return `true` if `no-modify-env` was set to `true`
    pub(crate) fn no_modify_env(&self) -> bool {
        self.no_modify_env
    }
}

/// Representing the execution mode of this program.
///
/// # Example
/// - In [`Installer`](Mode::Installer) (a.k.a `setup` mode), this program
///     does initial setup and install rust toolkit for the user.
/// - In [`Manager`](Mode::Manager) mode, this program can be used for
///     updating, uninstalling the toolkits etc.
pub enum Mode {
    Manager(Box<cli::Manager>),
    Installer(Box<cli::Installer>),
}

impl Mode {
    fn manager(manager_callback: Option<Box<dyn FnOnce(&cli::Manager)>>) -> Self {
        let cli = cli::parse_manager_cli();
        if let Some(cb) = manager_callback {
            cb(&cli);
        }

        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: t!("manager_title", product = t!("product")).into(),
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            is_manager: true,
        });

        Self::Manager(Box::new(cli))
    }
    fn installer(installer_callback: Option<Box<dyn FnOnce(&cli::Installer)>>) -> Self {
        let cli = cli::parse_installer_cli();
        if let Some(cb) = installer_callback {
            cb(&cli);
        }

        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: t!("installer_title", product = t!("product")).into(),
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            is_manager: false,
        });

        Self::Installer(Box::new(cli))
    }

    /// Automatically determain which mode that this program is running as.
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
            Err(_) => match utils::lowercase_program_name() {
                Some(s) if s.contains("manager") => Self::manager(manager_callback),
                // fallback to installer mode
                _ => Self::installer(installer_callback),
            },
        }
    }
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
            version: env!("CARGO_PKG_VERSION").to_string(),
            is_manager: false,
        }
    }
}

impl AppInfo {
    pub fn get() -> &'static Self {
        APP_INFO.get_or_init(|| Self::default())
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
        // no-modfy-path is dictated by no-modify-env, because PATH is part of env var
        assert_eq!(opts.no_modify_path(), true);
    }
}
