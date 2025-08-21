//! The major configuration file for this app, containing information about which version to skip,
//! when the updates are checked, how long until next updates will be checked etc.

use crate::setter;
use crate::{dirs::rim_config_dir, types::TomlParser};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq, Clone)]
pub struct Configuration {
    pub language: Option<Language>,
    pub update: UpdateConfig,
}

impl TomlParser for Configuration {
    const FILENAME: &'static str = "configuration.toml";
}

impl Configuration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try loading from [`rim_config_dir`], return `None` if it doesn't exists yet.
    pub fn try_load_from_config_dir() -> Option<Self> {
        Self::load_from_dir(rim_config_dir()).ok()
    }

    /// Loading from [`rim_config_dir`] or return default.
    ///
    /// This guarantee to return an object,
    /// even if the file does not exists, the default will got returned.
    pub fn load_from_config_dir() -> Self {
        Self::try_load_from_config_dir().unwrap_or_default()
    }

    /// Write the configuration to [`rim_config_dir`].
    pub fn write(&self) -> Result<()> {
        self.write_to_dir(rim_config_dir())
    }

    setter!(set_language(self.language, val: Language) { Some(val) });
    setter!(set_manager_update_channel(
        self.update.manager_update_channel,
        ReleaseChannel
    ));
    setter!(set_auto_check_manager_updates(
        self.update.auto_check_manager_updates,
        bool
    ));
    setter!(set_auto_check_toolkit_updates(
        self.update.auto_check_toolkit_updates,
        bool
    ));
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum Language {
    #[serde(rename = "zh-CN")]
    CN,
    #[default]
    #[serde(rename = "en-US")]
    EN,
}

impl Language {
    pub fn possible_values() -> &'static [Language] {
        &[Self::CN, Self::EN]
    }
    /// This is the `str` used for setting locale,
    /// make sure the values match the filenames under `<root>/locales`.
    pub fn locale_str(&self) -> &str {
        match self {
            Self::CN => "zh-CN",
            Self::EN => "en-US",
        }
    }
}

impl FromStr for Language {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cn" | "zh-cn" => Ok(Self::CN),
            "en" | "en-us" => Ok(Self::EN),
            _ => Err(anyhow::anyhow!(
                "invalid or unsupported language option: {s}"
            )),
        }
    }
}

/// App update channel.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ReleaseChannel {
    #[default]
    Stable,
    Beta,
}

impl Display for ReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}").to_lowercase())
    }
}

/// Representing the configuration for update checker.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateConfig {
    /// Channel of manager (app) updates to check, i.e. "stable", "beta"
    #[serde(default)]
    pub manager_update_channel: ReleaseChannel,
    /// Automatically checking for manager (app) updates.
    #[serde(default = "bool_true")]
    pub auto_check_manager_updates: bool,
    /// Automatically checking for toolkit updates.
    #[serde(default = "bool_true")]
    pub auto_check_toolkit_updates: bool,
}

// Return `true` for the serde default arg.
// really? there's no better way?
fn bool_true() -> bool {
    true
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            manager_update_channel: ReleaseChannel::default(),
            auto_check_manager_updates: true,
            auto_check_toolkit_updates: true,
        }
    }
}

impl UpdateConfig {
    pub fn new() -> Self {
        Self::default()
    }

    setter!(manager_update_channel(
        self.manager_update_channel,
        ReleaseChannel
    ));
    setter!(auto_check_manager_updates(
        self.auto_check_manager_updates,
        bool
    ));
    setter!(auto_check_toolkit_updates(
        self.auto_check_toolkit_updates,
        bool
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backward_compatible() {
        let input = r#"
[update]
manager = { skip = "0.1.0", last-run = "1970-01-01T00:00:00" }
toolkit = { skip = "1.0.0", last-run = "1970-01-01T00:00:00" }"#;
        let expected = Configuration::from_str(input).unwrap();
        assert_eq!(expected, Configuration::default());
    }

    #[test]
    fn default_serialization() {
        let conf = Configuration::new();

        assert_eq!(
            conf.to_toml().unwrap(),
            r#"[update]
manager-update-channel = "stable"
auto-check-manager-updates = true
auto-check-toolkit-updates = true
"#
        );
    }

    #[test]
    fn configured_serialization() {
        let conf = Configuration::new()
            .set_language(Language::CN)
            .set_manager_update_channel(ReleaseChannel::Beta)
            .set_auto_check_manager_updates(false)
            .set_auto_check_toolkit_updates(false);

        let expected = conf.to_toml().unwrap();
        assert_eq!(
            expected,
            r#"language = "CN"

[update]
manager-update-channel = "beta"
auto-check-manager-updates = false
auto-check-toolkit-updates = false
"#
        );
    }

    #[test]
    fn lang_config() {
        let input = "language = \"CN\"\n[update]";

        let expected = Configuration::from_str(input).unwrap();
        assert_eq!(expected.language, Some(Language::CN));

        // check if the language consistence since we have a `FromStr` impl for it.
        let back_to_str = toml::to_string(&expected).unwrap();
        assert_eq!(
            back_to_str,
            "language = \"CN\"\n\n\
            [update]\n\
            manager-update-channel = \"stable\"\n\
            auto-check-manager-updates = true\n\
            auto-check-toolkit-updates = true\n"
        );
    }
}
