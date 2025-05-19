use super::{ExecStatus, ManagerSubcommands};
use crate::core::check;
use anyhow::Result;

/// Execute `install` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::Check { extra_args } = subcommand else {
        return Ok(ExecStatus::default());
    };

    check::run(extra_args)?;
    Ok(ExecStatus::new_executed().no_pause(true))
}
