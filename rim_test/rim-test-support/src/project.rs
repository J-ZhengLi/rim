use snapbox::cmd::Command;
use std::path::PathBuf;

use crate::paths;
use crate::paths::TestPathExt;
use crate::worker::SnapboxCommandExt;

pub struct ProjectBuilder {
    root: PathBuf,
    cmd: Command,
}

impl ProjectBuilder {
    /// Generate installer test process
    pub fn rim_cli_process() -> ProjectBuilder {
        let root = paths::test_home();
        let cmd = Command::rim_cli();
        ProjectBuilder { root, cmd }
    }

    /// Generate installer test process
    pub fn installer_process() -> ProjectBuilder {
        let root = paths::test_home();
        let cmd = Command::installer();
        ProjectBuilder { root, cmd }
    }

    /// Generate manager test process
    pub fn manager_process() -> ProjectBuilder {
        let root = paths::test_home();
        let cmd = Command::manager();
        ProjectBuilder { root, cmd }
    }

    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    pub fn build(self) -> Command {
        // clean the home directory.
        self.root().rm_rf();
        // create the home directory
        self.root().mkdir_p();

        let ProjectBuilder { cmd, .. } = self;
        cmd
    }
}
