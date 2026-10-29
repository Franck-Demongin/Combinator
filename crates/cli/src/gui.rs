#![cfg(feature = "slint-ui")]

use anyhow::Result;
use slint::ComponentHandle;

pub const APP_ID: &str = "io.github.Franck-Demongin.Combinator";

mod toast;
use toast::{ToastLevel, Toaster};

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui =
        MainWindow::new().map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    if let Err(error) = slint::set_xdg_app_id(APP_ID) {
        eprintln!("warning: app_id not applied: {error}");
    }

    let toaster = Toaster::install(&ui);

    let success_toaster = toaster.clone();
    ui.on_show_success(move || {
        success_toaster.show("Hello World ! Toast de succes.", ToastLevel::Success);
    });

    let warning_toaster = toaster.clone();
    ui.on_show_warning(move || {
        warning_toaster.show("Hello World ! Toast d'avertissement.", ToastLevel::Warning);
    });

    let error_toaster = toaster.clone();
    ui.on_show_error(move || {
        error_toaster.show("Hello World ! Toast d'erreur.", ToastLevel::Error);
    });

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI run failed: {error}"))?;
    Ok(())
}
