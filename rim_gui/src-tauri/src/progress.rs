//! GUI progress bar module

use rim_common::utils::{ProgressHandler, ProgressStyle};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const MAIN_PROGRESS_START_EVENT: &str = "progress:main-start";
const MAIN_PROGRESS_UPDATE_EVENT: &str = "progress:main-update";
const MAIN_PROGRESS_END_EVENT: &str = "progress:main-end";
const SUB_PROGRESS_START_EVENT: &str = "progress:sub-start";
const SUB_PROGRESS_UPDATE_EVENT: &str = "progress:sub-update";
const SUB_PROGRESS_END_EVENT: &str = "progress:sub-end";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum GuiProgressStyle {
    Fill,
    Spinner,
    Hidden,
}

#[derive(Debug, Clone, Serialize)]
struct ProgressPayload {
    message: String,
    length: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct GuiProgress {
    handle: AppHandle,
    style: GuiProgressStyle,
    length: Option<u64>,
}

impl GuiProgress {
    pub(crate) fn new(handle: AppHandle) -> Self {
        Self {
            handle,
            style: GuiProgressStyle::Hidden,
            length: None,
        }
    }
}

impl ProgressHandler for GuiProgress {
    fn set_style(&mut self, style: ProgressStyle) -> anyhow::Result<()> {
        match style {
            ProgressStyle::Bytes(len) | ProgressStyle::Len(len) => {
                self.length = Some(len);
                self.style = GuiProgressStyle::Fill;
            }
            ProgressStyle::Spinner { .. } => {
                self.length = None;
                self.style = GuiProgressStyle::Spinner;
            }
            ProgressStyle::Hidden => {
                self.length = None;
                self.style = GuiProgressStyle::Hidden;
            }
        }

        Ok(())
    }

    fn start(&self, msg: String) -> anyhow::Result<()> {
        self.handle.emit_all(SUB_PROGRESS_START_EVENT, msg)?;
        Ok(())
    }

    fn update(&self, value: Option<u64>) -> anyhow::Result<()> {
        self.handle.emit_all(SUB_PROGRESS_UPDATE_EVENT, value)?;
        Ok(())
    }

    fn finish(&self, msg: String) -> anyhow::Result<()> {
        self.handle.emit_all(SUB_PROGRESS_END_EVENT, msg)?;
        Ok(())
    }

    fn start_master(&mut self, msg: String, length: Option<u64>) -> anyhow::Result<()> {
        self.length = length;

        let payload = ProgressPayload {
            message: msg,
            length,
        };

        self.handle.emit_all(MAIN_PROGRESS_START_EVENT, payload)?;
        Ok(())
    }

    fn update_master(&self, value: Option<u64>) -> anyhow::Result<()> {
        self.handle.emit_all(MAIN_PROGRESS_UPDATE_EVENT, value)?;
        Ok(())
    }

    fn finish_master(&self, msg: String) -> anyhow::Result<()> {
        self.handle.emit_all(MAIN_PROGRESS_END_EVENT, msg)?;
        Ok(())
    }
}
