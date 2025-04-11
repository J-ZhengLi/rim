#![deny(unused_must_use)]
#![allow(clippy::ptr_arg, clippy::type_complexity)]

#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate log;
#[macro_use]
extern crate rim_common;

pub mod cli;
mod core;

// Exports
pub use core::install::{default_install_dir, EnvConfig, InstallConfiguration};
pub use core::parser::{configuration, fingerprint};
pub use core::try_it::try_it;
pub use core::uninstall::UninstallConfiguration;
pub use core::{components, toolkit, update, AppInfo, Language, Mode};
pub use core::{get_toolkit_manifest, ToolkitManifestExt};

i18n!("locales", fallback = "en");
