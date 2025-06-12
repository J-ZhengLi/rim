#[macro_use]
extern crate log;
#[macro_use]
extern crate rust_i18n;

pub mod dirs;
pub mod types;
pub mod utils;
pub mod version_info;

use types::BuildConfig;

i18n!("../locales", fallback = "en");

/// Loads build configurations, such as the default URLs that this program needs.
pub fn build_config() -> &'static BuildConfig {
    BuildConfig::load()
}

#[macro_export]
macro_rules! cfg_locale {
    ($lang:expr, $key:expr) => {
        $crate::build_config()
            .locale
            .get($lang)
            .and_then(|_m_| _m_.get($key))
            .map(|_s_| _s_.as_str())
            .unwrap_or($key)
    };
}
