use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Wrapper to [`std::fs::read_to_string`] but with additional error context.
pub fn read_to_string<P: AsRef<Path>>(name: &str, path: P) -> Result<String> {
    fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "failed to read {name} file at given location: '{}'",
            path.as_ref().display()
        )
    })
}

pub fn stringify_path<P: AsRef<Path>>(path: P) -> Result<String> {
    path.as_ref()
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            anyhow!(
                "failed to stringify path '{}'",
                path.as_ref().to_string_lossy().to_string()
            )
        })
}

pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if !path.as_ref().is_dir() {
        fs::create_dir_all(path.as_ref()).with_context(|| {
            format!(
                "unable to create specified directory '{}'",
                path.as_ref().display()
            )
        })?;
    }
    Ok(())
}

pub fn ensure_parent_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if let Some(p) = path.as_ref().parent() {
        ensure_dir(p)?;
    }
    Ok(())
}

/// Convert the given path to absolute path without `.` or `..` components.
///
/// - If the `path` is already an absolute path, this will just go through each component
///   and attempt to "remove" `.` and `..` components.
/// - If the `root` is not specified, this will assume that `path` is related to current directory.
///
/// # Error
/// If the `root` is not given, and the current directory cannot be determined, an error will be returned.
pub fn to_normalized_absolute_path<P: AsRef<Path>>(
    path: P,
    root: Option<&Path>,
) -> Result<PathBuf> {
    let abs_pathbuf = if path.as_ref().is_absolute() {
        path.as_ref().to_path_buf()
    } else {
        root.map(|p| Ok(p.to_path_buf()))
            .unwrap_or_else(|| env::current_dir().context("current directory cannot be determined"))
            .map(|mut cd| {
                cd.push(path);
                cd
            })?
    };
    // Remove any `.` and `..` from origin path
    let mut normalized_path = PathBuf::new();
    for path_component in abs_pathbuf.components() {
        match path_component {
            Component::CurDir => (),
            Component::ParentDir => {
                normalized_path.pop();
            }
            _ => normalized_path.push(path_component),
        }
    }

    Ok(normalized_path)
}

pub fn write_file<P: AsRef<Path>>(path: P, content: &str, append: bool) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    if append {
        options.append(true);
    } else {
        options.truncate(true).write(true);
    }
    let mut file = options.create(true).open(path)?;
    writeln!(file, "{content}")?;
    file.sync_data()?;
    Ok(())
}

pub fn write_bytes<P: AsRef<Path>>(path: P, content: &[u8], append: bool) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    if append {
        options.append(true);
    } else {
        options.truncate(true).write(true);
    }
    let mut file = options.create(true).open(path)?;
    file.write_all(content)?;
    file.sync_data()?;
    Ok(())
}

/// An [`fs::copy`] wrapper that only copies a file if:
///
/// - `to` does not exist yet.
/// - `to` exists but have different modified date.
///
/// Will attempt to create parent directory if not exists.
pub fn copy_file<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    // Make sure no redundant work is done
    if let (Ok(src_modify_time), Ok(dest_modify_time)) = (
        fs::metadata(&from).and_then(|m| m.modified()),
        fs::metadata(&to).and_then(|m| m.modified()),
    ) {
        if src_modify_time == dest_modify_time {
            return Ok(());
        }
    }

    ensure_parent_dir(&to)?;
    fs::copy(&from, &to).with_context(|| {
        format!(
            "could not copy file '{}' to '{}'",
            from.as_ref().display(),
            to.as_ref().display()
        )
    })?;
    Ok(())
}

/// Copy file or directory into a directory, and return the full path after copying.
pub fn copy_into<P, Q>(from: P, to: Q) -> Result<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let dest = to.as_ref().join(from.as_ref().file_name().ok_or_else(|| {
        anyhow!(
            "path '{}' does not have a file name",
            from.as_ref().display()
        )
    })?);

    copy_as(from, &dest)?;
    Ok(dest)
}

