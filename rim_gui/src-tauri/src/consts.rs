/// The preferred window width.
pub(crate) const WINDOW_WIDTH: f64 = 960.0;
/// The preferred window height.
pub(crate) const WINDOW_HEIGHT: f64 = WINDOW_WIDTH / 16.0 * 10.0;

// Events
pub(crate) const MESSAGE_UPDATE_EVENT: &str = "update-message";
pub(crate) const ON_COMPLETE_EVENT: &str = "on-complete";
pub(crate) const BLOCK_EXIT_EVENT: &str = "toggle-exit-blocker";
pub(crate) const LOADING_TEXT: &str = "loading-text";
pub(crate) const LOADING_FINISHED: &str = "loading-finished";
pub(crate) const TOOLKIT_UPDATE_EVENT: &str = "toolkit-update";
pub(crate) const MANAGER_UPDATE_EVENT: &str = "manager-update";

// Window labels
// Note: If you add more windows to the app, make sure to allow its permissions by
// inserting a label in `src-tauri/capabilities/migrated.json`.
pub(crate) const MANAGER_WINDOW_LABEL: &str = "manager_window";
pub(crate) const INSTALLER_WINDOW_LABEL: &str = "installer_window";
