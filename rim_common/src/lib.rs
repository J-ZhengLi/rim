mod build_config;

use build_config::BuildConfig;

/// Loads build configurations, such as the default URLs that this program needs.
pub fn build_config() -> &'static BuildConfig {
    BuildConfig::load()
}
