#![cfg_attr(all(windows, feature = "slint-ui"), windows_subsystem = "windows")]

#[cfg(feature = "slint-ui")]
mod slint_ui;

#[cfg(feature = "slint-ui")]
fn main() {
    if let Err(error) = slint_ui::run() {
        eprintln!("Slint error: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "slint-ui"))]
fn main() {
    println!("Combinator GUI requires the 'slint-ui' feature.");
}
