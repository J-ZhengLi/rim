use std::collections::HashMap;
use std::{env, path::Path};

use crate::core::install::{EnvConfig, InstallConfiguration};
use crate::core::uninstall::{UninstallConfiguration, Uninstallation};
use crate::core::GlobalOpts;
use anyhow::{Context, Result};
use indexmap::IndexSet;
use rim_common::utils;

impl EnvConfig for InstallConfiguration<'_> {
    // On linux, persistent env vars needs to be written in `.profile`, `.bash_profile`, etc.
    // Rustup already did all the dirty work by writing an entry in those files
    // to invoke `$CARGO_HOME/env.{sh|fish}`. Sadly we'll have to re-implement a similar procedure here,
    // because rustup will not write those file if a user has choose to pass `--no-modify-path`.
    // Which is not ideal for env vars such as `RUSTUP_DIST_SERVER`.
    fn config_env_vars(&self) -> Result<()> {
        let vars_raw = self.env_vars()?;

        if !GlobalOpts::get().no_modify_env() {
            info!("{}", t!("install_env_config"));

            let backup_dir = self.install_dir.join("backup");
            utils::ensure_dir(&backup_dir)?;
            for sh in shell::get_available_shells() {
                // This string will be wrapped in a certain identifier comments.
                for rc in sh.update_rcs() {
                    // Do NOT fail installation if backup fails
                    _ = create_backup_for_rc(&rc, &backup_dir);

                    let old_content = utils::read_to_string("rc", &rc).unwrap_or_default();
                    let new_content =
                        rc_content_with_env_vars(sh.as_ref(), &old_content, &vars_raw);

                    utils::write_file(&rc, &new_content, false).with_context(|| {
                        format!(
                            "failed to append environment vars to shell profile: '{}'",
                            rc.display()
                        )
                    })?;
                }
            }
        }

        // Update vars for current process, this is a MUST to ensure this installation
        // can be done correctly.
        for (key, val) in vars_raw {
            env::set_var(key, val);
        }

        self.inc_progress(2.0)
    }
}

/// In case we mess up the user environment
fn create_backup_for_rc(path: &Path, backup_dir: &Path) -> Result<()> {
    // Safe to unwrap as long as the path is one of the `sh.update_rcs()`
    let rc_filename = path.file_name().unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let mut backup_filename = rc_filename.to_os_string();
    backup_filename.push("_");
    backup_filename.push(timestamp.to_string());
    backup_filename.push(".bak");
    let backup_path = backup_dir.join(backup_filename);

    utils::copy_as(path, backup_path)
}

impl Uninstallation for UninstallConfiguration<'_> {
    // This is basically removing the section marked with `rustup config section` in shell profiles.
    fn remove_rustup_env_vars(&self) -> Result<()> {
        if GlobalOpts::get().no_modify_env() {
            return Ok(());
        }
        remove_all_config_section()
    }

    fn remove_self(&self) -> Result<()> {
        // Remove the installer dir.
        std::fs::remove_dir_all(&self.install_dir)?;
        Ok(())
    }
}

fn remove_section_or_warn_<F>(path: &Path, to_remove_sum: &str, operation: F) -> Result<()>
where
    F: FnOnce(String) -> Option<String>,
{
    let content = utils::read_to_string("rc", path)?;
    if operation(content)
        .and_then(|s| utils::write_file(path, &s, false).ok())
        .is_none()
    {
        warn!(
            "{}",
            t!(
                "unix_remove_env_fail_warn",
                path = path.display(),
                val = to_remove_sum
            )
        );
    }
    Ok(())
}

fn remove_all_config_section() -> Result<()> {
    // Remove the profiles content wrapped between `RC_FILE_SECTION_START` to `RC_FILE_SECTION_END`,
    // which is our dedicated configuration sections.
    let start = shell::RC_FILE_SECTION_START;
    let end = shell::RC_FILE_SECTION_END;
    for sh in shell::get_available_shells() {
        for rc in sh.rcfiles().iter().filter(|rc| rc.is_file()) {
            let to_remove_summary = format!("{start}\n...\n{end}");
            remove_section_or_warn_(rc, &to_remove_summary, |cont| {
                remove_sub_string_between(cont, start, end)
            })?;
        }
    }

    Ok(())
}

