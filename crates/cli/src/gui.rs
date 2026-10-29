#![cfg(feature = "slint-ui")]

use crate::run::{ProgressUpdate, collect_source_files, generate_collected};
use anyhow::Result;
use rfd::FileDialog;
use slint::ComponentHandle;
use slint::Model;
use slint::ModelRc;
use slint::VecModel;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod toast;
use toast::{ToastLevel, Toaster};

slint::include_modules!();

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "heic", "heif", "pdf"];

pub const APP_ID: &str = "io.github.Franck-Demongin.Combinator";

pub fn run() -> Result<()> {
    let ui =
        MainWindow::new().map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    if let Err(error) = slint::set_xdg_app_id(APP_ID) {
        eprintln!("warning: app_id not applied: {error}");
    }

    let toaster = Toaster::install(&ui);

    register_source_actions(&ui, toaster.clone());
    register_output_actions(&ui, toaster.clone());
    register_generation(&ui, toaster);

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI run failed: {error}"))?;
    Ok(())
}

fn register_source_actions(ui: &MainWindow, toaster: Toaster) {
    let weak = ui.as_weak();
    let toaster_files = toaster.clone();

    ui.on_add_files(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_busy() {
                return;
            }
            let mut current = source_list(&ui);
            if let Some(paths) = FileDialog::new()
                .add_filter("Sources", SUPPORTED_EXTENSIONS)
                .pick_files()
            {
                let mut skipped = 0;
                for path in paths {
                    let extension = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext.to_lowercase())
                        .unwrap_or_default();
                    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
                        skipped += 1;
                        continue;
                    }
                    let text = path.display().to_string();
                    if !current.contains(&text) {
                        current.push(text);
                    }
                }
                set_source_list(&ui, current);
                if skipped > 0 {
                    toaster_files.show(
                        format!("{skipped} fichier(s) non supporté(s) ignoré(s)."),
                        ToastLevel::Warning,
                    );
                }
            }
        }
    });

    let weak = ui.as_weak();
    let toaster_directory = toaster.clone();
    ui.on_add_directory(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_busy() {
                return;
            }
            let Some(folder) = FileDialog::new().pick_folder() else {
                return;
            };

            let mut current = source_list(&ui);
            let mut added = 0;
            match combinator_core::expand_directory(&folder) {
                Ok(files) => {
                    for path in files {
                        let text = path.display().to_string();
                        if !current.contains(&text) {
                            current.push(text);
                            added += 1;
                        }
                    }
                    set_source_list(&ui, current);
                    let skipped = std::fs::read_dir(&folder)
                        .ok()
                        .into_iter()
                        .flatten()
                        .filter_map(Result::ok)
                        .filter(|entry| {
                            let path = entry.path();
                            path.is_file() && !combinator_core::is_supported_path(&path)
                        })
                        .count();
                    if skipped > 0 {
                        toaster_directory.show(
                            format!(
                                "{skipped} fichier(s) non supporté(s) ignoré(s) dans le dossier."
                            ),
                            ToastLevel::Warning,
                        );
                    }
                    if added == 0 {
                        toaster_directory.show(
                            format!("Aucune source supportée dans {}.", folder.display()),
                            ToastLevel::Warning,
                        );
                    }
                }
                Err(error) => {
                    toaster_directory.show(error.to_string(), ToastLevel::Error);
                }
            }
        }
    });

    let weak = ui.as_weak();
    ui.on_clear_sources(move || {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
        {
            set_source_list(&ui, Vec::new());
        }
    });
}

fn register_output_actions(ui: &MainWindow, toaster: Toaster) {
    let weak = ui.as_weak();
    ui.on_choose_output(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_busy() {
                return;
            }
            let current = ui.get_output_path();
            let default_name = PathBuf::from(current.as_str())
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "combinator.pdf".to_owned());

            if let Some(path) = FileDialog::new()
                .add_filter("PDF", &["pdf"])
                .set_file_name(&default_name)
                .save_file()
            {
                ui.set_output_path(path.display().to_string().into());
            }
        }
    });

    let weak = ui.as_weak();
    let open_file_toaster = toaster.clone();
    ui.on_open_output_file(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(ui.get_output_path().as_str());
        if let Err(error) = open_path(&path) {
            open_file_toaster.show(error.to_string(), ToastLevel::Error);
        }
    });

    let weak = ui.as_weak();
    let open_folder_toaster = toaster.clone();
    ui.on_open_output_folder(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let path = PathBuf::from(ui.get_output_path().as_str());
        let folder = path.parent().unwrap_or_else(|| Path::new("."));
        if let Err(error) = open_path(folder) {
            open_folder_toaster.show(error.to_string(), ToastLevel::Error);
        }
    });
}

