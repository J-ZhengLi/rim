use std::path::Path;

use rim_common::{build_config, exe};
use rim_test_support::{prelude::*, project::ProcessBuilder};

#[rim_test]
fn insecure_installation() {
    let process = ProcessBuilder::installer_process();
    let root = process.root();
    process
        .command()
        .arg("-y")
        .arg("--insecure")
        .arg("--no-modify-env")
        .arg("--prefix")
        .arg(root)
        .assert()
        .success();

    check_installation(root, true);
}

fn check_installation(root: &Path, expect_rust_success: bool) {
    let cargo_home = root.join("cargo");
    let rustup_home = root.join("rustup");

    assert!(cargo_home.is_dir());
    assert!(cargo_home.join("bin").is_dir());
    assert!(cargo_home.join("config.toml").is_file());
    assert!(rustup_home.is_dir());
    assert!(root.join("temp").is_dir());
    assert!(root.join(".fingerprint.toml").is_file());
    assert!(root.join("toolset-manifest.toml").is_file());
    assert!(root.join(exe!(build_config().app_name())).is_file());

    if expect_rust_success {
        assert!(rustup_home.join("downloads").is_dir());
        assert!(rustup_home.join("tmp").is_dir());
        assert!(rustup_home.join("toolchains").is_dir());
        assert!(rustup_home.join("update-hashes").is_dir());
        assert!(rustup_home.join("settings.toml").is_file());
    }
}
