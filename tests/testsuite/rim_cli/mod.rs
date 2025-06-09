use rim_test_support::process::TestProcess;

mod complicated;
mod installer;
mod manager;

/// Use default install method and return the test process.
fn default_install() -> TestProcess {
    let process = TestProcess::installer();
    // install rust
    process.command().arg("-y").assert().success();
    process
}
