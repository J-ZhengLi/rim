#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate log;

mod command;
mod common;
mod consts;
mod error;
mod installer_mode;
mod manager_mode;
mod progress;

use anyhow::Result;
use rim::{cli::ExecutableCommand, Mode};
use rim_common::utils;
use std::sync::mpsc::{self, Receiver};

i18n!("../../locales", fallback = "en-US");

fn main() -> Result<()> {
    utils::use_current_locale();

    let mode = Mode::detect(None, None);
    let msg_recv = setup_logger();
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

/// Configure the logger to use a communication channel ([`mpsc`]),
/// allowing us to send logs across threads.
///
/// This will return a log message's receiver which can be used to emitting
/// messages onto [`tauri::Window`]
fn setup_logger() -> Receiver<String> {
    let (msg_sender, msg_recvr) = mpsc::channel::<String>();
    if let Err(e) = utils::Logger::new().sender(msg_sender).setup() {
        // TODO: make this error more obvious
        eprintln!(
            "Unable to setup logger, cause: {e}\n\
            The program will continues to run, but it might not functioning correctly."
        );
    }
    msg_recvr
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
