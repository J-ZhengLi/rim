use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use anyhow::{anyhow, bail, Result};
use rim_common::{
    types::{TomlParser, ToolInfo, ToolKind},
    utils,
};

use super::{
    directories::RimDir,
    parser::{cargo_config::CargoConfig, fingerprint::ToolRecord},
    GlobalOpts, PathExt, CARGO_HOME,
};
use crate::{
    core::{check::RUNNER_TOOLCHAIN_NAME, custom_instructions},
    InstallConfiguration,
};

/// All supported VS Code variants
pub(crate) static VSCODE_FAMILY: LazyLock<Vec<String>> = LazyLock::new(|| {
    #[cfg(windows)]
    let suffix = ".cmd";
    #[cfg(not(windows))]
    let suffix = "";
    // This list has a fallback order, DO NOT change the order.
    [
        "codearts-rust",
        "codium",
        "hwcode",
        "wecode",
        "code-exploration",
        "code-oss",
        "code",
    ]
    .iter()
    .map(|s| format!("{s}{suffix}"))
    .collect()
});

#[derive(Debug, Clone)]
pub(crate) struct Tool<'a> {
    name: String,
    /// The install location of this tool
    path: PathExt<'a>,
    pub(crate) kind: ToolKind,
    /// Additional args to run installer, currently only used for `cargo install`.
    install_args: Option<Vec<&'a str>>,
}

/// Helper struct used for uninstallation, including basic [`Tool`] and it's dependencies list.
#[derive(Debug)]
pub(crate) struct ToolWithDeps<'a> {
    pub(crate) tool: Tool<'a>,
    pub(crate) dependencies: &'a [String],
}

impl<'a> Tool<'a> {
    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn new(name: String, kind: ToolKind) -> Self {
        Self {
            name,
            kind,
            path: PathExt::default(),
            install_args: None,
        }
    }