/// Copy file or directory to a specified path.
pub fn copy_as<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fn copy_dir_(src: &Path, dest: &Path) -> Result<()> {
        ensure_dir(dest)?;
        for maybe_entry in src.read_dir()? {
            let entry = maybe_entry?;
            let src = entry.path();
            let dest = dest.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_(&src, &dest)?;
            } else {
                copy_file(&src, &dest)?;
            }
        }
        Ok(())
    }

    if !from.as_ref().exists() {
        bail!(
            "failed to copy '{}': path does not exist",
            from.as_ref().display()
        );
    }

    if from.as_ref().is_file() {
        copy_file(from, to)?;
    } else {
        copy_dir_(from.as_ref(), to.as_ref()).with_context(|| {
            format!(
                "could not copy directory '{}' to '{}'",
                from.as_ref().display(),
                to.as_ref().display()
            )
        })?;
    }
    Ok(())
}

/// Set file permissions (executable)
/// rwxr-xr-x: 0o755
#[cfg(not(windows))]
pub fn set_exec_permission<P: AsRef<Path>>(path: P) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(windows)]
pub fn set_exec_permission<P: AsRef<Path>>(_path: P) -> Result<()> {
    Ok(())
}

/// Attempts to read a directory path, then return a list of paths
/// that are inside the given directory, may or may not including sub folders.
pub fn walk_dir(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    fn collect_paths_(dir: &Path, paths: &mut Vec<PathBuf>, recursive: bool) -> Result<()> {
        for dir_entry in dir.read_dir()?.flatten() {
            paths.push(dir_entry.path());
            if recursive && matches!(dir_entry.file_type(), Ok(ty) if ty.is_dir()) {
                collect_paths_(&dir_entry.path(), paths, true)?;
            }
        }
        Ok(())
    }
    let mut paths = vec![];
    collect_paths_(dir, &mut paths, recursive)?;
    Ok(paths)
}

pub fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    #[cfg(windows)]
    let is_executable_ext = matches!(
        path.as_ref().extension().and_then(|ext| ext.to_str()),
        Some("exe")
    );
    #[cfg(not(windows))]
    let is_executable_ext = path.as_ref().extension().is_none();

    path.as_ref().is_file() && is_executable_ext
}

/// Delete a file or directory (recursively) from disk.
pub fn remove<P: AsRef<Path>>(src: P) -> Result<()> {
    if !src.as_ref().exists() {
        return Ok(());
    } else if src.as_ref().is_dir() {
        fs::remove_dir_all(&src)
            .with_context(|| format!("unable to remove directory '{}'", src.as_ref().display()))?;
    } else {
        fs::remove_file(&src)
            .with_context(|| format!("unable to remove file '{}'", src.as_ref().display()))?;
    }
    Ok(())
}

/// Move `src` path to `dest`.
pub fn move_to(src: &Path, dest: &Path, force: bool) -> Result<()> {
    if force && dest.exists() {
        remove(dest)?;
    }

    const RETRY_TIMES: u8 = 10;
    for _ in 0..RETRY_TIMES {
        match fs::rename(src, dest) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                warn!("{}", t!("remove_path_retry", path = src.display()));
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    }
    // If removing still doesn't work, likely because of some weird problem
    // caused by anti-virus software, try copy and delete instead.
    // And report error if the original path cannot be deleted.
    copy_as(src, dest)?;
    if remove(src).is_err() {
        warn!("{}", t!("remove_path_fail_warn", path = src.display()));
    }

    Ok(())
}

/// Get the parent directory of current executable.
///
/// # Error
/// This will fail if the path to current executable cannot be determined under some rare condition.
pub fn parent_dir_of_cur_exe() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("unable to locate current executable")?;
    let maybe_install_dir = exe_path
        .parent()
        .unwrap_or_else(|| unreachable!("executable should always have a parent directory"))
        .to_path_buf();
    Ok(maybe_install_dir)
}

