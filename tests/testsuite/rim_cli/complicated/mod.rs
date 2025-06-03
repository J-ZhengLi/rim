//! Tests here use both `installer` and `manager`

use rim_common::{build_config, exe, utils};
use rim_test_support::{project::ProcessBuilder, rim_test};

macro_rules! assert_files {
    ($($root:ident.$bin:expr),+) => {
        $(
            assert!($root.join(exe!($bin)).is_file());
        )*
    };
}

#[rim_test]
fn uninstall_toolkit_kept_rim_links() {
    let process = ProcessBuilder::installer_process();
    // install rust
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
    let status = std::process::Command::new(&rim)
        .args(["-y", "--no-modify-env", "uninstall", "--keep-self"])
        .status()
        .unwrap();
    assert!(status.success());

    let all_bin = utils::walk_dir(&cargo_bin_dir, false).unwrap();
    assert_eq!(all_bin.len(), 2);
    assert_files!(
        cargo_bin_dir."rim",
        cargo_bin_dir.rim_name
    );

    // uninstall self
    let status = std::process::Command::new(rim)
        .args(["-y", "--no-modify-env", "uninstall"])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!cargo_bin_dir.exists());
}
