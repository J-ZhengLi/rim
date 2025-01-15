use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::{env, fs};

use tempfile::TempDir;

use crate::t;

static GLOBAL_ROOT_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn set_global_root_dir(tmp_dir: Option<&'static str>) {
    let mut root_dir = GLOBAL_ROOT_DIR
        .get_or_init(Default::default)
        .lock()
        .unwrap();

    if root_dir.is_none() {
        let dir = match tmp_dir {
            Some(td) => PathBuf::from(td),
            None => {
                let mut path = t!(env::current_exe());
                path.pop(); // chop off exe name
                path.pop(); // chop off "deps"
                path.push("tmp");
                mkdir_p(&path);
                path
            }
        };

        *root_dir = Some(dir);
    }
}

fn global_root_dir() -> PathBuf {
    let root_dir = GLOBAL_ROOT_DIR
        .get_or_init(Default::default)
        .lock()
        .unwrap();
    match root_dir.as_ref() {
        Some(p) => p.clone(),
        None => unreachable!("GLOBAL ROOT DIR not set yet"),
    }
}

thread_local! {
    static TEST_ID: RefCell<Option<usize>> = const { RefCell::new(None) };
}

pub fn test_root() -> TempDir {
    let id = TEST_ID.with(|n| n.borrow().expect("Failed to get test thread id"));

    let test_root_dir = global_root_dir();
    let prefix = format!("t{}", id);
    TempDir::with_prefix_in(prefix, test_root_dir).expect("Failed to create temp test dir")
}

pub fn init_root(tmp_dir: Option<&'static str>) -> TempDir {
    static RUN_TEST_ID: AtomicUsize = AtomicUsize::new(0);

    let id = RUN_TEST_ID.fetch_add(1, Ordering::SeqCst);
    TEST_ID.with(|n| *n.borrow_mut() = Some(id));

    set_global_root_dir(tmp_dir);

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
