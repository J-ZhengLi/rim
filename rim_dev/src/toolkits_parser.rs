//! Types for deserializing `toolkits.toml` under resources.

use anyhow::Result;
use rim_common::types::ToolkitManifest;
use serde::Deserialize;
use std::fs;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use url::Url;

use crate::common::resources_dir;

pub(crate) const PACKAGE_DIR: &str = "packages";

#[derive(Debug, Deserialize)]
pub(crate) struct Toolkits {
    /// global configuration that used for vendoring packages
    pub(crate) config: Configuration,
    /// map of toolkits that we distribute
    pub(crate) toolkit: HashMap<String, Toolkit>,
}

impl Toolkits {
    pub(crate) fn load() -> Result<Self> {
        let toolkits_path = resources_dir().join("toolkits.toml");
        let toolkits_content = fs::read_to_string(toolkits_path)?;
        Ok(toml::from_str(&toolkits_content)?)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Configuration {
    /// the server to download Rust toolchain
    pub(crate) rust_server: Option<Url>,
    /// the server to download rustup
    pub(crate) rustup_server: Option<Url>,
    #[serde(default)]
    /// the targets that we support
    pub(crate) targets: Vec<Target>,
    #[serde(default)]
    /// the components that will be downloaded for offline packaging
    pub(crate) components: Vec<Component>,
    /// directory to download packages into
    #[serde(default = "default_package_dir")]
    package_dir: PathBuf,
}

impl Configuration {
    /// Return the absolute package directory path.
    pub(crate) fn abs_package_dir(&self) -> PathBuf {
        if self.package_dir.is_absolute() {
            self.package_dir.clone()
        } else {
            resources_dir().join(&self.package_dir)
        }
    }

    /// Combine a full URL with given `path` (without the `dist` component) from rust dist server.
    ///
    /// # Panic
    /// Panics if `rust_server` is `None`
    pub(crate) fn rust_dist_url(&self, path: &str) -> String {
        format!(
            "{}/dist/{path}",
            self.rust_server
                .as_ref()
                .expect("missing `rust-server` config value")
        )
    }

    /// Combine a full URL with given `path` (without the `dist` component) from rustup dist server.
    ///
    /// # Panic
    /// Panics if `rustup_server` is `None`
    pub(crate) fn rustup_dist_url(&self, path: &str) -> String {
        format!(
            "{}/dist/{path}",
            self.rustup_server
                .as_ref()
                .expect("missing `rustup-server` config value")
        )
    }
}

fn default_package_dir() -> PathBuf {
    PACKAGE_DIR.into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum Target {
    Simple(String),
    Detailed {
        triple: String,
        #[serde(rename = "release-mode")]
        release_mode: Option<ReleaseMode>,
    },
}

impl Target {
    pub(crate) fn triple(&self) -> &str {
        match self {
            Self::Simple(tri) => tri,
            Self::Detailed { triple, .. } => triple,
        }
    }
    pub(crate) fn release_mode(&self) -> Option<ReleaseMode> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { release_mode, .. } => *release_mode,
        }
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ReleaseMode {
    Cli,
    Gui,
    #[default]
    Both,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum Component {
    Simple(String),
    Detailed {
        name: String,
        target: Option<String>,
        /// optional flag to mark a component as supporting all targets (*), such as `rust-src`
        #[serde(default, rename = "wildcard-target")]
        wildcard_target: bool,
        #[serde(default, rename = "excluded-targets")]
        excluded_targets: HashSet<String>,
    },
}

/// Basically a [`ToolkitManifest`] with additional configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct Toolkit {
    pub(crate) config: ToolkitConfig,
    #[serde(alias = "value")]
    /// store `ToolkitManifest` as raw value
    pub(crate) manifest: ToolkitManifest,
}

impl Toolkit {
    /// Convert the value to toml string, which can be treated as `toolkit-manifest`.
    pub(crate) fn manifest_string(&self) -> Result<String> {
        Ok(toml::to_string(&self.manifest)?)
    }

    /// Try getting the **toolkit's** version string.
    pub(crate) fn version(&self) -> Option<&str> {
        self.manifest.version.as_deref()
    }

    /// Try getting the **toolkit's** actual name.
    pub(crate) fn name(&self) -> Option<&str> {
        self.manifest.name.as_deref()
    }

    /// Get the full name of this toolkit, which is the combination of
    /// its name and version.
    pub(crate) fn full_name(&self) -> String {
        format!(
            "{}{}",
            self.name().unwrap_or("UnknownToolkit"),
            self.version().map(|s| format!("-{s}")).unwrap_or_default()
        )
        .replace(' ', "-")
    }

    /// Try getting the version of rust, which is specified as
    /// ```toml
    /// [rust]
    /// version = "0.1.0"
    /// #          -----
    /// ```
    pub(crate) fn rust_version(&self) -> &str {
        &self.manifest.rust.channel
    }

    /// Convenient method the get the toolkit's release date,
    /// same as `toolkit.config.date`.
    pub(crate) fn date(&self) -> &str {
        &self.config.date
    }

    /// Overrides the `original` (global) configuration with the local one then return
    /// a "combined" configuration matching the below specification:
    ///
    /// 1. If local config is `None`, return a clone of the `original`.
    /// 2. When a field is empty in one of the config but non-empty in the other one,
    ///    then "combined" config will have the non-empty value field.
    /// 3. The local config has higher priority, meaning if a field is non-empty in both
    ///    local and `original` config, the local one will be used instead.
    pub(crate) fn overridden_config(&self, original: &Configuration) -> Configuration {
        let Some(local) = &self.config.overridden else {
            return original.clone();
        };

        #[allow(clippy::obfuscated_if_else, reason = "if-else does not looks clearer")]
        Configuration {
            rust_server: local
                .rust_server
                .clone()
                .or_else(|| original.rust_server.clone()),
            rustup_server: local
                .rustup_server
                .clone()
                .or_else(|| original.rustup_server.clone()),
            targets: local
                .targets
                .is_empty()
                .then(|| original.targets.clone())
                .unwrap_or_else(|| local.targets.clone()),
            components: local
                .components
                .is_empty()
                .then(|| original.components.clone())
                .unwrap_or_else(|| local.components.clone()),
            package_dir: local.package_dir.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ToolkitConfig {
    pub(crate) date: String,
    /// Overrides the global configuration
    #[serde(rename = "override")]
    overridden: Option<Configuration>,
    // TODO: add web-only related code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_override() {
        let raw = r#"
[config]
rust-server = "https://foo.com"
rustup-server = "https://foo.com/rustup"
targets = ["x86_64-unknown-linux-gnu"]
components = []

[toolkit.foo.config]
date = "1970-01-01"

[toolkit.foo.config.override]
rust-server = "https://bar.com"
rustup-server = "https://bar.com/rustup"
targets = []
package-dir = "/path/to/packages"

[toolkit.foo.value]
name = "Test"
version = "1.0.0"

[toolkit.foo.value.rust]
channel = "1.0.0"
"#;
        let de: Toolkits = toml::from_str(raw).unwrap();
        let config_for_foo = de.toolkit.get("foo").unwrap().overridden_config(&de.config);

        assert_eq!(config_for_foo.rust_server, "https://bar.com".parse().ok());
        assert_eq!(
            config_for_foo.rustup_server,
            "https://bar.com/rustup".parse().ok()
        );
        assert_eq!(config_for_foo.targets.len(), 1);
        assert!(config_for_foo.components.is_empty());
        assert_eq!(
            config_for_foo.package_dir,
            PathBuf::from("/path/to/packages")
        );
    }
}
