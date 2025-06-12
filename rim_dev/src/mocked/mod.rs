use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use rim_common::get_path_and_create;

pub mod installation;
pub mod manager;
pub mod server;

const TOOLKIT_NAME: &str = "Test-only Toolkit";

fn debug_dir() -> PathBuf {
    // safe to unwrap, binary file always have a parent dir
    crate::current_exe().parent().unwrap().to_path_buf()
}

fn mocked_dir() -> PathBuf {
    let dir = debug_dir().join("mocked");
    fs::create_dir_all(&dir)
        .unwrap_or_else(|_| panic!("unable to create mocked dir at {}", dir.display()));
    dir
}

pub(crate) fn mocked_home() -> &'static Path {
    get_path_and_create!(MOCKED_HOME_DIR, mocked_dir().join("home"))
}

fn install_dir() -> &'static Path {
    static INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();
    INSTALL_DIR.get_or_init(|| {
        let dir = mocked_home().join("installation");
        fs::create_dir_all(&dir)
            .unwrap_or_else(|_| panic!("unable to create mocked install dir at {}", dir.display()));
        dir
    })
}

fn rim_server_dir() -> PathBuf {
    let dir = mocked_dir().join("rim-server");
    fs::create_dir_all(&dir)
        .unwrap_or_else(|_| panic!("unable to create mocked server dir at {}", dir.display()));
    dir
}

fn rustup_server_dir() -> PathBuf {
    let dir = mocked_dir().join("rustup-server");
    fs::create_dir_all(&dir)
        .unwrap_or_else(|_| panic!("unable to create mocked server dir at {}", dir.display()));
    dir
}

fn manager_dir() -> &'static Path {
    static MANAGER_DIR: OnceLock<PathBuf> = OnceLock::new();
    MANAGER_DIR.get_or_init(|| {
        let dir = rim_server_dir().join("manager");
        fs::create_dir_all(&dir).unwrap_or_else(|_| {
            panic!(
                "unable to create mocked manager dist dir at {}",
                dir.display()
            )
        });
        dir
    })
}
