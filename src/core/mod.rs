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
mod toolkit_manifest_ext;
pub(crate) mod tools;
pub mod try_it;
pub(crate) mod uninstall;
pub mod update;

// re-exports
pub use locales::Language;
pub(crate) use path_ext::PathExt;
pub use toolkit_manifest_ext::*;

use crate::{cli, fingerprint::InstallationRecord};
use anyhow::{bail, Context, Result};
use rim_common::{build_config, types::TomlParser, utils};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
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
static GLOBAL_OPTS: OnceLock<GlobalOpts> = OnceLock::new();
static APP_INFO: OnceLock<AppInfo> = OnceLock::new();
static INSTALL_DIR_ONCE: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn default_rustup_dist_server() -> &'static Url {
    &build_config().rustup_dist_server
}

pub(crate) fn default_rustup_update_root() -> &'static Url {
    &build_config().rustup_update_root
}

pub(crate) fn rim_dist_server() -> Url {
    if let Ok(env_server) = env::var("RIM_DIST_SERVER") {
        if let Ok(url) = env_server.parse::<Url>() {
            return url;
        }
    }

    build_config().rim_dist_server.clone()
}

pub(crate) fn default_cargo_registry() -> (&'static str, &'static str) {
    let cfg = build_config();

    (&cfg.cargo.registry_name, &cfg.cargo.registry_url)
}

/// Representing the options that user pass to the program, such as
/// `--yes`, `--no-modify-path`, etc.
///
/// This struct will be stored globally for easy access, also make
/// sure the [`set`](GlobalOpts::set) function is called exactly once
/// to initialize the global singleton.
// TODO: add verbose and quiet options
#[derive(Debug, Default, Clone, Copy)]
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
    ///
    /// # Panic
    /// Panics if the options were already set.
    pub(crate) fn set(
        verbose: bool,
        quiet: bool,
        yes: bool,
        no_modify_env: bool,
        no_modify_path: bool,
    ) {
        let opts = Self {
            verbose,
            quiet,
            yes_to_all: yes,
            no_modify_env,
            no_modify_path,
        };
        GLOBAL_OPTS
            .set(opts)
            .expect("failed to set `GlobalOpts` as it was already set somewhere else");
    }

    /// Get the stored global options.
    pub(crate) fn get() -> &'static Self {
        GLOBAL_OPTS.get_or_init(|| {
            // no running options set, fallback to default
            GlobalOpts::default()
        })
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
/// Each variant contains a parsed CLI arg matches, which can fail
/// if the user pass some invalid args to the program.
///
/// # Example
/// - In [`Installer`](Mode::Installer) (a.k.a `setup` mode), this program
///   does initial setup and install rust toolkit for the user.
/// - In [`Manager`](Mode::Manager) mode, this program can be used for
///   updating, uninstalling toolkit etc.
pub enum Mode {
    Manager(Result<cli::Manager>),
    Installer(Result<cli::Installer>),
}

impl Mode {
    fn manager(manager_callback: Option<Box<dyn FnOnce(&cli::Manager)>>) -> Self {
        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: utils::build_cfg_locale("manager_title").into(),
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

        Self::Manager(maybe_args)
    }
    fn installer(installer_callback: Option<Box<dyn FnOnce(&cli::Installer)>>) -> Self {
        // cache app info
        APP_INFO.get_or_init(|| AppInfo {
            name: utils::build_cfg_locale("installer_title").into(),
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

    /// Try guessing the installation directory base on current exe path, and return the path.
    ///
    /// This program should be installed directly under `install_dir`,
    /// but in case someone accidentally put this binary into some other locations such as
    /// the root, we should definitely NOT remove the parent dir after installation.
    /// Therefor we need some checks:
    /// 1. Make sure the parent directory is not root.
    /// 2. Make sure there is a `.fingerprint` file alongside current binary.
    /// 3. Make sure the parent directory matches the recorded `root` path in the fingerprint file.
    ///
    /// # Panic
    /// If the program is not currently running in **manager** mode
    /// or any of the above check fails.
    pub fn get_installed_dir() -> &'static Path {
        if !Self::is_manager() {
            panic!("`get_installed_dir` should only be used in `manager` mode");
        }

        fn inner_() -> Result<PathBuf> {
            let maybe_install_dir = utils::parent_dir_of_cur_exe()?;

            // the first check
            if maybe_install_dir.parent().is_none() {
                bail!("it appears that this program was mistakenly installed in root directory");
            }
            // the second check
            if !maybe_install_dir
                .join(InstallationRecord::FILENAME)
                .is_file()
            {
                bail!("installation record cannot be found");
            }
            // the third check
            let fp = InstallationRecord::load_from_dir(&maybe_install_dir)
                .context("'.fingerprint' file exists but cannot be loaded")?;
            if fp.root != maybe_install_dir {
                bail!(
                    "`.fingerprint` file exists but the installation root in it \n\
                    does not match the one its in"
                );
            }

            Ok(maybe_install_dir.to_path_buf())
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
