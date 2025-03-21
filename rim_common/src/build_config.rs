use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};
use url::Url;

type LocaleMap = HashMap<String, String>;

static BUILD_CFG_SINGLETON: OnceLock<BuildConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    pub identifier: String,
    pub home_page_url: Url,
    pub rustup_dist_server: Url,
    pub rustup_update_root: Url,
    pub rim_dist_server: Url,
    pub cargo: CargoConfig,
    pub locale: HashMap<String, LocaleMap>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CargoConfig {
    pub registry_name: String,
    pub registry_url: String,
}

impl BuildConfig {
    pub(crate) fn load() -> &'static Self {
        BUILD_CFG_SINGLETON.get_or_init(|| {
            let raw = include_str!("../../configuration.toml");
            toml::from_str(raw).expect("unable to load build configuration")
        })
    }
}
