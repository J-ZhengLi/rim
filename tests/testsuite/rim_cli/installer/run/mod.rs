use std::path::Path;

use rim_common::{build_config, exe};
use rim_test_support::{prelude::*, process::TestProcess};

use crate::rim_cli::default_install;

#[rim_test]
fn default_installation_dir() {
    let process = TestProcess::installer();
    let res = process.command().arg("-y").assert().success();
    println!("{}", String::from_utf8_lossy(&res.get_output().stdout));

    check_installation(&process.default_install_dir());
}

#[rim_test]
fn custom_installation_dir() {
    let process = TestProcess::installer();
    let install_dir = process.root().join("install_prefix");
    process
        .command()
        .arg("--prefix")
        .arg(&install_dir)
        .arg("-y")
        .assert()
        .success();

    check_installation(&install_dir);
}

fn check_installation(root: &Path) {
    let cargo_home = root.join("cargo");
    let rustup_home = root.join("rustup");

    #[cfg(unix)]
    assert!(root.join("env").is_file());
    assert!(cargo_home.is_dir());
    assert!(cargo_home.join("bin").is_dir());
    assert!(rustup_home.is_dir());
    assert!(root.join("temp").is_dir());
    assert!(root.join("toolset-manifest.toml").is_file());
    assert!(root.join(exe!(build_config().app_name())).is_file());

    assert!(rustup_home.join("downloads").is_dir());
    assert!(rustup_home.join("tmp").is_dir());
    assert!(rustup_home.join("toolchains").is_dir());
    assert!(rustup_home.join("update-hashes").is_dir());
    assert!(rustup_home.join("settings.toml").is_file());
}

#[cfg(unix)]
#[rim_test]
fn env_and_path_configured() {
    use rim_common::utils;
    use rim_test_support::process::{local_rustup_update_root, mocked_dist_server};

    let process = TestProcess::installer();
    let root = process.default_install_dir();
    let res = process.command().arg("-y").assert().success();
    println!("{}", String::from_utf8_lossy(&res.get_output().stdout));

    let default_env_script = root.join("env");
    let fish_env_script = root.join("env.fish");

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
                mocked_dist_server().rustup,
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
                mocked_dist_server().rustup,
                local_rustup_update_root(),
                root.join("cargo").display(),
                root.join("rustup").display(),
                root.join("cargo").join("bin").display(),
            )
        );
    }
}

#[cfg(target_os = "linux")]
#[rim_test]
fn rc_files_are_created() {
    let process = TestProcess::installer();
    process.command().arg("-y").assert().success();

    let possible_bash_rcs = [".profile", ".bash_profile", ".bash_login", ".bashrc"]
        .map(|rc| process.home_dir().join(rc));
    let mut num_rc_checked = 0;
    for rc in possible_bash_rcs {
        if !rc.exists() {
            continue;
        }

        let content = std::fs::read_to_string(rc).unwrap();
        assert_eq!(
            content.trim(),
            format!(
                ". \"{}\"",
                process.default_install_dir().join("env").display()
            )
        );
        num_rc_checked += 1;
    }

    assert!(num_rc_checked > 0, "no rc file created");
}

#[cfg(target_os = "linux")]
#[rim_test]
fn installation_removes_legacy_config_section() {
    use rim_common::utils;

    let mut num_rc_checked = 0;
    let process = TestProcess::installer();
    let possible_bash_rcs = [".profile", ".bash_profile", ".bash_login", ".bashrc"]
        .map(|rc| process.home_dir().join(rc));
    let legacy_content = r#"
# ===== rustup config section START =====
export PATH="/path/to/foo:/path/to/bar:/path/to/rust:$PATH"
export RUSTUP_DIST_SERVER=https://example.com/
export RUSTUP_HOME=/path/to/rustup
export RUSTUP_UPDATE_ROOT=https://example.com/rustup
export CARGO_HOME=/path/to/cargo
# ===== rustup config section END =====
"#;

    for rc in &possible_bash_rcs {
        std::fs::write(rc, legacy_content).unwrap();
    }

    process.command().arg("-y").assert().success();

    for rc in &possible_bash_rcs {
        let content = std::fs::read_to_string(rc).unwrap();
        if content.contains(&format!(
            ". \"{}\"",
            process.default_install_dir().join("env").display()
        )) {
            assert!(!content.contains("# ===== rustup config section START ====="));
            num_rc_checked += 1;
        }
    }

    assert!(num_rc_checked > 0, "no rc file modified");

    // check if rc backup created
    let backup_home = process.default_install_dir().join("backup").join("HOME");
    let backup_files = utils::walk_dir(&backup_home, false).unwrap();
    println!("backup files: {backup_files:?}");
    assert_eq!(backup_files.len(), num_rc_checked);
    assert!(backup_files
        .iter()
        .all(|f| { std::fs::read_to_string(f).unwrap() == legacy_content }));
}

#[rim_test]
fn install_record_created() {
    let process = default_install(false);

    let config_dir = process.config_dir();
    let record = config_dir.join("install-record.toml");
    assert!(record.is_file());

    let record_content = std::fs::read_to_string(record).unwrap();
    println!("record content: {record_content}");
    assert!(record_content.contains(&format!("{}", process.default_install_dir().display())));
}

#[rim_test]
fn configuration_created() {
    let process = default_install(false);
    let config_dir = process.config_dir();

    let config = config_dir.join("configuration.toml");
    assert!(config.is_file());

    let config_content = std::fs::read_to_string(config).unwrap();
    assert!(config_content.contains("language ="));
}
