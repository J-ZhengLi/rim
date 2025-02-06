use env::consts::EXE_SUFFIX;
use snapbox::cmd::Command;
use std::env;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::paths;

pub struct ProjectBuilder {
    root: TempDir,
    cmd: PathBuf,
}

impl ProjectBuilder {
    /// Generate installer test process
    pub fn installer_process() -> ProjectBuilder {
        let name = &format!("installer-cli{EXE_SUFFIX}");
        let (root, cmd) = ensure_bin(name);
        ProjectBuilder { root, cmd }
    }

    /// Generate manager test process
    pub fn manager_process() -> ProjectBuilder {
        let name = &format!("manager-cli{EXE_SUFFIX}");
        let (root, cmd) = ensure_bin(name);
        ProjectBuilder { root, cmd }
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn build(&self) -> Command {
        // retain the modification entry.
        Command::new(&self.cmd)
    }

    /// Consume self and keep all temporary files.
    pub fn keep_temp(self) {
        let x = self.root.into_path();
        println!("keeping temporary files: {}", x.display());
    }
}

// Before any invoke of rim_cli,
// we should save a copy as `installer` and `manager`.
fn ensure_bin(name: &str) -> (TempDir, PathBuf) {
    let test_root = paths::test_root();
    let src = snapbox::cmd::cargo_bin("rim-cli");
    let dst = test_root.path().join(name);
    if !dst.exists() {
        std::fs::copy(src, &dst)
            .unwrap_or_else(|_| panic!("Failed to copy rim-cli{EXE_SUFFIX} to {name}"));
    }

    (test_root, dst)
}
