use super::components::{split_components, ToolchainComponent};
use super::dependency_handler::DependencyHandler;
use super::{
    components::Component,
    directories::RimDir,
    parser::{
        cargo_config::CargoConfig,
        fingerprint::{InstallationRecord, ToolRecord},
    },
    rustup::ToolchainInstaller,
    tools::Tool,
    GlobalOpts, CARGO_HOME, RUSTUP_DIST_SERVER, RUSTUP_HOME, RUSTUP_UPDATE_ROOT,
};
use crate::core::baked_in_manifest_raw;
use crate::core::os::add_to_path;
use crate::{default_cargo_registry, default_rustup_dist_server, default_rustup_update_root};
use anyhow::{anyhow, bail, Context, Result};
use rim_common::types::{
    CargoRegistry, TomlParser, ToolInfo, ToolMap, ToolSource, ToolkitManifest,
};
use rim_common::{build_config, utils};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
    /// Path to install everything.
    ///
    /// Note that this folder will includes `cargo` and `rustup` folders as well.
    /// And the default location will under `$HOME` directory (`%USERPROFILE%` on windows).
    /// So, even if the user didn't specify any install path, a pair of env vars will still
    /// be written (CARGO_HOME and RUSTUP_HOME), which will be under the default location
    /// defined by [`default_install_dir`].
    pub install_dir: PathBuf,
    /// cargo registry config via user input.
    cargo_registry: Option<CargoRegistry>,
    /// rustup dist server via user input.
    rustup_dist_server: Option<Url>,
    /// rustup update root via user input.
    rustup_update_root: Option<Url>,
    /// Indicates whether the rust toolchain was already installed,
    /// useful when installing third-party tools.
    pub toolchain_is_installed: bool,
    install_record: InstallationRecord,
    pub(crate) progress_indicator: Option<utils::Progress<'a>>,
    pub(crate) manifest: &'a ToolkitManifest,
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
        let install_record = if InstallationRecord::exists() {
            // TODO: handle existing record, maybe we want to enter manager mode directly?
            InstallationRecord::load_from_config_dir()?
        } else {
            InstallationRecord {
                install_dir: install_dir.to_path_buf(),
                ..Default::default()
            }
        };
        Ok(Self {
            install_dir: install_dir.to_path_buf(),
            install_record,
            cargo_registry: None,
            rustup_dist_server: None,
            rustup_update_root: None,
            toolchain_is_installed: false,
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
        utils::ensure_dir(install_dir)?;

        // Create a copy of the manifest which is later used for component management.
        // NB: This `setup` function only gets called during the first installation,
        // which means this manifest should always loaded from the baked-in one.
        // NB: If this is an offline build, meaning the manifest is likely to contain
        // local paths, which is not useful for adding components afterwards, therefore
        // we better store the online version instead,
        if self.manifest.is_offline {
            ToolkitManifest::from_str(baked_in_manifest_raw(false))?.write_to_dir(install_dir)?;
        } else {
            self.manifest.write_to_dir(install_dir)?;
        }

        // rename this installer to 'xxx-manager' and copy it into installer dir
        let self_exe = std::env::current_exe()?;
        let app_name = build_config().app_name();
        let manager_name = exe!(&app_name);
        let manager_exe = install_dir.join(&manager_name);
        utils::copy_as(self_exe, &manager_exe)?;

        // Write application icon (name: <APP_NAME>.ico) to the install dir for shortcut.
        // Note that this file currently have no use for CLI version, but we still put
        // it there to be future-proof.
        let ico_content = include_bytes!("../../rim_gui/public/favicon.ico");
        let ico_file_dest = install_dir.join(format!("{app_name}.ico"));
        utils::write_bytes(ico_file_dest, ico_content, false)?;

        // soft-link this binary into cargo bin, so it will be in th PATH
        // Note: we are creating two symlinks binary, one have the fullname,
        // and one with shorter name (rim)
        let link_full = self.cargo_bin().join(manager_name);
        let link_short = self.cargo_bin().join(exe!(env!("CARGO_PKG_NAME")));
        utils::create_link(&manager_exe, &link_full)
            .with_context(|| format!("unable to create a link as '{}'", link_full.display()))?;
        utils::create_link(&manager_exe, &link_short)
            .with_context(|| format!("unable to create a link as '{}'", link_short.display()))?;

        #[cfg(windows)]
        // Create registry entry to add this program into "installed programs".
        super::os::windows::do_add_to_programs(&manager_exe)?;

        if let Some(prog) = &self.progress_indicator {
            prog.inc(Some(5.0))?;
        }

        Ok(())
    }

    pub fn install(mut self, components: Vec<Component>) -> Result<()> {
        let inner_ = || {
            let (tc_components, tools) = split_components(components);
            reject_conflicting_tools(&tools)?;

            self.setup()?;
            self.config_env_vars()?;
            self.config_cargo()?;
            // This step taking cares of requirements, such as `MSVC`, also third-party app such as `VS Code`.
            self.install_tools(&tools)?;
            self.install_rust(&tc_components)?;
            self.install_tools_late(&tools)?;
            Ok(())
        };

        let install_result = inner_();
        if install_result.is_err() {
            // TODO: revert changes
        }

        install_result
    }

    pub(crate) fn inc_progress(&self, val: f32) -> Result<()> {
        if let Some(prog) = &self.progress_indicator {
            prog.inc(Some(val))?;
        }
        Ok(())
    }

    /// Getting the server url that used to download toolchain packages using rustup.
    ///
    /// This is guaranteed to return a value, and it has a fallback order as below:
    /// 1. `rustup-dist-server` from [`ToolkitManifest`]'s config.
    /// 2. `rustup-dist-server` from user input (`self.rustup_dist_server`), such as CLI options.
    /// 3. Default value that is configured through `./configuration.toml`,
    ///    and returned by [`default_rustup_dist_server`].
    pub(crate) fn rustup_dist_server(&self) -> &Url {
        self.manifest
            .config
            .rustup_dist_server
            .as_ref()
            .or(self.rustup_dist_server.as_ref())
            .unwrap_or_else(|| default_rustup_dist_server())
    }

    /// Getting the server url that used to download rustup update.
    ///
    /// This is guaranteed to return a value, and it has a fallback order as below:
    /// 1. `rustup-update-root` from [`ToolkitManifest`]'s config.
    /// 2. `rustup-update-root` from user input (`self.rustup_update_root`), such as CLI options.
    /// 3. Default value that is configured through `./configuration.toml`,
    ///    and returned by [`default_rustup_update_root`].
    pub(crate) fn rustup_update_root(&self) -> &Url {
        self.manifest
            .config
            .rustup_update_root
            .as_ref()
            .or(self.rustup_update_root.as_ref())
            .unwrap_or_else(|| default_rustup_update_root())
    }

    /// Getting the cargo registry config.
    ///
    /// This is guaranteed to return a value, and it has a fallback order as below:
    /// 1. `cargo_registry` from [`ToolkitManifest`]'s config.
    /// 2. `cargo_registry` from user input (`self.cargo_registry`), such as CLI options.
    /// 3. Default value that is configured through `./configuration.toml`,
    ///    and returned by [`default_cargo_registry`].
    pub(crate) fn cargo_registry(&self) -> CargoRegistry {
        self.manifest
            .config
            .cargo_registry
            .clone()
            .or(self.cargo_registry.clone())
            .unwrap_or_else(|| default_cargo_registry().into())
    }

    setter!(
        with_cargo_registry(self.cargo_registry, registry: Option<impl Into<CargoRegistry>>) {
            registry.map(|r| r.into())
        }
    );
    setter!(with_rustup_dist_server(self.rustup_dist_server, Option<Url>));
    setter!(with_rustup_update_root(self.rustup_update_root, Option<Url>));
    setter!(with_progress_indicator(self.progress_indicator, Option<utils::Progress<'a>>));
    setter!(insecure(self.insecure, bool));

    pub(crate) fn env_vars(&self) -> Result<Vec<(&'static str, String)>> {
        let cargo_home = self
            .cargo_home()
            .to_str()
            .map(ToOwned::to_owned)
            .context("`install-dir` cannot contains invalid unicode")?;
        // This `unwrap` is safe here because we've already make sure the `install_dir`'s path can be
        // converted to string with the `cargo_home` variable.
        let rustup_home = self.rustup_home().to_str().unwrap().to_string();

        let mut env_vars = Vec::from([
            (RUSTUP_DIST_SERVER, self.rustup_dist_server().to_string()),
            (RUSTUP_UPDATE_ROOT, self.rustup_update_root().to_string()),
            (RUSTUP_DIST_SERVER, self.rustup_dist_server().to_string()),
            (RUSTUP_UPDATE_ROOT, self.rustup_update_root().to_string()),
            (RUSTUP_DIST_SERVER, self.rustup_dist_server().to_string()),
            (RUSTUP_UPDATE_ROOT, self.rustup_update_root().to_string()),
            (RUSTUP_DIST_SERVER, self.rustup_dist_server().to_string()),
            (RUSTUP_UPDATE_ROOT, self.rustup_update_root().to_string()),
            (CARGO_HOME, cargo_home),
            (RUSTUP_HOME, rustup_home),
        ]);

        // Add proxy settings if has
        if let Some(proxy) = self.manifest.proxy_config() {
            if let Some(url) = &proxy.http {
                env_vars.push(("http_proxy", url.to_string()));
            }
            if let Some(url) = &proxy.https {
                env_vars.push(("https_proxy", url.to_string()));
            }
            if let Some(s) = &proxy.no_proxy {
                // keep use's original no_proxy var.
                #[cfg(windows)]
                let prev_np = std::env::var("no_proxy").unwrap_or_default();
                #[cfg(unix)]
                let prev_np = "$no_proxy";

                let no_proxy = if prev_np.is_empty() {
                    s.to_string()
                } else {
                    format!("{s},{prev_np}")
                };
                env_vars.push(("no_proxy", no_proxy));
            }
        }

        Ok(env_vars)
    }

    fn install_tools_(&mut self, use_rust: bool, tools: &ToolMap, weight: f32) -> Result<()> {
        let mut to_install = tools
            .iter()
            .filter(|(_, t)| {
                let requires_toolchain =
                    t.is_cargo_tool() || t.dependencies().iter().any(|s| s == "rust");
                if use_rust {
                    requires_toolchain
                } else {
                    !requires_toolchain
                }
            })
            .collect::<Vec<_>>();

        if to_install.is_empty() {
            return self.inc_progress(weight);
        }
        let sub_progress_delta = weight / to_install.len() as f32;

        to_install = to_install.topological_sorted();
        // topological sort place the tool with more dependencies at the back,
        // which is what we need to install first, therefore we need to reverse it.
        to_install.reverse();

        for (name, tool) in to_install {
            info!("{}", t!("installing_tool_info", name = name));
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

    /// A step to include `cargo install`, and any tools that requires rust to be installed
    pub fn install_tools_late(&mut self, tools: &ToolMap) -> Result<()> {
        info!("{}", t!("install_via_cargo"));
        self.install_tools_(true, tools, 30.0)
    }

    /// Install Rust toolchain with a list of components
    pub fn install_rust(&mut self, components: &[ToolchainComponent]) -> Result<()> {
        info!("{}", t!("install_toolchain"));

        let manifest = self.manifest;

        ToolchainInstaller::init(&*self)
            .insecure(self.insecure)
            .rustup_dist_server(Some(self.rustup_dist_server().clone()))
            .install(self, components)?;
        add_to_path(&*self, self.cargo_bin())?;
        self.toolchain_is_installed = true;

        // Add the rust info to the fingerprint.
        self.install_record
            .add_rust_record(&manifest.toolchain.channel, components);
        // record meta info
        // TODO(?): Maybe this should be moved as a separate step?
        self.install_record
            .clone_toolkit_meta_from_manifest(manifest);
        // write changes
        self.install_record.write()?;

        self.inc_progress(30.0)
    }

    /// Add toolchain components separately, typically used in `component add`.
    pub fn install_toolchain_components(
        &mut self,
        components: &[ToolchainComponent],
    ) -> Result<()> {
        ToolchainInstaller::init(&*self)
            .insecure(self.insecure)
            .rustup_dist_server(Some(self.rustup_dist_server().clone()))
            .add_components(self, components)?;

        self.install_record
            .add_rust_record(&self.manifest.toolchain.channel, components);
        self.install_record.write()?;
        Ok(())
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
                ToolSource::Path { path, .. } => {
                    self.try_install_from_path(name, path, tool, None)?
                }
                ToolSource::Url { url, .. } => self.download_and_try_install(name, url, tool)?,
                ToolSource::Restricted { source, .. } => {
                    // the source should be filled before installation, if not, then it means
                    // the program hasn't ask for user input yet, which we should through an error.
                    let real_source = source
                        .as_deref()
                        .with_context(|| t!("missing_restricted_source", name = name))?;
                    let maybe_path = PathBuf::from(real_source);
                    if maybe_path.exists() {
                        self.try_install_from_path(name, &maybe_path, tool, None)?
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
            .with_proxy(self.manifest.proxy_config().cloned())
            .blocking_download(url, &dest)?;

        self.try_install_from_path(name, &dest, info, Some(temp_dir))
    }

    fn try_install_from_path(
        &self,
        name: &str,
        path: &Path,
        info: &ToolInfo,
        dl_temp: Option<TempDir>,
    ) -> Result<ToolRecord> {
        let mut maybe_temp = dl_temp;
        let tool_installer_path = if path.is_dir() {
            path.to_path_buf()
        } else if utils::Extractable::is_supported(path) {
            let extract_temp = self.create_temp_dir(name)?;
            let tool_installer_path = self.extract_or_copy_to(path, extract_temp.path())?;
            // we don't need the download temp dir anymore,
            // we should keep the extraction temp dir alive instead.
            maybe_temp = Some(extract_temp);
            tool_installer_path
        } else if path.is_file() {
            path.to_path_buf()
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

        let res = tool_installer.install(self, info);
        drop(maybe_temp);
        res
    }

    /// Configuration options for `cargo`.
    ///
    /// This will write a `config.toml` file to `CARGO_HOME`.
    pub fn config_cargo(&self) -> Result<()> {
        info!("{}", t!("install_cargo_config"));

        let mut config = CargoConfig::new();
        let registry = self.cargo_registry();
        config.add_source(&registry.name, &registry.index, true);

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

        ToolchainInstaller::init(&*self)
            .insecure(self.insecure)
            .update(self, components)?;

        let record = &mut self.install_record;
        // Add the rust info to the fingerprint.
        record.add_rust_record(&self.manifest.toolchain.channel, components);
        // record meta info
        record.clone_toolkit_meta_from_manifest(self.manifest);
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
    rim_common::dirs::home_dir().join(DEFAULT_FOLDER_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn no_proxy_env_var() {
        let raw = r#"
[rust]
version = "1.0.0"

[proxy]
no_proxy = "localhost,.example.com,.foo.com"
"#;

        let manifest = ToolkitManifest::from_str(raw).unwrap();

        let mut cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        cache_dir.push("tests");
        cache_dir.push("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let install_root = tempfile::Builder::new().tempdir_in(&cache_dir).unwrap();

        let install_cfg = InstallConfiguration::new(install_root.path(), &manifest).unwrap();

        // Temporarily modify no_proxy var to test inheritance.
        let no_proxy_backup = std::env::var("no_proxy");
        std::env::remove_var("no_proxy");

        let env_vars = install_cfg.env_vars().unwrap();
        let new_no_proxy_var = &env_vars.iter().find(|(k, _)| *k == "no_proxy").unwrap().1;

        #[cfg(windows)]
        assert_eq!(new_no_proxy_var, "localhost,.example.com,.foo.com");
        #[cfg(unix)]
        assert_eq!(
            new_no_proxy_var,
            "localhost,.example.com,.foo.com,$no_proxy"
        );

        std::env::set_var("no_proxy", ".bar.com,baz.com");
        let env_vars = install_cfg.env_vars().unwrap();
        let new_no_proxy_var = &env_vars.iter().find(|(k, _)| *k == "no_proxy").unwrap().1;

        #[cfg(windows)]
        assert_eq!(
            new_no_proxy_var,
            "localhost,.example.com,.foo.com,.bar.com,baz.com"
        );
        #[cfg(unix)]
        assert_eq!(
            new_no_proxy_var,
            "localhost,.example.com,.foo.com,$no_proxy"
        );

        if let Ok(bck) = no_proxy_backup {
            std::env::set_var("no_proxy", bck);
        }
    }
}