fn remove_sub_string_between(input: String, start: &str, end: &str) -> Option<String> {
    // TODO: this might not be an optimized solution.
    let start_pos = input.lines().position(|line| line == start)?;
    let end_pos = input.lines().position(|line| line == end)?;
    assert!(
        end_pos >= start_pos,
        "Interal Error: Failed deleting sub string, the start pos is larger than end pos"
    );
    let result = input
        .lines()
        .take(start_pos)
        .chain(input.lines().skip(end_pos + 1))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string();
    Some(result)
}

/// Get the enclosing string between two desired **lines**.
fn get_sub_string_between(input: &str, start: &str, end: &str) -> Option<String> {
    let start_pos = input.lines().position(|line| line == start)?;
    let end_pos = input.lines().position(|line| line == end)?;
    assert!(
        end_pos >= start_pos,
        "Interal Error: Failed extracting sub string, the start pos is larger than end pos"
    );
    let result = input
        .lines()
        .skip(start_pos + 1)
        .take(end_pos - start_pos - 1)
        .collect::<Vec<_>>()
        .join("\n");
    Some(result)
}

fn modify_path(path: &Path, remove: bool) -> Result<()> {
    let path_str = utils::path_to_str(path)?;

    // Apply the new path to current process
    let old_path = env::var_os("PATH").unwrap_or_default();
    let mut splited = env::split_paths(&old_path).collect::<IndexSet<_>>();
    let should_update_current_env = if remove {
        splited.shift_remove(path)
    } else {
        splited.shift_insert(0, path.to_path_buf())
    };
    if should_update_current_env {
        env::set_var("PATH", env::join_paths(splited)?);
    }

    if GlobalOpts::get().no_modify_path() {
        return Ok(());
    }

    // Add the new path to bash profiles
    for sh in shell::get_available_shells() {
        for rc in sh.update_rcs().iter().filter(|rc| rc.is_file()) {
            let rc_content = utils::read_to_string("rc", rc)?;
            let Some(new_content) =
                rc_content_with_path(sh.as_ref(), path_str, &rc_content, remove)
            else {
                let warn = if remove {
                    t!(
                        "unix_remove_path_fail_warn",
                        val = path.display(),
                        rc_path = rc.display()
                    )
                } else {
                    t!(
                        "unix_add_path_fail_warn",
                        val = path.display(),
                        rc_path = rc.display()
                    )
                };
                warn!("{warn}");
                continue;
            };
            utils::write_file(rc, &new_content, false).with_context(|| {
                format!(
                    "failed to append PATH variable to shell profile: '{}'",
                    rc.display()
                )
            })?;
        }
    }

    Ok(())
}

fn rc_content_with_env_vars(
    sh: &dyn shell::UnixShell,
    old_content: &str,
    vars: &HashMap<&'static str, String>,
) -> String {
    // converts env vars such as [(KEY, value), (KEY2, value2)] to ["export KEY='value'"", "export KEY2='value2'"]
    let vars_as_exports = vars.iter().map(|(k, v)| sh.to_env_var_string(k, v));

    if let Some(existing_configs) = get_sub_string_between(
        old_content,
        shell::RC_FILE_SECTION_START,
        shell::RC_FILE_SECTION_END,
    ) {
        // Remove the old env var config
        let mut new_configs = existing_configs
            .lines()
            .filter(|line| !vars.keys().any(|key| line.contains(key)))
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        // push new env var config, even though they have the same value
        new_configs.extend(vars_as_exports);
        old_content.replace(&existing_configs, &new_configs.join("\n"))
    } else {
        let new_config_section = sh.config_section(&vars_as_exports.collect::<Vec<_>>().join("\n"));
        format!("{old_content}\n{new_config_section}")
    }
}

