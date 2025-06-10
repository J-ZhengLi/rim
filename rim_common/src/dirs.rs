//! Common Directories that are outside of install dir

use std::{
    env,
    path::{Path, PathBuf},
};

#[macro_export]
/// Declare a statically allocated `OnceLock` path, and create that directory if it does not exists.
macro_rules! get_path_and_create {
    ($path_ident:ident, $init:expr) => {{
        static $path_ident: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        let __path__ = $path_ident.get_or_init(|| $init);
        $crate::utils::ensure_dir(__path__)
            .expect("unable to create one of the directory under installation folder");
        __path__
    }};
}

/// Get a path to user's "home" directory.
///
/// The home directory is determined by a combination of ways
/// with a fallback order:
///
/// 1. The `HOME` environment variable
/// 2. The `USERPROFILE` environment variable (Windows only)
/// 3. The [`home_dir`](dirs::home_dir) function of the `dirs` crate
///
/// # Panic
///
/// Will panic if such directory cannot be determined by neither the `HOME` env var
/// nor [`dirs::home_dir`] function.
pub fn home_dir() -> PathBuf {
    let base = env::var_os("HOME").filter(|oss| !oss.is_empty());
    #[cfg(windows)]
    let base = base.or_else(|| env::var_os("USERPROFILE").filter(|oss| !oss.is_empty()));

    base.map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().expect("home directory cannot be determined."))
}

/// (User) Configuration directory to store all our configs.
///
/// Note: This dir will be stored under OS's `config_dir`, which can be:
/// - `$HOME/.config/rim` on Linux
/// - `$HOME/Library/Application Support/rim` on macOS
/// - `$HOME\AppData\Roaming\rim` on Windows
///
/// # Panic
/// Panic if the OS's config directory cannot be determined, which typically meaning
/// that the `HOME` env var is missing and the current OS is not support by the [`dirs`] crate.
pub fn rim_config_dir() -> &'static Path {
    get_path_and_create!(RIM_CONFIG_DIR, {
        let mut config_root = home_dir();

        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                config_root.push(".config")
            } else if #[cfg(windows)] {
                config_root.push("AppData");
                config_root.push("Roaming");
            } else if #[cfg(target_os = "macos")] {
                config_root.push("Library");
                config_root.push("Application Support");
            } else {
                // fallback to directly use `dir::config_dir`.
                // The reason we not directly using it was because we can
                // support mocked test on Windows by setting the `HOME` env var.
                dirs::config_dir().expect(
                    "unable to determine config directory, maybe your OS is not supported"
                );
            }
        }
        config_root.push("rim");
        config_root
    })
}
