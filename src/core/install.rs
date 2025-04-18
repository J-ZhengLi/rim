use super::components::ToolchainComponent;
use super::dependency_handler::DependencyHandler;
use super::{
    components::{component_list_to_tool_map, Component, ComponentType},
    directories::RimDir,
    parser::{
        cargo_config::CargoConfig,
        fingerprint::{InstallationRecord, ToolRecord},
    },
    rustup::ToolchainInstaller,
    tools::Tool,
    GlobalOpts, CARGO_HOME, RUSTUP_DIST_SERVER, RUSTUP_HOME, RUSTUP_UPDATE_ROOT,
};
use crate::core::os::add_to_path;
use anyhow::{anyhow, bail, Context, Result};
use rim_common::types::{TomlParser, ToolInfo, ToolMap, ToolSource, ToolkitManifest};
use rim_common::{build_config, utils};
use std::collections::HashSet;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use url::Url;

const DEFAULT_FOLDER_NAME: &str = "rust";

/// Contains definition of installation steps, including pre-install configs.
pub trait EnvConfig {
    /// Configure environment variables.
    ///
    /// This will set persistent environment variables including
    /// `RUSTUP_DIST_SERVER`, `RUSTUP_UPDATE_ROOT`, `CARGO_HOME`, `RUSTUP_HOME`, etc.
    fn config_env_vars(&self) -> Result<()>;
}

/// Contains every information that the installation process needs.
pub struct InstallConfiguration<'a> {
    pub cargo_registry: Option<(String, String)>,
    /// Path to install everything.
    ///
    /// Note that this folder will includes `cargo` and `rustup` folders as well.
    /// And the default location will under `$HOME` directory (`%USERPROFILE%` on windows).
    /// So, even if the user didn't specify any install path, a pair of env vars will still
    /// be written (CARGO_HOME and RUSTUP_HOME), which will be under the default location
    /// defined by [`default_install_dir`].
    pub install_dir: PathBuf,
    pub rustup_dist_server: Url,
    pub rustup_update_root: Url,
    /// Indicates whether `cargo` was already installed, useful when installing third-party tools.
    pub cargo_is_installed: bool,
    install_record: InstallationRecord,
    pub(crate) progress_indicator: Option<utils::Progress<'a>>,
    manifest: &'a ToolkitManifest,
    insecure: bool,
}

impl RimDir for InstallConfiguration<'_> {
    fn install_dir(&self) -> &Path {
        self.install_dir.as_path()
    }
}

impl RimDir for &InstallConfiguration<'_> {
    fn install_dir(&self) -> &Path {
        self.install_dir.as_path()
    }
}

impl<'a> InstallConfiguration<'a> {
    pub fn new(install_dir: &'a Path, manifest: &'a ToolkitManifest) -> Result<Self> {
        let (reg_name, reg_url) = super::default_cargo_registry();
        Ok(Self {
            install_dir: install_dir.to_path_buf(),
            // Note: `InstallationRecord::load_from_dir` creates `install_dir` if it does not exist
            install_record: InstallationRecord::load_from_dir(install_dir)?,
            cargo_registry: Some((reg_name.into(), reg_url.into())),
            rustup_dist_server: super::default_rustup_dist_server().clone(),
            rustup_update_root: super::default_rustup_update_root().clone(),
            cargo_is_installed: false,
            progress_indicator: None,
            manifest,
            insecure: false,
        })
    }
    /// Creating install directory and other preparations related to filesystem.
    ///
    /// This is suitable for first-time installation.
    pub fn setup(&mut self) -> Result<()> {
        let install_dir = &self.install_dir;

        info!("{}", t!("install_init", dir = install_dir.display()));

        // Create a copy of the manifest which is later used for component management.
        self.manifest.write_to_dir(install_dir)?;

        // Create a copy of this binary
        let self_exe = std::env::current_exe()?;
        // promote this installer to manager
        let id = &build_config().identifier;
        let manager_name = format!("{id}-manager");

        // Add this manager to the `PATH` environment
        let manager_exe = install_dir.join(exe!(manager_name));
        utils::copy_as(self_exe, &manager_exe)?;
        add_to_path(install_dir)?;

        #[cfg(windows)]
        // Create registry entry to add this program into "installed programs".
        super::os::windows::do_add_to_programs(&manager_exe)?;

        if let Some(prog) = &self.progress_indicator {
            prog.inc(Some(5.0))?;
        }

        Ok(())
    }

