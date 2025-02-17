//! Instructions for the open-sourced VS Code distribution - "Codium".
//! Project repository: https://github.com/VSCodium/vscodium

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{InstallConfiguration, UninstallConfiguration};
use super::vscode::VSCodeInstaller;

const VSCODE: VSCodeInstaller = VSCodeInstaller::new(
    "codium",
    "vscodium",
    "VSCodium",
    "VSCodium",
);

pub(super) fn install(path: &Path, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
    VSCODE.install(path, config)
}

pub(super) fn uninstall(config: &UninstallConfiguration) -> Result<()> {
    VSCODE.uninstall(config)
}

pub(super) fn already_installed() -> bool {
    VSCODE.already_installed()
}
