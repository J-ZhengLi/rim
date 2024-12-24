use rim::utils;

use rim_test_support::current_dir;
use rim_test_support::file;
use rim_test_support::paths;
use rim_test_support::prelude::*;

#[rim_test]
fn case() {
    let test_home = paths::test_home();
    let current_root = current_dir!();

    snapbox::cmd::Command::rim_dev()
        .arg("vendor")
        .env("RIM_WORKSPACE_DIR", &test_home)
        .env("RESOURCE_DIR", current_root)
        .assert()
        .success()
        .stdout_eq(file!["stdout.log"])
        .stderr_eq(file!["stderr.log"]);

    let dir_to_walk = test_home.join("packages").join("vd").join("1010-01-01");
    let entries = utils::walk_dir(&dir_to_walk, true).unwrap();

    let expected = vec![dir_to_walk.join("a.tar.xz"), dir_to_walk.join("b.tar.xz")];

    for exp in expected {
        assert!(entries.contains(&exp));
    }
}
