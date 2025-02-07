// Events
pub(crate) const MESSAGE_UPDATE_EVENT: &str = "update-message";
pub(crate) const PROGRESS_UPDATE_EVENT: &str = "update-progress";
pub(crate) const ON_COMPLETE_EVENT: &str = "on-complete";
pub(crate) const ON_FAILED_EVENT: &str = "on-failed";
pub(crate) const BLOCK_EXIT_EVENT: &str = "toggle-exit-blocker";
pub(crate) const LOADING_TEXT: &str = "loading-text";
pub(crate) const LOADING_FINISHED: &str = "loading-finished";
pub(crate) const TOOLKIT_UPDATE_EVENT: &str = "toolkit-update";
// Window related
pub(crate) const MANAGER_WINDOW_LABEL: &str = "manager_window";
pub(crate) const INSTALLER_WINDOW_LABEL: &str = "installer_window";

// The notification window appear to be bigger than normal on Windows,
// It might be due to the fact that windows has different scaling or something else.
#[cfg(unix)]
pub(crate) const NOTIFICATION_WINDOW_WIDTH: f64 = 450.0;
#[cfg(windows)]
pub(crate) const NOTIFICATION_WINDOW_WIDTH: f64 = 360.0;
#[cfg(unix)]
pub(crate) const NOTIFICATION_WINDOW_HEIGHT: f64 = 275.0;
#[cfg(windows)]
pub(crate) const NOTIFICATION_WINDOW_HEIGHT: f64 = 220.0;
