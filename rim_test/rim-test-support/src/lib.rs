#[macro_use]
extern crate rim_common;

use std::fmt::Write;

pub use rim_test_macro::rim_test;
pub use snapbox::file;
pub use snapbox::str;
pub use snapbox::utils::current_dir;

pub mod paths;
pub mod process;

pub mod prelude {
    pub use crate::rim_test;
}

/// Unwrap a `Result` with a useful panic message
#[macro_export]
macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(e) => $crate::panic_error(&format!("failed running {}", stringify!($e)), e),
        }
    };
}

/// `panic!`, reporting the specified error , see also [`t!`]
#[track_caller]
pub fn panic_error(what: &str, err: impl Into<anyhow::Error>) -> ! {
    let err = err.into();
    pe(what, err);
    #[track_caller]
    fn pe(what: &str, err: anyhow::Error) -> ! {
        let mut result = format!("{what}\nerror: {err}");
        for cause in err.chain().skip(1) {
            let _ = writeln!(result, "\nCaused by:");
            let _ = write!(result, "{cause}");
        }
        panic!("\n{result}");
    }
}