/// Create temporary file with or without specific directory as root.
pub fn make_temp_file(prefix: &str, root: Option<&Path>) -> Result<NamedTempFile> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix);

    if let Some(r) = root {
        builder
            .tempfile_in(r)
            .with_context(|| format!("unable to create temporary file under {}", r.display()))
    } else {
        builder
            .tempfile()
            .context("unable to create temporary file")
    }
}

/// Try getting the extension of a `path` as `str`.
pub fn extension_str(path: &Path) -> Option<&str> {
    path.extension().and_then(|ext| ext.to_str())
}

/// Creates a new link on the filesystem.
///
/// If the link already exists, it will simply get updated.
///
/// This function will attempt to create a symbolic link at first,
/// and will fallback to create hard-link if that fails.
///
/// # Error
/// Return error if
/// 1. The link exists and cannot be removed.
/// 2. [`fs::hard_link`] failes, meaning that the `original` is likely a
///    directory or doesn't exists at all.
pub fn create_link<P, Q>(original: P, link: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    remove(&link)?;

    let create_sym_link = || -> Result<()> {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                Ok(std::os::unix::fs::symlink(&original, &link)?)
            } else if #[cfg(windows)] {
                if original.as_ref().is_dir() {
                    Ok(std::os::windows::fs::symlink_dir(&original, &link)?)
                } else {
                    Ok(std::os::windows::fs::symlink_file(&original, &link)?)
                }
            } else {
                bail!("not supported, use hard-link directly");
            }
        }
    };

    if create_sym_link().is_err() {
        debug!("unable to create symbolic link, creating hard link instead");
        fs::hard_link(original, link).context("unable to create hard link")?;
    }
    Ok(())
}

/// Checks if `maybe_link` is a (symbolic or hard) link to the `source`.
pub fn is_link_of<P, Q>(maybe_link: P, source: Q) -> Result<bool>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if maybe_link.as_ref() == source.as_ref() {
        // same file, return false
        Ok(false)
    } else if maybe_link
        .as_ref()
        .read_link()
        .map(|p| {
            if p.is_relative() {
                let p_abs = maybe_link.as_ref().with_file_name(p).canonicalize().ok();
                let s_abs = source.as_ref().canonicalize().ok();
                p_abs.is_some() && p_abs == s_abs
            } else {
                p == source.as_ref()
            }
        })
        .unwrap_or_default()
    {
        // sym link to source, return true
        Ok(true)
    } else {
        // check if the two files are hard-linked, which means that they have same file id,
        // but not in the exact same path.
        let maybe_link_fid = file_id::get_file_id(maybe_link)?;
        let source_fid = file_id::get_file_id(source)?;

        Ok(maybe_link_fid == source_fid)
    }
}

#[derive(Debug, Default)]
#[allow(unused, reason = "Most of fields only work on Linux")]
/// Helper struct to create/delete desktop shortcut for any application.
pub struct ApplicationShortcut<'a> {
    /// Name of the shortcut.
    pub name: &'a str,
    /// Path of the application to create shortcut for.
    pub path: PathBuf,
    /// (Linux Only) Path to the shortcut's icon
    pub icon: Option<PathBuf>,
    /// (Linux Only) Application description.
    pub comment: Option<&'a str>,
    /// (Linux Only) Application category description.
    pub generic_name: Option<&'a str>,
    /// (Linux Only) Field code for execute command, such as `%f`, `%F`, `%u`, `%U` etc.
    pub field_code: Option<&'a str>,
    /// (Linux Only) whether or not to enable startup feedback, such as showing
    /// `busy` cursor during launch
    pub startup_notify: bool,
    /// (Linux Only) Class for multiple window management, used to group windows on taskbar.
    pub startup_wm_class: Option<&'a str>,
    /// (Linux Only) Menu categorization hierarchy, follows freedesktop.org
    /// [`standards`](https://specifications.freedesktop.org/menu-spec/latest/category-registry.html)
    pub categories: &'a [&'a str],
    /// (Linux Only) for file association, such as `text/html`, `x-scheme-handler/https` etc.
    pub mime_type: &'a [&'a str],
    /// (Linux Only) A list of keywords that can be used to search this application.
    pub keywords: &'a [&'a str],
}

