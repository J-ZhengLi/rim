use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::process::TestProcess;

#[rim_test]
fn case() {
    TestProcess::manager()
        .command()
        .arg("try-it")
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"]);
}
