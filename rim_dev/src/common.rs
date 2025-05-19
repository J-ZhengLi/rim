use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use rim_common::utils::{copy_as, walk_dir};

fn rim_gui_dir() -> &'static Path {
    static RIM_GUI_DIR: OnceLock<PathBuf> = OnceLock::new();
    RIM_GUI_DIR.get_or_init(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("rim_gui"))
}

pub(crate) fn test_asset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .with_file_name("tests")
        .join("assets")
}

/// Return the base command to run `pnpm`.
pub fn pnpm_cmd() -> Command {
    cfg_if::cfg_if! {
        if #[cfg(windows)] {
            let mut cmd = Command::new("cmd.exe");
            cmd.current_dir(rim_gui_dir()).args(["/C", "pnpm"]);
        } else {
            let mut cmd = Command::new("pnpm");
            cmd.current_dir(rim_gui_dir());
        }
    }
    cmd
}

pub fn install_gui_deps() {
    println!("running `pnpm i`");
    let fail_msg = "unable to run `pnpm i`, \
            please manually cd to `rim_gui/` then run the command manually";

    let gui_crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).with_file_name("rim_gui");
    assert!(gui_crate_dir.exists());

    let status = pnpm_cmd()
        .arg("i")
        .current_dir(gui_crate_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let Ok(st) = status else {
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
    // The official example uses 9, but also says 6 is a reasonable default.
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
        // NB (J-ZhengLi): Other methods appear to have a bug that causing
        // `channel-rust.xxx.sha256` fails to extract by Windows's native zip program.
        // https://github.com/zip-rs/zip2/issues/291
        .compression_method(zip::CompressionMethod::Stored)
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

/// Download a file from `url` to local disk, do nothing if it already exists.
pub fn download<P: AsRef<Path>>(url: &str, dest: P) -> Result<()> {
    if dest.as_ref().is_file() {
        return Ok(());
    }

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
