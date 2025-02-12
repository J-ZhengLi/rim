use std::env;

const TARGET_OVERRIDE_ENV: &str = "HOST_TRIPLE";
const FILES_TO_TRIGGER_REBUILD: &[&str] = &["../locales/en.json", "../locales/zh-CN.json"];
const EDITION_OVERRIDE_ENV: &str = "EDITION";
/// Default toolkit edition, such as `basic`, `community`, more to come.
const DEFAULT_EDITION: &str = "basic";

fn main() {
    println!("cargo:rerun-if-env-changed={TARGET_OVERRIDE_ENV}");
    println!("cargo:rerun-if-env-changed={EDITION_OVERRIDE_ENV}");
    for file in FILES_TO_TRIGGER_REBUILD {
        println!("cargo:rerun-if-changed={file}");
    }

    // this env was set by cargo, it's guaranteed to be present.
    let build_target = env::var("TARGET").unwrap();
    let host_triple = env::var(TARGET_OVERRIDE_ENV);
    let mut differ = false;
    if let Ok(triple) = &host_triple {
        if triple != &build_target {
            differ = true;
            println!("cargo::warning=overriding target triple from '{build_target}' to '{triple}'");
        }
    }
    println!("cargo:rustc-env=BUILD_TARGET_OVERRIDEN={differ}");
    println!(
        "cargo:rustc-env=TARGET={}",
        host_triple.unwrap_or(build_target)
    );

    let edition = env::var(EDITION_OVERRIDE_ENV).unwrap_or(DEFAULT_EDITION.to_string());
    println!("cargo::rustc-env=EDITION={edition}");
}
