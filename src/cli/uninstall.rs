//! Separated module to handle uninstallation in command line.

use crate::core::uninstall::UninstallConfiguration;

use super::{common, ManagerSubcommands};

use anyhow::Result;
use rim_common::build_config;

/// Execute `uninstall` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<bool> {
    let ManagerSubcommands::Uninstall { keep_self } = subcommand else {
        return Ok(false);
    };

    let config = UninstallConfiguration::init(None)?;
    let installed = config.install_record.print_installation();

    // Ask confirmation
    let prompt = if !keep_self {
        let id = &build_config().identifier;
        t!("uninstall_all_confirmation", vendor = id, list = installed)
    } else {
        t!("uninstall_confirmation", list = installed)
    };
    if !common::confirm(prompt, false)? {
        return Ok(true);
    }

    config.uninstall(!keep_self)?;

    Ok(true)
}
