//! Instructions for the open-sourced VS Code distribution - "Codium".
//! Project repository: https://github.com/VSCodium/vscodium

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{core::directories::RimDir, InstallConfiguration};
use super::vscode::VSCodeInstaller;

const VSCODE: VSCodeInstaller = VSCodeInstaller {
    cmd: "codium",
    tool_name: "vscodium",
    shortcut_name: "VSCodium",
    binary_name: "VSCodium"
};

pub(super) fn install(path: &Path, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
    VSCODE.install(path, config)
}

pub(super) fn uninstall<T: RimDir>(config: T) -> Result<()> {
    VSCODE.uninstall(config)
}

pub(super) fn already_installed() -> bool {
    VSCODE.already_installed()
}
