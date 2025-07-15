//! Tests here use both `installer` and `manager`

use rim_common::{build_config, exe, utils};
use rim_test_support::{
    process::{mocked_dist_server, TestProcess},
    rim_test,
};
use std::path::Path;

macro_rules! assert_files {
    ($($root:ident.$bin:expr),+) => {
        $(
            assert!($root.join(exe!($bin)).is_file());
        )*
    };
}

#[rim_test]
fn uninstall_toolkit_kept_rim_links() {
    let process = super::default_install(true);
    let root = process.default_install_dir();
    let config_dir = process.config_dir();

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
    let status = process
        .rim_command(&rim)
        .args(["-y", "uninstall", "--keep-self"])
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
    assert!(config_dir.join("install-record.toml").is_file());

    // uninstall self
    let status = process
        .rim_command(&rim)
        .args(["-y", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!cargo_bin_dir.exists());
    assert!(!config_dir.exists());
}

#[rim_test]
fn uninstall_using_linked_rim() {
    let process = super::default_install(true);
    let install_dir = process.default_install_dir();

    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));
    let status = process
        .rim_command(&rim)
        .args(["-y", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(process.root().exists());
    assert!(!process.config_dir().exists());

    #[cfg(unix)]
    {
        // on windows, these file won't be deleted right away,
        // so we ignore the residual files for this test
        assert!(!install_dir.exists() || utils::walk_dir(&install_dir, true).unwrap().is_empty());
    }
}

#[rim_test]
fn uninstall_using_linked_manager() {
    let process = super::default_install(true);
    let install_dir = process.default_install_dir();

    let rim = install_dir
        .join("cargo")
        .join("bin")
        .join(exe!(build_config().app_name()));
    let status = process
        .rim_command(&rim)
        .args(["-y", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(process.root().exists());
    assert!(!process.config_dir().exists());

    #[cfg(unix)]
    {
        // on windows, these file won't be deleted right away,
        // so we ignore the residual files for this test
        assert!(!install_dir.exists() || utils::walk_dir(&install_dir, true).unwrap().is_empty());
    }
}

fn list_component_output(process: &TestProcess, rim: &Path) -> String {
    let list_comp_output = process
        .rim_command(&rim)
        .args(["list", "component"])
        .output()
        .unwrap();
    if !list_comp_output.status.success() {
        panic!("{}", String::from_utf8_lossy(&list_comp_output.stderr));
    }
    // output might contains debug messages, which will not appear after release
    // anyway, filter those out for test as well.
    String::from_utf8_lossy(&list_comp_output.stdout)
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with("\u{1b}[35mdebug\u{1b}[0m"))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn add_or_rm_component(process: &TestProcess, rim: &Path, comp: &str, remove: bool) {
    let op = if remove { "remove" } else { "add" };
    let mut cmd = process.rim_command(&rim);
    cmd.args(["-y", "component", op, comp]);
    if !remove {
        cmd.args(["--rustup-dist-server", mocked_dist_server().rustup.as_str()]);
    }
    let status = cmd.status().unwrap();
    assert!(status.success())
}

#[rim_test]
fn manage_components_using_linked_rim() {
    let process = super::default_install(true);
    let install_dir = process.default_install_dir();
    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));

    let output = list_component_output(&process, &rim);
    assert_eq!(
        output,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
llvm-tools
rust-docs"
    );

    // add more components
    add_or_rm_component(&process, &rim, "llvm-tools", false);
    add_or_rm_component(&process, &rim, "rust-docs", false);
    let output = list_component_output(&process, &rim);
    assert_eq!(
        output,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
llvm-tools (installed)
rust-docs (installed)"
    );

    // remove middle
    add_or_rm_component(&process, &rim, "llvm-tools", true);
    let output = list_component_output(&process, &rim);
    assert_eq!(
        output,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
rust-docs (installed)
llvm-tools"
    );
}

#[rim_test]
fn install_with_specific_components() {
    let process = TestProcess::combined();
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
clippy
rustfmt
rust-src
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
    let installed_components = list_component_output(&process, &rim);
    assert_eq!(
        installed_components,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
rust-docs (installed)
llvm-tools"
    );
}

#[rim_test]
fn linked_rim_install_then_update() {
    let process = super::default_install(true);
    let install_dir = process.default_install_dir();
    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));

    // list all toolkit
    let list_output = process
        .rim_command(&rim)
        .env("RIM_DIST_SERVER", mocked_dist_server().rim.as_str())
        .args(["list", "toolkit"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    assert!(String::from_utf8_lossy(&list_output.stdout)
        .trim()
        .ends_with(
            "Test-only Toolkit stable-1.87.0
Test-only Toolkit stable-1.86.0
Test-only Toolkit stable-1.82.0
Test-only Toolkit stable-1.81.0"
        ),);

    // TODO: install old toolchain, but `manager install` is not implemented yet
    let output = process
        .rim_command(&rim)
        // the version must one of the `VERSIONS` in `rim_dev/mocked/server.rs`
        .args([
            "-y",
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

#[rim_test]
fn configs_migration() {
    let process = super::default_install(true);
    let config_dir = process.config_dir();
    let install_dir = process.default_install_dir();

    // pretend this was installed using old rim
    let old_rec_path = install_dir.join(".fingerprint.toml");
    let new_rec_path = config_dir.join("install-record.toml");
    assert!(new_rec_path.is_file());
    utils::move_to(&new_rec_path, &old_rec_path, true).unwrap();
    assert!(!new_rec_path.exists());
    assert!(old_rec_path.is_file());

    // when running rim again, the file should be moved to the new location
    // if it's not done already.
    let rim = install_dir.join("cargo").join("bin").join(exe!("rim"));
    let list_output = list_component_output(&process, &rim);
    assert!(!old_rec_path.exists());
    assert!(new_rec_path.is_file());
    assert_eq!(
        list_output,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
llvm-tools
rust-docs"
    );

    // if there are duplicated config files, this program should acknowledge that
    utils::write_file(&old_rec_path, "", false).unwrap();
    let list_output = list_component_output(&process, &rim);
    assert_eq!(
        list_output,
        "Basic (installed)
clippy (installed)
rustfmt (installed)
rust-src (installed)
llvm-tools
rust-docs"
    );
}