impl ApplicationShortcut<'_> {
    /// Creating desktop shortcut for a specific application.
    ///
    /// # Platform Specific Behavior
    /// - On Windows, this will just execute a powershell command to add shortcut for us.
    /// - On Linux, this writes two entries with `.desktop` extension to user `applications` folder
    ///   and `Desktop` folder.
    pub fn create(&self) -> Result<()> {
        let desktop_dir =
            dirs::desktop_dir().unwrap_or_else(|| crate::dirs::home_dir().join("Desktop"));
        #[cfg(windows)]
        let application_dir: Option<PathBuf> = None;
        #[cfg(target_os = "linux")]
        let application_dir: Option<PathBuf> =
            dirs::data_local_dir().map(|dld| dld.join("applications"));

        self.create_shortcut_(&desktop_dir, application_dir.as_deref())
    }

    /// Remove desktop shortcut
    pub fn remove(&self) -> Result<()> {
        let desktop_dir =
            dirs::desktop_dir().unwrap_or_else(|| crate::dirs::home_dir().join("Desktop"));
        #[cfg(windows)]
        let application_dir: Option<PathBuf> = None;
        #[cfg(target_os = "linux")]
        let application_dir: Option<PathBuf> =
            dirs::data_local_dir().map(|dld| dld.join("applications"));

        self.remove_shortcut_(&desktop_dir, application_dir.as_deref())
    }

    #[cfg(windows)]
    fn create_shortcut_(&self, desktop_dir: &Path, _application_dir: Option<&Path>) -> Result<()> {
        let shortcut_path = desktop_dir.join(format!("{}.lnk", self.name));
        let weird_powershell_cmd = format!(
            "$s=(New-Object -COM WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.Save()",
            shortcut_path.display(),
            self.path.display(),
        );
        if crate::run!("powershell", weird_powershell_cmd).is_err() {
            bail!(
                "unable to create a shortcut for '{}', skipping...",
                self.name
            );
        }

        // TODO: add windows menu shortcut, but I currently don't know how
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn create_shortcut_(&self, desktop_dir: &Path, application_dir: Option<&Path>) -> Result<()> {
        macro_rules! shortcut_file_content {
            ($($key:literal = $val:expr);+) => {{
                let mut _base_content_ = std::string::String::from(
                    "[Desktop Entry]\nType=Application\n"
                );
                $(
                    if let Some(_val_) = $val {
                        _base_content_.push_str(&std::format!("{}={_val_}\n", $key));
                    }
                )*
                _base_content_
            }};
        }

        let shortcut_content = shortcut_file_content!(
            "Name" = Some(self.name);
            "Exec" = Some(format!(
                "{}{}",
                self.path.display(),
                self.field_code.map(|c| format!(" {c}")).unwrap_or_default()
            ));
            "Icon" = self.icon.as_ref().map(|p| format!("{}", p.display()));
            "Comment" = self.comment;
            "GenericName" = self.generic_name;
            "StartupNotify" = Some(self.startup_notify);
            "StartupWMClass" = self.startup_wm_class;
            "MimeType" = (!self.mime_type.is_empty()).then_some(self.mime_type.join(";") + ";");
            "Categories" = (!self.categories.is_empty()).then_some(self.categories.join(";") + ";");
            "Keywords" = (!self.keywords.is_empty()).then_some(self.keywords.join(";") + ";")
        );

        let shortcut_filename = format!("{}.desktop", self.name);
        // write shortcut file to desktop folder
        let desktop_shortcut = desktop_dir.join(&shortcut_filename);
        ensure_dir(desktop_dir)?;
        write_file(&desktop_shortcut, &shortcut_content, false)?;
        set_exec_permission(&desktop_shortcut)?;

        // write shortcut file to application folder (for application menu)
        if let Some(app_dir) = application_dir {
            let app_shortcut = app_dir.join(&shortcut_filename);
            ensure_dir(app_dir)?;
            write_file(&app_shortcut, &shortcut_content, false)?;
            set_exec_permission(&app_shortcut)?;
        }

        Ok(())
    }

    #[cfg(windows)]
    fn remove_shortcut_(&self, desktop_dir: &Path, _application_dir: Option<&Path>) -> Result<()> {
        // remove the desktop shortcut
        let shortcut_path = desktop_dir.join(format!("{}.lnk", self.name));
        if shortcut_path.is_file() {
            remove(shortcut_path)
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    fn remove_shortcut_(&self, desktop_dir: &Path, application_dir: Option<&Path>) -> Result<()> {
        let shortcut_filename = format!("{}.desktop", self.name);

        // remove the desktop shortcut
        let desktop_shortcut = desktop_dir.join(&shortcut_filename);
        if desktop_shortcut.is_file() {
            remove(desktop_shortcut)?;
        }

        // remove the application shortcut
        if let Some(application_sc) = application_dir
            .map(|d| d.join(shortcut_filename))
            .filter(|d| d.is_file())
        {
            remove(application_sc)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_ambiguity() {
        let with_dots = PathBuf::from("/path/to/home/./my_app/../my_app");
        let without_dots = PathBuf::from("/path/to/home/my_app");
        assert_ne!(with_dots, without_dots);

        let with_dots_comps: PathBuf = with_dots.components().collect();
        let without_dots_comps: PathBuf = without_dots.components().collect();
        // Components take `..` accountable in case of symlink.
        assert_ne!(with_dots_comps, without_dots_comps);

        let with_dots_normalized = to_normalized_absolute_path(&with_dots, None).unwrap();
        let without_dots_normalized = to_normalized_absolute_path(&without_dots, None).unwrap();
        assert_eq!(with_dots_normalized, without_dots_normalized);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn shortcut_creation() {
        let debug_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .with_file_name("target")
            .join("debug");
        assert!(debug_dir.is_dir());

        let tmp_dir = debug_dir.join("tmp");
        ensure_dir(&tmp_dir).unwrap();

        let temp_home = tempfile::tempdir_in(tmp_dir).unwrap();
        let temp_desktop_dir = temp_home.path().join("Desktop");
        let temp_application_dir = temp_home.path().join("applications");

        let sc = ApplicationShortcut {
            name: "test-shortcut",
            path: "/path/to/test-shortcut".into(),
            icon: Some("/path/to/test-shortcut-icon".into()),
            comment: Some("a non-exist program for test"),
            generic_name: Some("Tests"),
            field_code: Some("%F"),
            startup_notify: true,
            startup_wm_class: Some("test-shortcut"),
            categories: &["Development", "Utility"],
            mime_type: &["text/plain", "application/pdf"],
            keywords: &["test", "rust"],
        };

        // create shortcuts
        sc.create_shortcut_(&temp_desktop_dir, Some(&temp_application_dir))
            .unwrap();
        let desktop_sc_file = temp_desktop_dir.join("test-shortcut.desktop");
        let applica_sc_file = temp_application_dir.join("test-shortcut.desktop");
        assert!(desktop_sc_file.is_file());
        assert!(applica_sc_file.is_file());

        let desktop_sc_content = read_to_string("", desktop_sc_file).unwrap();
        let applica_sc_content = read_to_string("", applica_sc_file).unwrap();
        assert_eq!(&desktop_sc_content, &applica_sc_content);
        assert_eq!(
            desktop_sc_content,
            "[Desktop Entry]
Type=Application
Name=test-shortcut
Exec=/path/to/test-shortcut %F
Icon=/path/to/test-shortcut-icon
Comment=a non-exist program for test
GenericName=Tests
StartupNotify=true
StartupWMClass=test-shortcut
MimeType=text/plain;application/pdf;
Categories=Development;Utility;
Keywords=test;rust;\n\n"
        );
    }
}