    setter!(with_path(self.path, path: impl Into<PathExt<'a>>) { path.into() });
    setter!(with_install_args(self.install_args, Option<Vec<&'a str>>));

    pub(crate) fn from_path(name: &str, path: &'a Path) -> Result<Self> {
        if !path.exists() {
            bail!(
                "the path for '{name}' specified as '{}' does not exist.",
                path.display()
            );
        }
        let name = name.to_string();

        // Step 1: Looking for custom instruction
        if custom_instructions::is_supported(&name) {
            return Ok(Self::new(name, ToolKind::Custom).with_path(path));
        }

        // Step 2: Identify from file extension (if it's a file ofc).
        if utils::is_executable(path) {
            return Ok(Self::new(name, ToolKind::Executables).with_path(path));
        } else if Plugin::is_supported(path) {
            return Ok(Self::new(name, ToolKind::Plugin).with_path(path));
        }
        // TODO: Well, we got a directory, things are getting complicated, there could be one of this scenarios:
        // 1. Directory contains some executable files and nothing else
        //      Throw these executable files into cargo bin folder
        // 2. Directory contains sub-directory, which look like `bin/ lib/ etc/ ...`
        //      Throw and merge this directories into cargo home. (might be bad, therefore we need a `Manifest.in`!!!)
        // 3. Directory doesn't fit all previous characteristics.
        //      We don't know how to install this tool, throw an error instead.
        else if path.is_dir() {
            // Step 3: read directory to find characteristics.
            let entries = utils::walk_dir(path, false)?;
            // Check for file named `Cargo.toml`
            if entries.iter().any(|path| path.ends_with("Cargo.toml")) {
                return Ok(Self::new(name, ToolKind::Crate).with_path(path));
            }
            // Check if there is any folder that looks like `bin`
            // Then assuming this is `UsrDirs` type installer.
            if entries.iter().any(|path| path.ends_with("bin")) {
                return Ok(Self::new(name, ToolKind::DirWithBin).with_path(path));
            }
            // If no sub folder exists, and there are binaries lays directly in the folder
            if !entries.iter().any(|path| path.is_dir()) {
                let assumed_binaries = entries
                    .iter()
                    .filter_map(|path| utils::is_executable(path).then_some(path.to_path_buf()))
                    .collect::<Vec<_>>();
                return Ok(Self::new(name, ToolKind::Executables).with_path(assumed_binaries));
            }
        }

        warn!("unknown tool '{name}', it's installer doesn't fit any predefined characteristic");
        Ok(Self::new(name, ToolKind::Unknown).with_path(path))
    }

    /// Specify as a tool that managed by `cargo`.
    ///
    /// Note: `extra_args` should not contains "install" and `name`.
    pub(crate) fn cargo_tool(name: &str, extra_args: Option<Vec<&'a str>>) -> Self {
        Self::new(name.to_string(), ToolKind::CargoTool).with_install_args(extra_args)
    }

    /// Convert a single [`tool record`](ToolRecord) into `Self`, return `None`
    /// if this tool with `name` was not installed
    pub(crate) fn from_installed(name: &str, tool_record: &'a ToolRecord) -> Option<Self> {
        let kind = tool_record.tool_kind();
        let tool = match kind {
            ToolKind::CargoTool => Tool::cargo_tool(name, None),
            ToolKind::Unknown => {
                if let [path] = tool_record.paths.as_slice() {
                    // don't interrupt uninstallation if the path of some tools cannot be found,
                    // as the user might have manually remove them
                    let Ok(tool) = Tool::from_path(name, path) else {
                        warn!(
                            "{}: {}",
                            t!("uninstall_tool_skipped", tool = name),
                            t!("path_to_installation_not_found", path = path.display())
                        );
                        return None;
                    };
                    tool
                } else if !tool_record.paths.is_empty() {
                    Tool::new(name.into(), ToolKind::Executables)
                        .with_path(tool_record.paths.clone())
                } else {
                    warn!("{}", t!("uninstall_unknown_tool_warn", tool = name));
                    return None;
                }
            }
            _ => Tool::new(name.into(), kind).with_path(tool_record.paths.clone()),
        };
        Some(tool)
    }

    pub(crate) fn install(
        &self,
        config: &InstallConfiguration,
        info: &ToolInfo,
    ) -> Result<ToolRecord> {
        let paths = match self.kind {
            ToolKind::CargoTool => {
                if !config.toolchain_is_installed {
                    bail!(
                        "trying to install '{}' using cargo, but cargo is not installed",
                        self.name()
                    );
                }

                cargo_install_or_uninstall(
                    "install",
                    self.install_args.as_deref().unwrap_or(&[self.name()]),
                    config.cargo_home(),
                )?;
                return Ok(ToolRecord::cargo_tool().with_version(info.version()));
            }
            ToolKind::Executables => {
                let mut res = vec![];
                for exe in self.path.iter() {
                    res.push(utils::copy_into(exe, config.cargo_bin())?);
                }
                res
            }
            ToolKind::Custom => {
                custom_instructions::install(self.name(), self.path.single()?, config)?
            }
            ToolKind::DirWithBin => {
                let tool_dir = install_dir_with_bin_(config, self.name(), self.path.single()?)?;
                vec![tool_dir]
            }
            ToolKind::Plugin => {
                let path = self.path.single()?;
                // run the installation command.
                Plugin::install(path)?;
                // we need to "cache" to installer, so that we could uninstall with it.
                let plugin_backup = utils::copy_into(path, config.tools_dir())?;
                vec![plugin_backup]
            }
            ToolKind::Installer => {
                let path = self.path.single()?;
                // Just run the installer and wait for finish.
                #[cfg(windows)]
                run!("powershell", "-Command", "Start-Process", "-Wait", path)?;
                #[cfg(not(windows))]
                run!(path)?;
                // Make a backup for this installer, in some case,
                // it can be used for uninstallation
                let backup = utils::copy_into(path, config.tools_dir())?;
                vec![backup]
            }
            ToolKind::RuleSet => install_rule_set(&self.path, config)?,
            ToolKind::Crate => install_crate(self.name(), &self.path, config)?,
            // Just throw it under `tools` dir
            ToolKind::Unknown => {
                vec![move_to_tools(config, self.name(), self.path.single()?)?]
            }
        };

        Ok(ToolRecord::new(self.kind)
            .with_paths(paths)
            .with_version(info.version())
            .with_dependencies(info.dependencies().to_vec()))
    }

    /// Remove a tool from user's machine.
    pub(crate) fn uninstall<T: RimDir>(&self, config: T) -> Result<()> {
        match self.kind {
            ToolKind::CargoTool => {
                cargo_install_or_uninstall(
                    "uninstall",
                    self.install_args.as_deref().unwrap_or(&[self.name()]),
                    config.cargo_home(),
                )?;
            }
            ToolKind::Executables => {
                for binary in self.path.iter() {
                    fs::remove_file(binary)?;
                }
            }
            ToolKind::Custom => custom_instructions::uninstall(self.name(), config)?,
            ToolKind::DirWithBin => uninstall_dir_with_bin_(self.path.single()?)?,
            ToolKind::Plugin => Plugin::uninstall(self.path.single()?)?,
            ToolKind::Installer => {
                // TODO: some installer have uninstall functionality but some may not,
                // make a list of those and only execute it if it can be used for uninstallation
                utils::remove(self.path.single()?)?;
            }
            ToolKind::Crate => uninstall_crate(self.name(), &self.path, config)?,
            ToolKind::RuleSet | ToolKind::Unknown => {
                utils::remove(self.path.single()?)?;
            }
        }
        Ok(())
    }
}

fn cargo_install_or_uninstall(op: &str, args: &[&str], cargo_home: &Path) -> Result<()> {
    let mut cargo_bin = cargo_home.to_path_buf();
    cargo_bin.push("bin");
    cargo_bin.push(exe!("cargo"));

    let mut cmd = cmd!([CARGO_HOME=cargo_home] cargo_bin, op);
    let mut full_args = vec![];

    if GlobalOpts::get().verbose {
        full_args.push("-v");
    } else if GlobalOpts::get().quiet {
        full_args.push("-q");
    }
    full_args.extend_from_slice(args);
    cmd.args(full_args);
    utils::execute(cmd)?;
    Ok(())
}

/// Move one path (file/dir) to a new folder with `name` under tools dir.
fn move_to_tools(config: &InstallConfiguration, name: &str, path: &Path) -> Result<PathBuf> {
    let dir = config.tools_dir().join(name);
    utils::move_to(path, &dir, true)?;
    Ok(dir)
}

/// Install [`ToolKind::DirWithBin`], with a couple steps:
/// - Move the `tool_dir` to [`tools_dir`](InstallConfiguration::tools_dir).
/// - Add the `bin_dir` to PATH
fn install_dir_with_bin_(
    config: &InstallConfiguration,
    name: &str,
    path: &Path,
) -> Result<PathBuf> {
    let dir = move_to_tools(config, name, path)?;
    let bin_dir_after_move = dir.join("bin");
    super::os::add_to_path(&bin_dir_after_move)?;
    Ok(dir)
}

fn install_rule_set(path: &PathExt<'_>, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
    let src_dir = path.single()?;

    // make a directory to store it under `tools`
    let dest_root = config.tools_dir().join("ruleset");
    // make a runner directory to store the runner toolchain.
    // (since we are using a dedicated toolchain with customized clippy, the runner
    // should be the only directory under `ruleset`. If we use `cargo-dylint` in the future
    // we'll need another directory to store custom lints)
    let runner_dir = dest_root.join("runner");
    utils::copy_as(src_dir, &runner_dir)?;

    if !config.toolchain_is_installed {
        bail!(t!("no_toolchain_installed"));
    }

    let path_to_rustup = config.cargo_bin().join(exe!("rustup"));
    // link the runner toolchain using rustup
    run!([CARGO_HOME = config.cargo_home()] path_to_rustup, "toolchain", "link", RUNNER_TOOLCHAIN_NAME, &runner_dir)?;

    Ok(vec![runner_dir])
}

fn install_crate(
    name: &str,
    path: &PathExt<'_>,
    config: &InstallConfiguration,
) -> Result<Vec<PathBuf>> {
    let path = path.single()?;

    // Step 1: copy the directory path to `crates/`
    let crate_dir = utils::copy_into(path, config.crates_dir())?;
    // Step 2: modify `cargo/config.toml` to update patch information
    // FIXME: This method might disrupt existing configuration that was manually altered by user,
    // use `toml-edit` to modify it instead.
    let mut cargo_config = CargoConfig::load_from_dir(config.cargo_home())?;
    cargo_config
        .add_patch(name, &crate_dir)
        .write_to_dir(config.cargo_home())?;

    Ok(vec![crate_dir])
}

fn uninstall_crate<T: RimDir>(name: &str, path: &PathExt<'_>, config: T) -> Result<()> {
    let path = path.single()?;

    // remove the source code dir
    utils::remove(path)?;

    // update cargo config
    let mut cargo_config = CargoConfig::load_from_dir(config.cargo_home())?;
    cargo_config
        .remove_patch(name)
        .write_to_dir(config.cargo_home())?;

    Ok(())
}

/// Uninstalling a tool with bin folder is as simple as removing the directory,
/// and removing the `bin` dir from `PATH`.
fn uninstall_dir_with_bin_(tool_path: &Path) -> Result<()> {
    // Remove from `PATH` at first.
    let bin_dir = tool_path.join("bin");
    super::os::remove_from_path(&bin_dir)?;

    fs::remove_dir_all(tool_path)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[non_exhaustive]
pub(crate) enum Plugin {
    Vsix,
}

impl FromStr for Plugin {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "vsix" => Ok(Self::Vsix),
            _ => bail!("unsupported plugin file type '{s}'"),
        }
    }
}

impl Plugin {
    /// Examine the extension of a file path. and return `true` if it's a supported plugin type.
    fn is_supported(path: &Path) -> bool {
        let Some(ext) = utils::extension_str(path) else {
            return false;
        };

        matches!(ext, "vsix")
    }

    fn install(plugin_path: &Path) -> Result<()> {
        Self::install_or_uninstall_(plugin_path, false)
    }

    fn uninstall(plugin_path: &Path) -> Result<()> {
        Self::install_or_uninstall_(plugin_path, true)
    }

    fn install_or_uninstall_(plugin_path: &Path, uninstall: bool) -> Result<()> {
        let ty = utils::extension_str(plugin_path)
            .and_then(|ext| Self::from_str(ext).ok())
            .ok_or_else(|| anyhow!("unsupported plugin file '{}'", plugin_path.display()))?;

        match ty {
            Plugin::Vsix => {
                for program in VSCODE_FAMILY.as_slice() {
                    if !utils::cmd_exist(program) {
                        continue;
                    }

                    let op = if uninstall { "uninstall" } else { "install" };
                    let arg_opt = format!("--{op}-extension");
                    info!(
                        "{}",
                        t!(
                            "handling_extension_info",
                            op = t!(op),
                            ext = plugin_path.display(),
                            program = program
                        )
                    );
                    match run!(program, arg_opt, plugin_path) {
                        Ok(_) => continue,
                        // Ignore error when uninstalling.
                        Err(_) if uninstall => {
                            info!(
                                "{}",
                                t!(
                                    "skip_extension_uninstall_warn",
                                    ext = plugin_path.display(),
                                    program = program
                                )
                            );
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }

                // Remove the plugin file if uninstalling
                if uninstall {
                    utils::remove(plugin_path)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_order() {
        let mut tools = vec![];

        tools.push(ToolKind::Executables);
        tools.push(ToolKind::CargoTool);
        tools.push(ToolKind::Custom);
        tools.push(ToolKind::Plugin);
        tools.push(ToolKind::DirWithBin);
        tools.push(ToolKind::Executables);

        tools.sort();

        let mut tools_iter = tools.iter();
        assert!(matches!(tools_iter.next(), Some(ToolKind::DirWithBin)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Executables)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Executables)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Custom)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Plugin)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::CargoTool)));
        assert!(matches!(tools_iter.next(), None));
    }

    #[test]
    fn tools_order_reversed() {
        let mut tools = vec![];

        tools.push(ToolKind::Executables);
        tools.push(ToolKind::CargoTool);
        tools.push(ToolKind::Custom);
        tools.push(ToolKind::Plugin);
        tools.push(ToolKind::DirWithBin);
        tools.push(ToolKind::Executables);

        tools.sort_by(|a, b| b.cmp(a));

        let mut tools_iter = tools.iter();
        assert!(matches!(tools_iter.next(), Some(ToolKind::CargoTool)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Plugin)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Custom)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Executables)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::Executables)));
        assert!(matches!(tools_iter.next(), Some(ToolKind::DirWithBin)));
        assert!(matches!(tools_iter.next(), None));
    }
}
