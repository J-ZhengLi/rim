use anyhow::{anyhow, Context, Result};
use rim_common::types::{TomlParser, ToolKind, ToolkitManifest};
use rim_common::utils;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::components::ToolchainComponent;
use crate::AppInfo;

/// Load fingerprint file just to get the list of installed tools.
pub(crate) fn installed_tools(root: &Path) -> Result<HashMap<String, ToolRecord>> {
    Ok(InstallationRecord::load_from_dir(root)?.tools)
}

/// Holds Installation record.
///
/// This tracks what tools/components we have installed, and where they are installed.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct InstallationRecord {
    /// Name of the bundle, such as `my-rust-stable`
    pub name: Option<String>,
    pub version: Option<String>,
    pub edition: Option<String>,
    pub root: PathBuf,
    pub rust: Option<RustRecord>,
    #[serde(default)]
    pub tools: HashMap<String, ToolRecord>,
}

impl TomlParser for InstallationRecord {
    const FILENAME: &'static str = ".fingerprint.toml";

    /// Load fingerprint from a given root.
    ///
    /// This will create one and return the default if it doesn't exist.
    fn load_from_dir<P: AsRef<Path>>(root: P) -> Result<InstallationRecord>
    where
        Self: Sized + serde::de::DeserializeOwned,
    {
        utils::ensure_dir(root.as_ref())?;

        let fp_path = root.as_ref().join(Self::FILENAME);
        if fp_path.is_file() {
            let raw = utils::read_to_string("installation fingerprint", &fp_path)?;
            Self::from_str(&raw)
        } else {
            let default = InstallationRecord {
                root: root.as_ref().to_path_buf(),
                ..Default::default()
            };
            default.write()?;
            Ok(default)
        }
    }
}

impl InstallationRecord {
    /// Used to detect whether a fingerprint file exists in parent directory.
    ///
    /// This is useful when you want to know it without causing
    /// the program to panic using [`get_installed_dir`](AppInfo::get_installed_dir).
    pub fn exists() -> Result<bool> {
        let parent_dir = utils::parent_dir_of_cur_exe()?;
        Ok(parent_dir.join(Self::FILENAME).is_file())
    }

    /// Load installation record from a presumed install directory,
    /// which is typically the parent directory of the current executable.
    // TODO: Cache the result using a `Cell` or `RwLock` or combined.
    pub(crate) fn load_from_install_dir() -> Result<Self> {
        let root = AppInfo::get_installed_dir();
        Self::load_from_dir(root)
    }

    pub(crate) fn write(&self) -> Result<()> {
        let path = self.root.join(Self::FILENAME);
        let content = self
            .to_toml()
            .context("unable to serialize installation fingerprint")?;
        debug!("writing installation record into '{}'", path.display());
        utils::write_bytes(&path, content.as_bytes(), false).with_context(|| {
            anyhow!(
                "unable to write fingerprint file to the given location: '{}'",
                path.display()
            )
        })
    }

    pub(crate) fn clone_toolkit_meta_from_manifest(&mut self, manifest: &ToolkitManifest) {
        self.name.clone_from(&manifest.name);
        self.version.clone_from(&manifest.version);
        self.edition.clone_from(&manifest.edition);
    }

    pub(crate) fn remove_toolkit_meta(&mut self) {
        self.name = None;
        self.version = None;
    }

    /// Adds installation record for Rust toolchain
    pub(crate) fn add_rust_record(&mut self, version: &str, components: &[ToolchainComponent]) {
        self.rust = Some(RustRecord {
            version: version.to_string(),
            components: components.iter().map(|tc| tc.name.clone()).collect(),
        });
    }

    pub(crate) fn add_tool_record(&mut self, name: &str, record: ToolRecord) {
        self.tools.insert(name.into(), record);
    }

    pub fn remove_rust_record(&mut self) {
        self.rust = None;
    }

    /// Remove a list of toolchain components from record
    pub fn remove_component_record(&mut self, components: &[ToolchainComponent]) {
        let Some(rust) = self.rust.as_mut() else {
            return;
        };

        let components_to_remove: HashSet<&String> =
            HashSet::from_iter(components.iter().map(|c| &c.name));
        if components_to_remove.is_empty() {
            return;
        }
        let components_after_remove = rust
            .components
            .iter()
            .filter_map(|c| (!components_to_remove.contains(c)).then_some(c.clone()))
            .collect();

        rust.components = components_after_remove;
    }

