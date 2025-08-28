//! Separated module to handle uninstallation in command line.

use crate::core::uninstall::UninstallConfiguration;

use super::{common, ExecStatus, ManagerSubcommands};

use anyhow::Result;
use rim_common::{build_config, utils::CliProgress};

/// Execute `uninstall` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::Uninstall { keep_self } = subcommand else {
        return Ok(ExecStatus::default());
    };

    let config = UninstallConfiguration::init(CliProgress::default())?;
    let installed = config.install_record.print_installation();

    // Ask confirmation
    let prompt: String = if !keep_self {
        let app_name = &build_config().app_name();
        if installed.trim().is_empty() {
            t!("uninstall_confirmation", name = app_name).into()
        } else {
            format!(
                "{}\n\n{installed}\n",
                t!("uninstall_all_confirmation", app = app_name)
            )
        }
    } else {
        let toolkit_name = config
            .install_record
            .name
            .clone()
            .unwrap_or_else(|| t!("toolkit").to_string());
        format!(
            "{}\n\n{installed}\n",
            t!("uninstall_confirmation", name = toolkit_name)
        )
    };
    if !common::confirm(prompt, false)? {
        return Ok(ExecStatus::new_executed());
    }

    config.uninstall(!keep_self)?;

    Ok(ExecStatus::new_executed())
}