    pub fn install(mut self, components: Vec<Component>) -> Result<()> {
        let (tc_components, tools) = split_components(components);
        reject_conflicting_tools(&tools)?;

        self.setup()?;
        self.config_env_vars()?;
        self.config_cargo()?;
        // This step taking cares of requirements, such as `MSVC`, also third-party app such as `VS Code`.
        self.install_tools(&tools)?;
        self.install_rust(&tc_components)?;
        // install third-party tools via cargo that got installed by rustup
        self.cargo_install(&tools)?;
        Ok(())
    }

    pub(crate) fn inc_progress(&self, val: f32) -> Result<()> {
        if let Some(prog) = &self.progress_indicator {
            prog.inc(Some(val))?;
        }
        Ok(())
    }

    setter!(
        with_cargo_registry(self.cargo_registry, name: impl ToString, value: impl ToString) {
            Some((name.to_string(), value.to_string()))
        }
    );
    setter!(with_rustup_dist_server(self.rustup_dist_server, Url));
    setter!(with_rustup_update_root(self.rustup_update_root, Url));
    setter!(with_progress_indicator(self.progress_indicator, Option<utils::Progress<'a>>));
    setter!(insecure(self.insecure, bool));

    pub(crate) fn env_vars(&self) -> Result<HashMap<&'static str, String>> {
        let cargo_home = self
            .cargo_home()
            .to_str()
            .map(ToOwned::to_owned)
            .context("`install-dir` cannot contains invalid unicode")?;
        // This `unwrap` is safe here because we've already make sure the `install_dir`'s path can be
        // converted to string with the `cargo_home` variable.
        let rustup_home = self.rustup_home().to_str().unwrap().to_string();

        let mut env_vars = HashMap::from([
            (RUSTUP_DIST_SERVER, self.rustup_dist_server.to_string()),
            (RUSTUP_UPDATE_ROOT, self.rustup_update_root.to_string()),
            (CARGO_HOME, cargo_home),
            (RUSTUP_HOME, rustup_home),
        ]);

        // Add proxy settings if has
        if let Some(proxy) = &self.manifest.proxy {
            if let Some(url) = &proxy.http {
                env_vars.insert("http_proxy", url.to_string());
            }
            if let Some(url) = &proxy.https {
                env_vars.insert("https_proxy", url.to_string());
            }
            if let Some(s) = &proxy.no_proxy {
                env_vars.insert("no_proxy", s.to_string());
            }
        }

        Ok(env_vars)
    }

    fn install_tools_(&mut self, use_cargo: bool, tools: &ToolMap, weight: f32) -> Result<()> {
        let mut to_install = tools
            .iter()
            .filter(|(_, t)| {
                if use_cargo {
                    t.is_cargo_tool()
                } else {
                    !t.is_cargo_tool()
                }
            })
            .collect::<Vec<_>>();

        if to_install.is_empty() {
            return self.inc_progress(weight);
        }
        let sub_progress_delta = weight / to_install.len() as f32;

        if !use_cargo {
            to_install = to_install.topological_sorted();
        }

        for (name, tool) in to_install {
            let info = if use_cargo {
                t!("installing_via_cargo_info", name = name)
            } else {
                t!("installing_tool_info", name = name)
            };
            info!("{info}");

            self.install_tool(name, tool)?;

            self.inc_progress(sub_progress_delta)?;
        }

        self.install_record.write()?;

        Ok(())
    }

    pub fn install_tools(&mut self, tools: &ToolMap) -> Result<()> {
        info!("{}", t!("install_tools"));
        self.install_tools_(false, tools, 30.0)
    }

    pub fn cargo_install(&mut self, tools: &ToolMap) -> Result<()> {
        info!("{}", t!("install_via_cargo"));
        self.install_tools_(true, tools, 30.0)
    }

    pub fn install_rust(&mut self, components: &[ToolchainComponent]) -> Result<()> {
        info!("{}", t!("install_toolchain"));

        let manifest = self.manifest;

        ToolchainInstaller::init()
            .insecure(self.insecure)
            .install(self, manifest, components)?;
        add_to_path(self.cargo_bin())?;
        self.cargo_is_installed = true;

        // Add the rust info to the fingerprint.
        self.install_record
            .add_rust_record(&manifest.rust.channel, components);
        // record meta info
        // TODO(?): Maybe this should be moved as a separate step?
        self.install_record
            .clone_toolkit_meta_from_manifest(manifest);
        // write changes
        self.install_record.write()?;

        self.inc_progress(30.0)
    }

    fn install_tool(&mut self, name: &str, tool: &ToolInfo) -> Result<()> {
        self.remove_obsoleted_tools(tool)?;

        let record = match tool {
            ToolInfo::Basic(version) => {
                Tool::cargo_tool(name, Some(vec![name, "--version", version]))
                    .install(self, tool)?
            }
            ToolInfo::Complex(details) => match details.source.as_ref().with_context(|| {
                format!("tool '{name}' cannot be installed because it's lacking a package source")
            })? {
                ToolSource::Version { version } => {
                    Tool::cargo_tool(name, Some(vec![name, "--version", version]))
                        .install(self, tool)?
                }
                ToolSource::Git {
                    git,
                    branch,
                    tag,
                    rev,
                } => {
                    let mut args = vec!["--git", git.as_str()];
                    if let Some(s) = &branch {
                        args.extend(["--branch", s]);
                    }
                    if let Some(s) = &tag {
                        args.extend(["--tag", s]);
                    }
                    if let Some(s) = &rev {
                        args.extend(["--rev", s]);
                    }

                    Tool::cargo_tool(name, Some(args)).install(self, tool)?
                }
                ToolSource::Path { path, .. } => self.try_install_from_path(name, path, tool)?,
                ToolSource::Url { url, .. } => self.download_and_try_install(name, url, tool)?,
                ToolSource::Restricted { source, .. } => {
                    // the source should be filled before installation, if not, then it means
                    // the program hasn't ask for user input yet, which we should through an error.
                    let real_source = source
                        .as_deref()
                        .with_context(|| t!("missing_restricted_source", name = name))?;
                    let maybe_path = PathBuf::from(real_source);
                    if maybe_path.exists() {
                        self.try_install_from_path(name, &maybe_path, tool)?
                    } else {
                        self.download_and_try_install(
                            name,
                            &real_source.parse().with_context(|| {
                                format!("'{real_source}' is not an existing path nor a valid URL")
                            })?,
                            tool,
                        )?
                    }
                }
            },
        };

        self.install_record.add_tool_record(name, record);

        Ok(())
    }

    fn download_and_try_install(
        &self,
        name: &str,
        url: &Url,
        info: &ToolInfo,
    ) -> Result<ToolRecord> {
        let temp_dir = self.create_temp_dir("download")?;
        let downloaded_file_name = if let Some(name) = info.filename() {
            name
        } else {
            url.path_segments()
                .ok_or_else(|| anyhow!("unsupported url format '{url}'"))?
                .next_back()
                // Sadly, a path segment could be empty string, so we need to filter that out
                .filter(|seg| !seg.is_empty())
                .ok_or_else(|| anyhow!("'{url}' doesn't appear to be a downloadable file"))?
        };
        let dest = temp_dir.path().join(downloaded_file_name);
        utils::DownloadOpt::new(name, GlobalOpts::get().quiet)
            .with_proxy(self.manifest.proxy.clone())
            .blocking_download(url, &dest)?;

        self.try_install_from_path(name, &dest, info)
    }

    fn try_install_from_path(
        &self,
        name: &str,
        path: &Path,
        info: &ToolInfo,
    ) -> Result<ToolRecord> {
        let (tool_installer_path, _maybe_temp) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else if utils::Extractable::is_supported(path) {
            let temp_dir = self.create_temp_dir(name)?;
            let tool_installer_path = self.extract_or_copy_to(path, temp_dir.path())?;
            (tool_installer_path, Some(temp_dir))
        } else if path.is_file() {
            (path.to_path_buf(), None)
        } else {
            bail!(
                "unable to install '{name}' because the path to it's installer '{}' does not exist.",
                path.display()
            );
        };

        let tool_installer = if let Some(kind) = info.kind() {
            Tool::new(name.into(), kind).with_path(tool_installer_path.as_path())
        } else {
            Tool::from_path(name, &tool_installer_path)
                .with_context(|| format!("no install method for tool '{name}'"))?
        };
        tool_installer.install(self, info)
    }

    /// Configuration options for `cargo`.
    ///
    /// This will write a `config.toml` file to `CARGO_HOME`.
    pub fn config_cargo(&self) -> Result<()> {
        info!("{}", t!("install_cargo_config"));

        let mut config = CargoConfig::new();
        if let Some((name, url)) = &self.cargo_registry {
            config.add_source(name, url, true);
        }

        let config_toml = config.to_toml()?;
        if !config_toml.trim().is_empty() {
            let config_path = self.cargo_home().join(CargoConfig::FILENAME);
            utils::write_file(config_path, &config_toml, false)?;
        }

        self.inc_progress(3.0)
    }

    /// Creates a temporary directory under `install_dir/temp`, with a certain prefix.
    pub(crate) fn create_temp_dir(&self, prefix: &str) -> Result<TempDir> {
        let root = self.temp_dir();

        tempfile::Builder::new()
            .prefix(&format!("{prefix}_"))
            .tempdir_in(root)
            .with_context(|| format!("unable to create temp directory under '{}'", root.display()))
    }

    /// Perform extraction or copy action base on the given path.
    ///
    /// If `maybe_file` is a path to compressed file, this will try to extract it to `dest`;
    /// otherwise this will copy that file into dest.
    fn extract_or_copy_to(&self, maybe_file: &Path, dest: &Path) -> Result<PathBuf> {
        if let Ok(extractable) = utils::Extractable::load(maybe_file, None) {
            extractable
                .quiet(GlobalOpts::get().quiet)
                .extract_then_skip_solo_dir(dest, Some("bin"))
        } else {
            utils::copy_into(maybe_file, dest)
        }
    }
}

// For updates
impl InstallConfiguration<'_> {
    pub fn update(mut self, components: Vec<Component>) -> Result<()> {
        // Create a copy of the manifest which is later used for component management.
        self.manifest.write_to_dir(&self.install_dir)?;

        let (toolchain, tools) = split_components(components);
        // setup env for current process
        for (key, val) in self.env_vars()? {
            std::env::set_var(key, val);
        }
        self.inc_progress(10.0)?;

        // don't update toolchain if no toolchain components are selected
        if !toolchain.is_empty() {
            self.update_toolchain(&toolchain)?;
        }
        self.update_tools(&tools)?;
        Ok(())
    }