    pub fn remove_tool_record(&mut self, tool_name: &str) {
        self.tools.remove(tool_name);
    }

    /// Retrieve a list of installed toolchain components only
    pub fn installed_toolchain_components(&self) -> Vec<ToolchainComponent> {
        let Some(rr) = &self.rust else { return vec![] };

        rr.components.iter().map(ToolchainComponent::new).collect()
    }

    /// Returns the rust toolchain channel name (such as `stable`, `nightly`, `1.80.1`, etc.),
    /// and a slice of installed components.
    pub fn installed_toolchain(&self) -> Option<(&str, &[String])> {
        self.rust
            .as_ref()
            .map(|rr| (rr.version.as_str(), rr.components.as_slice()))
    }

    pub(crate) fn print_installation(&self) -> String {
        let mut installed = String::new();
        if let Some(rust) = &self.rust {
            installed.push_str(&rust.print_rust_info());
        }
        for tool in self.tools.iter() {
            installed.push_str(&format!("tools: {:?} \n", tool.0));
        }
        installed
    }

    pub fn get_tool_version(&self, name: &str) -> Option<&str> {
        self.tools.get(name).and_then(|rec| rec.version.as_deref())
    }

    /// Check if any of the specific type of tool was installed
    pub fn type_of_tool_is_installed(&self, kind: ToolKind) -> bool {
        self.tools.iter().any(|(_, rec)| rec.kind == kind)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RustRecord {
    version: String,
    /// Rust toolchain components, including the base profile (minimal/default/etc.),
    /// and extra components selected by user.
    #[serde(default)]
    pub(crate) components: Vec<String>,
}

impl RustRecord {
    pub(crate) fn print_rust_info(&self) -> String {
        format!(
            "rust-version: {}\ncomponents: {:?}\n",
            self.version, self.components
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolRecord {
    #[deprecated(since = "0.3.1", note = "use `.tool_kind()` instead")]
    #[serde(
        default,
        skip_serializing,
        deserialize_with = "de_deprecated_use_cargo"
    )]
    use_cargo: Option<ToolKind>,
    #[serde(default)]
    kind: ToolKind,
    version: Option<String>,
    #[serde(default)]
    pub(crate) paths: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) dependencies: Vec<String>,
}

impl ToolRecord {
    pub(crate) fn cargo_tool() -> Self {
        ToolRecord {
            kind: ToolKind::CargoTool,
            ..Default::default()
        }
    }

    pub(crate) fn new(kind: ToolKind) -> Self {
        Self {
            kind,
            ..Default::default()
        }
    }

    pub(crate) fn tool_kind(&self) -> ToolKind {
        #[allow(deprecated)]
        self.use_cargo.unwrap_or(self.kind)
    }

    pub(crate) fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    setter!(with_paths(self.paths, Vec<PathBuf>));
    setter!(with_version(self.version, ver: Option<impl Into<String>>) { ver.map(Into::into) });
    setter!(with_dependencies(self.dependencies, Vec<String>));
}

// `use-cargo = true/false` was used during [0.2.0, 0.3.0], in order not to break
// the compatibility for those versions, we need to deserialize it to the new api.
fn de_deprecated_use_cargo<'de, D>(deserializer: D) -> Result<Option<ToolKind>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ToolKindVisitor;

    impl Visitor<'_> for ToolKindVisitor {
        type Value = Option<ToolKind>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a `true` or `false` string")
        }

        fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.then_some(ToolKind::CargoTool))
        }
    }

    deserializer.deserialize_bool(ToolKindVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    // there is an inconsistency between OSs when serialize paths
    #[cfg(not(windows))]
    const QUOTE: &str = "\"";
    #[cfg(windows)]
    const QUOTE: &str = "'";

    #[test]
    fn create_local_install_info() {
        let install_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let mut fp = InstallationRecord::load_from_dir(&install_dir).unwrap();
        let rust_components = vec![
            ToolchainComponent::new("rustfmt"),
            ToolchainComponent::new("cargo"),
        ];

        fp.add_rust_record("stable", &rust_components);
        fp.add_tool_record(
            "aaa",
            ToolRecord::new(ToolKind::Custom).with_paths(vec![install_dir.join("aaa")]),
        );

        let v0 = format!(
            "\
root = {QUOTE}{}{QUOTE}

[rust]
version = \"stable\"
components = [\"rustfmt\", \"cargo\"]

[tools.aaa]
kind = \"custom\"
paths = [{QUOTE}{}{QUOTE}]
",
            install_dir.display(),
            install_dir.join("aaa").display()
        );
        assert_eq!(v0, fp.to_toml().unwrap());
    }

    #[test]
    fn with_name_and_ver() {
        let input = r#"
name = "rust bundle (experimental)"
version = "0.1"
edition = "professional"
root = '/path/to/something'"#;

        let expected = InstallationRecord::from_str(input).unwrap();
        assert_eq!(expected.name.unwrap(), "rust bundle (experimental)");
        assert_eq!(expected.version.unwrap(), "0.1");
        assert_eq!(expected.edition.unwrap(), "professional");
        assert_eq!(expected.root, PathBuf::from("/path/to/something"));
    }

    #[test]
    fn all_tool_kinds() {
        let input = r#"
root = '/path/to/something'

[tools]
a = { kind = 'cargo-tool', paths = [] }
b = { kind = 'custom', paths = [] }
c = { kind = 'dir-with-bin', paths = [] }
d = { kind = 'executables', paths = []}
e = { kind = 'plugin', paths = []}
"#;

        let kinds = HashMap::from([
            ("a", ToolKind::CargoTool),
            ("b", ToolKind::Custom),
            ("c", ToolKind::DirWithBin),
            ("d", ToolKind::Executables),
            ("e", ToolKind::Plugin),
        ]);
        let expected = InstallationRecord::from_str(input).unwrap();
        let all_kinds = expected
            .tools
            .iter()
            .map(|(name, rec)| (name.as_str(), rec.kind))
            .collect::<HashMap<_, _>>();

        assert_eq!(all_kinds, kinds);
    }

    #[test]
    fn de_use_cargo_and_default_toolkind() {
        let input = r#"
root = '/path/to/something'

[tools]
a = { use-cargo = true, paths = [] }
b = { use-cargo = false, paths = ['some/path'] }
c = { paths = ['some/other/path'] }"#;

        let expected = InstallationRecord::from_str(input).unwrap();
        let hm = expected
            .tools
            .iter()
            .map(|(name, rec)| (name.as_str(), rec.tool_kind()))
            .collect::<HashMap<&str, ToolKind>>();

        assert_eq!(hm["a"], ToolKind::CargoTool);
        assert_eq!(hm["b"], ToolKind::Unknown);
        assert_eq!(hm["c"], ToolKind::Unknown);
    }

    #[test]
    fn do_not_ser_use_cargo() {
        let record = InstallationRecord {
            root: "/some/path".into(),
            tools: HashMap::from([("a".into(), ToolRecord::cargo_tool())]),
            ..Default::default()
        };
        let ser = record.to_toml().unwrap();
        let expected = r#"root = "/some/path"

[tools.a]
kind = "cargo-tool"
paths = []
"#;
        assert_eq!(ser, expected);
    }

    #[test]
    fn with_tool_version() {
        let input = r#"
root = '/path/to/something'

[tools]
a = { kind = "custom", version = "1.2.0", paths = ["/some/path"] }
[tools.b]
kind = "executables"
paths = ["/some/other/path"]"#;

        let rec = InstallationRecord::from_str(input).unwrap();
        let ver_rec = rec
            .tools
            .iter()
            .map(|(name, r)| (name.as_str(), r.version.as_deref()))
            .collect::<HashMap<_, _>>();
        let expecting = HashMap::from([("a", Some("1.2.0")), ("b", None)]);
        assert_eq!(ver_rec, expecting);
    }

    #[test]
    fn with_dependencies() {
        let input = r#"
root = '/path/to/something'

[tools]
a = { kind = "custom", version = "1.2.0", paths = ["/some/path"], dependencies = ["b"] }
b = { kind = "executables", paths = ["/some/other/path"] }
"#;

        let rec = InstallationRecord::from_str(input).unwrap();
        let a = rec.tools.get("a").unwrap();
        assert_eq!(a.dependencies, ["b"]);
    }
}
