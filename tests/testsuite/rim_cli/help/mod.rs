use rim_test_support::file;
use rim_test_support::prelude::*;
use rim_test_support::project::ProjectBuilder;

#[rim_test]
fn case() {
    let project = ProjectBuilder::rim_cli_process().build();
    project
        .arg("--help")
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"])
        .stderr_eq(file!["stderr.log"]);
}