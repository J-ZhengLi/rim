#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate rust_i18n;

mod common;
mod error;
mod installer_mode;
mod manager_mode;
mod notification;

use std::path::PathBuf;
use std::sync::OnceLock;

use error::Result;
use rim::{utils, Mode};

i18n!("../../locales", fallback = "en");

static INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

fn main() -> Result<()> {
    utils::use_current_locale();

    let mode = Mode::detect(
        Some(Box::new(|installer| {
            if let Some(dir) = installer.install_dir() {
                _ = INSTALL_DIR.set(dir.to_path_buf());
            }
        })),
        None,
    );
    match mode {
        Mode::Manager(cli) if cli.no_gui => cli.execute()?,
        Mode::Manager(_) => manager_mode::main()?,
        Mode::Installer(cli) if cli.no_gui => cli.execute()?,
        Mode::Installer(_) => installer_mode::main()?,
    }
    Ok(())
}
