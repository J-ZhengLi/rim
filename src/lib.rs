#![deny(unused_must_use)]
#![allow(clippy::ptr_arg, clippy::type_complexity)]

#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate log;

pub mod cli;
mod core;
pub mod utils;

// Exports
pub use core::install::{default_install_dir, EnvConfig, InstallConfiguration};
pub use core::parser::{fingerprint, get_installed_dir, toolset_manifest, update_checker};
pub use core::try_it::try_it;
pub use core::uninstall::UninstallConfiguration;
pub use core::{components, toolkit, update, AppInfo, Language, Mode};

i18n!("locales", fallback = "en");
