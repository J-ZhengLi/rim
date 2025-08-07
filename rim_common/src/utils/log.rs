use anyhow::Result;
use chrono::Local;
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;

use super::file_system::{ensure_dir, parent_dir_of_cur_exe};

static LOGGER_SET: OnceLock<bool> = OnceLock::new();

#[derive(Debug)]
pub struct Logger {
    output_sender: Option<Sender<String>>,
    /// This level only effects displayed log,
    /// the file logger will still be using max log level.
    level: LevelFilter,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        #[cfg(not(debug_assertions))]
        let level = LevelFilter::Info;
        #[cfg(debug_assertions)]
        let level = LevelFilter::Debug;

        Self {
            output_sender: None,
            level,
        }
    }
    /// Set verbose output, this will print `trace!` messages as well.
    pub fn verbose(mut self, v: bool) -> Self {
        if v {
            self.level = LevelFilter::Trace;
        }
        self
    }
    /// Ignore most output, keep only the `error` messages.
    pub fn quiet(mut self, q: bool) -> Self {
        if q {
            self.level = LevelFilter::Error;
        }
        self
    }
    /// Send output using a specific sender rather than printing on `stdout`.
    pub fn sender(mut self, sender: Sender<String>) -> Self {
        self.output_sender = Some(sender);
        self
    }

    /// Setup logger using [`log`] and [`fern`], this must be called first before
    /// any of the `info!`, `warn!`, `trace!`, `debug!`, `error!` macros.
    ///
    /// - If [`verbose`](Logger::verbose) was called with `true`, this will output more
    ///   detailed log messages including `debug!`.
    /// - If [`quiet`](Logger::quiet) was called with `true`, this will not output any message
    ///   on `stdout`, but will still output them into log file.
    pub fn setup(self) -> Result<()> {
        let mut dispatch = fern::Dispatch::new()
            .level(LevelFilter::Trace)
            .level_for("tao", LevelFilter::Error);
        let filter_log_for_output = move |md: &log::Metadata| -> bool {
            md.level() <= self.level && md.level() != LevelFilter::Trace
        };

        // log to standard output (colored info label)
        let stdout = fern::Dispatch::new()
            .filter(filter_log_for_output)
            .format(|out, msg, rec| {
                out.finish(format_args!(
                    "{}: {msg}",
                    ColoredLevelConfig::new()
                        .info(Color::BrightBlue)
                        .debug(Color::Magenta)
                        .color(rec.level())
                        .to_string()
                        .to_lowercase(),
                ));
            })
            .chain(std::io::stdout());
        dispatch = dispatch.chain(stdout);
        // log to file (detailed trace with timestamp)
        let file_config = fern::Dispatch::new()
            .format(|out, msg, rec| {
                out.finish(format_args!(
                    "[{} {} {}] {msg}",
                    Local::now().to_rfc3339(),
                    rec.target(),
                    rec.level(),
                ))
            })
            .chain(fern::log_file(log_file_path()?)?);
        dispatch = dispatch.chain(file_config);
        // log to custom channel if available (regular style)
        if let Some(sender) = self.output_sender {
            let custom = fern::Dispatch::new()
                .filter(filter_log_for_output)
                .format(|out, msg, rec| {
                    out.finish(format_args!(
                        "{}: {msg}",
                        rec.level().to_string().to_lowercase()
                    ));
                })
                .chain(sender);
            dispatch = dispatch.chain(custom);
        }

        if dispatch.apply().is_ok() {
            LOGGER_SET.set(true).unwrap_or_else(|_| {
                unreachable!("logger setup will fail before reaching this point")
            });
        }
        Ok(())
    }
}

static LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();
/// Get the path to log file to write.
///
/// We put the log directory besides current binary, so that it should be easier for users to find them.
/// If for some reason the path to current binary cannot be found, we'll assume the user is running this
/// binary in their current working dir, and create a log dir there.
///
/// Note: the log file might not exists.
///
/// # Error
///
/// Because this will attempt to create a directory named `log` to place the actual log file,
/// this function might fail if it cannot be created.
pub fn log_file_path() -> Result<&'static Path> {
    let mut log_dir = parent_dir_of_cur_exe().unwrap_or(PathBuf::from("."));
    log_dir.push("log");
    ensure_dir(&log_dir)?;

    let bin_name = super::lowercase_program_name().unwrap_or(env!("CARGO_PKG_NAME").to_string());

    Ok(LOG_FILE_PATH
        .get_or_init(|| log_dir.join(format!("{bin_name}-{}.log", Local::now().date_naive()))))
}

/// Return `true` if the logger was already initialized.
pub fn logger_is_set() -> bool {
    LOGGER_SET.get().copied().unwrap_or_default()
}
