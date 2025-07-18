use super::{TomlParser, ToolMap};
use crate::{setter, types::CargoRegistry, utils};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolkitManifest {
    /// Configuration of this toolkit, including download source config,
    /// user proxy config, etc.
    // NB: Removing this `flatten` will cause compatibility issues.
    #[serde(flatten)]
    pub config: ToolkitConfig,

    /// Product name to be cached after installation, so that we can show it as `installed`
    pub name: Option<String>,
    /// Product version to be cached after installation, so that we can show it as `installed`
    pub version: Option<String>,
    /// Toolkit edition, such as `basic`, `community`
    pub edition: Option<String>,

    #[serde(alias = "rust")]
    pub toolchain: RustToolchain,
    #[serde(default)]
    pub tools: Tools,

    /// Path to the manifest file.
    #[serde(skip)]
    pub path: Option<PathBuf>,
    /// A boolean flag to indicate whether this manifest is for offline package.
    #[serde(skip)]
    pub is_offline: bool,
}

impl TomlParser for ToolkitManifest {
    const FILENAME: &'static str = "toolset-manifest.toml";

    fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let raw = utils::read_to_string("manifest", &path)?;
        let mut temp_manifest = Self::from_str(&raw)?;
        temp_manifest.path = Some(path.as_ref().to_path_buf());
        Ok(temp_manifest)
    }
}

impl ToolkitManifest {
    /// Get a iterator of all optional component names in rust toolchain, along
    /// with a flag indicating whether it is an optional component or not.
    pub fn toolchain_components(&self) -> Vec<(&str, bool)> {
        self.toolchain
            .components
            .iter()
            .map(|s| (s.as_str(), false))
            .chain(
                self.toolchain
                    .optional_components
                    .iter()
                    .map(|s| (s.as_str(), true)),
            )
            .collect()
    }

    /// Get the description of a specific tool.
    pub fn get_tool_description(&self, tool: &str) -> Option<&str> {
        self.tools.descriptions.get(tool).map(|s| s.as_str())
    }

    /// Get the group name of a certain tool.
    pub fn group_name(&self, tool: &str) -> Option<&str> {
        self.tools
            .group
            .iter()
            .find_map(|(group, tools)| tools.contains(tool).then_some(group.as_str()))
    }

    /// A convenient wrapper to get proxy configs in [`ToolkitConfig`]
    pub fn proxy_config(&self) -> Option<&Proxy> {
        self.config.proxy.as_ref()
    }

    setter!(offline(self.is_offline, bool));
}

