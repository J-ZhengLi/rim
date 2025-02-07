use env::consts::EXE_SUFFIX;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use rim::utils::Extractable;
use rim_test_support::prelude::*;
use rim_test_support::project::ProjectBuilder;

fn mocked_dist_server() -> &'static str {
    static DIST_SERVER: OnceLock<String> = OnceLock::new();
    DIST_SERVER.get_or_init(|| {
        let rustup_server = env::current_exe()
            .unwrap()
            .parent() // strip deps
            .unwrap()
            .with_file_name("mocked")
            .join("rustup-server");
        if !rustup_server.is_dir() {
            // make sure the template file exists
            let templates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("templates");
            let template = templates_dir.join("channel-rust.template");
            Extractable::load(&template, Some("gz"))
                .unwrap()
                .extract_to(&templates_dir)
                .unwrap();
            // generate now
            Command::new("cargo")
                .args(&["dev", "mock-rustup-server"])
                .status()
                .unwrap();
        }
        url::Url::from_directory_path(rustup_server)
            .unwrap()
            .to_string()
    })
}

#[rim_test]
fn insecure_installation() {
    let test_process = ProjectBuilder::installer_process();
    let root = test_process.root();
    test_process
        .build()
        .arg("-y")
        .arg("--insecure")
        .arg("--no-modify-env")
        .arg("--prefix")
        .arg(root)
        .args(["--rustup-dist-server", mocked_dist_server()])
        .assert()
        .success();

    check_installation(root, true);
}

fn check_installation(root: &Path, expect_rust_success: bool) {
    let cargo_home = root.join(".cargo");
    let rustup_home = root.join(".rustup");

    assert!(cargo_home.is_dir());
    assert!(cargo_home.join("bin").is_dir());
    assert!(cargo_home.join("config.toml").is_file());
    assert!(rustup_home.is_dir());
    assert!(root.join("temp").is_dir());
    assert!(root.join(".fingerprint.toml").is_file());
    assert!(root.join("toolset-manifest.toml").is_file());
    assert!(root
        .join(format!("xuanwu-rust-manager{EXE_SUFFIX}"))
        .is_file());

    if expect_rust_success {
        assert!(rustup_home.join("downloads").is_dir());
        assert!(rustup_home.join("tmp").is_dir());
        assert!(rustup_home.join("toolchains").is_dir());
        assert!(rustup_home.join("update-hashes").is_dir());
        assert!(rustup_home.join("settings.toml").is_file());
    }
}
