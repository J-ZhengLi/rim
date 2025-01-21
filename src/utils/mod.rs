//! Utility functions/types to use across the whole crate.
//!
//! NOTE: Most of these are moved from the `experimental` branch,
//! some of them might turns out to be unused, so remember to clean those after version `1.0`.

mod download;
mod extraction;
mod file_system;
mod log;
mod process;
mod progress_bar;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Duration,
};

pub use download::DownloadOpt;
pub use extraction::Extractable;
pub use file_system::*;
pub use log::{log_file_path, Logger};
pub use process::*;
pub use progress_bar::{CliProgress, Progress, Style as CliProgressStyle};

use anyhow::Result;
use url::Url;

/// Insert a `.exe` postfix to given input.
///
/// # Example
///
/// ```ignore
/// let this_works = rim::exe!("hello_world");
///
/// #[cfg(windows)]
/// {
///     assert!(this_works, "hello_world.exe");
/// }
///
/// #[cfg(not(windows))]
/// {
///     assert!(this_works, "hello_world");
/// }
/// ```
macro_rules! exe {
    ($input:expr) => {{
        #[cfg(windows)]
        {
            format!("{}.exe", $input)
        }
        #[cfg(not(windows))]
        {
            $input
        }
    }};
}
pub(crate) use exe;

/// A convenient macro to write struct variables setter.
///
/// # Usage
///
/// ```rust
/// # use rim::setter;
/// #[derive(Default)]
/// struct Foo {
///     a: bool,
///     b: u32,
///     c: Option<u8>,
/// }
///
/// impl Foo {
///     setter!(a(self.a, bool));
///     setter!(with_b(self.b, u32));
///     setter!(set_c(self.c, value: u8) { Some(value) });
/// }
///
/// let foo = Foo::default()
///     .a(true)
///     .with_b(10)
///     .set_c(100);
/// assert_eq!(foo.a, true);
/// assert_eq!(foo.b, 10);
/// assert_eq!(foo.c, Some(100));
/// ```
// FIXME(?): Find a proper way to provide function visibility instead of all `pub`.
#[macro_export]
macro_rules! setter {
    ($name:ident ($self:ident.$self_param:ident, $t:ty)) => {
        #[allow(clippy::wrong_self_convention)]
        pub fn $name(mut $self, val: $t) -> Self {
            $self.$self_param = val;
            $self
        }
    };
    ($name:ident ($self:ident.$self_param:ident, $($val:ident : $t:ty),*) { $init_val:expr }) => {
        pub fn $name(mut $self, $($val: $t),*) -> Self {
            $self.$self_param = $init_val;
            $self
        }
    };
}

/// Run asynchronous code to completion, with the cost of blocking the current thread.
///
/// # Example
/// ```ignore
/// async fn async_func() {
///     // ...
/// }
///
/// fn normal_func() {
///     blocking!(async_func());
/// }
/// ```
macro_rules! blocking {
    ($blk:expr) => {
        tokio::runtime::Runtime::new()?.block_on($blk)
    };
}
pub(crate) use blocking;

/// Forcefully parsing a `&str` to [`Url`].
///
/// # Panic
///
/// Causes panic if the given string cannot be parsed as `Url`.
pub fn force_parse_url(url: &str) -> Url {
    Url::parse(url).unwrap_or_else(|e| panic!("failed to parse url '{url}': {e}"))
}

/// Basically [`Url::join`], but will push a forward slash (`/`) to the root if necessary.
///
/// [`Url::join`] will replace the last part of a root if the root does not have trailing slash,
/// and this function is to make sure of that, so the `root` will always join with `s`.
pub fn url_join(root: &Url, s: &str) -> Result<Url> {
    let result = if root.as_str().ends_with('/') {
        root.join(s)?
    } else {
        Url::parse(&format!("{}/{s}", root.as_str()))?
    };

    Ok(result)
}

pub fn path_to_str(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "path '{}' cannot be convert to str as it may contains invalid unicode characters.",
            path.display()
        )
    })
}

/// Returns `true` if the `Path` is root directory.
///
/// * On Unix, root directory is just `/`.
///
/// * On Windows, a path is root if it has a root (check [`has_root`](Path::has_root) for details)
///     and has no child components.
pub fn is_root_dir<P: AsRef<Path>>(path: P) -> bool {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            use std::path::Component;
            let has_root = path.as_ref().has_root();
            let has_children = || path
                .as_ref()
                .components()
                .any(|c| matches!(c, Component::CurDir | Component::ParentDir | Component::Normal(_)));
            has_root && !has_children()
        } else {
            matches!(path.as_ref().to_str(), Some("/"))
        }
    }
}

/// Get the binary name of current executing binary, a.k.a `arg[0]`.
pub fn lowercase_program_name() -> Option<String> {
    let mut program_executable = std::env::args().next().map(PathBuf::from)?;
    program_executable.set_extension("");

    let program_name = program_executable
        .file_name()
        .and_then(|oss| oss.to_str())?;
    Some(program_name.to_lowercase())
}

/// Lossy convert any [`OsStr`] representation into [`String`].
///
/// Check [`OsStr::to_string_lossy`] for detailed conversion.
pub fn to_string_lossy<S: AsRef<OsStr>>(s: S) -> String {
    s.as_ref().to_string_lossy().to_string()
}

/// Allowing the i18n framework to use the current system locale.
pub fn use_current_locale() {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    set_locale(&locale);
}

pub fn set_locale(loc: &str) {
    rust_i18n::set_locale(loc);
}

/// Waits until `duration` has elapsed.
///
/// Note: Use this in `async` context rather than [`std::thread::sleep`].
pub async fn async_sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(test)]
mod tests {
    use super::is_root_dir;

    #[test]
    fn root_dirs() {
        assert!(is_root_dir("/"));
        assert!(!is_root_dir("/bin"));
        assert!(!is_root_dir("root"));
        assert!(!is_root_dir("C:\\Windows\\System32"));

        // These are considered relative paths in Unix (which can be created using `mkdir`)
        #[cfg(windows)]
        {
            assert!(is_root_dir("D:\\"));
            assert!(is_root_dir("C:\\\\"));
        }
    }
}
