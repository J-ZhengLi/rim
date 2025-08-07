use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{core::directories::RimDir, InstallConfiguration};
use super::vscode::VSCodeInstaller;

const VSCODE: VSCodeInstaller = VSCodeInstaller {
    cmd: "codearts-rust",
    tool_name: "codearts-rust",
    shortcut_name: "CodeArts IDE for Rust",
    binary_name: "codearts-rust"
};

pub(super) fn install<T>(path: &Path, config: &InstallConfiguration<T>) -> Result<Vec<PathBuf>> {
    VSCODE.install(path, config)
}

pub(super) fn uninstall<T: RimDir + Copy>(config: T) -> Result<()> {
    VSCODE.uninstall(config)
}

pub(super) fn is_installed() -> bool {
    VSCODE.is_installed()
}
