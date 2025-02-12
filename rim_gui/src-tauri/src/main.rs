#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate log;

mod common;
mod consts;
mod error;
mod installer_mode;
mod manager_mode;
mod notification;

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;
use rim::{configuration::Configuration, utils, AppInfo, Mode};

i18n!("../../locales", fallback = "en");

static INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

fn main() -> Result<()> {
    utils::use_current_locale();
    let msg_recv = common::setup_logger();

    let mode = Mode::detect(
        Some(Box::new(|installer| {
            if let Some(dir) = installer.install_dir() {
                _ = INSTALL_DIR.set(dir.to_path_buf());
            }
        })),
        None,
    );
    match mode {
        Mode::Manager(maybe_args) => {
            if let Ok(args) = maybe_args {
                if args.no_gui {
                    args.execute()?;
                    return Ok(());
                }
            }
            if let Err(e) = handle_autostart() {
                // log the error but do NOT abort the program
                error!("unable to setup autostart: {e}");
            }
            manager_mode::main(msg_recv)?;
        }
        Mode::Installer(maybe_args) => {
            if let Ok(args) = maybe_args {
                if args.no_gui {
                    args.execute()?;
                    return Ok(());
                }
            }
            installer_mode::main(msg_recv)?;
        }
    }
    Ok(())
}

// TODO: add user setting for this
fn handle_autostart() -> Result<()> {
    // Load configuration to check if autostart is allowed
    let allow_autostart = Configuration::load_from_install_dir().autostart;

    let cur_exe = std::env::current_exe()?;
    let Some(exe_path) = cur_exe.to_str() else {
        log::error!("the path to this application contains invalid UTF-8 character");
        return Ok(());
    };

    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name(AppInfo::name())
        .set_app_path(exe_path)
        .set_use_launch_agent(true)
        .build()?;

    if allow_autostart {
        auto.enable()?;
    } else if auto.is_enabled().unwrap_or_default() {
        auto.disable()?;
    }
    Ok(())
}
