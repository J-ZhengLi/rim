//! Module for parsing the cargo manifest (Cargo.toml) file.
//! Mainly used to handle crate installation

use anyhow::Result;
use glob::glob;
use rim_common::{types::TomlParser, utils};
use serde::Deserialize;
use std::{collections::HashSet, path::PathBuf};

/// Cargo manifest, a.k.a the `Cargo.toml`
#[derive(Debug, Deserialize)]
pub(crate) struct CargoManifest {
    pub(crate) package: Option<Package>,
    pub(crate) workspace: Option<Workspace>,
}

impl TomlParser for CargoManifest {
    const FILENAME: &'static str = "Cargo.toml";

    fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let raw = utils::read_to_string("cargo manifest", &path)?;
        let mut temp_manifest = Self::from_str(&raw)?;

        if let Some(ws) = temp_manifest.workspace.as_mut() {
            // safe to unwrap, we know it is a file since we have already read its content
            // and a file should have a parent directory
            ws.root_dir = path.as_ref().parent().unwrap().to_path_buf();
        }

        Ok(temp_manifest)
    }
}

/// Package metadata of a cargo manifest, a.k.a. the `[package]` section in `Cargo.toml`
#[derive(Debug, Deserialize)]
pub(crate) struct Package {
    pub(crate) name: String,
}

/// Cargo workspace configuration, a.k.a the `[workspace]` section in `Cargo.toml`
#[derive(Debug, Deserialize)]
pub(crate) struct Workspace {
    #[serde(skip)]
    /// path to the directory where this workspace manifest was loaded
    root_dir: PathBuf,
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

impl Workspace {
    /// Return a set of paths to *valid* workspace member.
    ///
    /// A valid workspace member means that it is in `[workspace.members]`
    /// but not in `[workspace.exclude]`.
    ///
    /// # Error
    /// Return error if a value in the `members` list or `exclude` list
    /// contains non UTF-8 character or is not valid glob.
    pub(crate) fn member_paths(&self) -> Result<HashSet<PathBuf>> {
        let mut members: HashSet<PathBuf> = HashSet::new();

        for member in &self.members {
            let full_path = self.root_dir.join(member);
            for glob_entry in glob(utils::path_to_str(&full_path)?)? {
                let member_path = glob_entry?;
                members.insert(member_path);
            }
        }

        for excluded in &self.exclude {
            let full_path = self.root_dir.join(excluded);
            for glob_entry in glob(utils::path_to_str(&full_path)?)? {
                let excluded_path = glob_entry?;
                members.remove(&excluded_path);
            }
        }

        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_manifest() {
        let raw = r#"
[workspace]
members = ["rim_gui/src-tauri", "rim_dev", "rim_common", "rim_test/*"]
exclude = ["rim_test/rim-test-macro"]"#;

        let m = CargoManifest::from_str(raw).unwrap();
        let mut ws = m.workspace.unwrap();
        ws.root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(ws.members.len(), 4);
        assert_eq!(ws.exclude.len(), 1);
        assert_eq!(ws.member_paths().unwrap().len(), 4);
        assert!(m.package.is_none());
    }
}
