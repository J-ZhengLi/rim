use rim_test_support::project::ProjectBuilder;
use std::sync::LazyLock;

mod help;
mod try_it_help;
mod uninstall_help;
mod update_help;

static MANAGER_PROCESS: LazyLock<ProjectBuilder> =
    LazyLock::new(|| ProjectBuilder::manager_process());
