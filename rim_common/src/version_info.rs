//! Retrieve detailed version information with the help of `git`
//!
//! (Inspired by clippy's [`rustc_tools_utils` crate](https://crates.io/crates/rustc_tools_util))

use std::path::PathBuf;

use crate::{cmd, utils::command_output};

/// Get version string that includes git commit information.
#[macro_export]
macro_rules! get_version_info {
    () => {{
        let version = std::option_env!("TAG_VERSION").unwrap_or(std::env!("CARGO_PKG_VERSION"));
        let commit_hash = std::option_env!("GIT_HASH");
        let commit_date = std::option_env!("COMMIT_DATE");

        if let (Some(hash), Some(date)) = (commit_hash, commit_date) {
            std::format!("{version} ({} {})", hash.trim(), date.trim())
        } else {
            version.to_string()
        }
    }};
}

#[macro_export]
macro_rules! setup_version_info {
    () => {{
        $crate::version_info::rerun_if_git_changes();
        if let Some(hash_) = $crate::version_info::git_commit_hash() {
            println!("cargo:rustc-env=GIT_HASH={hash_}");
        }
        if let Some(date_) = $crate::version_info::git_commit_date() {
            println!("cargo:rustc-env=COMMIT_DATE={date_}");
        }
        if let Some(tag_) = $crate::version_info::git_tag_as_version() {
            println!("cargo:rustc-env=TAG_VERSION={tag_}");
        }
    }};
}

pub fn rerun_if_git_changes() {
    let Ok(git_head_file) =
        command_output(cmd!("git", "rev-parse", "--git-path", "HEAD")).map(PathBuf::from)
    else {
        return;
    };
    if git_head_file.exists() {
        println!("cargo::rerun-if-changed={}", git_head_file.display());
    }

    let Ok(git_head_ref_file) = command_output(cmd!("git", "symbolic-ref", "-q", "HEAD"))
        .and_then(|head_ref| command_output(cmd!("git", "rev-parse", "--git-path", &head_ref)))
        .map(PathBuf::from)
    else {
        return;
    };
    if git_head_ref_file.exists() {
        println!("cargo::rerun-if-changed={}", git_head_ref_file.display());
    }
}

/// Getting git tag name if that exists.
pub fn git_tag_as_version() -> Option<String> {
    let raw = command_output(cmd!("git", "describe", "--exact-match", "--tags")).ok()?;
    // remove the leading `v`
    Some(raw.trim_start_matches('v').into())
}

pub fn git_commit_hash() -> Option<String> {
    command_output(cmd!("git", "rev-parse", "--short", "HEAD")).ok()
}

pub fn git_commit_date() -> Option<String> {
    command_output(cmd!(
        "git",
        "log",
        "-1",
        "--date=short",
        "--pretty=format:%cd"
    ))
    .ok()
}
