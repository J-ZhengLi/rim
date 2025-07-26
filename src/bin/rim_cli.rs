use anyhow::Result;
use rim::{cli::ExecutableCommand, Mode};

fn main() -> Result<()> {
    let status = {
        match Mode::detect(None, None) {
            Mode::Installer(cli) => cli?.execute(),
            Mode::Manager(cli) => cli?.execute(),
        }
    }?;

    if status.no_pause {
        #[cfg(windows)]
        rim::cli::pause().expect("unable to pause terminal window");
    }

    Ok(())
}
