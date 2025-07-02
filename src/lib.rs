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
pub use core::parser::fingerprint;
pub use core::try_it::try_it;
pub use core::uninstall::UninstallConfiguration;
pub use core::{components, toolkit, update, AppInfo, GlobalOpts, Mode, ToolkitManifestExt};
pub use core::{
    default_cargo_registry, default_rustup_dist_server, default_rustup_update_root,
    get_toolkit_manifest,
};

i18n!("locales", fallback = "en-US");