/// Attempt to add path `path_str` to config section, return None if nothing needs to be done.
///
/// i.e.:
///
/// - If there was no config section, create one with `export PATH="{path_str};$PATH"`.
/// - If there was a config section but no `export PATH` line,
///   insert `export PATH="{path_str};$PATH"` at the end of the config section.
/// - If there was a config section and an `export PATH` line with it,
///   push the `path_str` at the start of the `PATH` value, such as `export PATH="{path_str};/old/value;$PATH"`
fn rc_content_with_path(
    sh: &dyn shell::UnixShell,
    path_str: &str,
    old_content: &str,
    remove: bool,
) -> Option<String> {
    if let Some(existing_configs) = get_sub_string_between(
        old_content,
        shell::RC_FILE_SECTION_START,
        shell::RC_FILE_SECTION_END,
    ) {
        // Find the line that is setting path variable
        let maybe_setting_path = existing_configs.lines().find(|line| line.contains("PATH"));

        // Check if the path was already exported.
        if let Some(path_export) = maybe_setting_path {
            if path_export.contains(path_str) && !remove {
                return None;
            }
        }

        let new_content = sh.command_to_update_path(maybe_setting_path, path_str, remove)?;

        let mut new_configs = existing_configs.clone();
        if let Some(setting_path) = maybe_setting_path {
            new_configs = existing_configs.replace(setting_path, &new_content);
        } else {
            new_configs.push('\n');
            new_configs.push_str(&new_content);
        }

        Some(old_content.replace(&existing_configs, &new_configs))
    } else {
        let path_configs = sh.command_to_update_path(None, path_str, false)?;
        let new_config_section = sh.config_section(&path_configs);
        Some(format!("{old_content}\n{new_config_section}"))
    }
}

pub(super) fn add_to_path(path: &Path) -> Result<()> {
    modify_path(path, false)
}

pub(super) fn remove_from_path(path: &Path) -> Result<()> {
    modify_path(path, true)
}

/// Returns a string that looks like `source [rc]` where `[rc]` is a path
/// to any rc file of any available shell in the user mechine.
pub(crate) fn source_command() -> Option<String> {
    let rcs = shell::get_available_shells().next()?.update_rcs();
    let any_rc = rcs.first()?;
    Some(format!("source \"{}\"", any_rc.display()))
}

/// Unix shell module, contains methods that are dedicated in configuring rustup env vars.
// TODO?: Most code in this module are modified from rustup's `shell.rs`, this is not ideal for long term,
// as the file in rustup could change drasically in the future and somehow we'll need to update
// this as well. But as for now, this looks like the only feasible solution.
mod shell {
    // Suggestion of this lint looks worse and doesn't have any improvement.
    #![allow(clippy::collapsible_else_if)]

    use super::utils;
    use anyhow::{bail, Result};
    use std::{env, path::PathBuf};

    type Shell = Box<dyn UnixShell>;

    pub(super) const RC_FILE_SECTION_START: &str = "# ===== rustup config section START =====";
    pub(super) const RC_FILE_SECTION_END: &str = "# ===== rustup config section END =====";

    pub(super) trait UnixShell {
        // Detects if a shell "exists". Users have multiple shells, so an "eager"
        // heuristic should be used, assuming shells exist if any traces do.
        fn does_exist(&self) -> bool;

        // Gives all rcfiles of a given shell that Rustup is concerned with.
        // Used primarily in checking rcfiles for cleanup.
        fn rcfiles(&self) -> Vec<PathBuf>;

        // Gives rcs that should be written to.
        fn update_rcs(&self) -> Vec<PathBuf>;

        /// Format a shell command to set env var.
        fn to_env_var_string(&self, key: &'static str, val: &str) -> String {
            format!("export {key}={val}")
        }

        /// Wraps given content between a pair of identifiers.
        ///
        /// Such identifiers are comments defined as [`RC_FILE_SECTION_START`] and [`RC_FILE_SECTION_END`].
        fn config_section(&self, raw_content: &str) -> String {
            format!(
                "{RC_FILE_SECTION_START}\n\
                {raw_content}\n\
                {RC_FILE_SECTION_END}"
            )
        }

        /// Update the PATH export command, which should be `export PATH="..."` on bash like shells,
        /// and `set -Ux PATH ...` on fish shell.
        ///
        /// If the remove flag is set to `true`, this will attempt to return the `old_command` but without `path_str`.
        fn command_to_update_path(
            &self,
            old_command: Option<&str>,
            path_str: &str,
            remove: bool,
        ) -> Option<String> {
            if let Some(cmd) = old_command {
                let path_str_with_spliter = format!("{path_str}:");
                if remove {
                    Some(cmd.replace(&path_str_with_spliter, ""))
                } else {
                    let where_to_insert = cmd.find('\"')? + 1;
                    let mut new_cmd = cmd.to_string();
                    new_cmd.insert_str(where_to_insert, &path_str_with_spliter);
                    Some(new_cmd)
                }
            } else {
                if remove {
                    None
                } else {
                    Some(self.to_env_var_string("PATH", &format!("\"{path_str}:$PATH\"")))
                }
            }
        }
    }

