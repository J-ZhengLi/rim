use anyhow::Result;
use rim_common::utils::Progress;
use std::{collections::HashMap, path::PathBuf};

use super::{
    dependency_handler::DependencyHandler,
    directories::RimDir,
    parser::fingerprint::{installed_tools, InstallationRecord, ToolRecord},
    rustup::ToolchainInstaller,
    tools::ToolWithDeps,
};
use crate::core::tools::Tool;

/// Contains definition of uninstallation steps.
pub(crate) trait Uninstallation {
    /// Remove persistent environment variables for `rustup`.
    ///
    /// This will remove persistent environment variables including
    /// `RUSTUP_DIST_SERVER`, `RUSTUP_UPDATE_ROOT`, `CARGO_HOME`, `RUSTUP_HOME`.
    fn remove_rustup_env_vars(&self) -> Result<()>;
    /// The last step of uninstallation, this will remove the binary itself, along with
    /// the folder it's in.
    fn remove_self(&self) -> Result<()>;
}

/// Contains every information that the uninstallation process needs.
pub struct UninstallConfiguration<'a> {
    /// The installation directory that holds every tools, configuration files,
    /// including the manager binary.
    pub(crate) install_dir: PathBuf,
    pub(crate) install_record: InstallationRecord,
    pub(crate) progress_indicator: Option<Progress<'a>>,
}

impl RimDir for UninstallConfiguration<'_> {
    fn install_dir(&self) -> &std::path::Path {
        self.install_dir.as_path()
    }
}

impl RimDir for &UninstallConfiguration<'_> {
    fn install_dir(&self) -> &std::path::Path {
        self.install_dir.as_path()
    }
}

impl<'a> UninstallConfiguration<'a> {
    pub fn init(progress: Option<Progress<'a>>) -> Result<Self> {
        let install_record = InstallationRecord::load_from_install_dir()?;
        Ok(Self {
            install_dir: install_record.root.clone(),
            install_record,
            progress_indicator: progress,
        })
    }

    pub(crate) fn inc_progress(&self, val: f32) -> Result<()> {
        if let Some(prog) = &self.progress_indicator {
            prog.inc(Some(val))?;
        }
        Ok(())
    }

    pub fn uninstall(mut self, remove_self: bool) -> Result<()> {
        // remove all tools.
        info!("{}", t!("uninstalling_third_party_tools"));
        self.remove_tools(installed_tools(&self.install_dir)?, 40.0)?;

        // Remove rust toolchain via rustup.
        if self.install_record.rust.is_some() {
            if let Err(e) = ToolchainInstaller::init().remove_self(&self) {
                // if user has manually uninstall rustup, this will fails,
                // then we can assume it has been removed.
                // TODO: add an error type to indicate `rustup` cannot be found
                warn!("{}: {e}", t!("uninstall_rust_toolchain_failed"));
            }
            self.install_record.remove_rust_record();
            self.install_record.write()?;
        }
        self.inc_progress(40.0)?;

        // remove all env configuration.
        info!("{}", t!("uninstall_env_config"));
        self.remove_rustup_env_vars()?;
        self.inc_progress(10.0)?;

        // remove the manager binary itself or update install record
        if remove_self {
            info!("{}", t!("uninstall_self"));
            self.remove_self()?;
        } else {
            self.install_record.remove_toolkit_meta();
            self.install_record.write()?;
        }
        self.inc_progress(10.0)?;

        Ok(())
    }

    /// Uninstall all tools
    fn remove_tools(&mut self, tools: HashMap<String, ToolRecord>, weight: f32) -> Result<()> {
        let mut tools_to_uninstall = vec![];
        for (name, tool_detail) in &tools {
            let Some(tool) = Tool::from_installed(name, tool_detail) else {
                continue;
            };
            tools_to_uninstall.push(ToolWithDeps {
                tool,
                dependencies: &tool_detail.dependencies,
            });
        }

        if tools_to_uninstall.is_empty() {
            return self.inc_progress(weight);
        }
        let progress_dt = weight / tools_to_uninstall.len() as f32;

        // in previous builds (< 0.6.0), we didn't support dependencies handling,
        // instead, we sorted the tools by its kind. Therefore we use a fallback
        // method to sort the tools here if there's no dependencies info to be found,
        // making sure the tools are always sorted to prevent uninstallation failure.
        let have_deps = tools_to_uninstall
            .iter()
            .any(|t| !t.dependencies.is_empty());

        let sorted = if have_deps {
            tools_to_uninstall.topological_sorted()
        } else {
            tools_to_uninstall.sorted()
        };
        for tool in sorted.iter().rev() {
            info!("{}", t!("uninstalling_for", name = tool.name()));
            if tool.uninstall(&*self).is_err() {
                info!(
                    "{}: {}",
                    t!("uninstall_tool_skipped", tool = tool.name()),
                    t!("maybe_uninstalled_already")
                );
            }
            self.install_record.remove_tool_record(tool.name());
            self.install_record.write()?;
            self.inc_progress(progress_dt)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rim_common::types::ToolKind;

    #[test]
    fn uninstallation_order() {
        let (a_deps, b_deps) = (
            vec!["b".to_string(), "c".to_string()],
            vec!["c".to_string()],
        );
        let tools: Vec<ToolWithDeps> = vec![
            ToolWithDeps {
                tool: Tool::new("b".into(), ToolKind::Custom),
                dependencies: &b_deps,
            },
            ToolWithDeps {
                tool: Tool::new("a".into(), ToolKind::Custom),
                dependencies: &a_deps,
            },
            ToolWithDeps {
                tool: Tool::new("c".into(), ToolKind::Custom),
                dependencies: &[],
            },
        ];
        let mut sorted = tools.topological_sorted();
        assert_eq!(sorted.pop().unwrap().name(), "c");
        assert_eq!(sorted.pop().unwrap().name(), "b");
        assert_eq!(sorted.pop().unwrap().name(), "a");
    }

    #[test]
    fn uninstallation_fallback_order() {
        let tools: Vec<ToolWithDeps> = vec![
            ToolWithDeps {
                tool: Tool::new("b".into(), ToolKind::CargoTool),
                dependencies: &[],
            },
            ToolWithDeps {
                tool: Tool::new("a".into(), ToolKind::DirWithBin),
                dependencies: &[],
            },
            ToolWithDeps {
                tool: Tool::new("c".into(), ToolKind::Plugin),
                dependencies: &[],
            },
        ];
        let mut sorted = tools.sorted();
        assert_eq!(sorted.pop().unwrap().name(), "b");
        assert_eq!(sorted.pop().unwrap().name(), "c");
        assert_eq!(sorted.pop().unwrap().name(), "a");
    }
}
