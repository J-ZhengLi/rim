//! `ToolsetManifest` contains information about each dist package,
//! such as its name, version, and what's included etc.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use rim_common::types::{TomlParser, ToolInfo, ToolkitManifest};
use rim_common::utils::{self, HiddenProgress};
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use url::Url;

use crate::components::{Component, ComponentType};
use crate::core::custom_instructions;

use super::AppInfo;

/// Extension trait for [`ToolkitManifest`],
/// offering extra functionalities for installation/uninstallation.
pub trait ToolkitManifestExt
where
    Self: TomlParser + Sized + DeserializeOwned,
{
    /// Load toolset manifest from installed root.
    ///
    /// # Note
    /// Only use this during **manager** mode.
    fn load_from_install_dir() -> Result<Self> {
        let root = AppInfo::get_installed_dir();
        Self::load(root.join(Self::FILENAME))
    }

    /// Get the tools that are only available in current target.
    ///
    /// Return empty iterator if there are no available tools in the current target.
    fn current_target_tools(&self) -> impl Iterator<Item = (&str, &ToolInfo)>;

    /// Like [`current_target_tools`](ToolkitExt::current_target_tools) but
    /// getting a list of tools and components as [`Component`].
    ///
    /// If `check_for_existence` is `true`, this function will look through user's environment
    /// to see if a specific tool is already installed or not.
    fn current_target_components(&self, check_for_existence: bool) -> Result<Vec<Component>>;

    /// Get the path to bundled `rustup-init` binary if there has one.
    fn rustup_bin(&self) -> Result<Option<PathBuf>>;

    /// Returns the absolute path of the package root.
    ///
    /// A package root is:
    /// - The folder to store tools' packages such as `tools/hello-world.tar.xz`, etc.
    /// - The folder to store local rustup dist server such as `toolchain/`, where all
    ///   the rust installer stuffs stored, such as `toolchain/channel-rust-x.xx.x.toml`.
    /// - Usually the parent directory of this manifest file.
    ///
    /// Note: In `release` build, because this program has an embedded toolkit manifest,
    /// therefore it assumes the parent directory of this running binary as the package root.
    /// But in `debug` build, because we have cached all those packages inside of
    /// `resource/packages` folder, we will be assuming it as the package root.
    fn package_root(&self) -> Result<PathBuf>;

    /// Get configured local dist server path and parse it to `Url`.
    fn offline_dist_server(&self) -> Result<Option<Url>>;

    /// Turn all the relative paths in the `tools` section to some absolute paths.
    ///
    /// There are some rules applied when converting, including:
    /// 1. If the manifest was loaded from a path,
    ///    all relative paths will be forced to combine with the path loading from.
    /// 2. If the manifest was not loaded from path,
    ///    all relative paths will be forced to combine with the parent directory of this executable.
    ///    (Assuming the manifest was baked in the executable)
    ///
    /// # Errors
    /// Return `Result::Err` if the manifest was not loaded from path, and the current executable path
    /// cannot be determined as well.
    fn adjust_paths(&mut self) -> Result<()>;

    /// Some package source might be missing if it has [`ToolSource::Restricted`],
    /// thus this function is required for the installation to work properly.
    ///
    /// When calling this function, a list of component name is needed to,
    /// which is a list of components that user selected for installation
    /// (we don't need to fill the source if they don't intend to install those).
    /// Then, this will apply a `callback` function trying to modify the source
    /// with a certain string returned from the callback function.
    fn fill_missing_package_source<F>(
        &mut self,
        components: &mut Vec<Component>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(String, Option<&str>) -> Result<String>;
}

impl ToolkitManifestExt for ToolkitManifest {
    fn rustup_bin(&self) -> Result<Option<PathBuf>> {
        let cur_target = env!("TARGET");
        let par_dir = self.package_root()?;
        let rel_path = self.toolchain.rustup.get(cur_target);

        Ok(rel_path.map(|p| par_dir.join(p)))
    }

    fn offline_dist_server(&self) -> Result<Option<Url>> {
        let Some(server) = &self.toolchain.offline_dist_server else {
            return Ok(None);
        };
        let par_dir = self.package_root()?;
        let full_path = par_dir.join(server);

        Url::from_directory_path(&full_path)
            .map(Option::Some)
            .map_err(|_| anyhow!("path '{}' cannot be converted to URL", full_path.display()))
    }

    fn current_target_tools(&self) -> impl Iterator<Item = (&str, &ToolInfo)> {
        let direct_target_tools = self
            .tools
            .target
            .get(env!("TARGET"))
            .map(|map| map.iter())
            .unwrap_or_default();
        let all_target_tools = self
            .tools
            .target
            .get("all")
            .map(|map| map.iter())
            .unwrap_or_default();
        direct_target_tools.chain(all_target_tools)
    }

    fn current_target_components(&self, check_for_existence: bool) -> Result<Vec<Component>> {
        let tc_channel = &self.toolchain.channel;

        let profile_name = self.toolchain.name();
        let default_cate_name = t!("other").to_string();
        let tc_group = self
            .toolchain
            .group
            .as_deref()
            .unwrap_or(&default_cate_name);
        // Add a component that represents rust toolchain
        let mut components = vec![Component::new(profile_name)
            .with_description(self.toolchain.description())
            .with_category(tc_group)
            .with_type(ComponentType::ToolchainProfile)
            .required(true)
            .with_version(Some(tc_channel))];

        for (component, is_optional) in self.toolchain_components() {
            components.push(
                Component::new(component)
                    .with_description(self.get_tool_description(component))
                    .with_category(tc_group)
                    .optional(is_optional)
                    .with_type(ComponentType::ToolchainComponent)
                    // toolchain component's version are unified
                    .with_version(Some(tc_channel)),
            );
        }

        for (tool_name, tool_info) in self.current_target_tools() {
            // filter out GUI tools on non-GUI environment
            if tool_info.is_gui_only() && !utils::has_desktop_environment() {
                continue;
            }

            let installed_in_env =
                check_for_existence && custom_instructions::is_installed(tool_name);
            let version = if check_for_existence && installed_in_env {
                // if the tool is already installed but we are doing a fresh install here,
                // which means it was installed by user not by `rim`,
                // therefore we don't know the version.
                None
            } else {
                tool_info.version()
            };
            components.push(
                Component::new(tool_name)
                    .with_description(self.get_tool_description(tool_name))
                    .with_category(self.group_name(tool_name).unwrap_or(&default_cate_name))
                    .with_tool_installer(tool_info)
                    .required(tool_info.is_required())
                    .optional(tool_info.is_optional())
                    .installed(installed_in_env)
                    .with_version(version)
                    .with_display_name(tool_info.display_name().unwrap_or(tool_name))
                    .with_type(ComponentType::Tool),
            );
        }

        Ok(components)
    }

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

    fn adjust_paths(&mut self) -> Result<()> {
        let parent_dir = self.package_root()?;

        for tool in self.tools.target.values_mut() {
            for tool_info in tool.values_mut() {
                if let Some(path) = tool_info.path_mut() {
                    *path = utils::to_normalized_absolute_path(path.as_path(), Some(&parent_dir))?;
                }
            }
        }
        Ok(())
    }

    fn fill_missing_package_source<F>(
        &mut self,
        components: &mut Vec<Component>,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(String, Option<&str>) -> Result<String>,
    {
        for tool in self.tools.target.values_mut() {
            for (name, tool_info) in tool.iter_mut() {
                let Some(comp_to_modify) = components.iter_mut().find(|c| &c.name == name) else {
                    continue;
                };
                let display_name = tool_info.display_name().unwrap_or(name).to_string();

                if let Some((source, default)) = tool_info.restricted_source_mut() {
                    let new_val = callback(display_name, default.as_deref())?;
                    *source = Some(new_val.clone());

                    // try modify the ones in components as well
                    if let Some((s, _)) = comp_to_modify
                        .tool_installer
                        .as_mut()
                        .and_then(|c| c.restricted_source_mut())
                    {
                        *s = Some(new_val);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Get the content of baked-in toolset manifest as `str`.
pub(crate) fn baked_in_manifest_raw(no_web: bool) -> &'static str {
    if no_web {
        include_str!(concat!(
            "../../resources/toolkit-manifest/offline/",
            env!("EDITION"),
            ".toml"
        ))
    } else {
        include_str!(concat!(
            "../../resources/toolkit-manifest/online/",
            env!("EDITION"),
            ".toml"
        ))
    }
}

/// Get a [`ToolsetManifest`] by either:
///
/// - Download from specific url, which could have file schema.
/// - Load from `baked_in_manifest_raw`.
///
pub async fn get_toolkit_manifest(url: Option<Url>, insecure: bool) -> Result<ToolkitManifest> {
    /// During the lifetime of program (in manager mode), manifest could be loaded multiple times,
    /// each time requires communicating with server if not cached, which is not ideal.
    /// Therefore we are caching those globally, identified by its URL.
    // NB: This might becomes a problem if we ended up has a ton of toolset to distribute,
    // or the size of manifest files are very big, then we need to switch the caching location
    // to disk. But right now, each `ToolsetManifest` only takes up a few KB, so it's fine to
    // store them in memory.
    // NB: This will reduce the time and IO load with repeating calls
    static CACHED_MANIFESTS: OnceLock<Mutex<HashMap<Option<Url>, ToolkitManifest>>> =
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
        if let Ok(path) = url.to_file_path() {
            ToolkitManifest::load(path)?
        } else {
            debug!("downloading toolset manifest from {url}");
            let temp = utils::make_temp_file("toolset-manifest-", None)?;

            // this file is very small, no need for progress bar.
            utils::DownloadOpt::new("toolset manifest", Box::new(HiddenProgress))
                .insecure(insecure)
                .download(url, temp.path())
                .await?;
            ToolkitManifest::load(temp.path())?
        }
    } else {
        debug!("loading built-in toolset manifest");
        cfg_if::cfg_if! {
            if #[cfg(feature = "no-web")] {
                ToolkitManifest::from_str(baked_in_manifest_raw(true))?.offline(true)
            } else {
                ToolkitManifest::from_str(baked_in_manifest_raw(false))?
            }
        }
    };
    debug!("caching toolset manifest in memory");
    guard.insert(url, manifest.clone());

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_common::types::ToolKind;

    #[test]
    fn current_target_tools_are_correct() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.x86_64-pc-windows-gnu]
a = "0.1.0"
[tools.target.x86_64-pc-windows-msvc]
b = "0.1.0"
[tools.target.aarch64-unknown-linux-gnu]
c = "0.1.0"
[tools.target.x86_64-unknown-linux-gnu]
d = "0.1.0"
"#;

        let manifest = ToolkitManifest::from_str(input).unwrap();
        let mut tools = manifest.current_target_tools();

        cfg_if::cfg_if! {
            if #[cfg(all(windows, target_env = "gnu"))] {
                let name = tools.next().unwrap().0;
                assert_eq!(name, "a");
            } else if #[cfg(all(windows, target_env = "msvc"))] {
                let name = tools.next().unwrap().0;
                assert_eq!(name, "b");
            } else if #[cfg(all(target_arch = "aarch64", target_os = "linux", target_env = "gnu"))] {
                let name = tools.next().unwrap().0;
                assert_eq!(name, "c");
            } else if #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))] {
                let name = tools.next().unwrap().0;
                assert_eq!(name, "d");
            } else {
                assert!(tools.is_none());
            }
        }
    }

    #[test]
    fn with_offline_dist_server() {
        let input = r#"
name = "kit"
[rust]
version = "1.0.0"
offline-dist-server = "packages/"
"#;
        let expected = ToolkitManifest::from_str(input).unwrap();
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
        let expected = ToolkitManifest::from_str(input).unwrap();

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
    fn complex_tools_deser_and_ser() {
        let input = r#"[toolchain]
channel = "1.0.0"
components = []
optional-components = []

[toolchain.rustup]

[tools.descriptions]

[tools.group]

[tools.target.x86_64-pc-windows-msvc]
plain_version = "0.1.0"

[tools.target.x86_64-pc-windows-msvc.detailed_version]
optional = true
identifier = "hello"
version = "0.2.0"

[tools.target.x86_64-pc-windows-msvc.url_tool]
required = true
url = "http://example.com/"
filename = "hello.zip"

[tools.target.x86_64-pc-windows-msvc.path_tool]
version = "0.3.0"
path = "path/to/bin"

[tools.target.x86_64-pc-windows-msvc.git_tool]
git = "https://example.git/"
branch = "dev"
"#;
        let obj = ToolkitManifest::from_str(input).unwrap();
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

        let expected = ToolkitManifest::from_str(input).unwrap();
        let (target, tool) = expected.tools.target.iter().next().unwrap();
        let (name, info) = tool.first().unwrap();
        assert_eq!(target, "x86_64-pc-windows-msvc");
        assert_eq!(name, "vscode-installer");
        assert_eq!(info.kind(), Some(ToolKind::Installer));
    }

    #[test]
    fn all_target_tools() {
        let input = r#"
[rust]
version = "1.0.0"

[tools.target.all]
a = "0.1.0"
"#;

        let manifest = ToolkitManifest::from_str(input).unwrap();
        let mut tools = manifest.current_target_tools();

        assert_eq!(tools.next().unwrap().0, "a");
        assert_eq!(tools.next(), None);
    }
}
