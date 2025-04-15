//! Types for deserializing `toolkits.toml` under resources.

use anyhow::Result;
use rim_common::types::{RustToolchain, ToolMap, ToolkitManifest};
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

/// Basically a [`ToolkitManifest`] with additional configuration.
#[derive(Debug, Deserialize)]
pub(crate) struct Toolkit {
    pub(crate) config: ToolkitConfig,
    /// store `ToolkitManifest` as raw value
    value: ToolkitManifest,
}

impl Toolkit {
    /// Convert the value to toml string, which can be treated as `toolkit-manifest`.
    pub(crate) fn manifest_string(&self) -> Result<String> {
        Ok(toml::to_string(&self.value)?)
    }

    /// Try getting the mutable `[tools.target]` map of the toolkit-manifest,
    /// return `None` if it can't be found, which means that this toolkit
    /// does not offer any third party tools.
    pub(crate) fn targeted_tools_mut(&mut self) -> &mut HashMap<String, ToolMap> {
        &mut self.value.tools.target
    }

    /// Try getting the mutable `[rust]` map of the toolkit-manifest.
    pub(crate) fn rust_section_mut(&mut self) -> &mut RustToolchain {
        &mut self.value.rust
    }

    /// Try getting the **toolkit's** version string.
    pub(crate) fn version(&self) -> Option<&str> {
        self.value.version.as_deref()
    }

    /// Try getting the **toolkit's** actual name.
    pub(crate) fn name(&self) -> Option<&str> {
        self.value.name.as_deref()
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
        &self.value.rust.channel
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
    // TODO: add web-only related code
}
