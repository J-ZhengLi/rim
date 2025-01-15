use env::consts::EXE_SUFFIX;
use snapbox::cmd::Command;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::worker::SnapboxCommandExt;

pub struct ProjectBuilder {
    root: TempDir,
    cmd: PathBuf,
}

impl ProjectBuilder {
    /// Generate installer test process
    pub fn installer_process() -> ProjectBuilder {
        let name = &format!("installer-cli{EXE_SUFFIX}");
        let (root, cmd) = Command::cmd_bin(name);
        ProjectBuilder { root, cmd }
    }

    /// Generate manager test process
    pub fn manager_process() -> ProjectBuilder {
        let name = &format!("manager-cli{EXE_SUFFIX}");
        let (root, cmd) = Command::cmd_bin(name);
        ProjectBuilder { root, cmd }
    }

    pub fn root(&self) -> PathBuf {
        self.root.path().to_path_buf()
    }

    pub fn build(&self) -> Command {
        // retain the modification entry.
        Command::new(self.cmd.clone())
    }
}
