use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};
use url::Url;

type LocaleMap = HashMap<String, String>;

static BUILD_CFG_SINGLETON: OnceLock<BuildConfig> = OnceLock::new();

macro_rules! overridden {
    ($name:ident (&self) -> &$ret:ty) => {
        pub fn $name<'a>(&'a self, toolkit_name_: &str) -> &'a $ret {
            self.overrides
                .get(toolkit_name_)
                .map(|cfg_| &cfg_.$name)
                .unwrap_or(&self.overridable.$name)
        }
    };
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuildConfig {
    pub identifier: String,
    pub home_page_url: Url,
    #[serde(flatten)]
    overridable: OverridableConfig,
    pub cargo: CargoConfig,
    pub locale: HashMap<String, LocaleMap>,
    #[serde(rename = "override")]
    overrides: HashMap<String, OverridableConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct OverridableConfig {
    rustup_dist_server: Url,
    rustup_update_root: Url,
    rim_dist_server: Url,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CargoConfig {
    pub registry_name: String,
    pub registry_url: String,
}

impl BuildConfig {
    pub fn load() -> &'static Self {
        BUILD_CFG_SINGLETON.get_or_init(|| {
            let raw = include_str!("../../../configuration.toml");
            toml::from_str(raw).expect("unable to load build configuration")
        })
    }

    overridden!(rustup_dist_server(&self) -> &Url);
    overridden!(rustup_update_root(&self) -> &Url);
    overridden!(rim_dist_server(&self) -> &Url);
}
