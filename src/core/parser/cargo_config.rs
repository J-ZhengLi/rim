//! Module defining types that could be serialized to a working `config.toml` for cargo.

use std::path::PathBuf;

use indexmap::IndexMap;
use rim_common::types::TomlParser;
use serde::{Deserialize, Serialize};

/// A simple struct representing the fields in `config.toml`.
///
/// Only covers a small range of options we need to configure.
/// Fwiw, the full set of configuration options can be found
/// in the [Cargo Configuration Book](https://doc.rust-lang.org/cargo/reference/config.html).
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct CargoConfig {
    net: Option<CargoNetConfig>,
    http: Option<CargoHttpConfig>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    source: IndexMap<String, Source>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    patch: IndexMap<String, DependencyPatch>,
}

impl TomlParser for CargoConfig {
    const FILENAME: &'static str = "config.toml";

    /// Load from a directory or return a default if it doesn't exists
    fn load_from_dir<P: AsRef<std::path::Path>>(parent: P) -> anyhow::Result<Self>
    where
        Self: Sized + serde::de::DeserializeOwned + Default,
    {
        let src: PathBuf = parent.as_ref().join(Self::FILENAME);
        if !src.is_file() {
            Ok(Self::default())
        } else {
            Self::load(src)
        }
    }
}

// FIXME: remove this `allow` before 0.1.0 release.
#[allow(unused)]
impl CargoConfig {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn git_fetch_with_cli(&mut self, yes: bool) -> &mut Self {
        self.net = Some(CargoNetConfig {
            git_fetch_with_cli: Some(yes),
        });
        self
    }

    pub(crate) fn check_revoke(&mut self, yes: bool) -> &mut Self {
        self.http = Some(CargoHttpConfig {
            check_revoke: Some(yes),
        });
        self
    }

    /// Insert a source.
    ///
    /// NB: The first `add_source` call will also insert a `crates-io` source.
    ///
    /// - `key` is the name of the source.
    /// - `url` is the registry url.
    /// - `as_default` specify whether this source is used as a replaced source of `crates-io`,
    ///   note the first `add_source` call will always be default.
    pub(crate) fn add_source(&mut self, key: &str, url: &str, as_default: bool) -> &mut Self {
        self.source
            .entry("crates-io".to_string())
            .and_modify(|s| {
                if as_default {
                    s.replace_with = Some(key.to_string());
                }
            })
            .or_insert(Source {
                replace_with: Some(key.to_string()),
                ..Default::default()
            });

        self.source.insert(
            key.to_string(),
            Source {
                registry: Some(url.into()),
                ..Default::default()
            },
        );

        self
    }

    /// Insert a dependency patch ([`DependencyPatch`]) into the patch section.
    pub(crate) fn add_patch<S, P>(&mut self, name: S, patch_path: P) -> &mut Self
    where
        S: Into<String>,
        P: Into<PathBuf>,
    {
        let old_val = self.patch.entry("crates-io".into()).or_default();
        old_val.0.insert(
            name.into(),
            DependencyKind::Path {
                path: patch_path.into(),
            },
        );
        self
    }

    /// Remove a dependency patch ([`DependencyPatch`]) from the patch section.
    pub(crate) fn remove_patch(&mut self, name: &str) -> &mut Self {
        let Some(patches) = self.patch.get_mut("crates-io") else {
            return self;
        };

        patches.0.shift_remove(name);
        self
    }
}

/// This section can be used to override dependencies with other copies
///
/// For more information about `patch` configuration,
/// visit: https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct DependencyPatch(IndexMap<String, DependencyKind>);

/// Dependency kind, the syntax is similar to the `[dependencies]` section
#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged)]
pub(crate) enum DependencyKind {
    Path {
        #[serde(serialize_with = "flip_backslash")]
        path: PathBuf,
    },
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CargoNetConfig {
    git_fetch_with_cli: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct CargoHttpConfig {
    check_revoke: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Source {
    pub(crate) replace_with: Option<String>,
    pub(crate) registry: Option<String>,
}

/// Flip all backward splashes (`\`) to forward splash (`/`) when serializing paths.
/// To make sure `cargo` can read this config on Windows.
fn flip_backslash<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let path_str = format!("{}", path.display());
    serializer.serialize_str(&path_str.replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::{CargoConfig, TomlParser};

    #[test]
    fn cargo_config_default_serialize() {
        // serialized default should be an empty toml
        let default = CargoConfig::default();

        assert_eq!(default.to_toml().unwrap(), "");
    }

    #[test]
    fn cargo_config_serialize() {
        let config = CargoConfig::new()
            .git_fetch_with_cli(true)
            .check_revoke(false)
            .add_source("mirror", "https://example.com/registry", true)
            .to_toml()
            .unwrap();

        assert_eq!(
            config,
            r#"[net]
git-fetch-with-cli = true

[http]
check-revoke = false

[source.crates-io]
replace-with = "mirror"

[source.mirror]
registry = "https://example.com/registry"
"#
        );
    }

    #[test]
    fn cargo_config_insert_patch() {
        let config = CargoConfig::new()
            .add_patch("foo", "/path/to/foo")
            .add_patch("bar", "/path/to/bar")
            .to_toml()
            .unwrap();
        assert_eq!(
            config,
            "[patch.crates-io.foo]
path = \"/path/to/foo\"\n
[patch.crates-io.bar]
path = \"/path/to/bar\"\n"
        );
    }
}