fn register_generation(ui: &MainWindow, toaster: Toaster) {
    let ui_weak = ui.as_weak();

    ui.on_generate(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        if ui.get_busy() {
            return;
        }

        let inputs = source_list(&ui)
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if inputs.is_empty() {
            toaster.show("Aucune source selectionnee.", ToastLevel::Warning);
            return;
        }

        let output = PathBuf::from(ui.get_output_path().as_str());
        let force = ui.get_force_overwrite();

        ui.set_busy(true);
        ui.set_output_ready(false);
        ui.set_progress(0.0);
        ui.set_status_text("Demarrage...".into());

        let worker_ui = ui_weak.clone();
        let (result_tx, result_rx) =
            mpsc::channel::<Result<crate::run::GenerationSummary, String>>();

        let thread_ui = worker_ui.clone();
        thread::spawn(move || {
            let progress_ui = thread_ui;
            let result = (|| -> Result<crate::run::GenerationSummary, anyhow::Error> {
                let files = collect_source_files(&inputs)?;
                generate_collected(
                    &files,
                    &output,
                    None,
                    force,
                    &mut |update: ProgressUpdate| {
                        let progress_ui = progress_ui.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = progress_ui.upgrade() {
                                ui.set_status_text(update.message.into());
                                ui.set_progress(update.fraction);
                            }
                        });
                    },
                )
            })()
            .map_err(|error| error.to_string());

            let _ = result_tx.send(result);
        });

        let completion_toaster = toaster.clone();
        schedule_generation_result(worker_ui, result_rx, completion_toaster);
    });
}

fn schedule_generation_result(
    ui_weak: slint::Weak<MainWindow>,
    result_rx: mpsc::Receiver<Result<crate::run::GenerationSummary, String>>,
    toaster: Toaster,
) {
    slint::Timer::single_shot(Duration::from_millis(100), move || {
        match result_rx.try_recv() {
            Ok(result) => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_busy(false);
                    match result {
                        Ok(summary) => {
                            ui.set_output_path(summary.output.display().to_string().into());
                            ui.set_progress(1.0);
                            ui.set_status_text(
                                format!(
                                    "PDF genere : {} ({} sources, {} planches)",
                                    summary.output.display(),
                                    summary.source_count,
                                    summary.board_count
                                )
                                .into(),
                            );
                            ui.set_output_ready(true);
                            toaster.show(
                                format!(
                                    "PDF genere : {} sources, {} planches.",
                                    summary.source_count, summary.board_count
                                ),
                                ToastLevel::Success,
                            );
                        }
                        Err(message) => {
                            ui.set_output_ready(false);
                            ui.set_status_text(message.clone().into());
                            toaster.show(message, ToastLevel::Error);
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                schedule_generation_result(ui_weak, result_rx, toaster);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_busy(false);
                    ui.set_output_ready(false);
                    ui.set_status_text("Generation interrompue.".into());
                }
            }
        }
    });
}

fn open_path(path: &Path) -> Result<()> {
    let command = if cfg!(target_os = "windows") {
        "cmd"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let mut process = Command::new(command);
    if cfg!(target_os = "windows") {
        process.args(["/C", "start", ""]).arg(path);
    } else {
        process.arg(path);
    }

    let child = process
        .spawn()
        .map_err(|error| anyhow::anyhow!("impossible d'ouvrir {}: {error}", path.display()))?;
    let _ = child;
    Ok(())
}

fn source_list(ui: &MainWindow) -> Vec<String> {
    let model = ui.get_source_items();
    (0..model.row_count())
        .filter_map(|row| model.row_data(row))
        .map(|value| value.to_string())
        .collect()
}

fn set_source_list(ui: &MainWindow, sources: Vec<String>) {
    let items: Vec<slint::SharedString> =
        sources.into_iter().map(slint::SharedString::from).collect();
    ui.set_source_items(ModelRc::new(VecModel::from(items)));
}
