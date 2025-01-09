#![allow(unused_imports)]

use env::consts::EXE_SUFFIX;
use std::env;
use std::fmt::format;
use std::fs;
use std::path::PathBuf;
use tempfile::{TempDir, TempPath};

use crate::paths;

pub trait SnapboxCommandExt {
    fn rim_cli() -> Self;

    fn cmd_bin(name: &str) -> (TempDir, PathBuf);
}

impl SnapboxCommandExt for snapbox::cmd::Command {
    fn rim_cli() -> Self {
        Self::new(rim_cli())
    }

    fn cmd_bin(name: &str) -> (TempDir, PathBuf) {
        let (tmp_dir, cmd_path) = ensure_bin(name);
        (tmp_dir, cmd_path)
    }
}

/// Path to the rim-cli binary
fn rim_cli() -> PathBuf {
    snapbox::cmd::cargo_bin("rim-cli")
}

// Before any invoke of rim_cli,
// we should save a copy as `installer` and `manager`.
fn ensure_bin(name: &str) -> (TempDir, PathBuf) {
    let test_root = paths::test_root();
    let src = rim_cli();
    let dst = test_root.path().to_path_buf().join(name);
    if !dst.exists() {
        fs::copy(src, &dst)
            .unwrap_or_else(|_| panic!("Failed to copy rim-cli{EXE_SUFFIX} to {name}"));
    }

    (test_root, dst)
}
