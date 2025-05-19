use super::MANAGER_PROCESS;
use rim_test_support::file;
use rim_test_support::prelude::*;

#[rim_test]
fn case() {
    MANAGER_PROCESS
        .command()
        .arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"]);
}
