//! Module defining types that could be serialized to a working `config.toml` for cargo.

use std::{collections::HashSet, path::PathBuf};

use indexmap::IndexMap;
use rim_common::types::TomlParser;
use serde::{ser::SerializeMap, Deserialize, Serialize};

/// A simple struct representing the fields in `config.toml`.
///
/// Only covers a small range of options we need to configure.
/// Fwiw, the full set of configuration options can be found
/// in the [Cargo Configuration Book](https://doc.rust-lang.org/cargo/reference/config.html).
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct CargoConfig {
    /// Path dependency overrides
    paths: Option<HashSet<PathBuf>>,
    net: Option<CargoNetConfig>,
    http: Option<CargoHttpConfig>,
    #[serde(serialize_with = "serialize_source_map")]
    source: IndexMap<String, Source>,
}

impl TomlParser for CargoConfig {
    const FILENAME: &'static str = "config.toml";
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

    /// Add an overrided dependency path for this config.
    ///
    /// Note that the `paths` are stored in a `HashSet`,
    /// so no need to worry about duplicated values.
    ///
    /// For more information about `paths` configuration,
    /// visit: https://doc.rust-lang.org/cargo/reference/config.html#paths.
    pub(crate) fn add_path<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        let old_val = self.paths.get_or_insert_default();
        old_val.insert(path.into());
        self
    }
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

// Serialize empty map to an empty string.
fn serialize_source_map<S>(map: &IndexMap<String, Source>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if map.is_empty() {
        serializer.serialize_none()
    } else {
        let mut ser_map = serializer.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            ser_map.serialize_entry(k, v)?;
        }
        ser_map.end()
    }
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
    fn cargo_config_insert_paths() {
        let config = CargoConfig::new()
            .add_path("/path/to/foo")
            .add_path("/path/to/bar")
            .to_toml()
            .unwrap();
        assert_eq!(
            config,
            r#"paths = ["/path/to/foo", "/path/to/bar"]
"#
        );
    }
}
