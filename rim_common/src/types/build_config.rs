use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};
use url::Url;

use crate::types::CargoRegistry;

type LocaleMap = HashMap<String, String>;

static BUILD_CFG_SINGLETON: OnceLock<BuildConfig> = OnceLock::new();

macro_rules! getter {
    ($name:ident: &$ret:ty) => {
        pub fn $name(&self) -> &$ret {
            &self.config.$name
        }
    };
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    pub identifier: String,
    pub home_page_url: Url,
    #[serde(flatten)]
    config: SourceConfig,
    pub registry: CargoRegistry,
    pub locale: HashMap<String, LocaleMap>,
}

#[derive(Debug, Clone, Deserialize)]
struct SourceConfig {
    rustup_dist_server: Url,
    rustup_update_root: Url,
    rim_dist_server: Url,
}

impl BuildConfig {
    pub fn load() -> &'static Self {
        BUILD_CFG_SINGLETON.get_or_init(|| {
            let raw = include_str!("../../../configuration.toml");
            toml::from_str(raw).expect("unable to load build configuration")
        })
    }

    /// The application name, which should be the name of this binary after installation.
    pub fn app_name(&self) -> String {
        format!("{}-manager", self.identifier)
    }

    getter!(rustup_dist_server: &Url);
    getter!(rustup_update_root: &Url);
    getter!(rim_dist_server: &Url);
}
