// bool::then_some().unwrap_or*() is much clean, clippy, why be a hater?
#![allow(clippy::obfuscated_if_else)]

mod common;
mod dist;
mod locales;
mod mocked;
mod run;
mod toolkits_parser;
mod vendor;

use anyhow::{anyhow, Context, Result};
use dist::DIST_HELP;
use locales::LocaleCommand;
use mocked::server;
use run::RunMode;
use std::env;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use toolkits_parser::ReleaseMode;
use vendor::{VendorMode, VENDOR_HELP};

const HELP: &str = r#"
Usage: cargo dev [OPTIONS] [COMMAND]

Options:
    -h, -help       Print this help message

Commands:
    d, dist         Generate release binaries
    r, run          Build and run RIM for testing purpose
    vendor          Download packages for offline package build
    mock-server
                    Generate a mocked rustup dist server
    l, locale       Sort or check the locale JSON files
"#;
const MOCK_HELP: &str = r#"
Generate a mocked rustup dist server and rim dist server

Usage: cargo dev mock-server [OPTIONS]

Options:
        --root      Specify another directory for generated files
    -h, -help       Print this help message
"#;

#[derive(Debug)]
enum DevCmd {
    Dist {
        mode: ReleaseMode,
        binary_only: bool,
        build_target: String,
        dist_targets: Vec<String>,
        name: Option<String>,
    },
    Run {
        mode: RunMode,
        args: Vec<String>,
    },
    Mock {
        root: Option<PathBuf>,
    },
    Vendor {
        mode: VendorMode,
        name: Option<String>,
        targets: Vec<String>,
        all_targets: bool,
        clear: bool,
    },
    Locale(LocaleCommand),
}

impl DevCmd {
    fn execute(self) -> Result<()> {
        match self {
            Self::Dist {
                mode,
                binary_only,
                build_target,
                dist_targets,
                name,
            } => dist::dist(mode, binary_only, name, build_target, dist_targets)?,
            Self::Run { mode, args } => mode.run(&args)?,
            Self::Vendor {
                mode,
                name,
                targets,
                all_targets,
                clear,
            } => vendor::vendor(mode, name, targets, all_targets, clear)?,
            Self::Mock { root } => {
                server::generate_rim_server_files()?;
                server::generate_rustup_server_files(root)?
            }
            Self::Locale(cmd) => locales::run(cmd)?,
        }
        Ok(())
    }
}

fn current_exe() -> PathBuf {
    env::current_exe().expect("failed to get the path of current binary")
}

fn main() -> Result<ExitCode> {
    let mut args = std::env::args().skip(1);
    let mut stdout = stdout();

    let Some(subcmd) = args.next() else {
        writeln!(&mut stdout, "{HELP}")?;
        return Ok(ExitCode::FAILURE);
    };

    let cmd = match subcmd.to_lowercase().as_str() {
        "-h" | "--help" => {
            writeln!(&mut stdout, "{HELP}")?;
            return Ok(ExitCode::SUCCESS);
        }
        "d" | "dist" => {
            let mut binary_only = false;
            let mut mode = ReleaseMode::Both;
            let mut build_target = env!("TARGET").to_string();
            let mut dist_targets = vec![];
            let mut name = None;

            while let Some(arg) = args.next().as_deref() {
                match arg {
                    "-h" | "--help" => {
                        writeln!(&mut stdout, "{DIST_HELP}")?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "-n" | "--name" => name = args.next(),
                    "-t" | "--target" => {
                        build_target = args.next().context("expecting a target triple string")?
                    }
                    "--for" => dist_targets.extend(split_values_by_comma(args.next())?),
                    "--cli" => mode = ReleaseMode::Cli,
                    "--gui" => mode = ReleaseMode::Gui,
                    "-b" | "--binary-only" => binary_only = true,
                    _ => (),
                }
            }
            DevCmd::Dist {
                mode,
                binary_only,
                name,
                build_target,
                dist_targets,
            }
        }
        "vendor" => {
            let mut name = None;
            let mut mode = VendorMode::Regular;
            let mut targets = vec![];
            let mut all_targets = false;
            let mut clear = false;
            while let Some(arg) = args.next().as_deref() {
                match arg {
                    "-h" | "--help" => {
                        writeln!(&mut stdout, "{VENDOR_HELP}")?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "-a" | "--all-targets" => all_targets = true,
                    "-c" | "--clear" => clear = true,
                    "-n" | "--name" => name = args.next(),
                    "--download-only" => mode = VendorMode::DownloadOnly,
                    "--split-only" => mode = VendorMode::SplitOnly,
                    "--for" => targets.extend(split_values_by_comma(args.next())?),
                    s => {
                        writeln!(&mut stdout, "invalid argument '{s}'")?;
                        return Ok(ExitCode::FAILURE);
                    }
                }
            }
            if targets.is_empty() {
                targets.push(env!("TARGET").to_string());
            }
            DevCmd::Vendor {
                mode,
                name,
                targets,
                all_targets,
                clear,
            }
        }
        "run" => {
            let mut is_extra_arg = false;
            let mut extra_args = vec![];
            let mut mode = RunMode::default();

            while let Some(arg) = args.next().as_deref() {
                match arg {
                    "-h" | "--help" if !is_extra_arg => {
                        writeln!(&mut stdout, "{}", run::RUN_HELP)?;
                        return Ok(ExitCode::SUCCESS);
                    }
                    "-c" | "--cli" => mode.set_no_gui(true),
                    "-g" | "--gui" => mode.set_no_gui(false),
                    "-i" | "--installer" => mode.switch_to_installer(),
                    "-m" | "--manager" => mode.switch_to_manager(),
                    "--" if !is_extra_arg => is_extra_arg = true,
                    a => {
                        if is_extra_arg {
                            extra_args.push(a.into());
                        }
                    }
                }
            }

            DevCmd::Run {
                mode,
                args: extra_args,
            }
        }
        "mock-server" => match args.next().as_deref() {
            Some("-r" | "--root") => DevCmd::Mock {
                root: Some(args.next().expect("missing arg value for 'root'").into()),
            },
            // includes '-h', '--help', and all other unspecified input to show help
            Some(_) => {
                writeln!(&mut stdout, "{MOCK_HELP}")?;
                return Ok(ExitCode::SUCCESS);
            }
            None => DevCmd::Mock { root: None },
        },
        "l" | "locale" => match args.next().as_deref() {
            Some("c" | "check") => DevCmd::Locale(LocaleCommand::Check),
            Some("f" | "fmt") => DevCmd::Locale(LocaleCommand::Fmt),
            _ => {
                locales::print_help(&mut stdout)?;
                return Ok(ExitCode::SUCCESS);
            }
        },
        s => {
            writeln!(
                &mut stdout,
                "invalid argument '{s}', check 'cargo dev --help' for available options"
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };
    cmd.execute()?;

    Ok(ExitCode::SUCCESS)
}

fn split_values_by_comma(maybe_args: Option<String>) -> Result<Vec<String>> {
    maybe_args
        .map(|t| t.split(',').map(ToOwned::to_owned).collect::<Vec<_>>())
        .ok_or_else(|| anyhow!("expecting comma separated target triple string"))
}
