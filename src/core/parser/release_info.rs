use anyhow::Result;
use rim_common::types::{Configuration, ReleaseChannel, TomlParser};
use semver::Version;
use serde::{de, Deserialize, Deserializer};

/// A type designed to contain the information about the newest `manager` release.
///
/// This only contains software `version` for now.
#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseInfo {
    #[serde(deserialize_with = "de_version")]
    pub(crate) version: Version,
}

fn de_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Version::parse(&s)
        .map_err(|e| de::Error::custom(format!("invalid semantic version, reason: {e}")))
}

/// Manifest of RIM version releases.
#[derive(Debug, Deserialize)]
pub(crate) struct Releases {
    /// Required stable release.
    ///
    /// Note: This is marked flatten for compatibility reason,
    /// so that the old release file syntax works:
    /// ```toml
    /// version = '0.8.0'
    /// ```
    ///
    /// And the new syntax will also works on old RIM releases:
    ///
    /// ```toml
    /// version = '0.8.0'
    ///
    /// [beta]
    /// version = '0.9.0-beta'
    /// ```
    #[serde(flatten)]
    stable: ReleaseInfo,
    beta: Option<ReleaseInfo>,
}

impl TomlParser for Releases {
    const FILENAME: &'static str = "release.toml";
}

impl Releases {
    /// Get the release version of user configured channel.
    ///
    /// If there are multiple channel supported,
    /// This will try checking the user configuration to see what channel should be used,
    /// otherwise the version of default release channel will be returned.
    pub(crate) fn version(&self) -> &Version {
        if let Some(beta_ver) = &self.beta {
            let channel = Configuration::try_load_from_config_dir()
                .map(|conf| conf.update.manager_update_channel)
                .unwrap_or_default();
            if channel == ReleaseChannel::Beta {
                return &beta_ver.version;
            }
        }

        &self.stable.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PartialEq for ReleaseInfo {
        fn eq(&self, other: &Self) -> bool {
            self.version == other.version
        }
    }

    impl PartialEq for Releases {
        fn eq(&self, other: &Self) -> bool {
            self.stable == other.stable && self.beta == other.beta
        }
    }

    #[test]
    fn version_info() {
        let input = "version = '1.2.3'";
        let release = Releases::from_str(input).unwrap();
        let version = release.version();

        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre.as_str(), "beta.1");
    }

    #[test]
    #[should_panic(expected = "invalid semantic version")]
    fn bad_version() {
        let input = "version = 'stable'";
        let _release = Releases::from_str(input).unwrap();
    }

    #[test]
    fn multiple_release_channel() {
        let input = r#"
version = "0.1.0"
[beta]
version = "0.2.0-beta"
"#;
        let releases = Releases::from_str(input).unwrap();
        assert_eq!(
            releases,
            Releases {
                stable: ReleaseInfo {
                    version: "0.1.0".parse().unwrap()
                },
                beta: Some(ReleaseInfo {
                    version: "0.2.0-beta".parse().unwrap()
                })
            }
        );
    }
}