    pub(super) struct Posix;
    pub(super) struct Bash;
    pub(super) struct Zsh;
    pub(super) struct Fish;

    impl UnixShell for Posix {
        fn does_exist(&self) -> bool {
            true
        }

        fn rcfiles(&self) -> Vec<PathBuf> {
            vec![utils::home_dir().join(".profile")]
        }

        fn update_rcs(&self) -> Vec<PathBuf> {
            // Write to .profile even if it doesn't exist. It's the only rc in the
            // POSIX spec so it should always be set up.
            self.rcfiles()
        }
    }

    impl UnixShell for Bash {
        fn does_exist(&self) -> bool {
            !self.update_rcs().is_empty()
        }

        fn rcfiles(&self) -> Vec<PathBuf> {
            // Bash also may read .profile, however Rustup already includes handling
            // .profile as part of POSIX and always does setup for POSIX shells.
            [".bash_profile", ".bash_login", ".bashrc"]
                .iter()
                .map(|rc| utils::home_dir().join(rc))
                .collect()
        }

        fn update_rcs(&self) -> Vec<PathBuf> {
            self.rcfiles()
                .into_iter()
                .filter(|rc| rc.is_file())
                .collect()
        }
    }

    impl Zsh {
        fn zdotdir() -> Result<PathBuf> {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            if matches!(env::var("SHELL"), Ok(sh) if sh.contains("zsh")) {
                match env::var("ZDOTDIR") {
                    Ok(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
                    _ => bail!("Zsh setup failed."),
                }
            } else {
                match std::process::Command::new("zsh")
                    .args(["-c", "echo -n $ZDOTDIR"])
                    .output()
                {
                    Ok(io) if !io.stdout.is_empty() => {
                        Ok(PathBuf::from(OsStr::from_bytes(&io.stdout)))
                    }
                    _ => bail!("Zsh setup failed."),
                }
            }
        }
    }

    impl UnixShell for Zsh {
        fn does_exist(&self) -> bool {
            // zsh has to either be the shell or be callable for zsh setup.
            matches!(env::var("SHELL"), Ok(sh) if sh.contains("zsh")) || utils::cmd_exist("zsh")
        }

        fn rcfiles(&self) -> Vec<PathBuf> {
            [Zsh::zdotdir().ok(), Some(utils::home_dir())]
                .iter()
                .filter_map(|dir| dir.as_ref().map(|p| p.join(".zshenv")))
                .collect()
        }

        fn update_rcs(&self) -> Vec<PathBuf> {
            // zsh can change $ZDOTDIR both _before_ AND _during_ reading .zshenv,
            // so we: write to $ZDOTDIR/.zshenv if-exists ($ZDOTDIR changes before)
            // OR write to $HOME/.zshenv if it exists (change-during)
            // if neither exist, we create it ourselves, but using the same logic,
            // because we must still respond to whether $ZDOTDIR is set or unset.
            // In any case we only write once.
            self.rcfiles()
                .into_iter()
                .filter(|env| env.is_file())
                .chain(self.rcfiles())
                .take(1)
                .collect()
        }
    }

    impl UnixShell for Fish {
        fn does_exist(&self) -> bool {
            // fish has to either be the shell or be callable for fish setup.
            matches!(env::var("SHELL"), Ok(sh) if sh.contains("fish")) || utils::cmd_exist("fish")
        }

        // > "$XDG_CONFIG_HOME/fish/conf.d" (or "~/.config/fish/conf.d" if that variable is unset) for the user
        // from <https://github.com/fish-shell/fish-shell/issues/3170#issuecomment-228311857>
        fn rcfiles(&self) -> Vec<PathBuf> {
            let mut res = env::var("XDG_CONFIG_HOME")
                .ok()
                .map(|p| vec![PathBuf::from(p).join("fish/conf.d/rustup.fish")])
                .unwrap_or_default();
            res.push(utils::home_dir().join(".config/fish/conf.d/rustup.fish"));

            res
        }

        fn to_env_var_string(&self, key: &'static str, val: &str) -> String {
            format!("set -Ux {key} {val}")
        }

        fn update_rcs(&self) -> Vec<PathBuf> {
            // The first rcfile takes precedence.
            match self.rcfiles().into_iter().next() {
                Some(path) => vec![path],
                None => vec![],
            }
        }

