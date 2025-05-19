use super::MANAGER_PROCESS;
use rim_test_support::file;
use rim_test_support::prelude::*;

#[rim_test]
fn case() {
    let base = MANAGER_PROCESS.command().arg("--help").assert().success();

    #[cfg(feature = "gui")]
    base.stdout_eq(file!["stdout_gui.log"]);

    #[cfg(not(feature = "gui"))]
    base.stdout_eq(file!["stdout.log"]);
}
