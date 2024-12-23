use rim_test_support::current_dir;
use rim_test_support::file;
use rim_test_support::paths;
use rim_test_support::prelude::*;

#[rim_test]
fn case() {
    let test_home = paths::test_home();
    let current_root = current_dir!();

    snapbox::cmd::Command::rim_dev()
        .arg("dist")
        .env("RIM_WORKSPACE_DIR", &test_home)
        .env("RESOURCE_DIR", current_root)
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"])
        .stderr_eq(file!["stderr.log"]);
}
