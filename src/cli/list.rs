use std::io::Write;

use anyhow::Result;
use clap::Subcommand;

use super::{handle_user_choice, GlobalOpts, ManagerSubcommands};
use crate::{
    components,
    fingerprint::InstallationRecord,
    toolkit::{toolkits_from_server, Toolkit},
    toolset_manifest::ToolsetManifest,
    utils::blocking,
};

#[derive(Subcommand, Debug, Default, Clone, Copy)]
pub(super) enum ListCommand {
    /// Show components that are available in current target.
    #[default]
    Component,
    /// Show available toolkits.
    Toolkit,
}

impl ListCommand {
    fn execute(&self, installed: bool) -> Result<()> {
        match self {
            Self::Component => list_components(installed, None),
            Self::Toolkit => blocking!(list_toolkits(installed)),
        }
    }
}

pub(super) fn execute(cmd: &ManagerSubcommands) -> Result<bool> {
    let ManagerSubcommands::List { installed, command } = cmd else {
        return Ok(false);
    };

    // `command` should either be passed from commandline option or being repeatly
    // asked from user interaction until determined, which means it couldn't be `none`,
    // but we still fallback to default in case something bad happens.
    let sub_cmd = command.unwrap_or_default();
    sub_cmd.execute(*installed)?;

    Ok(true)
}

/// Ask user about list options, return a `bool` indicates whether the user wishs to continue.
pub(super) fn ask_list_command() -> Result<Option<ListCommand>> {
    let cmd = handle_user_choice!(
        t!("choose_an_option"), 1,
        {
            1 t!("component") => { Some(ListCommand::Component) },
            2 t!("toolkit") => { Some(ListCommand::Toolkit) },
            3 t!("back") => { None }
        }
    );
    Ok(cmd)
}

/// Print a list of components and return them.
pub(crate) fn list_components(
    installed_only: bool,
    manifest: Option<&ToolsetManifest>,
) -> Result<()> {
    let components = if let Some(mf) = manifest {
        mf.current_target_components(true)?
    } else {
        let fp = InstallationRecord::load_from_install_dir()?;
        components::all_components_from_installation(&fp)?
    };

    let comp_iter = components.iter().skip(1);
    let verbose = GlobalOpts::get().verbose;
    let mut stdout = std::io::stdout();

    writeln!(&mut stdout)?;
    if installed_only {
        let installed_comps = comp_iter
            .filter_map(|comp| {
                comp.installed.then_some(if verbose {
                    let version = comp
                        .version
                        .as_ref()
                        .map(|ver| format!(" {ver}"))
                        .unwrap_or_default();
                    format!("{}{version}", comp.name)
                } else {
                    comp.name.clone()
                })
            })
            .collect::<Vec<_>>();
        if installed_comps.is_empty() {
            writeln!(&mut stdout, "{}", t!("no_component_installed"))?;
        } else {
            for comp in installed_comps {
                writeln!(&mut stdout, "{comp}")?;
            }
        }
    } else {
        for comp in comp_iter {
            let version = if verbose {
                comp.version
                    .as_ref()
                    .map(|ver| format!(" {ver}"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let installed_suffix = if comp.installed {
                format!(" ({})", t!("installed"))
            } else {
                String::new()
            };
            writeln!(&mut stdout, "{}{version}{installed_suffix}", comp.name)?;
        }
    }
    Ok(())
}

async fn list_toolkits(installed_only: bool) -> Result<()> {
    let maybe_installed_tk = Toolkit::installed(false).await?;
    let mut stdout = std::io::stdout();

    writeln!(&mut stdout)?;
    if installed_only {
        if let Some(mutex) = maybe_installed_tk {
            let tk = mutex.lock().await;
            writeln!(&mut stdout, "{} {}", tk.name, tk.version)?;
        } else {
            writeln!(&mut stdout, "{}", t!("no_toolkit_installed"))?;
        }
    } else {
        let all_toolkits = toolkits_from_server(false).await?
            .into_iter()
            .map(|tk| async move {
                let installed_suffix = if matches!(maybe_installed_tk, Some(mutex) if *mutex.lock().await == tk) {
                    format!(" ({})", t!("installed"))
                } else {
                    String::new()
                };
                format!("{} {}{installed_suffix}", tk.name, tk.version)
            });
        for toolkit in all_toolkits {
            let toolkit = toolkit.await;
            writeln!(&mut stdout, "{toolkit}")?;
        }
    }
    Ok(())
}
