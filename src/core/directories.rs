use std::path::Path;

pub(crate) trait RimDir {
    /// The installation root, i.e. `rust/`
    fn install_dir(&self) -> &Path;

    /// The directory to store cargo files, a.k.a. `<INSTALL_DIR>/cargo/`
    fn cargo_home(&self) -> &Path {
        get_path_and_create!(CARGO_HOME_DIR, self.install_dir().join("cargo"))
    }

    /// The directory to store rustup proxy binaries or cargo tools,
    /// a.k.a. `<INSTALL_DIR>/cargo/bin/`
    fn cargo_bin(&self) -> &Path {
        get_path_and_create!(CARGO_BIN_DIR, self.cargo_home().join("bin"))
    }

    /// The directory to store rust toolchain files, a.k.a. `<INSTALL_DIR>/rustup/`
    fn rustup_home(&self) -> &Path {
        get_path_and_create!(RUSTUP_HOME_DIR, self.install_dir().join("rustup"))
    }

    /// The directory to store temporary files, a.k.a. `<INSTALL_DIR>/temp/`
    fn temp_dir(&self) -> &Path {
        get_path_and_create!(TEMP_DIR, self.install_dir().join("temp"))
    }

    /// The directory to store third-party tools, a.k.a. `<INSTALL_DIR>/tools/`
    fn tools_dir(&self) -> &Path {
        get_path_and_create!(TOOLS_DIR, self.install_dir().join("tools"))
    }

    /// The directory to store crate source code, a.k.a. `<INSTALL_DIR>/crates/`
    fn crates_dir(&self) -> &Path {
        get_path_and_create!(CRATES_DIR, self.install_dir().join("crates"))
    }

    #[cfg(unix)]
    fn backup_dir(&self) -> &Path {
        get_path_and_create!(BACKUP_DIR, self.install_dir().join("backup"))
    }
}
