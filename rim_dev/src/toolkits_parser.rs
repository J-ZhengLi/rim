//! Types for deserializing `toolkits.toml` under resources.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::fs;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use toml::{map::Map, Value};
use url::Url;

use crate::common::resources_dir;

pub(crate) const PACKAGE_DIR: &str = "packages";

#[derive(Debug, Deserialize)]
pub(crate) struct Toolkits {
    /// global configuration that used for vendoring packages
    pub(crate) config: GlobalConfig,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GlobalConfig {
    /// the server to download Rust toolchain
    pub(crate) rust_server: Url,
    /// the server to download rustup
    pub(crate) rustup_server: Url,
    /// the targets that we support
    pub(crate) targets: Vec<Target>,
    /// the compoents that will be downloaded for offline packaging
    pub(crate) components: Vec<Component>,
    /// directory to download packages into
    #[serde(default = "default_package_dir")]
    package_dir: PathBuf,
}

impl GlobalConfig {
    /// Return the absolute package directory path.
    pub(crate) fn abs_package_dir(&self) -> PathBuf {
        if self.package_dir.is_absolute() {
            self.package_dir.clone()
        } else {
            resources_dir().join(&self.package_dir)
        }
    }

    /// Combine a full URL with given `path` (without the `dist` component) from rust dist server.
    pub(crate) fn rust_dist_url(&self, path: &str) -> String {
        format!("{}/dist/{path}", self.rust_server)
    }
}

fn default_package_dir() -> PathBuf {
    PACKAGE_DIR.into()
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

/// Basically a `ToolkitManifest` with configuration.
///
/// However, instead of re-writing `ToolkitManifest` in the main crate,
/// this struct only uses a raw toml table ([`toml::Value`]) to represent it.
#[derive(Debug, Deserialize)]
pub(crate) struct Toolkit {
    pub(crate) config: ToolkitConfig,
    /// store `ToolkitManifest` as raw value
    #[serde(flatten)]
    pub(crate) value: Value,
}

impl Toolkit {
    pub(crate) fn manifest(&self) -> Result<&Map<String, Value>> {
        // The manifest value is behind a `value` key, we need to extract that.
        let inner_table = self
            .value
            .as_table()
            .and_then(|map| map.get("value"))
            .and_then(|m| m.as_table())
            .ok_or_else(|| anyhow!("invalid toolkit manifest"))?;
        Ok(inner_table)
    }

    pub(crate) fn manifest_mut(&mut self) -> Result<&mut Map<String, Value>> {
        // The manifest value is behind a `value` key, we need to extract that.
        let inner_table = self
            .value
            .as_table_mut()
            .and_then(|map| map.get_mut("value"))
            .and_then(|m| m.as_table_mut())
            .ok_or_else(|| anyhow!("invalid toolkit manifest"))?;
        Ok(inner_table)
    }
    /// Convert the value to toml string, which can be treated as `toolkit-manifest`.
    pub(crate) fn manifest_string(&self) -> Result<String> {
        Ok(toml::to_string(self.manifest()?)?)
    }

    /// Try getting the mutable `[tools.target]` map of the toolkit-manifest,
    /// return `None` if it can't be found, which means that this toolkit
    /// does not offer any third party tools.
    ///
    /// # Panic
    /// Panic when this toolkit manifest is invalid.
    pub(crate) fn targeted_tools_mut(&mut self) -> Option<&mut Map<String, Value>> {
        self.manifest_mut()
            .unwrap()
            .get_mut("tools")?
            .get_mut("target")?
            .as_table_mut()
    }

    /// Try getting the mutable `[rust]` map of the toolkit-manifest.
    ///
    /// # Panic
    /// Panic when this toolkit manifest is invalid.
    /// In addition, by the rules of toolkit manifest, missing a `[rust]` section
    /// also considered as invalid format.
    pub(crate) fn rust_section_mut(&mut self) -> &mut Map<String, Value> {
        self.manifest_mut()
            .unwrap()
            .get_mut("rust")
            .expect("invalid toolkit manifest: missing `[rust]` section")
            .as_table_mut()
            .expect("invalid `[rust]` section format")
    }

    /// Try getting the **toolkit's** version string.
    ///
    /// # Panic
    /// Panic when this toolkit manifest is invalid.
    pub(crate) fn version(&self) -> Option<&str> {
        let ver = self
            .manifest()
            .unwrap()
            .get("version")?
            .as_str()
            .expect("invalid version format");
        Some(ver)
    }

    /// Try getting the **toolkit's** actual name.
    ///
    /// # Panic
    /// Panic when this toolkit manifest is invalid.
    pub(crate) fn name(&self) -> Option<&str> {
        let ver = self
            .manifest()
            .unwrap()
            .get("name")?
            .as_str()
            .expect("invalid version format");
        Some(ver)
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

    /// Try getting the version of rust, which is specified as `[rust.version]`.
    ///
    /// # Panic
    /// Panic when this toolkit manifest is invalid.
    /// In addition, by the rules of toolkit manifest, both `[rust]` and `[rust.version]`
    /// are required fields, missing such values will also be considered as invalid.
    pub(crate) fn rust_version(&self) -> &str {
        let rust = self
            .manifest()
            .unwrap()
            .get("rust")
            .and_then(|v| v.as_table())
            .expect("invalid toolkit manifest: missing `[rust]` section");
        rust["version"]
            .as_str()
            .expect("rust toolchain version must be `str` object")
    }

    /// Convenient method the get the toolkit's release date,
    /// same as `toolkit.config.date`.
    pub(crate) fn date(&self) -> &str {
        &self.config.date
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ToolkitConfig {
    pub(crate) date: String,
}
