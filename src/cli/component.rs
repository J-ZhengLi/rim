use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use clap::{Subcommand, ValueHint};
use rim_common::types::{ToolInfo, ToolInfoDetails, ToolkitManifest};
use url::Url;

use crate::{
    components::{split_components, Component},
    fingerprint::InstallationRecord,
    AppInfo, InstallConfiguration, ToolkitManifestExt, UninstallConfiguration,
};

use super::{
    common::{self, ComponentDecoration, ComponentListBuilder, Confirm},
    ExecStatus, ManagerSubcommands,
};

#[derive(Subcommand, Debug, Clone)]
pub enum ComponentCommand {
    /// Install components
    #[command(alias = "add")]
    Install {
        /// Allow insecure connections when download packages from server.
        #[arg(short = 'k', long)]
        insecure: bool,
        /// The list of components to install, check `list component` for available options
        #[arg(value_name = "COMPONENTS", value_delimiter = ',')]
        components: Vec<String>,
        /// Specify another server to download Rust toolchain components.
        #[arg(long, value_name = "URL", value_hint = ValueHint::Url)]
        rustup_dist_server: Option<Url>,
    },
    /// Uninstall components
    #[command(alias = "remove")]
    Uninstall {
        /// The list of components to uninstall, check `list component --installed` for available options
        #[arg(value_name = "COMPONENTS", value_delimiter = ',')]
        components: Vec<String>,
    },
}

impl ComponentCommand {
    fn execute(&self) -> Result<()> {
        match self {
            Self::Install {
                components,
                insecure,
                rustup_dist_server,
            } => install_components(components, *insecure, rustup_dist_server),
            Self::Uninstall { components } => uninstall_components(components),
        }
    }
}

pub(super) fn execute(cmd: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::Component { command } = cmd else {
        return Ok(ExecStatus::default());
    };

    command.execute()?;

    Ok(ExecStatus::new_executed())
}

fn install_components(
    components: &[String],
    insecure: bool,
    rustup_dist_server: &Option<Url>,
) -> Result<()> {
    let manifest = ToolkitManifest::load_from_install_dir()?;
    let all_comps = manifest.current_target_components(true)?;

    // make a set out of components to:
    // 1. remove duplicates; 2. search faster;
    let mut comp_set: HashSet<&String> = components.iter().collect();
    // collect the components that needed to be installed
    let comps_to_install = all_comps
        .into_iter()
        .filter(|c| comp_set.remove(&c.name))
        .collect::<Vec<_>>();

    // some name of tools might not be installable component, reject them.
    if !comp_set.is_empty() {
        let names = comp_set
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        bail!(t!("invalid_components", list = names));
    }
    if comps_to_install.is_empty() {
        info!("{}", t!("task_success"));
        return Ok(());
    }

    let (tc_components, tools) = split_components(comps_to_install);

    let mut config = InstallConfiguration::new(AppInfo::get_installed_dir(), &manifest)?
        .insecure(insecure)
        .with_rustup_dist_server(rustup_dist_server.clone());
    config.install_toolchain_components(&tc_components)?;
    config.install_tools(&tools)?;

    info!("{}", t!("task_success"));

    // notify user that they might need to source the current shell again
    #[cfg(unix)]
    {
        use rim_common::types::ToolKind;
        let g_opts = crate::core::GlobalOpts::get();
        if !(g_opts.quiet || g_opts.no_modify_env())
            && tools.iter().any(|(_, info)| {
                matches!(info.kind(), Some(ToolKind::DirWithBin | ToolKind::Custom))
            })
        {
            common::show_source_hint(&config.install_dir);
        }
    }
    Ok(())
}

