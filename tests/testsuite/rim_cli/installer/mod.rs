use rim_test_support::project::ProjectBuilder;
use std::sync::LazyLock;

mod help;
mod run;

static INSTALLER_PROCESS: LazyLock<ProjectBuilder> =
    LazyLock::new(|| ProjectBuilder::installer_process());