        fn command_to_update_path(
            &self,
            old_command: Option<&str>,
            path_str: &str,
            remove: bool,
        ) -> Option<String> {
            if let Some(cmd) = old_command {
                let path_str_with_spliter = format!("{path_str} ");
                if remove {
                    Some(cmd.replace(&path_str_with_spliter, ""))
                } else {
                    let (before_path, after_path) = cmd.split_once("PATH")?;
                    Some(format!("{before_path}PATH {path_str}{after_path}"))
                }
            } else {
                if remove {
                    None
                } else {
                    Some(self.to_env_var_string("PATH", &format!("{path_str} $PATH")))
                }
            }
        }
    }

    pub(super) fn get_available_shells() -> impl Iterator<Item = Shell> {
        let supported_shells: Vec<Shell> = vec![
            Box::new(Posix),
            Box::new(Bash),
            Box::new(Zsh),
            Box::new(Fish),
        ];

        supported_shells.into_iter().filter(|sh| sh.does_exist())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        rc_content_with_path,
        shell::{self, UnixShell},
    };

    #[test]
    fn remove_labeled_section() {
        let mock_profile = "\
#
# ~/.bash_profile
#

[[ -f ~/.bashrc ]] && . ~/.bashrc

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export RUSTUP_DIST_SERVER='https://example.com'
export RUSTUP_UPDATE_ROOT='https://example.com/rustup'
# ===== rustup config section END =====
. \"$HOME/.cargo/env\"
";

        let new = super::remove_sub_string_between(
            mock_profile.to_string(),
            shell::RC_FILE_SECTION_START,
            shell::RC_FILE_SECTION_END,
        )
        .unwrap();
        assert_eq!(
            new,
            "\
#
# ~/.bash_profile
#

[[ -f ~/.bashrc ]] && . ~/.bashrc

. \"$HOME/.cargo/env\""
        );
    }

    #[test]
    fn labeled_section_at_the_end() {
        let mocked_profile = r#"
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/home/.cargo'
export RUSTUP_HOME='/home/.rustup'
# ===== rustup config section END ====="#;
        let new = super::remove_sub_string_between(
            mocked_profile.to_string(),
            shell::RC_FILE_SECTION_START,
            shell::RC_FILE_SECTION_END,
        )
        .unwrap();
        assert_eq!(
            new,
            r#"
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env""#
        );
    }

    #[test]
    fn estimated_install_dir() {
        let mocked_exe_path = PathBuf::from("/path/to/home/my_app/.cargo/bin/my_app");
        let anc_count = mocked_exe_path.components().count();
        // Components count root dir (/) as well, so there should be 8 components.
        assert_eq!(anc_count, 8);
        let maybe_install_dir: PathBuf = mocked_exe_path.components().take(anc_count - 3).collect();
        assert_eq!(maybe_install_dir, PathBuf::from("/path/to/home/my_app"));
    }

    #[test]
    fn extract_labeled_section() {
        let mock_profile = r#"\
#
# ~/.bash_profile
#

[[ -f ~/.bashrc ]] && . ~/.bashrc

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH="/path/to/bin:$PATH"
# ===== rustup config section END =====
. \"$HOME/.cargo/env\"
"#;

        let wanted = super::get_sub_string_between(
            mock_profile,
            shell::RC_FILE_SECTION_START,
            shell::RC_FILE_SECTION_END,
        )
        .unwrap();
        assert_eq!(
            wanted,
            r#"export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH="/path/to/bin:$PATH""#
        );
    }

    #[test]
    fn insert_path_default() {
        let shell = shell::Bash;
        let path_str = "/path/to/bin";
        let cmd = shell.command_to_update_path(None, path_str, false);

        assert_eq!(cmd, Some("export PATH=\"/path/to/bin:$PATH\"".to_string()));
    }

    #[test]
    fn insert_path_with_old_cmd_default() {
        let shell = shell::Bash;
        let path_str = "/path/to/bin";
        let old_cmd = r#"export PATH="/path/to/tool/bin:$PATH""#;
        let cmd = shell.command_to_update_path(Some(old_cmd), path_str, false);

        assert_eq!(
            cmd,
            Some("export PATH=\"/path/to/bin:/path/to/tool/bin:$PATH\"".to_string())
        );
    }

