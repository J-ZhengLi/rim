use super::INSTALLER_PROCESS;
use rim_test_support::file;
use rim_test_support::prelude::*;

#[rim_test]
fn case() {
    INSTALLER_PROCESS
        .command()
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"]);
}
