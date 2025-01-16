//! The `update` configuration file, containing information about which version to skip,
//! when the updates are checked, how long until next updates will be checked etc.

use super::{get_installed_dir, TomlParser};
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, time::Duration};

/// Default update check timeout is 1440 minutes (1 day)
const DEFAULT_UPDATE_CHECK_TIMEOUT_IN_MINUTES: u64 = 1440;
/// Default update check timeout in duration
pub const DEFAULT_UPDATE_CHECK_DURATION: Duration =
    Duration::from_secs(60 * DEFAULT_UPDATE_CHECK_TIMEOUT_IN_MINUTES);

// If we ever need to support more things for update checker,
// just add one in this enum, without breaking compatibility.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateTarget {
    Manager,
    Toolkit,
}

// The display implementation must return the same result as
// serde's serialization, which means it should be in 'kebab-case' as well.
impl Display for UpdateTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Manager => "manager",
            Self::Toolkit => "toolkit",
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateConf {
    /// The datetime when the last update check happened,
    /// defaulting to [`UNIX_EPOCH`](NaiveDateTime::UNIX_EPOCH)
    last_run: NaiveDateTime,
    /// Timeout (in minutes) until next update check
    timeout: Option<u64>,
    /// The specific version to disable auto check.
    ///
    /// If there's a newer version available, it will still
    skip: Option<String>,
}

impl Default for UpdateConf {
    fn default() -> Self {
        Self {
            last_run: NaiveDateTime::default(),
            timeout: Some(DEFAULT_UPDATE_CHECK_TIMEOUT_IN_MINUTES),
            skip: None,
        }
    }
}

impl UpdateConf {
    /// Get the timeout in duration until next update check.
    ///
    /// Default is 1 day.
    fn timeout(&self) -> Duration {
        self.timeout
            .map(|timeout_in_minutes| Duration::from_secs(timeout_in_minutes * 60))
            .unwrap_or(DEFAULT_UPDATE_CHECK_DURATION)
    }
}

/// Representing the configuration for update checker.
///
/// Containing information about what version to skip (by the user),
/// how often should we check for next update,
/// and when was the last check happened.
///
/// # Configuration example
///
/// ```toml
/// [manager]
/// last-run = "2024-01-01 10:30:05" # when was the last update check
/// timeout = 1440  # how long (in minutes) until we need to check for update since `last-run`,
/// skip = "0.5.0" # the version the user choose to skip
/// ```
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UpdateCheckerOpt(HashMap<UpdateTarget, UpdateConf>);

impl TomlParser for UpdateCheckerOpt {
    const FILENAME: &'static str = ".updates";
}

impl UpdateCheckerOpt {
    pub fn new() -> Self {
        Self::default()
    }

    fn conf_mut(&mut self, target: UpdateTarget) -> &mut UpdateConf {
        self.0.entry(target).or_default()
    }

    /// Mark a version as skipped.
    ///
    /// This function can be chained.
    pub fn skip<T: Into<String>>(mut self, target: UpdateTarget, version: T) -> Self {
        let conf = self.conf_mut(target);
        conf.skip = Some(version.into());
        self
    }

    /// Return `true` if the given `version` is marked as skipped before.
    pub fn is_skipped<T: AsRef<str>>(&self, target: UpdateTarget, version: T) -> bool {
        let Some(skipped) = self.0.get(&target).and_then(|conf| conf.skip.as_deref()) else {
            return false;
        };
        version.as_ref() == skipped
    }

    /// Change a target's update checkout timeout to a specific number in minutes.
    ///
    /// This function can be chained.
    pub fn remind_later(mut self, target: UpdateTarget, minutes: u64) -> Self {
        let conf = self.conf_mut(target);
        if let Some(t) = conf.timeout.as_mut() {
            *t += minutes
        } else {
            conf.timeout = Some(minutes);
        }
        self
    }

    /// Update the `last-run` value for given target.
    pub fn mark_checked(&mut self, target: UpdateTarget) -> &mut Self {
        let conf = self.conf_mut(target);
        conf.last_run = Utc::now().naive_utc();
        self
    }

    /// Return how much time (in duration) until the next update check.
    ///
    /// - If the update hasn't be checked yet, we should check now,
    ///     thus returning [`Duration::ZERO`].
    /// - If the update has been checked, but right now is not the time for the
    ///     next check, the remaining time will be returned.
    /// - If the update has been checked, and it's already past the time for the next
    ///     update check, then [`Duration::ZERO`] will be returned.
    pub fn duration_until_next_run(&self, target: UpdateTarget) -> Duration {
        let Some(conf) = self.0.get(&target) else {
            // return the full default duratin
            return Duration::ZERO;
        };
        let timeout = conf.timeout();
        let next_check_date = conf.last_run + timeout;
        let now = Utc::now().naive_utc();
        if next_check_date > now {
            let time_delta_in_secs = (next_check_date - now).num_seconds();
            // safe to unwrap, we are converting a known positive i64 to u64
            Duration::from_secs(time_delta_in_secs.try_into().unwrap())
        } else {
            Duration::ZERO
        }
    }

    /// Try loading from installation.
    ///
    /// This guarentee to return a [`VersionSkip`] object,
    /// even if the file does not exists, the default will got returned.
    pub fn load_from_install_dir() -> Self {
        let install_dir = get_installed_dir();
        Self::load_from_dir(install_dir).unwrap_or_default()
    }

    pub fn write_to_install_dir(&self) -> Result<()> {
        let install_dir = get_installed_dir();
        self.write_to_dir(install_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_update() {
        let input = r#"
manager = { skip = "0.1.0", last-run = "1970-01-01T00:00:00" }
toolkit = { skip = "1.0.0", last-run = "1970-01-01T00:00:00" }"#;
        let expected = UpdateCheckerOpt::from_str(input).unwrap();
        assert!(expected.is_skipped(UpdateTarget::Manager, "0.1.0"));
        assert!(expected.is_skipped(UpdateTarget::Toolkit, "1.0.0"));
    }

    #[test]
    fn skip_update_programmatically() {
        let vs = UpdateCheckerOpt::new()
            .skip(UpdateTarget::Manager, "0.1.0")
            .skip(UpdateTarget::Toolkit, "1.0.0");
        assert!(vs.is_skipped(UpdateTarget::Manager, "0.1.0"));
        assert!(vs.is_skipped(UpdateTarget::Toolkit, "1.0.0"));
    }

    #[test]
    fn remind_update_later() {
        let input = r#"
manager = { last-run = "1970-01-01T00:00:00" }"#;

        let mut expected = UpdateCheckerOpt::from_str(input).unwrap();
        let manager = UpdateTarget::Manager;
        assert_eq!(expected.conf_mut(manager).timeout, None);
        expected = expected.remind_later(manager, 60);
        assert_eq!(expected.conf_mut(manager).timeout, Some(60));
        expected = expected.remind_later(manager, 60);
        assert_eq!(expected.conf_mut(manager).timeout, Some(120));
    }
}
