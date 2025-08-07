use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use rim_common::{
    exe,
    utils::{cmd_exist, read_to_string, walk_dir, write_file},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, io::Write, path::PathBuf, process::Command};

#[derive(Debug)]
pub(super) enum LocaleCommand {
    /// Checks for typos, missing keys and mis-order of the keys in the locale files
    Check,
    /// Format the locale files
    Fmt,
}

#[derive(Debug, Deserialize, Serialize)]
struct Locale(IndexMap<String, String>);

#[derive(Debug)]
struct LocaleHelper {
    path: PathBuf,
    locale: Locale,
}

impl TryFrom<PathBuf> for LocaleHelper {
    type Error = anyhow::Error;
    fn try_from(value: PathBuf) -> Result<Self> {
        let raw = read_to_string("locale file", &value)?;
        let locale = serde_json::from_str::<Locale>(&raw)
            .with_context(|| format!("'{}' is not a valid locale file", value.display()))?;
        Ok(Self {
            path: value,
            locale,
        })
    }
}

impl LocaleHelper {
    fn format_locale(&mut self) -> Result<()> {
        self.locale.0.sort_keys();
        let new_content = serde_json::to_string_pretty(&self.locale.0)?;
        write_file(&self.path, &new_content, false)?;
        Ok(())
    }

    fn contains_typos(&self) -> Result<bool> {
        let status = Command::new("typos").arg(&self.path).status()?;
        if status.success() {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn contains_missing_keys(&self, all_keys: &HashSet<&String>) -> bool {
        let keys: HashSet<&String> = self.locale.0.keys().collect();
        let missing_keys = all_keys
            .difference(&keys)
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        if missing_keys.is_empty() {
            false
        } else {
            eprintln!(
                "warning: locale file '{}' contains missing translation: [{}]",
                self.path.display(),
                missing_keys.join(", "),
            );
            true
        }
    }
}

pub(super) fn print_help<T: Write>(mut writer: T) -> Result<()> {
    writeln!(
        writer,
        "Sort or check the locale JSON files

Usage: cargo dev locale [OPTIONS] [COMMAND]

Options:
    -h, -help       Print this help message

Commands:
    c, check        Checks for typos, missing keys and mis-order of the keys in the locale files
    f, fmt          Format the locale files and sort by keys"
    )?;

    Ok(())
}

pub(super) fn run(cmd: LocaleCommand) -> Result<()> {
    let locales_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("locales");
    let locale_files = walk_dir(&locales_dir, true)?;
    let mut locales = vec![];
    for file in locale_files {
        locales.push(LocaleHelper::try_from(file)?);
    }

    match cmd {
        LocaleCommand::Fmt => format_locales(locales)?,
        LocaleCommand::Check => check_locales(locales)?,
    }

    Ok(())
}

fn format_locales(mut locales: Vec<LocaleHelper>) -> Result<()> {
    for helper in &mut locales {
        helper.format_locale()?;
    }
    Ok(())
}

/// Checking locale files for the following things:
/// 1. Missing translation
/// 2. Typos
/// 3. Redundant key (TODO)
fn check_locales(locales: Vec<LocaleHelper>) -> Result<()> {
    ensure_typos_cli()?;

    let all_keys = locales
        .iter()
        .map(|h| h.locale.0.keys().collect::<HashSet<_>>())
        .reduce(|acc, e| acc.iter().copied().chain(e.iter().copied()).collect())
        .unwrap_or_default();

    let mut should_fail = false;

    for helper in &locales {
        // helper.check_typos()?;
        should_fail =
            should_fail || helper.contains_missing_keys(&all_keys) || helper.contains_typos()?;
    }

    if should_fail {
        bail!("locale check fails")
    } else {
        Ok(())
    }
}

fn ensure_typos_cli() -> Result<()> {
    if cmd_exist(exe!("typos")) {
        return Ok(());
    }

    // try installing typos-cli via cargo
    if !Command::new("cargo")
        .args(["install", "typos-cli"])
        .status()?
        .success()
    {
        bail!(
            "unable to install typos-cli, please try manually run `cargo install typos-cli` first"
        )
    } else {
        Ok(())
    }
}
