//! `DistManifest` includes a list of dist packages' info,
//! each of them contains brief information about it such as its
//! name, version, description, changelog, an url leading to the toolset manifest, and other info.

use rim_common::types::TomlParser;
use serde::Deserialize;
use url::Url;

#[allow(unused)]
#[derive(Debug, Deserialize)]
/// Represent a list of dist packages which user can download from the server.
pub struct DistManifest {
    #[serde(alias = "package")]
    pub packages: Vec<DistPackage>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct DistPackage {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
    pub desc: Option<String>,
    pub info: Option<String>,
    pub manifest_url: Url,
}

impl TomlParser for DistManifest {
    const FILENAME: &'static str = "distribution-manifest.toml";
}

impl DistManifest {}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist_package(
        name: &str,
        ver: &str,
        edition: Option<&str>,
        desc: Option<&str>,
        info: Option<&str>,
        url: &str,
    ) -> DistPackage {
        DistPackage {
            name: name.to_string(),
            version: ver.to_string(),
            edition: edition.map(ToString::to_string),
            desc: desc.map(ToString::to_string),
            info: info.map(ToString::to_string),
            manifest_url: url.parse().unwrap(),
        }
    }

    #[test]
    fn deserialize_basic_dist_package() {
        let input = r#"
[[packages]]
name = "A"
version = "1.0"
edition = "community"
desc = "A toolkit"
info = "initial version, includes nothing"
manifest-url = "https://example.com/path/to/a/manifest-1.0"

[[packages]]
name = "A"
version = "2.0"
edition = "education"
desc = "A toolkit"
info = "Second version, but still includes nothing"
manifest-url = "https://example.com/path/to/a/manifest-2.0"

[[packages]]
name = "B"
version = "1.0"
edition = "professional"
desc = "B toolkit"
info = "initial version, includes nothing"
manifest-url = "https://example.com/path/to/b/manifest-1.0"
"#;
        let parsed = DistManifest::from_str(input).unwrap();
        let expected = vec![
            dist_package(
                "A",
                "1.0",
                Some("community"),
                Some("A toolkit"),
                Some("initial version, includes nothing"),
                "https://example.com/path/to/a/manifest-1.0",
            ),
            dist_package(
                "A",
                "2.0",
                Some("education"),
                Some("A toolkit"),
                Some("Second version, but still includes nothing"),
                "https://example.com/path/to/a/manifest-2.0",
            ),
            dist_package(
                "B",
                "1.0",
                Some("professional"),
                Some("B toolkit"),
                Some("initial version, includes nothing"),
                "https://example.com/path/to/b/manifest-1.0",
            ),
        ];

        assert_eq!(parsed.packages.len(), 3);
        assert_eq!(parsed.packages, expected);
    }
}
