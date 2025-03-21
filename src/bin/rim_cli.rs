use anyhow::Result;
use rim::Mode;

fn main() -> Result<()> {
    match Mode::detect(None, None) {
        Mode::Installer(cli) => cli?.execute(),
        Mode::Manager(cli) => cli?.execute(),
    }
    Ok(())
}