    fn update_toolchain(&mut self, components: &[ToolchainComponent]) -> Result<()> {
        info!("{}", t!("update_toolchain"));

        let manifest = self.manifest;

        ToolchainInstaller::init()
            .insecure(self.insecure)
            .update(self, manifest)?;

        let record = &mut self.install_record;
        // Add the rust info to the fingerprint.
        record.add_rust_record(&manifest.rust.channel, components);
        // record meta info
        record.clone_toolkit_meta_from_manifest(manifest);
        // write changes
        record.write()?;

        self.inc_progress(60.0)
    }

    fn update_tools(&mut self, tools: &ToolMap) -> Result<()> {
        info!("{}", t!("update_tools"));
        self.install_tools_(false, tools, 15.0)?;
        self.install_tools_(true, tools, 15.0)?;
        Ok(())
    }

    fn remove_obsoleted_tools(&mut self, tool: &ToolInfo) -> Result<()> {
        let obsoleted_tool_names = tool.obsoletes();
        for obsolete in obsoleted_tool_names {
            // check if this tool was installed, if yes, get the installation record of it
            let Some(rec) = self.install_record.tools.get(obsolete) else {
                continue;
            };
            let Some(tool) = Tool::from_installed(obsolete, rec) else {
                continue;
            };

            info!("{}", t!("removing_obsolete_tool", name = obsolete));
            tool.uninstall(&*self)?;
            self.install_record.remove_tool_record(obsolete);
        }

        Ok(())
    }
}

