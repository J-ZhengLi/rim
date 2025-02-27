//! `ToolsetManifest` contains information about each dist package,
//! such as its name, version, and what's included etc.

use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use url::Url;

use crate::components::{Component, ComponentType};
use crate::core::custom_instructions;
use crate::core::tools::ToolKind;
use crate::{setter, utils};

use super::TomlParser;

/// A map of tools, contains the name and source package information.
///
/// This is basically a wrapper type to `IndexMap`, but with tailored functionalities to suit
/// the needs of tools' installation and uninstallation.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
pub struct ToolMap(IndexMap<String, ToolInfo>);

/// A Rust toolchain component, such as `rustc`, `cargo`, `rust-docs`
/// or even toolchain profile as as `minimal`, `default`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolchainComponent {
    pub name: String,
    pub is_profile: bool,
}

impl ToolchainComponent {
    pub fn new<T: ToString>(name: T) -> Self {
        Self {
            name: name.to_string(),
            is_profile: false,
        }
    }
    setter!(is_profile(self.is_profile, bool));
}

pub struct ToolMapIter<'a> {
    iter: indexmap::map::Iter<'a, String, ToolInfo>,
}

impl<'a> Iterator for ToolMapIter<'a> {
    type Item = (&'a str, &'a ToolInfo);
    fn next(&mut self) -> Option<Self::Item> {
        let (name, info) = self.iter.next()?;
        // The `key` of each iteration prefers the identifier over the name.
        let identifier = info.identifier().unwrap_or(name.as_str());
        Some((identifier, info))
    }
}

impl ToolMap {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn iter(&self) -> ToolMapIter<'_> {
        ToolMapIter {
            iter: self.0.iter(),
        }
    }
}

