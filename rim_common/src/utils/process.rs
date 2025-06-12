use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::{env, io};

use anyhow::{bail, Context, Result};

use super::set_exec_permission;

/// Convenient macro to run a [`Command`].
///
/// # Example
/// ```ignore
/// # use rim::utils::run;
/// run!("echo", "$HOME/.profile");
///
/// let program = "cargo";
/// run!(program, "build", "--release");
///
/// // With env vars
/// run!(["FOO"="foo", "BAR"="bar"] program, "cargo", "build");
/// ```
#[macro_export]
macro_rules! run {
    ($program:expr) => {{
        $crate::utils::execute(std::process::Command::new($program))
    }};
    ($program:expr $(, $arg:expr )* $(,)?) => {{
        $crate::run!([] $program $(,$arg)*)
    }};
    ([$($key:tt = $val:expr),*] $program:expr $(, $arg:expr )* $(,)?) => {{
        let cmd__ = $crate::cmd!([$($key=$val),*] $program $(,$arg)*);
        log::debug!("running command: {cmd__:?}");
        $crate::utils::execute(cmd__)
    }};
}

/// Convenient macro to create a [`Command`], using shell-like command syntax.
///
/// # Example
/// ```ignore
/// # use rim::utils::cmd;
/// cmd!("echo", "$HOME/.profile");
///
/// let program = "cargo";
/// cmd!(program, "build", "--release");
///
/// // With env vars
/// cmd!(["FOO"="foo", "BAR"="bar"] program, "cargo", "build");
/// ```
#[macro_export]
macro_rules! cmd {
    ($program:expr) => {{
        let mut cmd__ = std::process::Command::new($program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Prevent CMD window popup
            cmd__.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }
        cmd__
    }};
    ($program:expr $(, $arg:expr )* $(,)?) => {{
        let mut cmd__ = std::process::Command::new($program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Prevent CMD window popup
            cmd__.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }
        $(cmd__.arg($arg);)*
        cmd__
    }};
    ([$($key:tt = $val:expr),*] $program:expr $(, $arg:expr )* $(,)?) => {{
        let mut cmd__ = std::process::Command::new($program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Prevent CMD window popup
            cmd__.creation_flags(winapi::um::winbase::CREATE_NO_WINDOW);
        }
        $(cmd__.arg($arg);)*
        $(cmd__.env($key, $val);)*
        cmd__
    }};
}

/// Convenient function to execute a command to finish while logging its output.
pub fn execute(cmd: Command) -> Result<()> {
    execute_command(cmd, true, true).map(|_| ())
}

/// Covenient function to execute a command to finish then collect its output.
pub fn command_output(mut cmd: Command) -> Result<String> {
    let inner_ = |cmd: &mut Command| -> Result<String> {
        let output = cmd.output()?;
        if !output.status.success() {
            bail!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    };

    inner_(&mut cmd)
        .with_context(|| format!("failed when executing command: {}", cmd_to_string(cmd)))
}

/// Execute a command.
///
/// - When `expect_success` is `true`,
///   this will return `Ok` only if the command is successfully executed,
///   otherwise this will ignore execution error and return the error code wrapped in `Ok`.
/// - When `log_output` is `true`,
///   this will redirect the command output using [`os_pipe`] and log them using [`log`] interface.
pub fn execute_command(mut cmd: Command, expect_success: bool, log_output: bool) -> Result<i32> {
    // ensure the program can be executed.
    // NB: sometime the program might only be a name not a path,
    // therefore we need to trace back to its path and set exec bit if it has none.
    let program_name = cmd.get_program();
    let Some(program_full_path) = program_path(program_name) else {
        bail!(t!(
            "program_does_not_exits",
            program = program_name.to_string_lossy()
        ));
    };
    set_exec_permission(program_full_path)?;

    let (mut child, cmd_content) = if log_output {
        let (mut reader, stdout) = os_pipe::pipe()?;
        let stderr = stdout.try_clone()?;

        let child = cmd.stdout(stdout).stderr(stderr).spawn()?;

        // NB: to prevent deadlock, `cmd` must be dropped before reading from `reader`
        let cmd_content = cmd_to_string(cmd);
        output_to_log(&mut reader);

        (child, cmd_content)
    } else {
        (cmd.spawn()?, cmd_to_string(cmd))
    };

    let status = child.wait()?;
    let ret_code = get_ret_code(&status);
    if expect_success && !status.success() {
        anyhow::bail!(
            "program exited with code {ret_code}. \n\
            Command: {cmd_content}"
        );
    } else {
        Ok(ret_code)
    }
}

/// Consumes a [`Command`] and turn it into string using debug formatter.
///
/// It is important to call this before reading the output from `os_pipe`,
/// otherwise there will be deadlock. More information can be found in
/// [`os_pipe`'s documentation](https://docs.rs/os_pipe/1.2.1/os_pipe/#common-deadlocks-related-to-pipes)
fn cmd_to_string(cmd: Command) -> String {
    format!("{cmd:?}")
}

fn get_ret_code(status: &ExitStatus) -> i32 {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            // status code can only be `None` on Unix
            status.code().unwrap()
        } else {
            use std::os::unix::process::ExitStatusExt;
            status.into_raw()
        }
    }
}

/// Log the command output
fn output_to_log<R: io::Read>(from: &mut R) {
    let reader = BufReader::new(from);
    for line in reader.lines().map_while(Result::ok) {
        // prevent double 'info|warn|error:' labels, although this might be a dumb way to do it
        if let Some(info) = line.strip_prefix("info: ") {
            info!("{info}");
        } else if let Some(warn) = line.strip_prefix("warn: ") {
            warn!("{warn}");
        } else if let Some(error) = line.strip_prefix("error: ") {
            error!("{error}");
        } else if !line.is_empty() {
            info!("{line}");
        }
    }
}

/// Check if a command/program exist in the `PATH`.
pub fn cmd_exist<S: AsRef<Path>>(cmd: S) -> bool {
    program_path(cmd).is_some()
}

/// Return the full path of a program if it exists in the `PATH` variable
pub fn program_path<P: AsRef<Path>>(program: P) -> Option<PathBuf> {
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|p| p.join(program.as_ref()))
        .find(|p| p.exists())
}
