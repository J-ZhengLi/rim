//! The information about single tool in toolkit manifest.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

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
    Complex(Box<ToolInfoDetails>),
}

impl ToolInfo {
    /// Get a mutable reference of this tools' package source if it's from a local path,
    /// check [`ToolSource::Path`] for more info.
    pub fn path_mut(&mut self) -> Option<&mut PathBuf> {
        if let Self::Complex(details) = self {
            if let ToolSource::Path { path, .. } = &mut details.source {
                return Some(path);
            }
        }
        None
    }

    /// Get a mutable reference of this tools' package source if it's a URL,
    /// check [`ToolSource::Url`] for more info.
    pub fn url_mut(&mut self) -> Option<&mut Url> {
        if let Self::Complex(details) = self {
            if let ToolSource::Url { url, .. } = &mut details.source {
                return Some(url);
            }
        }
        None
    }

    /// Convert package source form [`Url`](ToolSource::Url) to [`Path`](ToolSource::Path).
    ///
    /// Do nothing if current tool's source is not a `Url` type.
    pub fn url_to_path<P: Into<PathBuf>>(&mut self, path: P) {
        if let Self::Complex(details) = self {
            let ToolSource::Url {
                version,
                url: _,
                filename: _,
            } = &details.source
            else {
                return;
            };

            details.source = ToolSource::Path {
                version: version.clone(),
                path: path.into(),
            };
        }
    }

    /// Get the mutable reference of package source if it's using restricted package source,
    /// check [`ToolSource::Restricted`] for more info.
    pub fn restricted_source_mut(&mut self) -> Option<&mut Option<String>> {
        if let Self::Complex(details) = self {
            if let ToolSource::Restricted { source, .. } = &mut details.source {
                return Some(source);
            }
        }
        None
    }

    /// Get the detailed information ([`ToolInfoDetails`]) of this tool, return `None` if
    /// it uses pure version string such as:
    /// ```toml
    /// tool = "0.1.0"
    /// ```
    pub fn details(&self) -> Option<&ToolInfoDetails> {
        if let Self::Complex(details) = self {
            Some(details)
        } else {
            None
        }
    }

    /// Return `true` if this tool is required to be installed.
    pub fn is_required(&self) -> bool {
        self.details().map(|d| d.required).unwrap_or_default()
    }

    /// Get the version of this tool, return `None` if it:
    ///
    /// 1. Uses `git` url as source without a `tag`.
    /// 2. Uses `path` or `url` as source without `version`.
    /// 3. Uses `restricted` source without specifying a `version`.
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

    /// Return `true` if the installation of this tool is optional.
    pub fn is_optional(&self) -> bool {
        self.details().map(|d| d.optional).unwrap_or_default()
    }

    /// Return `true` if this tool can be installed by `cargo`
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

    /// Return `true` if this tool uses restricted source, usually meaning that we cannot
    /// provide its package source without violating it's license term.
    pub fn is_restricted(&self) -> bool {
        matches!(
            self.details(),
            Some(ToolInfoDetails {
                source: ToolSource::Restricted { .. },
                ..
            })
        )
    }

    /// Return a list of names that this tool requires.
    pub fn dependencies(&self) -> &[String] {
        self.details()
            .map(|det| det.requires.as_slice())
            .unwrap_or_default()
    }

    /// Return a list of names that are obsoleted (replaced) by this tool.
    pub fn obsoletes(&self) -> &[String] {
        self.details()
            .map(|det| det.obsoletes.as_slice())
            .unwrap_or_default()
    }

    /// Return a list of names that are conflicting with this tool.
    pub fn conflicts(&self) -> &[String] {
        self.details()
            .map(|det| det.conflicts.as_slice())
            .unwrap_or_default()
    }

    /// Get a designated filename for `Url` source.
    pub fn filename(&self) -> Option<&str> {
        if let Some(det) = self.details() {
            if let ToolSource::Url { filename, .. } = &det.source {
                return filename.as_deref();
            }
        }
        None
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ToolInfoDetails {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    pub identifier: Option<String>,
    #[serde(flatten)]
    pub source: ToolSource,
    /// Pre-determined kind.
    /// If not provided, this will be automatically assumed when loading a tool using
    /// [`Tool::from_path`](crate::core::tools::Tool::from_path).
    pub kind: Option<ToolKind>,
    /// A name that only used for display purpose.
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// A list of tools that are obsoleted/replaced by this package.
    pub obsoletes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "dependencies")]
    /// A list of tools that this package requires.
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// A list of tools that this package conflicts with.
    pub conflicts: Vec<String>,
}

impl ToolInfoDetails {
    pub fn new(source: ToolSource) -> Self {
        Self {
            source,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Hash)]
#[serde(untagged)]
pub enum ToolSource {
    /// A tool that does not allowing redistribution are considered as `restricted`.
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

impl Default for ToolSource {
    fn default() -> Self {
        Self::Version {
            version: String::new(),
        }
    }
}

/// Representing the structure of an (extracted) tool's directory.
// NB: Mind the order of the variants, they are crucial to installation/uninstallation.
#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    /// Directory containing `bin` subfolder:
    /// ```text
    /// tool/
    /// ├─── bin/
    /// ├─── ...
    /// ```
    DirWithBin,
    /// Installer type, which need to be executed to install a certain tool.
    Installer,
    /// Pre-built executable files.
    /// i.e.:
    /// ```text
    /// ├─── some_binary.exe
    /// ├─── cargo-some_binary.exe
    /// ```
    Executables,
    /// We have a custom "script" for how to deal with such directory.
    Custom,
    /// Plugin file, such as `.vsix` files for Visual Studio.
    Plugin,
    // `Cargo` just don't make any sense
    #[allow(clippy::enum_variant_names)]
    CargoTool,
    /// Unknown tool, install and uninstall are not fully supported.
    #[default]
    Unknown,
}
