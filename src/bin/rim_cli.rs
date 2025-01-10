use anyhow::Result;
use rim::Mode;

fn main() -> Result<()> {
    match Mode::detect(None, None) {
        Mode::Installer(cli) => cli.execute()?,
        Mode::Manager(cli) => cli.execute()?,
    }

    // pause the console in case the user launch
    // the program with double click, which most people do.
    #[cfg(windows)]
    rim::cli::pause()?;

    Ok(())
}
