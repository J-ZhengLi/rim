#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate log;

mod common;
mod consts;
mod error;
mod installer_mode;
mod manager_mode;

use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::Result;
use rim::{cli::ExecutableCommand, Mode};
use rim_common::utils;

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
    let msg_recv = common::setup_logger();
    match mode {
        Mode::Manager(maybe_args) => {
            run_cli_else_hide_console(&maybe_args)?;
            manager_mode::main(msg_recv, maybe_args)?;
        }
        Mode::Installer(maybe_args) => {
            run_cli_else_hide_console(&maybe_args)?;
            installer_mode::main(msg_recv)?;
        }
    }
    Ok(())
}

/// This GUI program supports commandline interface as well, so if:
///
/// - This was started in CLI mode, execute the command then exit.
/// - This was started in GUI mode, hide the console window on Windows.
fn run_cli_else_hide_console<T: ExecutableCommand>(command_args: &anyhow::Result<T>) -> Result<()> {
    if let Ok(args) = command_args {
        if args.no_gui() {
            args.execute()?;
            std::process::exit(0);
        }
    }

    #[cfg(windows)]
    {
        use winapi::um::winuser::{ShowWindow, SW_HIDE};
        let window = unsafe { winapi::um::wincon::GetConsoleWindow() };
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }

    Ok(())
}
