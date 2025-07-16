use std::process::Command;

use anyhow::{bail, Result};
use rim_common::utils;

use crate::{
    common,
    mocked::{self, installation, manager, server},
};

pub(super) const RUN_HELP: &str = r#"
Build and run RIM for testing purpose

Usage: cargo dev run [OPTIONS]

Options:
    -c, --cli       Run with commandline interface
    -g, --gui       Run with graphical interface (default)
    -i, --installer
                    Run RIM in installer mode (default)
    -m, --manager   Run RIM in manager mode
    -h, -help       Print this help message
"#;

#[derive(Debug, Clone, Copy)]
pub(super) enum RunMode {
    Manager { no_gui: bool },
    Installer { no_gui: bool },
}

impl Default for RunMode {
    fn default() -> Self {
        Self::Installer { no_gui: false }
    }
}

impl RunMode {
    pub(super) fn switch_to_manager(&mut self) {
        match self {
            Self::Installer { no_gui } => *self = Self::Manager { no_gui: *no_gui },
            Self::Manager { .. } => (),
        }
    }
    pub(super) fn switch_to_installer(&mut self) {
        match self {
            Self::Installer { .. } => (),
            Self::Manager { no_gui } => *self = Self::Installer { no_gui: *no_gui },
        }
    }
    pub(super) fn set_no_gui(&mut self, yes: bool) {
        match self {
            Self::Manager { no_gui } | Self::Installer { no_gui } => *no_gui = yes,
        }
    }
    pub(super) fn run(&self, args: &[String]) -> Result<()> {
        println!("running with args: {args:?}");

        // replace home env to prevent modifying the actually HOME var
        let home = mocked::mocked_home();
        // remove previous artifacts including configs
        utils::remove(home)?;
        std::env::set_var("HOME", home);
        #[cfg(windows)]
        std::env::set_var("USERPROFILE", home);

        // use test toolkit by default unless specified
        let edition = std::env::var("DEBUG_EDITION").unwrap_or("test".to_string());

        match self {
            Self::Installer { no_gui } => {
                let status = if *no_gui {
                    Command::new("cargo")
                        .env("EDITION", edition)
                        .args(["run", "--"])
                        .args(args)
                        .status()?
                } else {
                    common::pnpm_cmd()
                        .env("EDITION", edition)
                        .args(["run", "tauri", "dev", "--"])
                        .args(args)
                        .status()?
                };

                if !status.success() {
                    bail!("unable to run rim in installer mode");
                }
            }
            Self::Manager { no_gui } => {
                // a mocked server is needed to run most of function in manager
                server::generate_rim_server_files()?;

                // generate a fake manager binary with higher version so we
                // can test the self update.
                if args.iter().any(|arg| arg == "update") {
                    manager::generate()?;
                }

                installation::generate_and_run_manager(*no_gui, args)?;
            }
        }
        Ok(())
    }
}
