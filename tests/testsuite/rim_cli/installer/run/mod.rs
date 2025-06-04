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

#[cfg(unix)]
#[rim_test]
fn env_and_path_configured() {
    use rim_common::utils;
    use rim_test_support::project::{local_rustup_update_root, mocked_dist_server};

    let process = ProcessBuilder::installer_process();
    let root = process.root();
    let res = process.command().arg("-y").assert().success();
    println!("{}", String::from_utf8_lossy(&res.get_output().stdout));

    let default_env_script = root.join("env");
    let fish_env_script = root.join("env.sh");

    if default_env_script.exists() {
        let content = utils::read_to_string("", default_env_script).unwrap();
        let template = include_str!("../../../../../resources/templates/env.sh");

        assert_eq!(
            content,
            format!(
                "{template}
export RUSTUP_DIST_SERVER=\"{}\"
export RUSTUP_UPDATE_ROOT=\"{}\"
export CARGO_HOME=\"{}\"
export RUSTUP_HOME=\"{}\"
add_to_path \"{}\"
",
                mocked_dist_server(),
                local_rustup_update_root(),
                root.join("cargo").display(),
                root.join("rustup").display(),
                root.join("cargo").join("bin").display(),
            )
        );
    } else if fish_env_script.exists() {
        let content = utils::read_to_string("", fish_env_script).unwrap();
        let template = include_str!("../../../../../resources/templates/env.fish");

        assert_eq!(
            content,
            format!(
                "{template}
set -Ux RUSTUP_DIST_SERVER \"{}\"
set -Ux RUSTUP_UPDATE_ROOT \"{}\"
set -Ux CARGO_HOME \"{}\"
set -Ux RUSTUP_HOME \"{}\"
add_to_path \"{}\"
",
                mocked_dist_server(),
                local_rustup_update_root(),
                root.join("cargo").display(),
                root.join("rustup").display(),
                root.join("cargo").join("bin").display(),
            )
        );
    }
}