fn uninstall_components(components: &[String]) -> Result<()> {
    let record = InstallationRecord::load_from_config_dir()?;

    // make a set out of components to:
    // 1. remove duplicates; 2. search faster;
    let mut comp_set: HashSet<&String> = components.iter().collect();
    // collect the toolchain components that needed to be removed
    let tc_comps_to_remove = record
        .installed_toolchain_components()
        .into_iter()
        .filter(|c| comp_set.remove(&c.name))
        .collect::<Vec<_>>();
    // collect the tools that needed to be removed
    let tools_to_remove = record
        .tools
        .into_iter()
        .filter(|(name, _)| comp_set.remove(name))
        .collect::<HashMap<_, _>>();

    // some tools are left out, those might have typos, or was already removed,
    // warn about them then move on.
    if !comp_set.is_empty() {
        let names = comp_set
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(",");
        warn!(
            "{}",
            t!("skip_non_exist_component_uninstallation", tool = names)
        );
    }
    if tc_comps_to_remove.is_empty() && tools_to_remove.is_empty() {
        info!("{}", t!("task_success"));
        return Ok(());
    }

    let mut config = UninstallConfiguration::init(None)?;
    config.remove_toolchain_components(&tc_comps_to_remove, 50.0)?;
    config.remove_tools(tools_to_remove, 50.0)?;
    info!("{}", t!("task_success"));
    Ok(())
}

/// Ask user about a list of component's name to install.
///
/// This is done by:
/// 1. Load installable components from local manifest.
/// 2. Print the component names
/// 3. Ask user input, which should be a list of indexes
/// 4. convert list of indexes into list of names then return it.
// TODO: reduce copy-pasta code from `cli::install::custom_component_choices`
pub(super) fn collect_components_to_add() -> Result<Vec<String>> {
    let manifest = ToolkitManifest::load_from_install_dir()?;
    let all_components = manifest.current_target_components(true)?;
    if all_components.is_empty() {
        bail!(t!("no_installable_components"));
    };

    // format component names to show when asking for user input
    let list_of_comps = ComponentListBuilder::new(&all_components)
        .show_desc(true)
        .decorate(ComponentDecoration::Selection)
        .build();
    // ask user input in a loop, breaks if user confirms the selection
    loop {
        let question = format!(
            "{}: ({})",
            t!("select_components_to_install"),
            t!("select_components_cli_hint")
        );
        let choices = common::question_multi_choices(question, &list_of_comps, "")?;
        // convert input vec to set for faster lookup
        // Note: user input index are started from 1.
        let index_set: HashSet<usize> = choices.into_iter().collect();

        // convert the input indexes to `ComponentChoices`,
        let choices = common::component_choices_with_constrains(&all_components, |idx, _| {
            index_set.contains(&(idx + 1))
        });

        common::show_confirmation(None, &choices, false)?;

        match common::confirm_options()? {
            Confirm::Yes => {
                return Ok(choices.values().map(|c| c.name.clone()).collect());
            }
            Confirm::No => (),
            Confirm::Abort => return Ok(vec![]),
        }
    }
}

/// Ask user about a list of component's name to uninstall.
///
/// This is done by:
/// 1. Load components from installation record.
/// 2. Print the component names
/// 3. Ask user input, which should be a list of indexes
/// 4. convert list of indexes into list of names then return it.
pub(super) fn collect_components_to_remove() -> Result<Vec<String>> {
    // step 1: load installed component names
    let record = InstallationRecord::load_from_config_dir()?;
    // we need to convert these records to `Component`
    let mut all_installed_comps = record
        .installed_toolchain_components()
        .iter()
        .map(Component::from)
        .collect::<Vec<_>>();
    let tools = record.tools.iter().map(|(name, tool_rec)| {
        let info = ToolInfo::new_detailed(
            ToolInfoDetails::new().with_dependencies(tool_rec.dependencies.clone()),
        );
        Component::new(name).with_tool_installer(&info)
    });
    all_installed_comps.extend(tools);

    // step 2: format component names to show when asking for user input
    let list_of_comps = ComponentListBuilder::new(&all_installed_comps).build();

    // ask user input in a loop, breaks if user confirms the selection
    loop {
        let choices =
            common::question_multi_choices(t!("select_components_to_remove"), &list_of_comps, "")?;
        // convert input vec to set for faster lookup
        // Note: user input index are started from 1.
        let index_set: HashSet<usize> = choices.into_iter().collect();

        // convert the input indexes to `ComponentChoices`,
        let choices = common::component_choices_with_constrains(&all_installed_comps, |idx, _| {
            index_set.contains(&(idx + 1))
        });

        common::show_confirmation(None, &choices, true)?;

        match common::confirm_options()? {
            Confirm::Yes => {
                return Ok(choices.values().map(|c| c.name.clone()).collect());
            }
            Confirm::No => (),
            Confirm::Abort => return Ok(vec![]),
        }
    }
}
