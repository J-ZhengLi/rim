use env::consts::EXE_SUFFIX;
use rim_common::{build_config, utils};
use snapbox::cmd::Command;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tempfile::TempDir;
use url::Url;

use crate::paths;

/// Download rustup-init and return the rustup-update-root path.
///
/// This making sure that the rustup-init binary is only downloaded
/// from the server once to save some time and bandwidth.
static DOWNLOAD_RUSTUP_ONCE: OnceLock<Url> = OnceLock::new();
pub fn local_rustup_update_root() -> &'static Url {
    DOWNLOAD_RUSTUP_ONCE.get_or_init(|| {
        let rustup_update_root = cache_dir().join("rustup");
        let bin_dir = rustup_update_root.join("dist").join(env!("TARGET"));
        utils::ensure_dir(&bin_dir).unwrap();
        let rustup_init_name = exe!("rustup-init");
        let dest = bin_dir.join(&rustup_init_name);

        if dest.is_file() {
            return Url::from_file_path(rustup_update_root).unwrap();
        }

        let orig_root = build_config().rustup_update_root("basic");
        let url = utils::url_join(
            orig_root,
            format!("dist/{}/{rustup_init_name}", env!("TARGET")),
        )
        .unwrap();
        // Download rustup-init
        println!("download and caching rustup-init...");
        utils::DownloadOpt::new("rustup-init", true)
            .insecure(true)
            .blocking_download(&url, &dest)
            .unwrap();
        utils::set_exec_permission(dest).unwrap();

        Url::from_file_path(rustup_update_root).unwrap()
    })
}
// Clear the mocked server once to make sure it's always up-to-date
static CLEAR_OBSCURE_MOCKED_SERVER_ONCE: OnceLock<()> = OnceLock::new();
pub fn mocked_dist_server() -> &'static Url {
    static DIST_SERVER: OnceLock<Url> = OnceLock::new();
    DIST_SERVER.get_or_init(|| {
        let rustup_server = env::current_exe()
            .unwrap()
            .parent() // strip deps
            .unwrap()
            .with_file_name("mocked")
            .join("rustup-server");
        CLEAR_OBSCURE_MOCKED_SERVER_ONCE.get_or_init(|| {
            _ = std::fs::remove_dir_all(&rustup_server);
        });
        if !rustup_server.is_dir() {
            // make sure the template file exists
            let templates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .with_file_name("resources")
                .join("templates");
            let template = templates_dir.join("channel-rust.template");
            utils::Extractable::load(&template, Some("gz"))
                .unwrap()
                .extract_to(&templates_dir)
                .unwrap();
            // generate now
            std::process::Command::new("cargo")
                .args(["dev", "mock-rustup-server"])
                .status()
                .unwrap();
        }
        url::Url::from_directory_path(rustup_server).unwrap()
    })
}

pub struct ProcessBuilder {
    root: TempDir,
    executable: PathBuf,
    is_manager: bool,
}

impl ProcessBuilder {
    /// Generate installer test process
    pub fn installer_process() -> ProcessBuilder {
        let name = &format!("installer-cli{EXE_SUFFIX}");
        let (root, executable) = ensure_bin(name);
        ProcessBuilder {
            root,
            executable,
            is_manager: false,
        }
    }

    /// Generate manager test process
    pub fn manager_process() -> ProcessBuilder {
        let name = &format!("manager-cli{EXE_SUFFIX}");
        let (root, executable) = ensure_bin(name);
        ProcessBuilder {
            root,
            executable,
            is_manager: true,
        }
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    /// Returns a new test [`Command`] to run test process.
    ///
    /// Some arguments are pre-configured to avoid messing with the environment
    /// (until we have a method to create an isolated test environment),
    /// or to avoid sending web requests, they are:
    ///
    /// 1. `--no-modify-env`: to prevent changing environment for the host machine
    /// 2. **Installer Mode** `--prefx`: to limit the installation in the tests directory only
    /// 3. **Installer Mode** `--rustup-update-root` & `--rustup-dist-server`: that pointing
    ///    to local server to avoid sending web requests.
    pub fn command(&self) -> Command {
        let base = Command::new(&self.executable);

        if !self.is_manager {
            base.arg("--prefix")
                .arg(self.root())
                .arg("--no-modify-env")
                .args(["--rustup-update-root", local_rustup_update_root().as_str()])
                .args(["--rustup-dist-server", mocked_dist_server().as_str()])
        } else {
            base.arg("--no-modify-env")
        }
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

fn cache_dir() -> PathBuf {
    let cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .with_file_name("tests")
        .join("cache");
    utils::ensure_dir(&cache_dir).unwrap();
    cache_dir
}
