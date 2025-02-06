use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::{env, fs};

use tempfile::TempDir;

use crate::t;

static GLOBAL_ROOT_DIR: OnceLock<PathBuf> = OnceLock::new();

fn global_root_dir() -> &'static Path {
    GLOBAL_ROOT_DIR.get_or_init(|| {
        option_env!("RIM_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let mut path = t!(env::current_exe());
                path.pop(); // chop off exe name
                path.pop(); // chop off "deps"
                path.push("tmp");
                mkdir_p(&path);
                path
            })
    })
}

thread_local! {
    // this lint has FP on Windows-GNU target (https://github.com/rust-lang/rust-clippy/issues/13422)
    #[allow(clippy::missing_const_for_thread_local)]
    static TEST_ID: RefCell<Option<usize>> = const { RefCell::new(None) };
}

pub fn test_root() -> TempDir {
    let id = TEST_ID.with(|n| n.borrow().expect("Failed to get test thread id"));

    let test_root_dir = global_root_dir();
    let prefix = format!("t{}", id);
    TempDir::with_prefix_in(prefix, test_root_dir).expect("Failed to create temp test dir")
}

pub fn init_root() -> TempDir {
    static RUN_TEST_ID: AtomicUsize = AtomicUsize::new(0);

    let id = RUN_TEST_ID.fetch_add(1, Ordering::SeqCst);
    TEST_ID.with(|n| *n.borrow_mut() = Some(id));

    test_root()
}

/// Path to the current test asset home
///
/// example: $CARGO_MANIFEST_DIR/asset
pub fn assets_home() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("tests");
    path.push("assets");
    path
}

fn mkdir_p(path: &PathBuf) {
    fs::create_dir_all(path)
        .unwrap_or_else(|e| panic!("failed to mkdir dir {}: \n cause: \n {}", path.display(), e))
}
