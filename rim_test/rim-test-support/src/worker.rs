#![allow(unused_imports)]

use env::consts::EXE_SUFFIX;
use std::{env, fs, path::PathBuf};

pub trait SnapboxCommandExt {
    fn rim_cli() -> Self;
}

impl SnapboxCommandExt for snapbox::cmd::Command {
    fn rim_cli() -> Self {
        Self::new(rim_cli())
    }
}

/// Path to the rim-cli binary
fn rim_cli() -> PathBuf {
    snapbox::cmd::cargo_bin("rim-cli")
}
