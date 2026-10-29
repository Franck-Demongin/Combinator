#![cfg(feature = "slint-ui")]

use anyhow::Result;
use slint::ComponentHandle;

pub const APP_ID: &str = "io.github.Franck-Demongin.Combinator";

mod toast;
use toast::Toaster;

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui =
        MainWindow::new().map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    if let Err(error) = slint::set_xdg_app_id(APP_ID) {
        eprintln!("warning: app_id not applied: {error}");
    }

    let _toaster: Toaster = Toaster::install(&ui);

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI run failed: {error}"))?;
    Ok(())
}