// TODO: Conflict resolve should take place during user interaction, not here,
// but it's kind hard to do with how we handle CLI interaction now, figure out a way.
fn reject_conflicting_tools(tools: &ToolMap) -> Result<()> {
    // use a HashSet to collect conflicting pairs to remove duplicates.
    let mut conflicts = HashSet::new();

    for (name, info) in tools {
        for conflicted_name in info.conflicts() {
            // ignore the tools that are not presented in the map
            if !tools.contains_key(conflicted_name) {
                continue;
            }

            // sort the conflicting pairs, so that (A, B) and (B, A) will both
            // resulting to (A, B), thus became unique in the set
            let pair = if name < conflicted_name.as_str() {
                (name, conflicted_name.as_str())
            } else {
                (conflicted_name.as_str(), name)
            };
            conflicts.insert(pair);
        }
    }

    if !conflicts.is_empty() {
        let conflict_list = conflicts
            .into_iter()
            .map(|(a, b)| format!("\t{a} ({})", t!("conflicts_with", name = b)))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("{}:\n{conflict_list}", t!("conflict_detected"));
    }

    Ok(())
}

/// Get the default installation directory,
/// which is a directory under [`home_dir`](utils::home_dir).
pub fn default_install_dir() -> PathBuf {
    utils::home_dir().join(DEFAULT_FOLDER_NAME)
}

