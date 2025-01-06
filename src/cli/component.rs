use std::collections::HashSet;

use anyhow::{bail, Result};
use clap::Subcommand;

use crate::{components::component_list_to_tool_map, get_installed_dir, toolset_manifest::ToolsetManifest, InstallConfiguration};

use super::ManagerSubcommands;

#[derive(Subcommand, Debug)]
pub(super) enum ComponentCommand {
    /// Install a list of components (separated by comma), check `list component` for available options
    #[command(alias = "add")]
    Install {
        /// Allow insecure connections when download packages from server.
        #[arg(short = 'k', long)]
        insecure: bool,
        /// The list of components to install
        #[arg(value_delimiter = ',')]
        components: Vec<String>,
    },
    /// Uninstall a list of components (separated by comma), check `list --installed component` for available options
    #[command(alias = "remove")]
    Uninstall {
        /// The list of components to uninstall
        #[arg(value_delimiter = ',')]
        components: Vec<String>,
    },
}

impl ComponentCommand {
    fn execute(&self) -> Result<()> {
        match self {
            Self::Install { components, insecure } => install_components(*insecure, components),
            Self::Uninstall { components } => todo!("uninstall components: {components:?}"),
        }
    }
}

pub(super) fn execute(cmd: &ManagerSubcommands) -> Result<bool> {
    let ManagerSubcommands::Component { command } = cmd else {
        return Ok(false);
    };

    command.execute()?;

    Ok(true)
}

fn install_components(insecure: bool, names: &[String]) -> Result<()> {
    let manifest = ToolsetManifest::load_from_install_dir()?;
    let all_comps = manifest.current_target_components(false)?;

    // first we remove the duplicated names
    let mut names: HashSet<&String> = HashSet::from_iter(names);
    
    // collect components that can be installed
    // Note: `good_comps = all_comps.iter().filter(|comp| names.remove(&comp.name))`
    // causes mutable borrow error.
    let mut good_comps = vec![];
    for comp in &all_comps {
        println!("comp: {:?}", comp.name);
        if names.remove(&comp.name) {
            good_comps.push(comp);
        }
    }
    
    // some invalid component names could be left-out, notify user and abort
    if !names.is_empty() {
        let names_s = names.into_iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");
        bail!(t!("invalid_components", components = names_s));
    }

    let install_dir = get_installed_dir();
    let tools = component_list_to_tool_map(good_comps);
    InstallConfiguration::new(install_dir, &manifest)?
        .insecure(insecure)
        .install_tools(&tools)?;

    Ok(())
}
