use anyhow::Result;
use rim::{cli::ExecutableCommand, Mode};

fn main() -> Result<()> {
    match Mode::detect(None, None) {
        Mode::Installer(cli) => cli?.execute(),
        Mode::Manager(cli) => cli?.execute(),
    }
}
