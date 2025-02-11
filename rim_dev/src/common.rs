use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};

// NB: If we end up using too many util functions from `rim`,
// consider separate the `utils` module as a separated crate.

/// Copy file or directory to a specified path.
pub fn copy_as<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fn copy_dir_(src: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest)?;
        for maybe_entry in src.read_dir()? {
            let entry = maybe_entry?;
            let src = entry.path();
            let dest = dest.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir_(&src, &dest)?;
            } else {
                copy(src, dest)?;
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
        copy(from, to)
    } else {
        copy_dir_(from.as_ref(), to.as_ref()).with_context(|| {
            format!(
                "could not copy directory '{}' to '{}'",
                from.as_ref().display(),
                to.as_ref().display()
            )
        })
    }
}

/// An [`fs::copy`] wrapper that only copies a file if:
///
/// - `to` does not exist yet.
/// - `to` exists but have different modified date.
///
/// Also, this function make sure the parent directory of `to` exists by creating one if not.
pub fn copy<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    // Make sure no redundent work is done
    if let (Ok(src_modif_time), Ok(dest_modif_time)) = (
        fs::metadata(&from).and_then(|m| m.modified()),
        fs::metadata(&to).and_then(|m| m.modified()),
    ) {
        if src_modif_time == dest_modif_time {
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

pub fn install_gui_deps() {
    println!("running `pnpm i`");
    let fail_msg = "unable to run `pnpm i`, \
            please manually cd to `rim_gui/` then run the command manually";

    let gui_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("rim_gui");
    assert!(gui_crate_dir.exists());

    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            let mut status = Command::new("cmd.exe");
            status.args(["/C", "pnpm", "i"]);
        } else {
            let mut status = Command::new("pnpm");
            status.arg("i");
        }
    }
    status
        .current_dir(gui_crate_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let Ok(st) = status.status() else {
        println!("{fail_msg}");
        return;
    };

    if !st.success() {
        println!("{fail_msg}: {}", st.code().unwrap_or(-1));
    }
}

pub fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("resources")
}

/// Convert a local path to file URL (with file schema: `file://`)
pub fn path_to_url<P: AsRef<Path>>(path: P) -> url::Url {
    url::Url::from_directory_path(&path).unwrap_or_else(|_| {
        panic!(
            "path {} cannot be converted to URL",
            path.as_ref().display()
        )
    })
}

pub fn compress_xz<S, D>(src: S, dest: D) -> Result<()>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    use xz2::write::XzEncoder;

    let tar_file = fs::File::create(dest)?;
    // NB (J-ZhengLi): compression needs a level, which is a number between 0-9.
    // The offcial example uses 9, but also says 6 is a reasonable default.
    // Well, don't know what that means, but I'm just gonna put 6 here.
    let encoding = XzEncoder::new(tar_file, 6);
    let mut tar = tar::Builder::new(encoding);

    let name = src.as_ref().file_name().unwrap_or(OsStr::new("/"));
    if src.as_ref().is_file() {
        tar.append_path_with_name(src.as_ref(), name)?;
    } else {
        tar.append_dir_all(name, src.as_ref())?;
    }
    tar.finish()?;
    Ok(())
}

pub fn compress_zip<S, D>(src: S, dest: D) -> Result<()>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    use zip::write::SimpleFileOptions;

    let zip_file = fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(zip_file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        // in case the file is too large
        .large_file(true)
        .unix_permissions(0o755);

    for path in walk_dir(src.as_ref(), true)? {
        let name = path.strip_prefix(src.as_ref())?;

        if path.is_file() {
            let mut file = fs::File::open(&path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            zip.start_file(name.to_string_lossy(), options)?;
            zip.write_all(&buffer)?;
        } else if path.is_dir() {
            zip.add_directory(name.to_string_lossy(), options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

/// Download a file from `url` to local disk.
pub fn download<P: AsRef<Path>>(url: &str, dest: P) -> Result<()> {
    println!("downloading: {url}");
    let resp = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?
        .get(url)
        .send()?;
    if !resp.status().is_success() {
        bail!("failed when downloading from: {url}");
    }

    let mut temp_file = tempfile::Builder::new().tempfile_in(
        dest.as_ref()
            .parent()
            .ok_or_else(|| anyhow!("cannot download to empty or root directory"))?,
    )?;
    let content = resp.bytes()?;
    temp_file.write_all(&content)?;

    // copy the tempfile to dest to prevent corrupt download
    copy_as(temp_file.path(), dest)?;
    Ok(())
}
