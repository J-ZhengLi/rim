use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::process::ProcessBuilder;

#[rim_test]
fn case() {
    ProcessBuilder::manager_process()
        .command()
        .arg("try-it")
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"]);
}