/// Configuration of this toolkit, including download source config, user proxy config, etc.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolkitConfig {
    /// Proxy settings that used for download.
    pub proxy: Option<Proxy>,
    /// Enforced rustup dist server url.
    /// This is the top priority when it comes to install toolchain.
    pub rustup_dist_server: Option<Url>,
    /// Enforced rustup update root url.
    /// This is the top priority when it comes to install rustup update.
    pub rustup_update_root: Option<Url>,
    /// Enforced cargo registry config.
    /// This is the top priority when it comes to writing cargo config
    /// after installing toolchain.
    pub cargo_registry: Option<CargoRegistry>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct RustToolchain {
    /// Toolchain channel name, such as stable/nightly/beta,
    /// a specific sematic version (x.x.x), or nightly with specific date (nightly-xxxx-xx-xx).
    #[serde(alias = "version")]
    pub channel: String,
    /// Prefer NOT to use this directly, use `profile()` method instead.
    profile: Option<ToolchainProfile>,
    /// Prefer NOT to use this directly, use `display_name()` method instead.
    #[serde(alias = "verbose-name")]
    display_name: Option<String>,
    /// Prefer NOT to use this directly, use `description()` method instead.
    description: Option<String>,
    /// Components are installed by default
    #[serde(default)]
    pub components: Vec<String>,
    /// Optional components are only installed if user choose to.
    #[serde(default)]
    pub optional_components: Vec<String>,
    /// Optional category (group) name for the rust toolchain,
    /// so you can group the toolchain components `Rust Toolchain` or something else.
    /// note that all optional components belong to this group as well.
    pub group: Option<String>,
    /// File [`Url`] to install rust toolchain.
    pub offline_dist_server: Option<String>,
    /// Contains target specific `rustup-init` binaries.
    #[serde(default)]
    pub rustup: IndexMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
enum ToolchainProfile {
    Basic(String),
    /// Semi-deprecated variant after 0.6.0, prefer not to use this at all.
    /// Because, we had the `profile` configured as a separated section as:
    /// ```toml
    /// [rust.profile]
    /// name = "minimal"
    /// description = "Minimal toolchain for basic functionality"
    /// verbose-name = "Basic"
    /// ```
    /// Which turns out to be confusing, we keep this variant here only
    /// for the sake of backward compatibility.
    Complex {
        name: String,
        #[serde(rename = "verbose-name")]
        verbose_name: Option<String>,
        description: Option<String>,
    },
}

impl<T: ToString> From<T> for ToolchainProfile {
    fn from(value: T) -> Self {
        Self::Basic(value.to_string())
    }
}

impl RustToolchain {
    pub fn new(ver: &str) -> Self {
        Self {
            channel: ver.to_string(),
            ..Default::default()
        }
    }

    /// Toolchain profile that rustup uses to install rust, such as default/minimal/complete.
    pub fn profile(&self) -> Option<&str> {
        self.profile.as_ref().map(|p| match p {
            ToolchainProfile::Basic(name) => name.as_str(),
            ToolchainProfile::Complex { name, .. } => name.as_str(),
        })
    }

    /// Optional name to label the rust toolchain on UI, allowing toolkit provider to
    /// label the toolchain with another name such as "Core", "Rust" etc.
    pub fn display_name(&self) -> Option<&str> {
        self.display_name
            .as_deref()
            .or(self.profile.as_ref().and_then(|p| {
                if let ToolchainProfile::Complex { verbose_name, .. } = p {
                    verbose_name.as_deref()
                } else {
                    None
                }
            }))
    }

    /// Optional description for the rust toolchain.
    pub fn description(&self) -> Option<&str> {
        self.description
            .as_deref()
            .or(self.profile.as_ref().and_then(|p| {
                if let ToolchainProfile::Complex { description, .. } = p {
                    description.as_deref()
                } else {
                    None
                }
            }))
    }

    /// The name of toolchain for display purpose.
    ///
    /// Name of the toolchain is not required but is good to have on UI,
    /// the returned value follows a specific fallback order:
    ///
    /// 1. The [display_name](RustToolchain::display_name) set in manifest file.
    /// 2. The [group](RustToolchain::group) name set in manifest file.
    /// 3. Simply just `"Rust"`
    pub fn name(&self) -> &str {
        self.display_name()
            .or(self.group.as_deref())
            .unwrap_or("Rust")
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
pub struct Tools {
    #[serde(default)]
    descriptions: IndexMap<String, String>,
    /// Containing groups of tools.
    ///
    /// Note that not all tools will have a group.
    #[serde(default)]
    group: IndexMap<String, IndexSet<String>>,
    #[serde(default)]
    pub target: IndexMap<String, ToolMap>,
}

impl Tools {
    pub fn new<I>(targeted_tools: I) -> Tools
    where
        I: IntoIterator<Item = (String, ToolMap)>,
    {
        Self {
            target: IndexMap::from_iter(targeted_tools),
            ..Default::default()
        }
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

#[cfg(test)]
mod tests {
    use crate::types::{ToolInfo, ToolInfoDetails, ToolSource};

    use super::*;

    fn complex_tool(tool_info: ToolInfoDetails) -> ToolInfo {
        ToolInfo::Complex(Box::new(tool_info))
    }

    /// Convenient macro to initialize **Non-Required** `ToolInfo`
    macro_rules! tool_info {
        ($version:literal) => {
            ToolInfo::Basic($version.into())
        };
        ($url_str:literal, $version:expr) => {
            complex_tool(ToolInfoDetails::new().with_source(ToolSource::Url {
                version: $version.map(ToString::to_string),
                url: $url_str.parse().unwrap(),
                filename: None,
            }))
        };
        ($git:literal, $branch:expr, $tag:expr, $rev:expr) => {
            complex_tool(ToolInfoDetails::new().with_source(ToolSource::Git {
                git: $git.parse().unwrap(),
                branch: $branch.map(ToString::to_string),
                tag: $tag.map(ToString::to_string),
                rev: $rev.map(ToString::to_string),
            }))
        };
        ($path:expr, $version:expr) => {
            complex_tool(ToolInfoDetails::new().with_source(ToolSource::Path {
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
            ToolkitManifest::from_str(input).unwrap(),
            ToolkitManifest {
                toolchain: RustToolchain::new("1.0.0"),
                ..Default::default()
            }
        )
    }

    #[test]
    fn deserialize_complicated_manifest() {
        let input = r#"
[rust]
version = "1.0.0"
profile = "minimal"
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

        let expected = ToolkitManifest {
            toolchain: RustToolchain {
                channel: "1.0.0".into(),
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

        assert_eq!(ToolkitManifest::from_str(input).unwrap(), expected);
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

        let expected = ToolkitManifest::from_str(input).unwrap();

        assert_eq!(
            expected.tools.descriptions,
            IndexMap::<String, String>::from_iter([
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

        let expected = ToolkitManifest::from_str(input).unwrap();
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

        let expected = ToolkitManifest::from_str(input).unwrap();
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        assert_eq!(
            expected.tools.group,
            IndexMap::<String, IndexSet<String>>::from_iter([
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        assert_eq!(&expected.toolchain.channel, "1.0.0");
        assert_eq!(expected.toolchain.components, vec!["c1", "c2"]);
        assert_eq!(
            expected.toolchain.optional_components,
            vec!["opt_c1", "opt_c2"]
        );
    }

    #[test]
    fn all_toolchain_components_with_flag() {
        let input = r#"
[rust]
version = "1.0.0"
components = ["c1", "c2"]
optional-components = ["opt_c1", "opt_c2"]
"#;

        let expected = ToolkitManifest::from_str(input).unwrap();
        let components = expected.toolchain_components();
        assert_eq!(
            components,
            &[
                ("c1", false),
                ("c2", false),
                ("opt_c1", true),
                ("opt_c2", true)
            ]
        );
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(tools.get("t1"), Some(&ToolInfo::Basic("0.1.0".into())));
        assert_eq!(
            tools.get("t2"),
            Some(&complex_tool(ToolInfoDetails {
                source: Some(ToolSource::Version {
                    version: "0.2.0".into(),
                }),
                required: true,
                ..Default::default()
            }))
        );
        assert_eq!(
            tools.get("t3"),
            Some(&complex_tool(ToolInfoDetails {
                source: Some(ToolSource::Version {
                    version: "0.3.0".into(),
                }),
                optional: true,
                ..Default::default()
            }))
        );
    }

    #[test]
    fn with_rust_toolchain_name() {
        let specified = r#"
[rust]
version = "1.0.0"
display-name = "Rust-lang"
"#;
        let expected = ToolkitManifest::from_str(specified).unwrap();
        assert_eq!(expected.toolchain.name(), "Rust-lang");

        let unspecified = "[rust]\nversion = \"1.0.0\"";
        let expected = ToolkitManifest::from_str(unspecified).unwrap();
        assert_eq!(expected.toolchain.name(), "Rust");
    }

    #[test]
    fn detailed_profile() {
        let basic = r#"
[rust]
version = "1.0.0"
profile = "complete"
verbose-name = "Everything"
description = "Everything provided by official Rust-lang"
"#;
        let expected = ToolkitManifest::from_str(basic).unwrap();
        assert_eq!(expected.toolchain.profile.unwrap(), "complete".into());
        assert_eq!(expected.toolchain.display_name.unwrap(), "Everything");
        assert_eq!(
            expected.toolchain.description.unwrap(),
            "Everything provided by official Rust-lang"
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
        let expected = ToolkitManifest::from_str(input).unwrap();
        assert_eq!(
            expected.config.proxy.unwrap(),
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
    fn with_product_info() {
        let input = r#"
name = "my toolkit"
version = "1.0"
edition = "professional"

[rust]
version = "1.0.0"
"#;
        let expected = ToolkitManifest::from_str(input).unwrap();
        assert_eq!(expected.name.unwrap(), "my toolkit");
        assert_eq!(expected.version.unwrap(), "1.0");
        assert_eq!(expected.edition.unwrap(), "professional");
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

        let expected = ToolkitManifest::from_str(input).unwrap();
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
            t2_info.details().unwrap(),
            ToolInfoDetails { identifier: Some(name), .. } if name == "surprise_program_2"
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        let tools = expected.tools.target.get("x86_64-pc-windows-msvc").unwrap();
        let mut iter = tools.iter().map(|(name, _)| name);
        assert_eq!(iter.next(), Some("surprise_program_1"));
        assert_eq!(iter.next(), Some("surprise_program_2"));
        assert_eq!(iter.next(), Some("t3"));
        assert_eq!(iter.next(), Some("t4"));
    }

    #[test]
    fn with_display_name() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
tool_a = { version = "1.97.1", display-name = "Tool A" }
"#;

        let expected = ToolkitManifest::from_str(input).unwrap();
        let (target, tool) = expected.tools.target.iter().next().unwrap();
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        let (_, tool) = expected.tools.target.iter().next().unwrap();
        let mut tools = tool.iter();
        let (name, info) = tools.next().unwrap();
        assert_eq!(name, "tool_a");
        assert_eq!(
            info.details().unwrap().source,
            Some(ToolSource::Restricted {
                restricted: true,
                default: None,
                source: None,
                version: Some("0.1.0".to_string())
            })
        );
        let (name, info) = tools.next().unwrap();
        assert_eq!(name, "tool_b");
        assert_eq!(
            info.details().unwrap().source,
            Some(ToolSource::Restricted {
                restricted: true,
                default: Some("https://example.com/installer.exe".into()),
                source: None,
                version: None
            })
        );
    }

    #[test]
    fn tool_dependency_control() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-msvc]
tool_a = { version = "0.1.0", requires = ["tool_b"], obsoletes = ["tool_c"], conflicts = ["tool_d"] }
tool_b = "0.1.0"
tool_c = "0.1.0"
tool_d = "0.1.0""#;

        let expected = ToolkitManifest::from_str(input).unwrap();
        let (_, tool) = expected.tools.target.iter().next().unwrap();
        let tool_a = tool["tool_a"].details().unwrap();
        assert_eq!(tool_a.requires, ["tool_b".to_string()]);
        assert_eq!(tool_a.obsoletes, ["tool_c".to_string()]);
        assert_eq!(tool_a.conflicts, ["tool_d".to_string()]);
    }

    #[test]
    fn rust_profile_backward_compatible() {
        let input = r#"
[rust]
version = "1.0.0"

[rust.profile]
name = "complete"
verbose-name = "Everything"
description = "Everything provided by official Rust-lang"
"#;

        let expected = ToolkitManifest::from_str(input).unwrap();

        assert_eq!(expected.toolchain.profile(), Some("complete"));
        assert_eq!(expected.toolchain.display_name(), Some("Everything"));
        assert_eq!(
            expected.toolchain.description(),
            Some("Everything provided by official Rust-lang")
        );
    }

    #[test]
    fn toolkit_enforced_configuration() {
        let raw = r#"
rustup-dist-server = "https://www.example.com"
rustup-update-root = "https://www.example.com/rustup"

[cargo-registry]
name = "my-registry"
index = "https://www.example.com/index"

[toolchain]
channel = "1.0.0"
"#;
        let expected = ToolkitManifest::from_str(raw).unwrap();

        assert_eq!(
            expected.config.rustup_dist_server,
            Some("https://www.example.com".parse().unwrap())
        );
        assert_eq!(
            expected.config.rustup_update_root,
            Some("https://www.example.com/rustup".parse().unwrap())
        );

        let registry = expected.config.cargo_registry.unwrap();
        assert_eq!(registry.name, "my-registry");
        assert_eq!(registry.index, "https://www.example.com/index");
        assert!(expected.config.proxy.is_none());
    }
}
