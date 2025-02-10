use std::env;

const TARGET_OVERRIDE_ENV: &str = "HOST_TRIPPLE";
const EDITION_OVERRIDE_ENV: &str = "EDITION";
/// Default toolkit edition, such as `basic`, `community`, more to come.
const DEFAULT_EDITION: &str = "basic";
const FILES_TO_TRIGGER_REBUILD: &[&str] = &["locales/en.json", "locales/zh-CN.json"];

fn main() {
    println!("cargo:rerun-if-env-changed={TARGET_OVERRIDE_ENV}");
    for file in FILES_TO_TRIGGER_REBUILD {
        println!("cargo:rerun-if-changed={file}");
    }

    let target = env::var(TARGET_OVERRIDE_ENV)
        .or(env::var("TARGET"))
        .unwrap();
    println!("cargo:rustc-env=TARGET={target}");

    let profile = env::var("PROFILE").unwrap();
    println!("cargo:rustc-env=PROFILE={profile}");

    let edition = env::var(EDITION_OVERRIDE_ENV).unwrap_or(DEFAULT_EDITION.to_string());
    println!("cargo::rustc-env=EDITION={edition}");
}
