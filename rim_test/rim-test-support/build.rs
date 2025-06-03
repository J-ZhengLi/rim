use std::env;

const TARGET_OVERRIDE_ENV: &str = "HOST_TRIPLE";

fn main() {
    println!("cargo:rerun-if-env-changed={TARGET_OVERRIDE_ENV}");

    let target = env::var(TARGET_OVERRIDE_ENV)
        .or(env::var("TARGET"))
        .unwrap();
    println!("cargo:rustc-env=TARGET={target}");
}
