use std::sync::{Mutex, OnceLock};

use crate::Result;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_positioner::{Position, WindowExt};

static CONTENT: OnceLock<Mutex<Notification>> = OnceLock::new();
pub(crate) const WINDOW_LABEL: &str = "notification_popup";

#[derive(Debug, Serialize, Clone)]
pub(crate) struct NotificationAction {
    pub(crate) label: String,
    pub(crate) icon: Option<String>,
    /// A tauri command and args (in serialized format) related to this action
    pub(crate) command: (String, Option<String>),
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct Notification {
    title: String,
    content: String,
    actions: Vec<NotificationAction>,
}

impl Notification {
    pub(crate) fn new(title: String, content: String, actions: Vec<NotificationAction>) -> Self {
        let this = Self {
            title,
            content,
            actions,
        };
        if let Some(existing) = CONTENT.get() {
            *existing.lock().unwrap() = this.clone();
        } else {
            CONTENT.set(Mutex::new(this.clone())).unwrap();
        }
        this
    }

    pub(crate) async fn show(self, app_handle: &AppHandle) -> Result<()> {
        if let Some(popup) = app_handle.get_window(WINDOW_LABEL) {
            popup.show()?;
            return Ok(());
        }

        let popup = tauri::WindowBuilder::new(
            app_handle,
            WINDOW_LABEL,
            tauri::WindowUrl::App("notification.html".into()),
        )
        .always_on_top(true)
        .decorations(false)
        .resizable(false)
        .title("notification")
        .skip_taskbar(true)
        .inner_size(360.0, 220.0)
        .build()?;

        popup.move_window(Position::BottomRight)?;
        Ok(())
    }
}

#[tauri::command]
pub(crate) async fn notification_content() -> Option<Notification> {
    Some(CONTENT.get()?.lock().unwrap().clone())
}

#[tauri::command]
pub(crate) async fn close(app: AppHandle) {
    let Some(window) = app.get_window(WINDOW_LABEL) else {
        return;
    };
    crate::common::close_window(&window).await;
}
