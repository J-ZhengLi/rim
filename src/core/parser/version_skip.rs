//! Module containing code logic that skip certain version updates by user requests.

use super::{get_installed_dir, TomlParser};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SkipFor {
    Manager,
    Toolkit,
}

/// Basically a map containing items and its version to be marked skipped.
///
/// Appeared as a locally stored file that contains the specific version to be skipped by user,
/// that will prevent the app from asking updates for those versions.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VersionSkip(HashMap<SkipFor, String>);

impl TomlParser for VersionSkip {
    const FILENAME: &'static str = ".skipped-updates";
}

impl VersionSkip {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a version as skipped.
    ///
    /// This function can be chained.
    pub fn skip<T: Into<String>>(mut self, skip_for: SkipFor, version: T) -> Self {
        self.0.insert(skip_for, version.into());
        self
    }

    /// Return `true` if the given `version` is marked as skipped before.
    pub fn is_skipped<T: AsRef<str>>(&self, skip_for: SkipFor, version: T) -> bool {
        let Some(skipped) = self.0.get(&skip_for) else {
            return false;
        };
        version.as_ref() == skipped
    }

    /// Try loading from installation.
    ///
    /// This guarentee to return a [`VersionSkip`] object,
    /// even if the file does not exists, the default will got returned.
    pub fn load_from_install_dir() -> Self {
        let install_dir = get_installed_dir();
        Self::load_from_dir(install_dir).unwrap_or_default()
    }

    pub fn write_to_install_dir(&self) -> Result<()> {
        let install_dir = get_installed_dir();
        self.write_to_dir(install_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_then_check() {
        let input = r#"
manager = "0.1.0"
toolkit = "1.0.0""#;
        let expected = VersionSkip::from_str(input).unwrap();
        assert!(expected.is_skipped(SkipFor::Manager, "0.1.0"));
        assert!(expected.is_skipped(SkipFor::Toolkit, "1.0.0"));
    }

    #[test]
    fn init_then_check() {
        let vs = VersionSkip::new()
            .skip(SkipFor::Manager, "0.1.0")
            .skip(SkipFor::Toolkit, "1.0.0");
        assert!(vs.is_skipped(SkipFor::Manager, "0.1.0"));
        assert!(vs.is_skipped(SkipFor::Toolkit, "1.0.0"));
    }
}
