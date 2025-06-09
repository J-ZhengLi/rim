//! Tests here use both `installer` and `manager`

use rim_common::{build_config, exe, utils};
use rim_test_support::{
    process::{mocked_dist_server, TestProcess},
    rim_test,
};
use std::{path::Path, process::Command as StdCommand};

macro_rules! assert_files {
    ($($root:ident.$bin:expr),+) => {
        $(
            assert!($root.join(exe!($bin)).is_file());
        )*
    };
}

#[rim_test]
fn uninstall_toolkit_kept_rim_links() {
    let process = super::default_install();
    let root = process.default_install_dir();

    let cargo_bin_dir = root.join("cargo").join("bin");
    let all_bin = utils::walk_dir(&cargo_bin_dir, false).unwrap();
    assert_eq!(all_bin.len(), 16);

    let rim_name = build_config().app_name();
    assert_files!(
        cargo_bin_dir."cargo",
        cargo_bin_dir."cargo-clippy",
        cargo_bin_dir."cargo-fmt",
        cargo_bin_dir."cargo-miri",
        cargo_bin_dir."clippy-driver",
        cargo_bin_dir."rls",
        cargo_bin_dir."rust-analyzer",
        cargo_bin_dir."rust-gdb",
        cargo_bin_dir."rust-gdbgui",
        cargo_bin_dir."rust-lldb",
        cargo_bin_dir."rustc",
        cargo_bin_dir."rustdoc",
        cargo_bin_dir."rustfmt",
        cargo_bin_dir."rustup",
        cargo_bin_dir."rim",
        cargo_bin_dir.rim_name
    );

    // uninstall toolkit
    let rim = root.join(exe!(rim_name));
    let status = StdCommand::new(&rim)
        .args(["-y", "--no-modify-env", "uninstall", "--keep-self"])
        .status()
        .unwrap();
    assert!(status.success());

    let all_bin = utils::walk_dir(&cargo_bin_dir, false).unwrap();
    println!("all bin: {all_bin:?}");
    assert_eq!(all_bin.len(), 2);
    assert_files!(
        cargo_bin_dir."rim",
        cargo_bin_dir.rim_name
    );

    // uninstall self
    let status = StdCommand::new(rim)
        .args(["-y", "--no-modify-env", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!cargo_bin_dir.exists());
}

#[rim_test]
fn uninstall_using_linked_rim() {
    let process = super::default_install();
    let install_dir = process.default_install_dir();

    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));
    let status = StdCommand::new(rim)
        .args(["-y", "--no-modify-env", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!install_dir.exists() || utils::walk_dir(&install_dir, false).unwrap().is_empty());
    assert!(process.root().exists());
}

#[rim_test]
fn uninstall_using_linked_manager() {
    let process = super::default_install();
    let install_dir = process.default_install_dir();

    let rim = install_dir
        .join("cargo")
        .join("bin")
        .join(exe!(build_config().app_name()));
    let status = StdCommand::new(rim)
        .args(["-y", "--no-modify-env", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!install_dir.exists() || utils::walk_dir(&install_dir, false).unwrap().is_empty());
    assert!(process.root().exists());
}

fn list_component_output(rim: &Path) -> String {
    let list_comp_output = StdCommand::new(&rim)
        .args(["list", "component"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&list_comp_output.stdout)
        .trim()
        .to_string()
}

fn add_or_rm_component(rim: &Path, comp: &str, remove: bool) {
    let op = if remove { "remove" } else { "add" };
    let mut cmd = StdCommand::new(&rim);
    cmd.args(["component", op, comp]);
    if !remove {
        cmd.args(["--rustup-dist-server", mocked_dist_server().rustup.as_str()]);
    }
    let status = cmd.status().unwrap();
    assert!(status.success())
}

#[rim_test]
fn manage_components_using_linked_rim() {
    let process = super::default_install();
    let install_dir = process.default_install_dir();
    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));

    let output = list_component_output(&rim);
    assert_eq!(
        output,
        "Basic (installed)
llvm-tools
rust-docs"
    );

    // add more components
    add_or_rm_component(&rim, "llvm-tools", false);
    add_or_rm_component(&rim, "rust-docs", false);
    let output = list_component_output(&rim);
    assert_eq!(
        output,
        "Basic (installed)
llvm-tools (installed)
rust-docs (installed)"
    );

    // remove middle
    add_or_rm_component(&rim, "llvm-tools", true);
    let output = list_component_output(&rim);
    assert_eq!(
        output,
        "Basic (installed)
rust-docs (installed)
llvm-tools"
    );
}

#[rim_test]
fn install_with_specific_components() {
    let process = TestProcess::installer();
    let list_res = process
        .command()
        .arg("--list-components")
        .assert()
        .success();
    let list_output = String::from_utf8_lossy(&list_res.get_output().stdout)
        .trim()
        .to_string();
    // output contains debug log output
    assert!(list_output.ends_with(
        "Basic
llvm-tools
rust-docs"
    ));

    let install_res = process
        .command()
        .args(["-y", "--component", "rust-docs"])
        .assert()
        .success();
    let install_output = String::from_utf8_lossy(&install_res.get_output().stdout);
    println!("{install_output}");
    assert!(install_output.contains("installing component 'rust-docs'"));

    let rim = process
        .default_install_dir()
        .join("cargo")
        .join("bin")
        .join(exe!("rim"));
    let installed_components = list_component_output(&rim);
    assert_eq!(
        installed_components,
        "Basic (installed)
rust-docs (installed)
llvm-tools"
    );
}

#[rim_test]
fn linked_rim_install_then_update() {
    let process = super::default_install();
    let install_dir = process.default_install_dir();
    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));

    // list all toolkit
    let list_output = StdCommand::new(&rim)
        .env("RIM_DIST_SERVER", mocked_dist_server().rim.as_str())
        .args(["list", "toolkit"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    assert!(String::from_utf8_lossy(&list_output.stdout)
        .trim()
        .ends_with(
            "Custom Rust Distribution stable-1.87.0
Custom Rust Distribution stable-1.86.0
Custom Rust Distribution stable-1.82.0
Custom Rust Distribution stable-1.81.0"
        ),);

    // TODO: install old toolchain, but `manager install` is not implemented yet
    let output = StdCommand::new(rim)
        .args(["-y", "--no-modify-env"])
        // the version must one of the `VERSIONS` in `rim_dev/mocked/server.rs`
        .args([
            "install",
            "--rustup-dist-server",
            mocked_dist_server().rustup.as_str(),
            "1.86.0",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("not yet implemented: install dist with version '1.86.0'"),);
}
