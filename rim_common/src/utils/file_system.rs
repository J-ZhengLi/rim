use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Get a path to user's "home" directory.
///
/// # Panic
///
/// Will panic if such directory cannot be determined,
/// which could be the result of missing certain environment variable at runtime,
/// check [`dirs::home_dir`] for more information.
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("home directory cannot be determined.")
}

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
    debug!(
        "setting execute permission for file '{}'",
        path.as_ref().display()
    );
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
}
