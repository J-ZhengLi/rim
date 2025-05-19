use anyhow::Result;
use rim::{cli::ExecutableCommand, Mode};

fn main() -> Result<()> {
    let res = {
        match Mode::detect(None, None) {
            Mode::Installer(cli) => cli?.execute(),
            Mode::Manager(cli) => cli?.execute(),
        }
    };

    if !matches!(&res, Ok(status) if status.no_pause) {
        #[cfg(windows)]
        rim::cli::pause().expect("unable to pause terminal window");
    }

    res.map(|_| ())
}
