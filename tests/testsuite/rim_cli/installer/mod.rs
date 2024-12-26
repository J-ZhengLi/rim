use env::consts::EXE_SUFFIX;
use std::env;
use std::path::PathBuf;

use rim_test_support::prelude::*;
use rim_test_support::project::ProjectBuilder;

#[rim_test]
fn case() {
    let project = ProjectBuilder::installer_process();
    let root = project.root();
    project
        .build()
        .arg("-y")
        .arg("--no-modify-path")
        .arg("--no-modify-env")
        .arg("--prefix")
        .arg(&root)
        .assert();

    check_installation(&root);
}

fn check_installation(root: &PathBuf) {
    assert!(root.join(".cargo").is_dir());
    assert!(root.join(".cargo").join("bin").is_dir());
    assert!(root.join(".cargo").join("config.toml").is_file());
    assert!(root.join(".rustup").is_dir());
    assert!(root.join(".rustup").join("downloads").is_dir());
    assert!(root.join(".rustup").join("tmp").is_dir());
    assert!(root.join(".rustup").join("toolchains").is_dir());
    assert!(root.join(".rustup").join("update-hashes").is_dir());
    assert!(root.join(".rustup").join("settings.toml").is_file());
    assert!(root.join("temp").is_dir());
    assert!(root.join(".fingerprint.toml").is_file());
    assert!(root.join("toolset-manifest.toml").is_file());
    assert!(root
        .join(format!("xuanwu-rust-manager{EXE_SUFFIX}"))
        .is_file());
}