/// Split components list to `toolchain_components` and `toolset_components`,
/// as we are running `rustup` to install toolchain components, but using other methods
/// for toolset components.
///
/// Note: the splitted `toolchain_components` contains the base profile name
/// such as `minimal` at first index.
fn split_components(components: Vec<Component>) -> (Vec<ToolchainComponent>, ToolMap) {
    let toolset_components = component_list_to_tool_map(
        components
            .iter()
            .filter(|cm| !cm.kind.is_from_toolchain())
            .collect(),
    );
    let toolchain_components: Vec<ToolchainComponent> = components
        .into_iter()
        .filter_map(|comp| match comp.kind {
            ComponentType::ToolchainComponent => Some(ToolchainComponent::new(&comp.name)),
            ComponentType::ToolchainProfile => {
                Some(ToolchainComponent::new(&comp.name).is_profile(true))
            }
            _ => None,
        })
        .collect();

    (toolchain_components, toolset_components)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::get_toolkit_manifest;

    #[tokio::test]
    async fn init_install_config() {
        let mut cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        cache_dir.push("tests");
        cache_dir.push("cache");

        std::fs::create_dir_all(&cache_dir).unwrap();

        let install_root = tempfile::Builder::new().tempdir_in(&cache_dir).unwrap();
        let manifest = get_toolkit_manifest(None, false).await.unwrap();
        let mut config = InstallConfiguration::new(install_root.path(), &manifest).unwrap();
        config.setup().unwrap();

        assert!(config.install_record.name.is_none());
        assert!(install_root
            .path()
            .join(InstallationRecord::FILENAME)
            .is_file());
    }

    #[test]
    fn detect_package_conflicts() {
        let raw = r#"
a = { version = "0.1.0", conflicts = ["b"] }
b = { version = "0.1.0", conflicts = ["a"] }
c = { version = "0.1.0", conflicts = ["d", "a"] }
"#;
        let map: ToolMap = toml::from_str(raw).unwrap();
        let conflicts = reject_conflicting_tools(&map);

        assert!(conflicts.is_err());

        let error = conflicts.expect_err("has conflicts");
        println!("{error}");
    }
}