    #[test]
    fn remove_path_with_no_old_cmd_default() {
        let shell = shell::Bash;
        let path_str = "/path/to/bin";
        let cmd = shell.command_to_update_path(None, path_str, true);

        assert!(cmd.is_none());
    }

    #[test]
    fn remove_path_with_old_cmd_default() {
        let shell = shell::Bash;
        let path_str = "/path/to/bin";
        let old_cmd = r#"export PATH="/path/to/tool/bin:/path/to/bin:$PATH""#;
        let cmd = shell.command_to_update_path(Some(old_cmd), path_str, true);

        assert_eq!(
            cmd,
            Some("export PATH=\"/path/to/tool/bin:$PATH\"".to_string())
        );
    }

    #[test]
    fn remove_path_with_existing_config_section() {
        let existing_rc = r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH=/path/to/other/bin:/path/to/bin:$PATH # Only modify this line
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#;
        let shell = shell::Bash;
        let path_str = "/path/to/bin";
        let new_rc = rc_content_with_path(&shell, path_str, existing_rc, true);

        assert_eq!(
            new_rc.unwrap(),
            r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH=/path/to/other/bin:$PATH # Only modify this line
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#
        );
    }

    #[test]
    fn remove_non_exist_path_with_existing_config_section() {
        let existing_rc = r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH=/path/to/bin:$PATH # Only modify this line
# ===== rustup config section END =====

export PATH=/path/to/bin:$PATH
"#;
        let shell = shell::Bash;
        let path_str = "/path/to/nonexist/bin";
        let new_rc = rc_content_with_path(&shell, path_str, existing_rc, true);

        assert_eq!(new_rc.unwrap(), existing_rc,);
    }

    #[test]
    fn insert_path_fish() {
        let shell = shell::Fish;
        let path_str = "/path/to/bin";
        let cmd = shell.command_to_update_path(None, path_str, false);

        assert_eq!(cmd, Some("set -Ux PATH /path/to/bin $PATH".to_string()));
    }

    #[test]
    fn insert_path_with_old_cmd_fish() {
        let shell = shell::Fish;
        let path_str = "/path/to/bin";
        let old_cmd = "set -Ux PATH /path/to/tool/bin $PATH";
        let cmd = shell.command_to_update_path(Some(old_cmd), path_str, false);

        assert_eq!(
            cmd,
            Some("set -Ux PATH /path/to/bin /path/to/tool/bin $PATH".to_string())
        );
    }

    #[test]
    fn remove_path_with_no_old_cmd_fish() {
        let shell = shell::Fish;
        let path_str = "/path/to/bin";
        let cmd = shell.command_to_update_path(None, path_str, true);

        assert!(cmd.is_none());
    }

    #[test]
    fn remove_path_with_old_cmd_fish() {
        let shell = shell::Fish;
        let path_str = "/path/to/bin";
        let old_cmd = "set -Ux PATH /path/to/tool/bin /path/to/bin $PATH";
        let cmd = shell.command_to_update_path(Some(old_cmd), path_str, true);

        assert_eq!(
            cmd,
            Some("set -Ux PATH /path/to/tool/bin $PATH".to_string())
        );
    }

    #[test]
    fn add_new_path_to_config_section() {
        let existing_rc = r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#;

        let path_to_add = "/path/to/rust/bin";
        let shell = shell::Bash;
        let new_content = rc_content_with_path(&shell, path_to_add, existing_rc, false).unwrap();

        assert_eq!(
            new_content,
            r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH="/path/to/rust/bin:$PATH"
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#
        );
    }

    #[test]
    fn add_path_to_config_section_with_existing_path() {
        let existing_rc = r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH="/path/to/rust/bin:$PATH"
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#;

        let path_to_add = "/path/to/python/bin";
        let another_path_to_add = "/path/to/ruby/bin";
        let shell = shell::Bash;
        let new_content = rc_content_with_path(&shell, path_to_add, existing_rc, false).unwrap();
        let new_content =
            rc_content_with_path(&shell, another_path_to_add, &new_content, false).unwrap();

        assert_eq!(
            new_content,
            r#"\
alias autoremove='sudo pacman -Rcns $(pacman -Qdtq)'
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH="/path/to/ruby/bin:/path/to/python/bin:/path/to/rust/bin:$PATH"
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#
        );
    }
}