impl Deref for ToolMap {
    type Target = IndexMap<String, ToolInfo>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(String, ToolInfo)> for ToolMap {
    fn from_iter<T: IntoIterator<Item = (String, ToolInfo)>>(iter: T) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<'a> IntoIterator for &'a ToolMap {
    type Item = (&'a str, &'a ToolInfo);
    type IntoIter = ToolMapIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ToolMapIter {
            iter: self.0.iter(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolsetManifest {
    /// Product name to be cached after installation, so that we can show it as `installed`
    pub name: Option<String>,
    /// Product version to be cached after installation, so that we can show it as `installed`
    pub version: Option<String>,

    pub(crate) rust: RustToolchain,
    #[serde(default)]
    pub(crate) tools: Tools,
    /// Proxy settings that used for download.
    pub proxy: Option<Proxy>,
    /// Path to the manifest file.
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl TomlParser for ToolsetManifest {
    const FILENAME: &'static str = "toolset-manifest.toml";

    fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let raw = utils::read_to_string("manifest", &path)?;
        let mut temp_manifest = Self::from_str(&raw)?;
        temp_manifest.path = Some(path.as_ref().to_path_buf());
        Ok(temp_manifest)
    }
}

impl ToolsetManifest {
    /// Load toolset manfest from installed root.
    ///
    /// # Note
    /// Only use this during **manager** mode.
    pub fn load_from_install_dir() -> Result<Self> {
        let root = super::get_installed_dir();
        Self::load(root.join(Self::FILENAME))
    }

    // Get a list of all optional componets.
    pub fn optional_toolchain_components(&self) -> &[String] {
        self.rust.optional_components.as_slice()
    }

    pub fn get_tool_description(&self, toolname: &str) -> Option<&str> {
        self.tools.descriptions.get(toolname).map(|s| s.as_str())
    }

    /// Get the group name of a certain tool, if exist.
    pub fn group_name(&self, toolname: &str) -> Option<&str> {
        self.tools
            .group
            .iter()
            .find_map(|(group, tools)| tools.contains(toolname).then_some(group.as_str()))
    }

    pub fn toolchain_group_name(&self) -> &str {
        self.rust.name.as_deref().unwrap_or("Rust Toolchain")
    }

    pub fn toolchain_profile(&self) -> Option<&ToolchainProfile> {
        self.rust.profile.as_ref()
    }

    /// Get the path to bundled `rustup-init` binary if there has one.
    pub fn rustup_bin(&self) -> Result<Option<PathBuf>> {
        let cur_target = env!("TARGET");
        let par_dir = self.package_root()?;
        let rel_path = self.rust.rustup.get(cur_target);

        Ok(rel_path.map(|p| par_dir.join(p)))
    }

    pub fn offline_dist_server(&self) -> Result<Option<Url>> {
        let Some(server) = &self.rust.offline_dist_server else {
            return Ok(None);
        };
        let par_dir = self.package_root()?;
        let full_path = par_dir.join(server);

        Url::from_directory_path(&full_path)
            .map(Option::Some)
            .map_err(|_| anyhow!("path '{}' cannot be converted to URL", full_path.display()))
    }

    /// Get the tools that are only available in current target.
    pub fn current_target_tools(&self) -> Option<&ToolMap> {
        let cur_target = env!("TARGET");
        self.tools.target.get(cur_target)
    }

    /// Get the mut reference to the tools that are only available in current target.
    ///
    /// Return `None` if there are no available tools in the current target.
    pub fn current_target_tools_mut(&mut self) -> Option<&mut ToolMap> {
        let cur_target = env!("TARGET");
        self.tools.target.get_mut(cur_target)
    }

    /// Like `current_target_tools` but instead of getting a map of tools,
    /// this will get a list of tools and components in [`Component`] format.
    ///
    /// If `fresh_install` is `true`, this function will look through user's environment to see if
    /// a specific tool is already installed or not.
    pub fn current_target_components(&self, fresh_install: bool) -> Result<Vec<Component>> {
        let tc_channel = self.rust_version();

        let profile = self.toolchain_profile().cloned().unwrap_or_default();
        let profile_name = profile.verbose_name.as_deref().unwrap_or(&profile.name);
        // Add a component that represents rust toolchain
        let mut components = vec![Component::new(
            profile_name,
            profile.description.as_deref().unwrap_or_default(),
        )
        .with_group(Some(self.toolchain_group_name()))
        .set_kind(ComponentType::ToolchainProfile)
        .required(true)
        .with_version(Some(tc_channel))];

        for component in self.optional_toolchain_components() {
            components.push(
                Component::new(
                    component,
                    self.get_tool_description(component).unwrap_or_default(),
                )
                .with_group(Some(self.toolchain_group_name()))
                .optional(true)
                .set_kind(ComponentType::ToolchainComponent)
                // toolchain component's version are unified
                .with_version(Some(tc_channel)),
            );
        }

        if let Some(tools) = self.current_target_tools() {
            let installed_in_env = if fresh_install {
                // components that are already installed in user's machine, such as vscode, or mingw.
                self.already_installed_tools()
            } else {
                vec![]
            };

            for (tool_name, tool_info) in tools {
                let installed = installed_in_env.contains(&tool_name);
                let version = if fresh_install && installed {
                    // if the tool is already installed but we are doing a fresh install here,
                    // which means it was installed by user not by `rim`,
                    // therefore we don't know the version.
                    None
                } else {
                    tool_info.version()
                };
                components.push(
                    Component::new(
                        tool_name,
                        self.get_tool_description(tool_name).unwrap_or_default(),
                    )
                    .with_group(self.group_name(tool_name))
                    .with_tool_installer(tool_info)
                    .required(tool_info.is_required())
                    .optional(tool_info.is_optional())
                    .installed(installed)
                    .with_version(version)
                    .with_display_name(tool_info.display_name().unwrap_or(tool_name)),
                );
            }
        }

        Ok(components)
    }

    /// Get a list of tool names if those are already installed in current target.
    pub fn already_installed_tools(&self) -> Vec<&str> {
        let Some(map) = self.current_target_tools() else {
            return vec![];
        };
        map.keys()
            .filter_map(|name| custom_instructions::is_installed(name).then_some(name.as_str()))
            .collect()
    }

    /// Returns the absolute path of the package root.
    ///
    /// A package root is:
    /// - The folder to store tools' packages such as `tools/hello-world.tar.xz`, etc.
    /// - The folder to store local rustup dist server such as `toolchain/`, where all
    ///     the rust installer stuffs stored, such as `toolchain/channel-rust-x.xx.x.toml`.
    /// - Usually the parent directory of this manifest file.
    ///
    /// Note: In `release` build, because this program has an embedded toolkit manifest,
    /// therefore it assumes the parent directory of this running binary as the package root.
    /// But in `debug` build, because we have cached all those packages inside of
    /// `resource/packages` folder, we will be assuming it as the pacakge root.
    fn package_root(&self) -> Result<PathBuf> {
        let res = if let Some(p) = &self.path {
            p.to_path_buf()
        } else if env!("PROFILE") == "debug" {
            let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            dir.push("resources");
            dir.push("packages");
            dir.push(format!(
                "{}{}",
                self.name.as_deref().unwrap_or("UnknownToolkit"),
                self.version
                    .as_ref()
                    .map(|s| format!("-{s}"))
                    .unwrap_or_default()
            ));
            dir.push(env!("TARGET"));
            dir
        } else {
            std::env::current_exe()?
                .parent()
                .unwrap_or_else(|| unreachable!("an executable always have a parent directory"))
                .to_path_buf()
        };
        Ok(res)
    }

    /// Turn all the relative paths in the `tools` section to some absolute paths.
    ///
    /// There are some rules applied when converting, including:
    /// 1. If the manifest was loaded from a path,
    ///     all relative paths will be forced to combine with the path loading from.
    /// 2. If the manifest was not loaded from path,
    ///     all relative paths will be forced to combine with the parent directory of this executable.
    ///     (Assuming the manifest was baked in the executable)
    ///
    /// # Errors
    /// Return `Result::Err` if the manifest was not loaded from path, and the current executable path
    /// cannot be determined as well.
    pub fn adjust_paths(&mut self) -> Result<()> {
        let parent_dir = self.package_root()?;

        for tool in self.tools.target.values_mut() {
            for tool_info in tool.values_mut() {
                if let Some(path) = tool_info.path_mut() {
                    *path = utils::to_nomalized_abspath(path.as_path(), Some(&parent_dir))?;
                }
            }
        }
        Ok(())
    }

    /// Some package source might be missing if it has [`ToolSource::Restricted`],
    /// thus this function is required for the installation to work properly.
    ///
    /// Whan calling this function, a list of component name is needed to,
    /// which is a list of components that user selected for installation
    /// (we don't need to fill the source if they don't intend to install those).
    /// Then, this will apply a `callback` function trying to modify the source
    /// with a certain string returned from the callback function.
    pub fn fill_missing_package_source<F>(
        &mut self,
        components: &mut Vec<Component>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(String) -> Result<String>,
    {
        for tool in self.tools.target.values_mut() {
            for (name, tool_info) in tool.iter_mut() {
                let Some(comp_to_modify) = components.iter_mut().find(|c| &c.name == name) else {
                    continue;
                };
                let display_name = tool_info.display_name().unwrap_or(name).to_string();

                if let Some(source) = tool_info.user_provided_source_mut() {
                    let new_val = callback(display_name)?;
                    *source = Some(new_val.clone());

                    // try modify the ones in components as well
                    if let Some(s) = comp_to_modify
                        .tool_installer
                        .as_mut()
                        .and_then(|c| c.user_provided_source_mut())
                    {
                        *s = Some(new_val);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn rust_version(&self) -> &str {
        self.rust.version.as_str()
    }

    pub fn toolchain_display_name(&self) -> Option<&str> {
        self.rust
            .profile
            .as_ref()
            .and_then(|p| p.verbose_name.as_deref())
    }
}

/// The proxy for download
#[derive(Debug, Deserialize, Default, Serialize, PartialEq, Eq, Clone)]
pub struct Proxy {
    pub http: Option<Url>,
    pub https: Option<Url>,
    #[serde(alias = "no-proxy")]
    pub no_proxy: Option<String>,
}

impl TryFrom<&Proxy> for reqwest::Proxy {
    type Error = anyhow::Error;
    fn try_from(value: &Proxy) -> std::result::Result<Self, Self::Error> {
        let base = match (&value.http, &value.https) {
            // When nothing provided, use env proxy if there is.
            (None, None) => reqwest::Proxy::custom(|url| env_proxy::for_url(url).to_url()),
            // When both are provided, use the provided https proxy.
            (Some(_), Some(https)) => reqwest::Proxy::all(https.clone())?,
            (Some(http), None) => reqwest::Proxy::http(http.clone())?,
            (None, Some(https)) => reqwest::Proxy::https(https.clone())?,
        };
        let with_no_proxy = if let Some(no_proxy) = &value.no_proxy {
            base.no_proxy(reqwest::NoProxy::from_string(no_proxy))
        } else {
            // Fallback to using env var
            base.no_proxy(reqwest::NoProxy::from_env())
        };
        Ok(with_no_proxy)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct RustToolchain {
    pub(crate) version: String,
    pub(crate) profile: Option<ToolchainProfile>,
    /// Components are installed by default
    #[serde(default)]
    pub(crate) components: Vec<String>,
    /// Optional components are only installed if user choose to.
    #[serde(default)]
    pub(crate) optional_components: Vec<String>,
    /// Specifies a verbose name if this was provided.
    #[serde(alias = "group")]
    pub(crate) name: Option<String>,
    /// File [`Url`] to install rust toolchain.
    offline_dist_server: Option<String>,
    /// Contains target specific `rustup-init` binaries.
    #[serde(default)]
    rustup: HashMap<String, String>,
}

impl RustToolchain {
    #[allow(unused)]
    pub(crate) fn new(ver: &str) -> Self {
        Self {
            version: ver.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolchainProfile {
    pub name: String,
    pub verbose_name: Option<String>,
    pub description: Option<String>,
}

impl Default for ToolchainProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            verbose_name: None,
            description: None,
        }
    }
}

impl From<&str> for ToolchainProfile {
    fn from(value: &str) -> Self {
        Self {
            name: value.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
pub(crate) struct Tools {
    #[serde(default)]
    descriptions: BTreeMap<String, String>,
    /// Containing groups of tools.
    ///
    /// Note that not all tools will have a group.
    #[serde(default)]
    group: BTreeMap<String, HashSet<String>>,
    #[serde(default)]
    target: BTreeMap<String, ToolMap>,
}

impl Tools {
    #[allow(unused)]
    pub(crate) fn new<I>(targeted_tools: I) -> Tools
    where
        I: IntoIterator<Item = (String, ToolMap)>,
    {
        Self {
            descriptions: BTreeMap::default(),
            group: BTreeMap::default(),
            target: BTreeMap::from_iter(targeted_tools),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ToolInfo {
    /// Basic crates version, contains only its version, used for `cargo install`.
    ///
    /// # Example
    ///
    /// ```toml
    /// basic = "0.1.0"
    /// ```
    Basic(String),
    /// Detailed tool information, contains different kind of [`ToolSource`] and other options.
    ///
    /// # Example
    ///
    /// ```toml
    /// expand = { version = "0.2.0", option = true, identifier = "cargo-expand" }
    /// hello_world = { version = "0.2.0", option = true, path = "path/to/hello.zip" }
    /// ```
    Complex(ToolInfoDetails),
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolInfoDetails {
    #[serde(default)]
    required: bool,
    #[serde(default)]
    optional: bool,
    identifier: Option<String>,
    #[serde(flatten)]
    pub source: ToolSource,
    /// Pre-determined kind.
    /// If not provided, this will be automatically assumed when loading a tool using
    /// [`Tool::from_path`](crate::core::tools::Tool::from_path).
    pub kind: Option<ToolKind>,
    /// A name that only used for display purpose.
    pub display_name: Option<String>,
}

impl ToolInfoDetails {
    pub fn new(source: ToolSource) -> Self {
        Self {
            source,
            required: false,
            optional: false,
            identifier: None,
            kind: None,
            display_name: None,
        }
    }
    setter!(required(self.required, bool));
    setter!(optional(self.optional, bool));
    setter!(identifier(self.identifier, value: String) { Some(value) });
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Hash)]
#[serde(untagged)]
pub enum ToolSource {
    /// A tool that does not allowing redistribution are considerred as `restricted`.
    ///
    /// Source of this tool remains unknown until the program asks for user input
    /// before installation, and if user has such package they can enter a path to it
    /// then we (this software) can make the installation process easier for them.
    /// Or if a `default` is available, which should be a link to the official website
    /// to download such package, we can help user download the package online then run
    /// the installation.
    Restricted {
        restricted: bool,
        default: Option<String>,
        source: Option<String>,
        version: Option<String>,
    },
    Git {
        git: Url,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
    Url {
        version: Option<String>,
        url: Url,
        filename: Option<String>,
    },
    Path {
        version: Option<String>,
        path: PathBuf,
    },
    Version {
        #[serde(alias = "ver")]
        version: String,
    },
}

impl ToolInfo {
    pub fn path_mut(&mut self) -> Option<&mut PathBuf> {
        if let Self::Complex(details) = self {
            if let ToolSource::Path { path, .. } = &mut details.source {
                return Some(path);
            }
        }
        None
    }

    pub fn user_provided_source_mut(&mut self) -> Option<&mut Option<String>> {
        if let Self::Complex(details) = self {
            if let ToolSource::Restricted { source, .. } = &mut details.source {
                return Some(source);
            }
        }
        None
    }

    pub fn details(&self) -> Option<&ToolInfoDetails> {
        if let Self::Complex(details) = self {
            Some(details)
        } else {
            None
        }
    }

    pub fn is_required(&self) -> bool {
        self.details().map(|d| d.required).unwrap_or_default()
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Basic(ver) => Some(ver),
            Self::Complex(details) => match &details.source {
                ToolSource::Git { tag, .. } => tag.as_deref(),
                ToolSource::Version { version } => Some(version),
                ToolSource::Path { version, .. }
                | ToolSource::Url { version, .. }
                | ToolSource::Restricted { version, .. } => version.as_deref(),
            },
        }
    }

    pub fn is_optional(&self) -> bool {
        self.details().map(|d| d.optional).unwrap_or_default()
    }

    pub fn is_cargo_tool(&self) -> bool {
        match self {
            ToolInfo::Basic(_) => true,
            ToolInfo::Complex(details) => matches!(
                &details.source,
                ToolSource::Git { .. } | ToolSource::Version { .. }
            ),
        }
    }

    /// Retrieve the identifier string of this tool.
    ///
    /// ```toml
    /// "My Program" = { path = "/path/to/package", identifier = "my_program" }
    /// #                                                         ^^^^^^^^^^
    /// ```
    pub fn identifier(&self) -> Option<&str> {
        self.details().and_then(|d| d.identifier.as_deref())
    }

    /// Get the [`ToolKind`] of this tool.
    ///
    /// ```toml
    /// some_installer = { path = "/path/to/package", kind = "installer" }
    /// #                                                     ^^^^^^^^^
    /// ```
    pub fn kind(&self) -> Option<ToolKind> {
        self.details().and_then(|d| d.kind)
    }

    /// Get the display name of this tool if it has one.
    pub fn display_name(&self) -> Option<&str> {
        self.details().and_then(|d| d.display_name.as_deref())
    }

    pub fn is_restricted(&self) -> bool {
        matches!(
            self.details(),
            Some(ToolInfoDetails {
                source: ToolSource::Restricted { .. },
                ..
            })
        )
    }
}

/// Get the content of baked-in toolset manifest as `str`.
fn baked_in_manifest_raw() -> &'static str {
    cfg_if::cfg_if! {
        if #[cfg(feature = "no-web")] {
            include_str!(
                concat!("../../../resources/toolkit-manifest/offline/", env!("EDITION"), ".toml")
            )
        } else {
            include_str!(
                concat!("../../../resources/toolkit-manifest/online/", env!("EDITION"), ".toml")
            )
        }
    }
}

/// Get a [`ToolsetManifest`] by either:
///
/// - Download from specific url, which could have file schema.
/// - Load from `baked_in_manifest_raw`.
///
pub async fn get_toolset_manifest(url: Option<Url>, insecure: bool) -> Result<ToolsetManifest> {
    /// During the lifetime of program (in manager mode), manifest could be loaded multiple times,
    /// each time requires communicating with server if not cached, which is not ideal.
    /// Therefore we are caching those globally, identified by its URL.
    // NB: This might becomes a problem if we ended up has a ton of toolset to distribute,
    // or the size of manifest files are very big, then we need to switch the caching location
    // to disk. But right now, each `ToolsetManifest` only takes up a few KB, so it's fine to
    // store them in memory.
    // NB: This will reduce the time and IO load with repeating calls, but will increase the
    // time for the initial call because of the `manifest.clone()`.
    static CACHED_MANIFESTS: OnceLock<Mutex<HashMap<Option<Url>, ToolsetManifest>>> =
        OnceLock::new();

    let mutex = CACHED_MANIFESTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = mutex.lock().await;

    // ============ We have it cached, clone and return it directly ===================
    if let Some(mf) = guard.get(&url) {
        debug!("using in memory cached toolset manifest");
        return Ok(mf.clone());
    }

    // ========== We don't have it yet, so, load the manifest and cache it ============
    let manifest = if let Some(url) = &url {
        debug!("downloading toolset manifest from {url}");
        let temp = utils::make_temp_file("toolset-manifest-", None)?;
        utils::DownloadOpt::new("toolset manifest")
            .insecure(insecure)
            .download(url, temp.path())
            .await?;
        ToolsetManifest::load(temp.path())
    } else {
        debug!("loading built-in toolset manifest");
        ToolsetManifest::from_str(baked_in_manifest_raw())
    }?;
    debug!("caching toolset manifest in memory");
    guard.insert(url, manifest.clone());

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenient macro to initialize **Non-Required** `ToolInfo`
    macro_rules! tool_info {
        ($version:literal) => {
            ToolInfo::Basic($version.into())
        };
        ($url_str:literal, $version:expr) => {
            ToolInfo::Complex(ToolInfoDetails::new(ToolSource::Url {
                version: $version.map(ToString::to_string),
                url: $url_str.parse().unwrap(),
                filename: None,
            }))
        };
        ($git:literal, $branch:expr, $tag:expr, $rev:expr) => {
            ToolInfo::Complex(ToolInfoDetails::new(ToolSource::Git {
                git: $git.parse().unwrap(),
                branch: $branch.map(ToString::to_string),
                tag: $tag.map(ToString::to_string),
                rev: $rev.map(ToString::to_string),
            }))
        };
        ($path:expr, $version:expr) => {
            ToolInfo::Complex(ToolInfoDetails::new(ToolSource::Path {
                path: $path,
                version: $version.map(ToString::to_string),
            }))
        };
    }

    #[test]
    fn deserialize_minimal_manifest() {
        let input = r#"
[rust]
version = "1.0.0"
"#;
        assert_eq!(
            ToolsetManifest::from_str(input).unwrap(),
            ToolsetManifest {
                rust: RustToolchain::new("1.0.0"),
                ..Default::default()
            }
        )
    }

    #[test]
    fn deserialize_complicated_manifest() {
        let input = r#"
[rust]
version = "1.0.0"
profile = { name = "minimal" }
components = ["clippy-preview", "llvm-tools-preview"]

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }
t3 = { url = "https://example.com/path/to/tool" }

[tools.target.x86_64-unknown-linux-gnu]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }

[tools.target.aarch64-unknown-linux-gnu]
t1 = "0.1.0"
t4 = { git = "https://git.example.com/org/tool", branch = "stable" }
"#;

        let mut x86_64_windows_msvc_tools = ToolMap::new();
        x86_64_windows_msvc_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        x86_64_windows_msvc_tools.insert(
            "t2".to_string(),
            tool_info!(PathBuf::from("/path/to/local"), None::<&str>),
        );
        x86_64_windows_msvc_tools.insert(
            "t3".to_string(),
            tool_info!("https://example.com/path/to/tool", None::<&str>),
        );

        let mut x86_64_linux_gnu_tools = ToolMap::new();
        x86_64_linux_gnu_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        x86_64_linux_gnu_tools.insert(
            "t2".to_string(),
            tool_info!(PathBuf::from("/path/to/local"), None::<&str>),
        );

        let mut aarch64_linux_gnu_tools = ToolMap::new();
        aarch64_linux_gnu_tools.insert("t1".to_string(), tool_info!("0.1.0"));
        aarch64_linux_gnu_tools.insert(
            "t4".to_string(),
            tool_info!(
                "https://git.example.com/org/tool",
                Some("stable"),
                None::<&str>,
                None::<&str>
            ),
        );

        let expected = ToolsetManifest {
            rust: RustToolchain {
                version: "1.0.0".into(),
                profile: Some("minimal".into()),
                components: vec!["clippy-preview".into(), "llvm-tools-preview".into()],
                ..Default::default()
            },
            tools: Tools::new([
                (
                    "x86_64-pc-windows-msvc".to_string(),
                    x86_64_windows_msvc_tools,
                ),
                (
                    "x86_64-unknown-linux-gnu".to_string(),
                    x86_64_linux_gnu_tools,
                ),
                (
                    "aarch64-unknown-linux-gnu".to_string(),
                    aarch64_linux_gnu_tools,
                ),
            ]),
            ..Default::default()
        };

        assert_eq!(ToolsetManifest::from_str(input).unwrap(), expected);
    }

    #[test]
    fn deserialize_realworld_manifest() {
        let input = include_str!("../../../tests/assets/toolset_manifest.toml");
        let expected = ToolsetManifest {
            rust: RustToolchain {
                version: "stable".into(),
                profile: Some("minimal".into()),
                components: vec!["clippy-preview".into(), "rustfmt".into()],
                ..Default::default()
            },
            tools: Tools::new([
                (
                    "x86_64-pc-windows-msvc".into(),
                    ToolMap::from_iter([
                        ("buildtools".to_string(), tool_info!(PathBuf::from("tests/cache/BuildTools-With-SDK.zip"), Some("1"))),
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-pc-windows-msvc.zip", Some("0.6.11"))),
                        ("vscode".to_string(), tool_info!(PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"), Some("1.91.1"))),
                        ("vscode-rust-analyzer".to_string(), tool_info!(PathBuf::from("tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"), Some("0.4.2054"))),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "x86_64-pc-windows-gnu".into(),
                    ToolMap::from_iter([
                        ("mingw64".to_string(), tool_info!(PathBuf::from("tests/cache/x86_64-13.2.0-release-posix-seh-msvcrt-rt_v11-rev1.7z"), Some("13.2.0"))),
                        ("vscode".to_string(), tool_info!(PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"), Some("1.91.1"))),
                        ("vscode-rust-analyzer".to_string(), tool_info!(PathBuf::from("tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"), Some("0.4.2054"))),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "x86_64-unknown-linux-gnu".into(),
                    ToolMap::from_iter([
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz", Some("0.6.11"))),
                        ("flamegraph".to_string(), tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
                (
                    "aarch64-apple-darwin".into(),
                    ToolMap::from_iter([
                        ("cargo-llvm-cov".to_string(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-aarch64-apple-darwin.tar.gz", Some("0.6.11"))),
                        ("flamegraph".to_string(), tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
                        ("cargo-expand".to_string(), tool_info!("1.0.88")),
                    ]),
                ),
            ]),
            ..Default::default()
        };
        assert_eq!(ToolsetManifest::from_str(input).unwrap(), expected);
    }

    #[test]
    fn current_target_tools_are_correct() {
        let input = include_str!("../../../tests/assets/toolset_manifest.toml");
        let manifest = ToolsetManifest::from_str(input).unwrap();
        let tools = manifest.current_target_tools();

        #[cfg(all(windows, target_env = "gnu"))]
        assert_eq!(
            tools.unwrap(),
            &ToolMap::from_iter([
                (
                    "mingw64".into(),
                    tool_info!(
                        PathBuf::from(
                            "tests/cache/x86_64-13.2.0-release-posix-seh-msvcrt-rt_v11-rev1.7z"
                        ),
                        Some("13.2.0")
                    )
                ),
                (
                    "vscode".into(),
                    tool_info!(
                        PathBuf::from("tests/cache/VSCode-win32-x64-1.91.1.zip"),
                        Some("1.91.1")
                    )
                ),
                (
                    "vscode-rust-analyzer".into(),
                    tool_info!(
                        PathBuf::from(
                            "tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix"
                        ),
                        Some("0.4.2054")
                    )
                ),
                ("cargo-expand".into(), tool_info!("1.0.88")),
            ])
        );

        #[cfg(all(windows, target_env = "msvc"))]
        assert_eq!(
            tools.unwrap(),
            &ToolMap::from_iter([
                (
                    "buildtools".into(),
                    tool_info!(
                        "tests/cache/BuildTools-With-SDK.zip".into(),
                        Some("1")
                    )
                ),
                (
                    "cargo-llvm-cov".into(),
                    tool_info!(
                        "https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-pc-windows-msvc.zip",
                        Some("0.6.11")
                    )
                ),
                (
                    "vscode".into(),
                    tool_info!(
                        "tests/cache/VSCode-win32-x64-1.91.1.zip".into(),
                        Some("1.91.1")
                    )
                ),
                (
                    "vscode-rust-analyzer".into(),
                    tool_info!(
                        "tests/cache/rust-lang.rust-analyzer-0.4.2054@win32-x64.vsix".into(),
                        Some("0.4.2054")
                    )
                ),
                (
                    "cargo-expand".into(),
                    tool_info!("1.0.88"),
                ),
            ])
        );

        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
        assert_eq!(tools.unwrap(), &ToolMap::from_iter([
            ("cargo-llvm-cov".into(), tool_info!("https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.6.11/cargo-llvm-cov-x86_64-unknown-linux-gnu.tar.gz", Some("0.6.11"))),
            ("flamegraph".into(), tool_info!("https://github.com/flamegraph-rs/flamegraph", None::<&str>, Some("v0.6.5"), None::<&str>)),
            ("cargo-expand".into(), tool_info!("1.0.88")),
        ]));

        // TODO: Add test for macos.
    }

    #[test]
    fn with_tools_descriptions() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.descriptions]
t1 = "desc for t1"
# t2 does not have desc
t3 = "desc for t3"
t4 = "desc for t4 that might not exist"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local" }
t3 = { url = "https://example.com/path/to/tool" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();

        assert_eq!(
            expected.tools.descriptions,
            BTreeMap::from_iter([
                ("t1".to_string(), "desc for t1".to_string()),
                ("t3".to_string(), "desc for t3".to_string()),
                (
                    "t4".to_string(),
                    "desc for t4 that might not exist".to_string()
                ),
            ])
        );
    }

    #[test]
    fn with_required_property() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local", required = true }
t3 = { url = "https://example.com/path/to/tool", required = true }
t4 = { git = "https://git.example.com/org/tool", branch = "stable", required = true }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        assert!(!tools.get("t1").unwrap().is_required());
        assert!(tools.get("t2").unwrap().is_required());
        assert!(tools.get("t3").unwrap().is_required());
        assert!(tools.get("t4").unwrap().is_required());
    }

    #[test]
    fn with_optional_property() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { path = "/path/to/local", optional = true }
t3 = { url = "https://example.com/path/to/tool", optional = true }
t4 = { git = "https://git.example.com/org/tool", branch = "stable", optional = true }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        assert!(!tools.get("t1").unwrap().is_optional());
        assert!(tools.get("t2").unwrap().is_optional());
        assert!(tools.get("t3").unwrap().is_optional());
        assert!(tools.get("t4").unwrap().is_optional());
    }

    #[test]
    fn with_tools_group() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.group]
"Some Group" = [ "t1", "t2" ]
Others = [ "t3", "t4" ]
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        assert_eq!(
            expected.tools.group,
            BTreeMap::from_iter([
                (
                    "Some Group".to_string(),
                    ["t1".to_string(), "t2".to_string()].into_iter().collect()
                ),
                (
                    "Others".to_string(),
                    ["t3".to_string(), "t4".to_string()].into_iter().collect()
                )
            ])
        );
        assert_eq!(expected.group_name("t3"), Some("Others"));
        assert_eq!(expected.group_name("t1"), Some("Some Group"));
        assert_eq!(expected.group_name("t100"), None);
    }

    #[test]
    fn with_optional_toolchain_components() {
        let input = r#"
[rust]
version = "1.0.0"
components = ["c1", "c2"]
optional-components = ["opt_c1", "opt_c2"]
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        assert_eq!(&expected.rust.version, "1.0.0");
        assert_eq!(expected.rust.components, vec!["c1", "c2"]);
        assert_eq!(expected.rust.optional_components, vec!["opt_c1", "opt_c2"]);
    }

    #[test]
    fn all_toolchain_components_with_flag() {
        let input = r#"
[rust]
version = "1.0.0"
components = ["c1", "c2"]
optional-components = ["opt_c1", "opt_c2"]
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let opt_components = expected.optional_toolchain_components();
        assert_eq!(opt_components, &["opt_c1", "opt_c2"]);
    }

    #[test]
    fn with_detailed_version_tool() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = "0.1.0" # use cargo install
t2 = { ver = "0.2.0", required = true } # use cargo install
t3 = { ver = "0.3.0", optional = true } # use cargo install
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(tools.get("t1"), Some(&ToolInfo::Basic("0.1.0".into())));
        assert_eq!(
            tools.get("t2"),
            Some(&ToolInfo::Complex(
                ToolInfoDetails::new(ToolSource::Version {
                    version: "0.2.0".into(),
                })
                .required(true)
            ))
        );
        assert_eq!(
            tools.get("t3"),
            Some(&ToolInfo::Complex(
                ToolInfoDetails::new(ToolSource::Version {
                    version: "0.3.0".into(),
                })
                .optional(true)
            ))
        );
    }

    #[test]
    fn with_rust_toolchain_name() {
        let specified = r#"
[rust]
version = "1.0.0"
name = "Rust-lang"
"#;
        let expected = ToolsetManifest::from_str(specified).unwrap();
        assert_eq!(expected.toolchain_group_name(), "Rust-lang");

        let unspecified = "[rust]\nversion = \"1.0.0\"";
        let expected = ToolsetManifest::from_str(unspecified).unwrap();
        assert_eq!(expected.toolchain_group_name(), "Rust Toolchain");
    }

    #[test]
    fn detailed_profile() {
        let basic = r#"
[rust]
version = "1.0.0"
[rust.profile]
name = "minimal"
"#;
        let expected = ToolsetManifest::from_str(basic).unwrap();
        assert_eq!(
            expected.rust.profile.unwrap(),
            ToolchainProfile {
                name: "minimal".into(),
                ..Default::default()
            }
        );

        let full = r#"
[rust]
version = "1.0.0"
[rust.profile]
name = "complete"
verbose-name = "Everything"
description = "Everything provided by official Rust-lang"
"#;
        let expected = ToolsetManifest::from_str(full).unwrap();
        assert_eq!(
            expected.rust.profile.unwrap(),
            ToolchainProfile {
                name: "complete".into(),
                verbose_name: Some("Everything".into()),
                description: Some("Everything provided by official Rust-lang".into()),
            }
        );
    }

    #[test]
    fn with_proxy() {
        let input = r#"
[rust]
version = "1.0.0"
[proxy]
http = "http://username:password@proxy.example.com:8080"
https = "https://username:password@proxy.example.com:8080"
no-proxy = "localhost,some.domain.com"
"#;
        let expected = ToolsetManifest::from_str(input).unwrap();
        assert_eq!(
            expected.proxy.unwrap(),
            Proxy {
                http: Some(Url::parse("http://username:password@proxy.example.com:8080").unwrap()),
                https: Some(
                    Url::parse("https://username:password@proxy.example.com:8080").unwrap()
                ),
                no_proxy: Some("localhost,some.domain.com".into())
            }
        );
    }

    #[test]
    fn with_offline_dist_server() {
        let input = r#"
name = "kit"
[rust]
version = "1.0.0"
offline-dist-server = "packages/"
"#;
        let expected = ToolsetManifest::from_str(input).unwrap();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("packages")
            .join("kit")
            .join(env!("TARGET"))
            .join("packages");
        assert_eq!(
            expected
                .offline_dist_server()
                .unwrap()
                .unwrap()
                .to_file_path()
                .unwrap(),
            path
        );
    }

    #[test]
    fn with_bundled_rustup() {
        let input = r#"
name = "kit"
[rust]
version = "1.0.0"
[rust.rustup]
x86_64-pc-windows-msvc = "tools/rustup-init.exe"
x86_64-pc-windows-gnu = "tools/rustup-init.exe"
x86_64-unknown-linux-gnu = "tools/rustup-init"
"#;
        let expected = ToolsetManifest::from_str(input).unwrap();

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources");
        path.push("packages");
        path.push("kit");
        cfg_if::cfg_if! {
            if #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "msvc"))] {
                path.push("x86_64-pc-windows-msvc/tools/rustup-init.exe");
            } else if #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "gnu"))] {
                path.push("x86_64-pc-windows-gnu/tools/rustup-init.exe");
            } else if #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))] {
                path.push("x86_64-unknown-linux-gnu/tools/rustup-init");
            } else {
                assert_eq!(expected.rustup_bin().unwrap(), None);
                return;
            }
        }

        assert_eq!(expected.rustup_bin().unwrap().unwrap(), path);
    }

    #[test]
    fn with_product_info() {
        let input = r#"
name = "my toolkit"
version = "1.0"

[rust]
version = "1.0.0"
"#;
        let expected = ToolsetManifest::from_str(input).unwrap();
        assert_eq!(expected.name.unwrap(), "my toolkit");
        assert_eq!(expected.version.unwrap(), "1.0");
    }

    #[test]
    fn with_tool_identifier() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = { ver = "0.2.0", identifier = "surprise_program_1" }
t2 = { path = "/some/path", identifier = "surprise_program_2" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let mut tools = expected
            .tools
            .target
            .get("x86_64-pc-windows-msvc")
            .unwrap()
            .iter();
        let (_, t1_info) = tools.next().unwrap();
        let (_, t2_info) = tools.next().unwrap();
        assert_eq!(t1_info.identifier(), Some("surprise_program_1"));
        assert!(matches!(
            t2_info,
            ToolInfo::Complex(ToolInfoDetails { identifier: Some(name), .. })
                if name == "surprise_program_2"
        ));
    }

    #[test]
    fn toolmap_iterator_uses_identifier_as_key() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
t1 = { ver = "0.2.0", identifier = "surprise_program_1" }
t2 = { path = "/some/path", identifier = "surprise_program_2" }
t3 = "0.1.0"
t4 = { url = "https://example.com/t4.zip" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        let mut iter = tools.iter().map(|(name, _)| name);
        assert_eq!(iter.next(), Some("surprise_program_1"));
        assert_eq!(iter.next(), Some("surprise_program_2"));
        assert_eq!(iter.next(), Some("t3"));
        assert_eq!(iter.next(), Some("t4"));
    }

    #[test]
    fn complex_tools_deser_and_ser() {
        let input = r#"[rust]
version = "1.0.0"
components = []
optional-components = []

[rust.rustup]

[tools.descriptions]

[tools.group]

[tools.target.x86_64-pc-windows-msvc]
plain_version = "0.1.0"

[tools.target.x86_64-pc-windows-msvc.detailed_version]
required = false
optional = true
identifier = "hello"
version = "0.2.0"

[tools.target.x86_64-pc-windows-msvc.url_tool]
required = true
optional = false
url = "http://example.com/"
filename = "hello.zip"

[tools.target.x86_64-pc-windows-msvc.path_tool]
required = false
optional = false
version = "0.3.0"
path = "path/to/bin"

[tools.target.x86_64-pc-windows-msvc.git_tool]
required = false
optional = false
git = "https://example.git/"
branch = "dev"
"#;
        let obj = ToolsetManifest::from_str(input).unwrap();
        let expected_ser = obj.to_toml().unwrap();
        assert_eq!(input, expected_ser);
    }

    #[test]
    fn with_tool_kind() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
vscode-installer = { version = "1.97.1", url = "https://example.com", kind = "installer" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let (target, tool) = expected.tools.target.first_key_value().unwrap();
        let (name, info) = tool.first().unwrap();
        assert_eq!(target, "x86_64-pc-windows-msvc");
        assert_eq!(name, "vscode-installer");
        assert_eq!(info.kind(), Some(ToolKind::Installer));
    }

    #[test]
    fn with_display_name() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
tool_a = { version = "1.97.1", display-name = "Tool A" }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let (target, tool) = expected.tools.target.first_key_value().unwrap();
        let (name, info) = tool.first().unwrap();
        assert_eq!(target, "x86_64-pc-windows-msvc");
        assert_eq!(name, "tool_a");
        assert_eq!(info.display_name(), Some("Tool A"));
    }

    #[test]
    fn user_provided_package_sources() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
tool_a = { version = "0.1.0", restricted = true }
tool_b = { default = "https://example.com/installer.exe", restricted = true }
"#;

        let expected = ToolsetManifest::from_str(input).unwrap();
        let (_, tool) = expected.tools.target.first_key_value().unwrap();
        let mut tools = tool.iter();
        let (name, info) = tools.next().unwrap();
        assert_eq!(name, "tool_a");
        assert_eq!(
            info.details().unwrap().source,
            ToolSource::Restricted {
                restricted: true,
                default: None,
                source: None,
                version: Some("0.1.0".to_string())
            }
        );
        let (name, info) = tools.next().unwrap();
        assert_eq!(name, "tool_b");
        assert_eq!(
            info.details().unwrap().source,
            ToolSource::Restricted {
                restricted: true,
                default: Some("https://example.com/installer.exe".into()),
                source: None,
                version: None
            }
        );
    }
}
