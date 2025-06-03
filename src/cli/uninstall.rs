//! Separated module to handle uninstallation in command line.

use crate::core::uninstall::UninstallConfiguration;

use super::{common, ExecStatus, ManagerSubcommands};

use anyhow::Result;
use rim_common::build_config;

/// Execute `uninstall` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::Uninstall { keep_self } = subcommand else {
        return Ok(ExecStatus::default());
    };

    let config = UninstallConfiguration::init(None)?;
    let installed = config.install_record.print_installation();

    // Ask confirmation
    let prompt = if !keep_self {
        let app_name = &build_config().app_name();
        if installed.trim().is_empty() {
            t!("uninstall_self_confirmation", app = app_name)
        } else {
            t!(
                "uninstall_all_confirmation",
                app = app_name,
                list = installed
            )
        }
    } else {
        t!("uninstall_confirmation", list = installed)
    };
    if !common::confirm(prompt, false)? {
        return Ok(ExecStatus::new_executed());
    }

    config.uninstall(!keep_self)?;

    Ok(ExecStatus::new_executed())
}
