use anyhow::Result;
use rim_common::types::BuildConfig;
use std::{env::consts::EXE_SUFFIX, fs, path::PathBuf, process::Command};

struct FakeRim {
    main_rs: String,
    cargo_toml: String,
    version: String,
}

impl FakeRim {
    fn new(version: &str) -> Self {
        let main_rs = format!(
            "
fn main() {{
    if std::env::args().any(|arg| arg == \"--version\") {{
        println!(\"rim {version}\");
    }}
}}"
        );
        let cargo_toml = format!(
            "
[package]
name = \"rim\"
version = \"{version}\"
edition = \"2021\"
[workspace]"
        );

        Self {
            main_rs,
            cargo_toml,
            version: version.into(),
        }
    }

    fn build(self, name: &str) -> Result<()> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let temp_dir_root = manifest_dir.with_file_name("target").join("tmp");
        fs::create_dir_all(&temp_dir_root)?;

        let temp_dir = temp_dir_root.join("mocked_rim");
        let src_dir = temp_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        fs::write(src_dir.join("main.rs"), self.main_rs)?;
        fs::write(temp_dir.join("Cargo.toml"), self.cargo_toml)?;

        // Build the mocked crate
        let mut c = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&temp_dir)
            .spawn()?;
        c.wait()?;

        // Collect built artifact
        let mut binary_path = temp_dir.join("target");
        binary_path.push("release");
        binary_path.push(format!("rim{}", std::env::consts::EXE_SUFFIX));
        let mut dest_dir = super::manager_dir().join("archive");
        dest_dir.push(&self.version);
        dest_dir.push(env!("TARGET"));
        fs::create_dir_all(&dest_dir)?;

        let gui_name = format!("{name}-manager{EXE_SUFFIX}");
        let cli_name = format!("{name}-manager-cli{EXE_SUFFIX}");
        fs::copy(&binary_path, dest_dir.join(gui_name))?;
        fs::copy(&binary_path, dest_dir.join(cli_name))?;

        Ok(())
    }
}

/// Generate a `release.toml` for self update, that the version will always be newer.
fn gen_release_toml(version: &MockedRimVersion) -> Result<()> {
    let release_toml = super::manager_dir().join("release.toml");

    let desired_content = format!(
        "version = '{}'
[beta]
version = '{}'",
        version.stable, version.beta
    );
    fs::write(release_toml, desired_content)?;
    Ok(())
}

struct MockedRimVersion {
    stable: String,
    beta: String,
}

/// Generate mocked release version base on the current rim version.
///
/// The mocked stable version will always be one major release ahead of the current version,
/// so if the current version is `1.0.0`, the target version will be `2.0.0`.
/// And the mocked beta version will always be two major release ahead,
/// which will be `3.0.0-beta` in the same context.
fn mocked_rim_versions() -> MockedRimVersion {
    let ws_manifest_content = include_str!("../../../Cargo.toml");
    let cur_ver = ws_manifest_content
        .lines()
        .find_map(|line| {
            if let Some((_, ver_with_quote)) = line.trim().split_once("version = ") {
                Some(ver_with_quote.trim_matches(['\'', '"']))
            } else {
                None
            }
        })
        .unwrap_or_else(|| unreachable!("'version' field is required in any cargo manifest"));

    // safe to unwrap the below lines, otherwise cargo would fails the build.
    let raw_ver = cur_ver.split('-').next().unwrap();
    let (major, rest) = raw_ver.split_once('.').unwrap();
    let major_number: usize = major.parse().unwrap();

    let stable = format!("{}.{rest}", major_number + 1);
    let beta = format!("{}.{rest}-beta", major_number + 2);

    MockedRimVersion { stable, beta }
}

/// Generate mocked manager binary for self updating tests.
pub(crate) fn generate() -> Result<()> {
    let vers = mocked_rim_versions();

    gen_release_toml(&vers)?;
    // Generate mocked binaries
    let identifier = &BuildConfig::load().identifier;
    FakeRim::new(&vers.stable).build(identifier)?;
    FakeRim::new(&vers.beta).build(identifier)?;
    Ok(())
}
