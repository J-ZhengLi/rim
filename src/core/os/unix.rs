use crate::core::directories::RimDir;
use crate::core::install::{EnvConfig, InstallConfiguration};
use crate::core::uninstall::{UninstallConfiguration, Uninstallation};
use crate::core::GlobalOpts;
use anyhow::{Context, Result};
use indexmap::IndexSet;
use rim_common::utils;
use std::path::PathBuf;
use std::{env, path::Path};

impl EnvConfig for InstallConfiguration<'_> {
    // On linux, persistent env vars needs to be written in `.profile`, `.bash_profile`, etc.
    // Rustup already did all the dirty work by writing an entry in those files
    // to invoke `$CARGO_HOME/env.{sh|fish}`. Sadly we'll have to re-implement a similar procedure here,
    // because we need to support additional env vars such as `RUSTUP_DIST_SERVER`, also paths
    // for third-party tools.
    fn config_env_vars(&self) -> Result<()> {
        let vars_raw = self.env_vars()?;

        info!("{}", t!("install_env_config"));
        for sh in shell::get_available_shells() {
            // first, modify the env script (CARGO_HOME/env)
            let script = sh.env_script();
            let script_path = self.install_dir.join(script.name);
            let mut env_content = if script_path.is_file() {
                utils::read_to_string("env script", &script_path)?
                    .trim_end()
                    .to_string()
            } else {
                script.content.to_string()
            };
            for (key, val) in &vars_raw {
                update_content(&mut env_content, &sh.export_string(key, val), false);
            }
            utils::write_file(&script_path, &env_content, false)?;

            // secondly, insert a source command to rc files if needed
            if GlobalOpts::get().no_modify_env() {
                info!("{}", t!("skip_env_modification"));
            } else {
                let rcs = sh.update_rcs();
                create_rc_backup(&rcs, self.backup_dir())?;
                ensure_env_config_in_rcs(self, &sh, rcs.iter())?;
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
fn create_rc_backup(rc_files: &[PathBuf], backup_dir: &Path) -> Result<()> {
    let rc_bak_dir = backup_dir.join("HOME");
    utils::ensure_dir(&rc_bak_dir)?;

    for path in rc_files {
        if !path.is_file() {
            continue;
        }

        // Safe to unwrap as long as the path is one of the `sh.update_rcs()`
        let rc_filename = path.file_name().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let mut backup_filename = rc_filename.to_os_string();
        backup_filename.push(format!("_{timestamp}"));
        let backup_path = rc_bak_dir.join(backup_filename);

        utils::copy_as(path, backup_path)?;
    }

    Ok(())
}

impl Uninstallation for UninstallConfiguration<'_> {
    // This is basically removing the env script source command in shell profiles.
    fn remove_rustup_env_vars(&self) -> Result<()> {
        if GlobalOpts::get().no_modify_env() {
            info!("{}", t!("skip_env_modification"));
            return Ok(());
        }

        for sh in shell::get_available_shells() {
            let env_script_path = self.install_dir.join(sh.env_script().name);
            let source_command = sh.source_string(utils::path_to_str(&env_script_path)?);
            for rc in sh.rcfiles().iter().filter(|f| f.is_file()) {
                let mut content = utils::read_to_string("rc file", rc)?;
                remove_legacy_config_section(&mut content);
                if update_content(&mut content, &source_command, true) {
                    utils::write_file(rc, &content, false)?;
                }
            }
        }

        Ok(())
    }

    fn remove_self(&self) -> Result<()> {
        // Remove the installer dir.
        std::fs::remove_dir_all(&self.install_dir)?;
        Ok(())
    }
}

/// Remove the legacy config section in each
// NB(J-Zheng): In early builds, I wrote env configuration in rc files that wrapped between certain
// identifier comments, which is kinda dumb I gotta admit. It works fine at the start, since not
// many lines need to be added, but then as we support more and more tools, more path and env vars
// need to be added, that's when I realized I need to go for the rustup approach, which is
// writing a master env script, and invoke that script in each rc file only. (smh)
fn remove_legacy_config_section(content: &mut String) {
    let start = "# ===== rustup config section START =====";
    let end = "# ===== rustup config section END =====";

    let Some(start_pos) = content.lines().position(|line| line == start) else {
        return;
    };
    let Some(end_pos) = content.lines().position(|line| line == end) else {
        return;
    };
    assert!(
        end_pos >= start_pos,
        "Interal Error: Failed deleting sub string, the start pos is larger than end pos"
    );
    *content = content
        .lines()
        .take(start_pos)
        .chain(content.lines().skip(end_pos + 1))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string();
}

fn modify_path<T: RimDir + Copy>(config: T, path: &Path, remove: bool) -> Result<()> {
    let path_str = utils::path_to_str(path)?;

    // Apply the new path to current process
    let old_path = env::var_os("PATH").unwrap_or_default();
    let mut splited = env::split_paths(&old_path).collect::<IndexSet<_>>();
    let should_update_current_env = if remove {
        splited.shift_remove(path)
    } else {
        splited.shift_insert(0, path.to_path_buf());
        // shift_insert return false even it modify the order of the
        // existing value, therefore we should modify for current process
        // as well, just to be sure.
        true
    };
    if should_update_current_env {
        env::set_var("PATH", env::join_paths(splited)?);
    }

    // modifying the env shell script by modifying and replacing the `env` script
    // in install dir.
    for sh in shell::get_available_shells() {
        let script = sh.env_script();
        let script_path = config.install_dir().join(script.name);
        let mut env_content = if script_path.is_file() {
            utils::read_to_string("env script", &script_path)?
                .trim_end()
                .to_string()
        } else {
            if remove {
                continue;
            }
            script.content.to_string()
        };

        if update_content(&mut env_content, &sh.add_path_string(path_str), remove) {
            utils::write_file(&script_path, &env_content, false).with_context(|| {
                format!(
                    "failed to modify PATH variable in env script: '{}'",
                    script_path.display()
                )
            })?;
        }

        if !GlobalOpts::get().no_modify_path() {
            ensure_env_config_in_rcs(
                config,
                &sh,
                sh.update_rcs().iter().filter(|rc| rc.is_file()),
            )?;
        }
    }

    Ok(())
}

/// Ensure the given rc files contain a command that sourcing our env script.
fn ensure_env_config_in_rcs<'a, I, T>(config: T, sh: &shell::Shell, rc_files: I) -> Result<()>
where
    I: Iterator<Item = &'a PathBuf>,
    T: RimDir,
{
    let script_path = config.install_dir().join(sh.env_script().name);
    let source_cmd = sh.source_string(utils::path_to_str(&script_path)?);
    for rc in rc_files {
        let mut rc_content = utils::read_to_string("rc", rc).unwrap_or_default();
        remove_legacy_config_section(&mut rc_content);
        if update_content(&mut rc_content, &source_cmd, false) {
            utils::write_file(rc, &rc_content, false).with_context(|| {
                format!(
                    "failed to append environment vars to shell profile: '{}'",
                    rc.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Update content by inserting or removing a line, do nothing if:
///
/// 1. `remove` set to true but there is nothing to remove.
/// 2. `remove` is false, meaning the `line` need to be added
///    but the `line` was already in the `content`.
///
/// Return `true` if the content was updated.
fn update_content(content: &mut String, line: &str, remove: bool) -> bool {
    let line_exists = content.lines().any(|ln| ln == line);
    match (remove, line_exists) {
        (true, false) | (false, true) => return false,
        (true, true) => {
            debug!("removing line '{line}' from the env script");
            // remove existing content
            *content = content
                .lines()
                .filter(|ln| *ln != line)
                .collect::<Vec<_>>()
                .join("\n")
                .trim_end()
                .to_string();
        }
        (false, false) => {
            // add new content at the end
            content.push('\n');
            content.push_str(line);
        }
    }
    true
}

pub(super) fn add_to_path<T: RimDir + Copy>(config: T, path: &Path) -> Result<()> {
    modify_path(config, path, false)
}

pub(super) fn remove_from_path<T: RimDir + Copy>(config: T, path: &Path) -> Result<()> {
    modify_path(config, path, true)
}

/// Returns a string that looks like `source [rc]` where `[rc]` is a path
/// to any rc file of any available shell in the user mechine.
pub(crate) fn env_script_path(install_dir: &Path) -> Option<PathBuf> {
    let sh = shell::get_available_shells().next()?;
    Some(install_dir.join(sh.env_script().name))
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

    pub(super) type Shell = Box<dyn UnixShell>;

    pub(super) struct ShellScript {
        pub(super) name: &'static str,
        pub(super) content: &'static str,
    }

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
        fn export_string(&self, key: &'static str, val: &str) -> String {
            format!("export {key}=\"{val}\"")
        }

        /// Return the string for sourcing current shell env.
        fn source_string(&self, path_to_env: &str) -> String {
            format!(". \"{path_to_env}\"")
        }

        /// Return the string to add a new path to shell's PATH env.
        fn add_path_string(&self, path_to_add: &str) -> String {
            // NOTE: each shell template file has a function `add_to_path` pre-defined,
            // make sure the name matches in the below line, otherwise the source cmd will fail.
            format!("add_to_path \"{path_to_add}\"")
        }

        fn env_script(&self) -> ShellScript {
            ShellScript {
                name: "env",
                content: include_str!("../../../resources/templates/env.sh"),
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

        fn export_string(&self, key: &'static str, val: &str) -> String {
            format!("set -Ux {key} \"{val}\"")
        }

        fn update_rcs(&self) -> Vec<PathBuf> {
            // The first rcfile takes precedence.
            match self.rcfiles().into_iter().next() {
                Some(path) => vec![path],
                None => vec![],
            }
        }

        fn env_script(&self) -> ShellScript {
            ShellScript {
                name: "env.fish",
                content: "../../../resources/templates/env.fish",
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
    use super::*;
    use crate::core::os::unix::shell::UnixShell;
    use std::path::PathBuf;

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
    fn insert_path_default() {
        let shell = shell::Bash;
        let mut orig = "...".to_string();
        update_content(&mut orig, &shell.add_path_string("/path/to/foo"), false);
        update_content(&mut orig, &shell.add_path_string("/path/to/bar"), false);
        update_content(&mut orig, &shell.add_path_string("/path/to/baz"), false);

        assert_eq!(
            orig,
            r#"...
add_to_path "/path/to/foo"
add_to_path "/path/to/bar"
add_to_path "/path/to/baz""#
        );
    }

    #[test]
    fn remove_path_default() {
        let shell = shell::Bash;
        let mut orig = r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo""#
            .to_string();
        update_content(&mut orig, &shell.add_path_string("/path/to/foo"), true);
        update_content(&mut orig, &shell.add_path_string("/path/to/bar"), true);
        update_content(&mut orig, &shell.add_path_string("/path/to/baz"), true);

        assert_eq!(
            orig,
            r#"
add_to_path "/path/to/bin""#
        );
    }

    #[test]
    fn test_remove_legacy_config_section() {
        let mut existing_rc = r#"\
. "$HOME/.cargo/env"

# ===== rustup config section START =====
export CARGO_HOME='/path/to/cargo'
export RUSTUP_HOME='/path/to/rustup'
export PATH=/path/to/other/bin:/path/to/bin:$PATH # Only modify this line
# ===== rustup config section END =====

export PATH=/some/user/defined/bin:$PATH
"#
        .to_string();

        remove_legacy_config_section(&mut existing_rc);

        assert_eq!(
            existing_rc,
            r#"\
. "$HOME/.cargo/env"


export PATH=/some/user/defined/bin:$PATH"#
        );
    }

    #[test]
    fn add_new_env_var_bash() {
        let shell = shell::Bash;
        let mut orig = r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo""#
            .to_string();
        update_content(&mut orig, &shell.export_string("FOO", "1"), false);
        update_content(&mut orig, &shell.export_string("BAR", "2"), false);

        assert_eq!(
            orig,
            r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo"
export FOO="1"
export BAR="2""#
        );
    }

    #[test]
    fn add_and_remove_env_var_fish() {
        let shell = shell::Fish;
        let mut orig = r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo""#
            .to_string();
        update_content(&mut orig, &shell.export_string("FOO", "1"), false);
        update_content(&mut orig, &shell.export_string("BAR", "2"), false);

        assert_eq!(
            orig,
            r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo"
set -Ux FOO "1"
set -Ux BAR "2""#
        );

        update_content(&mut orig, &shell.export_string("FOO", "1"), true);

        assert_eq!(
            orig,
            r#"
add_to_path "/path/to/bin"
add_to_path "/path/to/foo"
set -Ux BAR "2""#
        );
    }
}
