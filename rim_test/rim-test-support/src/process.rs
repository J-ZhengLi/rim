use env::consts::EXE_SUFFIX;
use rim_common::{build_config, utils};
use snapbox::cmd::Command;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
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

pub struct MockedServer {
    pub rustup: Url,
    pub rim: Url,
}

pub fn mocked_dist_server() -> &'static MockedServer {
    static DIST_SERVER: OnceLock<MockedServer> = OnceLock::new();
    DIST_SERVER.get_or_init(|| {
        let rustup_server = env::current_exe()
            .unwrap()
            .parent() // strip deps
            .unwrap()
            .with_file_name("mocked")
            .join("rustup-server");
        let rim_server = rustup_server.with_file_name("rim-server");
        if !(rustup_server.is_dir() && rim_server.is_dir()) {
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
                .args(["dev", "mock-server"])
                .status()
                .unwrap();
        }

        MockedServer {
            rim: Url::from_directory_path(rim_server).unwrap(),
            rustup: Url::from_directory_path(rustup_server).unwrap(),
        }
    })
}

pub struct TestProcess {
    root: TempDir,
    executable: PathBuf,
    kind: TestProcessKind,
}

#[derive(Clone, Copy)]
enum TestProcessKind {
    Manager,
    Installer,
    Combined,
}

impl TestProcess {
    /// Generate installer test process
    pub fn installer() -> TestProcess {
        let name = &format!("installer-cli{EXE_SUFFIX}");
        let (root, executable) = ensure_bin(name);
        TestProcess {
            root,
            executable,
            kind: TestProcessKind::Installer,
        }
    }

    /// Generate manager test process
    pub fn manager() -> TestProcess {
        let name = &format!("manager-cli{EXE_SUFFIX}");
        let (root, executable) = ensure_bin(name);
        TestProcess {
            root,
            executable,
            kind: TestProcessKind::Manager,
        }
    }

    pub fn combined() -> TestProcess {
        let mut res = Self::installer();
        res.kind = TestProcessKind::Combined;
        res
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    /// Return the path to a mocked home directory under temporary test folder
    pub fn home_dir(&self) -> PathBuf {
        let home = self.root().join("home");
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    /// Return the default installation directory of rim
    pub fn default_install_dir(&self) -> PathBuf {
        self.home_dir().join("rust")
    }

    /// Returns a new test [`Command`] to run test process.
    ///
    /// The command is pre-configured to avoid messing with the environment
    /// (until we have a method to create an isolated test environment),
    /// or to avoid sending web requests, some configurations are:
    ///
    /// 1. `$HOME`: to avoid messing with the actual home environment
    /// 2. **Installer Mode** `--rustup-update-root` & `--rustup-dist-server`: that pointing
    ///    to local server to avoid sending web requests.
    pub fn command(&self) -> Command {
        // used to override `HOME`, this is to ensure that the test program doesn't change
        // the actual environment
        let home_dir = &self.home_dir();

        #[cfg(unix)]
        let base = Command::new(&self.executable).env("HOME", home_dir);
        // On Windows, env vars are directly added, which make it a bit
        // harder to rollback after running the tests (rustup also struggle with this).
        // So it might be better to disable env modification until we figure out
        // a clever way to do it.
        #[cfg(windows)]
        let base = Command::new(&self.executable)
            .env("HOME", home_dir)
            .env("USERPROFILE", home_dir)
            .arg("--no-modify-env");

        if !matches!(self.kind, TestProcessKind::Manager) {
            base.args(["--rustup-update-root", local_rustup_update_root().as_str()])
                .args(["--rustup-dist-server", mocked_dist_server().rustup.as_str()])
        } else {
            base
        }
    }

    /// Return an `std::process::Command` that invokes the manager after installation,
    /// this only works in combined test process.
    pub fn rim_command(&self, program: &Path) -> StdCommand {
        if !matches!(self.kind, TestProcessKind::Combined) {
            panic!("rim command only works on test process with combined kind");
        }

        let mut base = StdCommand::new(program);
        let home_dir = self.home_dir();
        base.env("HOME", &home_dir);
        #[cfg(windows)]
        base.env("USERPROFILE", &home_dir);
        base
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
