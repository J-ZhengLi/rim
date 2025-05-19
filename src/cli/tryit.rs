use super::{ExecStatus, ManagerSubcommands};
use crate::core::try_it;
use anyhow::Result;

/// Execute `install` command.
pub(super) fn execute(subcommand: &ManagerSubcommands) -> Result<ExecStatus> {
    let ManagerSubcommands::TryIt { path } = subcommand else {
        return Ok(ExecStatus::default());
    };

    try_it::try_it(path.as_deref())?;
    Ok(ExecStatus::new_executed().no_pause(true))
}
