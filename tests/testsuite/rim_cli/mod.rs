use rim_test_support::process::TestProcess;

mod complicated;
mod installer;
mod manager;

/// Use default install method and return the test process.
fn default_install(combined: bool) -> TestProcess {
    let process = if combined {
        TestProcess::combined()
    } else {
        TestProcess::installer()
    };
    // install rust
    let res = process.command().arg("-y").assert().success();
    println!("{}", String::from_utf8_lossy(&res.get_output().stdout));
    process
}
