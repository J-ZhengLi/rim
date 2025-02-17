//! Custom install method for `Visual Studio Code`.
//! 
//! Because we are using the archive version instead of the official installer,
//! we need to extract it into the tools directory, set path variable with it,
//! and then create a desktop shortcut. The last part is a bit harder to do,
//! there's currently no suitable solution other than execute some commands to hack it.

use std::path::{Path, PathBuf};
use crate::core::directories::RimDir;
use crate::core::install::InstallConfiguration;
use crate::core::uninstall::UninstallConfiguration;
use crate::{core::os::add_to_path, utils};
use anyhow::Result;

#[derive(Debug)]
pub(crate) struct VSCodeInstaller<'a> {
    /// The command to invoke VSCode, defaulting to `code`.
    pub(crate) cmd: &'a str,
    pub(crate) tool_name: &'a str,
    /// The full verbose name of this VSCode variant, used when creating desktop shortcut.
    pub(crate) verbose_name: &'a str,
    /// The name of the main binary, which is located under the extracted folder,
    /// this is used to create desktop shortcut.
    #[cfg(windows)]
    pub(crate) binary_name: &'a str,
}

impl Default for VSCodeInstaller<'_> {
    fn default() -> Self {
        Self {
            cmd: "code",
            tool_name: "vscode",
            verbose_name: "Visual Studio Code",
            #[cfg(windows)]
            binary_name: "Code"
        }
    }
}

impl<'a> VSCodeInstaller<'a> {
    #[cfg(windows)]
    pub(crate) const fn new(cmd: &'a str, tool_name: &'a str, display_name: &'a str, binary_name: &'a str) -> Self {
        Self {
            cmd, tool_name, verbose_name: display_name, binary_name
        }
    }
    #[cfg(not(windows))]
    pub(crate) const fn new(cmd: &'a str, tool_name: &'a str, display_name: &'a str) -> Self {
        Self {
            cmd, tool_name, verbose_name: display_name,
        }
    }
}

impl VSCodeInstaller<'_> {
    pub(crate) fn install(&self, path: &Path, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
        // Step 1: Move the root of the directory into `tools` directory
        let vscode_dir = config.tools_dir().join(self.tool_name);
        utils::move_to(path, &vscode_dir, false)?;

        // Step 2: Add the `bin/` folder to path
        let bin_dir = vscode_dir.join("bin");
        add_to_path(&bin_dir)?;

        // Step 3: Create a shortcuts
        // Shortcuts are not important, make sure it won't throw error even if it fails.
        #[cfg(windows)]
        {
            // TODO: (?) do we need to create a start menu shortcut as well?
            let shortcut_path = if let Some(dir) = dirs::desktop_dir() {
                dir.join(format!("{}.lnk", self.verbose_name))
            } else {
                warn!(
                    "unable to determine which directory to put shortcut for '{}', skipping...",
                    self.tool_name
                );
                return Ok(vec![vscode_dir]);
            };
            let target_path = vscode_dir.join(format!("{}.exe", self.binary_name));
            let weird_powershell_cmd = format!(
                "$s=(New-Object -COM WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.Save()",
                utils::path_to_str(&shortcut_path)?,
                utils::path_to_str(&target_path)?,
            );
            if utils::run!("powershell", weird_powershell_cmd).is_err() {
                warn!(
                    "unable to create a shortcut for '{}', skipping...",
                    self.tool_name
                );
            }
        }
        #[cfg(unix)]
        {
            // FIXME: There's no icon for this shortcut yet, installing it requires root.
            // Maybe we should install vscode using it's deb/rpm file?
            let desktop_sc = format!(
                "
# Generated by {}
[Desktop Entry]
Name={}
Comment=Code Editing. Redefined.
GenericName=Text Editor
Exec={cmd} %F
Type=Application
StartupNotify=false
StartupWMClass={cmd}
Categories=TextEditor;Development;IDE;
MimeType=application/x-{cmd}-workspace;
Keywords=vscode;
",
                env!("CARGO_PKG_NAME"),
                self.verbose_name,
                cmd = self.cmd,
            );

            let Some(mut path_to_write)  = dirs::data_local_dir().map(|d| d.join("applications")) else {
                warn!(
                    "unable to determine which directory to put shortcut for '{}', skipping...",
                    self.tool_name
                );
                return Ok(vec![vscode_dir]);
            };
            let _ = utils::ensure_dir(&path_to_write);
            path_to_write.push(format!("{}.desktop", self.cmd));
            if utils::write_file(&path_to_write, &desktop_sc, false).is_err() {
                warn!(
                    "unable to create a shortcut for '{}', skipping...",
                    self.tool_name
                );
                return Ok(vec![vscode_dir])
            }
            let _ = utils::set_exec_permission(&path_to_write);
        }

        Ok(vec![vscode_dir])
    }

    pub(crate) fn uninstall(&self, config: &UninstallConfiguration) -> Result<()> {
        use crate::core::os::remove_from_path;

        // We've added a path for VSCode at `<InstallDir>/tools/vscode/bin`, try removing it from `PATH`.
        let mut vscode_path = config.tools_dir().to_path_buf();
        vscode_path.push(self.tool_name);
        vscode_path.push("bin");
        remove_from_path(&vscode_path)?;

        // TODO: Remove desktop shortcut and `%USERPROFILE%/.vscode`.
        // We need to see if the shortcut has the correct target before removing it,
        // and we also need to ask user if they want to remove the user profile
        // before doing so, since that folder might be shared with other vscode varients.
        #[cfg(unix)]
        {
            let Some(filepath)  = dirs::data_local_dir()
                .map(|d| d.join(format!("applications/{}.desktop", self.cmd)))
                .filter(|f| f.is_file())
            else {
                return Ok(());
            };
            if let Ok(content) = utils::read_to_string("program shortcut", &filepath) {
                if content.contains(&format!("# Generated by {}", env!("CARGO_PKG_NAME"))) && utils::remove(&filepath).is_err() {
                    warn!("{}", t!("remove_vscode_shortcut_warn", path = filepath.display()));
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    pub(crate) fn already_installed(&self) -> bool {
        utils::cmd_exist(self.cmd)
    }
}

pub(super) fn install(path: &Path, config: &InstallConfiguration) -> Result<Vec<PathBuf>> {
    VSCodeInstaller::default().install(path, config)
}

pub(super) fn uninstall(config: &UninstallConfiguration) -> Result<()> {
    VSCodeInstaller::default().uninstall(config)
}

pub(super) fn already_installed() -> bool {
    VSCodeInstaller::default().already_installed()
}
