//! Module for coding guideline lint checks

use std::env;

use crate::fingerprint::InstallationRecord;
use anyhow::{bail, Context, Result};
use rim_common::{types::ToolKind, utils};

pub(crate) const RUNNER_TOOLCHAIN_NAME: &str = "guidelines_runner";

static LINT_LIST: &[&str] = &[
    "clippy::implicit_abi",
    "clippy::infinite_loop",
    "clippy::unsafe_block_in_proc_macro",
    "clippy::untrusted_lib_loading",
    "clippy::passing_string_to_c_functions",
    "clippy::extern_without_expr",
    "clippy::mem_unsafe_functions",
    "clippy::non_reentrant_functions",
    "clippy::blocking_op_in_async",
    "clippy::fallible_memory_allocation",
    "clippy::null_ptr_dereference",
    "clippy::dangling_ptr_dereference",
    "clippy::return_stack_address",
    "clippy::ptr_double_free",
    "clippy::invalid_char_range",
    "clippy::unconstrained_numeric_literal",
];

pub(crate) fn run(extra_args: &[String]) -> Result<()> {
    // ensure that rustup are installed
    // (why rustup? because we need to call `cargo +TOOLCHAIN` here, which is basically calling
    // rustup that work as a proxy to `cargo`)
    let rustup = exe!("rustup");
    if !utils::cmd_exist(rustup) {
        bail!(t!("no_toolchain_installed"));
    }

    let mut toolchain_override = String::new();
    if env::var_os("SKIP_RULESET_VALIDATION").is_none() {
        if !InstallationRecord::load_from_install_dir()?
            .type_of_tool_is_installed(ToolKind::RuleSet)
        {
            bail!(t!("no_rule_set_installed"));
        }

        let installed_toolchains_output = cmd!(rustup, "toolchain", "list").output()?;
        let installed_toolchains = String::from_utf8(installed_toolchains_output.stdout)?;
        if installed_toolchains.contains(RUNNER_TOOLCHAIN_NAME) {
            toolchain_override = format!("+{RUNNER_TOOLCHAIN_NAME}");
        } else {
            bail!(t!(
                "no_check_runner_installed",
                name = RUNNER_TOOLCHAIN_NAME
            ));
        }
    }

    let prev_rust_flags = env::var("RUSTFLAGS").unwrap_or_default();
    let lint_flags = LINT_LIST
        .iter()
        .fold(String::new(), |acc, e| format!("-W {e} {acc}"));
    let new_rust_flags = format!("{prev_rust_flags} {lint_flags}");

    let cargo = exe!("cargo");
    let mut cmd = cmd!(["RUSTFLAGS" = new_rust_flags.trim()] cargo);
    if !toolchain_override.is_empty() {
        cmd.arg(toolchain_override);
    };
    cmd.arg("clippy");
    if !extra_args.is_empty() {
        cmd.args(extra_args);
    }
    utils::execute_command(cmd, true, false).with_context(|| t!("unable_to_run_check"))?;

    Ok(())
}
