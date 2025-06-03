use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::project::ProcessBuilder;

#[rim_test]
fn case() {
    ProcessBuilder::manager_process()
        .command()
        .arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"]);
}
