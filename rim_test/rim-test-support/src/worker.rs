#![allow(unused_imports)]

use env::consts::EXE_SUFFIX;
use std::env;
use std::fmt::format;
use std::fs;
use std::path::PathBuf;

use crate::paths;

pub trait SnapboxCommandExt {
    fn rim_cli() -> Self;

    fn installer() -> Self;

    fn manager() -> Self;
}

impl SnapboxCommandExt for snapbox::cmd::Command {
    fn rim_cli() -> Self {
        Self::new(rim_cli())
    }

    fn installer() -> Self {
        Self::new(installer_cli())
    }

    fn manager() -> Self {
        Self::new(manager_cli())
    }
}

/// Path to the rim-cli binary
fn rim_cli() -> PathBuf {
    snapbox::cmd::cargo_bin("rim-cli")
}

/// Path to the installer-cli binary
fn installer_cli() -> PathBuf {
    ensure_bin(&format!("installer-cli{EXE_SUFFIX}"))
}

/// Path to the manager-cli binary
fn manager_cli() -> PathBuf {
    ensure_bin(&format!("manager-cli{EXE_SUFFIX}"))
}

// Before any invoke of rim_cli,
// we should save a copy as `installer` and `manager`.
fn ensure_bin(name: &str) -> PathBuf {
    let src = rim_cli();
    let dst = paths::test_home().join(name);
    if !dst.exists() {
        fs::copy(src, &dst)
            .unwrap_or_else(|_| panic!("Failed to copy rim-cli{EXE_SUFFIX} to {name}"));
    }

    dst
}
