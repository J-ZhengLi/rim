//! Progress bar indicator for commandline user interface.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use indicatif::{ProgressBar as CliProgressBar, ProgressState, ProgressStyle};

struct ProgressPos(Mutex<f32>);

impl ProgressPos {
    fn new(value: f32) -> Self {
        Self(Mutex::new(value))
    }
    fn load(&self) -> f32 {
        *self.0.lock().unwrap()
    }
    /// Increment position value, and ensure the end result not exceeding 100.
    fn add(&self, value: f32) {
        let mut guard = self.0.lock().unwrap();
        *guard = (*guard + value).min(100.0);
    }
}

#[derive(Clone)]
pub struct Progress<'a> {
    pos: Arc<ProgressPos>,
    pub len: f32,
    pos_callback: &'a dyn Fn(f32) -> Result<()>,
}

impl<'a> Progress<'a> {
    pub fn new<P>(pos_cb: &'a P) -> Self
    where
        P: Fn(f32) -> Result<()>,
    {
        Self {
            pos: Arc::new(ProgressPos::new(0.0)),
            len: 0.0,
            pos_callback: pos_cb,
        }
    }

    pub fn with_len(mut self, len: f32) -> Self {
        self.len = len;
        self
    }

    /// Update the position of progress bar by increment a certain value.
    ///
    /// If a value given is `None`, this will increase the position by the whole `len`,
    /// otherwise it will increase the desired value instead.
    // FIXME: split `inc(None)` to a new function, such as `inc_len`, cuz this is kinda confusing.
    pub fn inc(&self, value: Option<f32>) -> Result<()> {
        let delta = value.unwrap_or(self.len);
        self.pos.add(delta);
        (self.pos_callback)(self.pos.load())?;
        Ok(())
    }
}

/// Convenient struct with methods that are useful to indicate various progress.
#[derive(Debug, Clone, Copy)]
pub struct CliProgress<T: Sized> {
    /// A start/initializing function which will be called to setup progress bar.
    pub start: fn(String, Style) -> Result<T>,
    /// A update function that will be called upon each step completion.
    pub update: fn(&T, Option<u64>),
    /// A function that will be called once to terminate progress.
    pub stop: fn(&T, String),
}

#[derive(Debug, Clone, Copy)]
pub enum Style {
    /// Display the progress base on number of bytes.
    Bytes(u64),
    /// Display the progress base on position & length parameters.
    Len(u64),
    /// A spinner that spins as the progress goes, this does not require
    /// length information.
    Spinner {
        /// Set continuous spinning for a given amount of time.
        auto_tick_duration: Option<Duration>,
    },
}

impl Style {
    fn pattern(&self) -> &str {
        match self {
            Style::Bytes(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            Style::Len(_) => "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            Style::Spinner{..} => "{spinner:.green} [{elapsed_precise}] {msg}"
        }
    }
}

// TODO: Mark this with cfg(feature = "cli")
impl CliProgress<CliProgressBar> {
    /// Create a new progress bar for CLI to indicate download progress.
    ///
    /// When `hidden` is set to `true`, no progress bar will be shown.
    pub fn new(hidden: bool) -> Self {
        fn start(msg: String, style: Style) -> Result<CliProgressBar> {
            let apply_custom_style = |pb: &CliProgressBar, pattern: &str| -> Result<()> {
                pb.set_style(
                    ProgressStyle::with_template(pattern)?
                        .with_key(
                            "eta",
                            |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                                write!(w, "{:.1}s", state.eta().as_secs_f64())
                                    .expect("unable to display progress bar")
                            },
                        )
                        .progress_chars("#>-"),
                );
                Ok(())
            };
            let pb = match style {
                Style::Bytes(total) | Style::Len(total) => CliProgressBar::new(total),
                Style::Spinner { auto_tick_duration } => {
                    let spinner = CliProgressBar::new_spinner();
                    if let Some(dur) = auto_tick_duration {
                        spinner.enable_steady_tick(dur);
                    }
                    spinner
                }
            };
            apply_custom_style(&pb, style.pattern())?;
            pb.set_message(msg);
            Ok(pb)
        }
        fn update(pb: &CliProgressBar, pos: Option<u64>) {
            if let Some(p) = pos {
                pb.set_position(p);
            } else {
                pb.tick();
            }
        }
        fn stop(pb: &CliProgressBar, msg: String) {
            pb.finish_with_message(msg);
        }

        if hidden {
            CliProgress {
                start: |_: String, _: Style| Ok(CliProgressBar::hidden()),
                update: |_: &CliProgressBar, _: Option<u64>| {},
                stop: |_: &CliProgressBar, _: String| {},
            }
        } else {
            CliProgress {
                start,
                update,
                stop,
            }
        }
    }
}

impl Default for CliProgress<CliProgressBar> {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::ProgressPos;

    #[test]
    fn progress_pos_add() {
        let orig = ProgressPos::new(0.0);

        orig.add(1.0);
        assert_eq!(orig.load(), 1.0);
        orig.add(2.0);
        assert_eq!(orig.load(), 3.0);
        orig.add(10.0);
        assert_eq!(orig.load(), 13.0);
    }
}
