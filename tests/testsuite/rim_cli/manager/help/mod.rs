use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::project::ProcessBuilder;

#[rim_test]
fn case() {
    let base = ProcessBuilder::manager_process()
        .command()
        .arg("--help")
        .assert()
        .success();

    #[cfg(feature = "gui")]
    base.stdout_eq(file!["stdout_gui.log"]);

    #[cfg(not(feature = "gui"))]
    base.stdout_eq(file!["stdout.log"]);
}
