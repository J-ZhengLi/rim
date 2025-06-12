use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::process::TestProcess;

#[rim_test]
fn case() {
    let base = TestProcess::installer()
        .command()
        .arg("--help")
        .assert()
        .success();

    #[cfg(feature = "gui")]
    base.stdout_eq(file!["stdout_gui.log"]);

    #[cfg(not(feature = "gui"))]
    base.stdout_eq(file!["stdout.log"]);
}
