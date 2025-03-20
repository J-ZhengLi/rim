mod build_config;

use build_config::BuildConfig;

/// Loads build configurations, such as the default URLs that this program needs.
pub fn build_config() -> &'static BuildConfig {
    BuildConfig::load()
}

#[macro_export]
macro_rules! cfg_locale {
    ($lang:expr, $key:expr) => {
        rim_common::build_config()
            .locale
            .get($lang)
            .and_then(|_m_| _m_.get($key))
            .map(|_s_| _s_.as_str())
            .unwrap_or($key)
    };
}
