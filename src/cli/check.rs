use super::ManagerSubcommands;
use crate::core::check;
use anyhow::Result;

/// Execute `install` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<bool> {
    let ManagerSubcommands::Check { extra_args } = subcommand else {
        return Ok(false);
    };

    check::run(extra_args)?;
    Ok(true)
}
