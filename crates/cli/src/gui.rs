#![cfg(feature = "slint-ui")]

use crate::run::{ProgressUpdate, collect_source_files, generate_collected};
use anyhow::Result;
use rfd::FileDialog;
use slint::ComponentHandle;
use slint::Model;
use slint::ModelRc;
use slint::VecModel;
use std::cell::RefCell;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod toast;
use toast::{ToastLevel, Toaster};

slint::include_modules!();

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "heic", "heif", "pdf"];

pub const APP_ID: &str = "io.github.Franck-Demongin.Combinator";

pub fn run() -> Result<()> {
    let ui =
        MainWindow::new().map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    if let Err(error) = slint::set_xdg_app_id(APP_ID) {
        eprintln!("warning: app_id not applied: {error}");
    }

    let toaster = Toaster::install(&ui);

    ui.set_output_path(default_output_path().display().to_string().into());

    let report_dialog = Rc::new(RefCell::new(None::<ReportDialog>));
    let pending_drop_target = Rc::new(RefCell::new(None::<usize>));

    register_source_actions(&ui, toaster.clone(), pending_drop_target);
    register_output_actions(&ui, toaster.clone(), report_dialog);
    register_generation(&ui, toaster);

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI run failed: {error}"))?;
    Ok(())
}

fn register_source_actions(
    ui: &MainWindow,
    toaster: Toaster,
    pending_drop_target: Rc<RefCell<Option<usize>>>,
) {
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

    let weak = ui.as_weak();
    ui.on_remove_source(move |index: i32| {
        if let Some(ui) = weak.upgrade()
            && !ui.get_busy()
        {
            let mut current = source_list(&ui);
            let index = index as usize;
            if index < current.len() {
                current.remove(index);
                set_source_list(&ui, current);
            }
        }
    });

    let transfer = ui.global::<SourceTransfer>();
    transfer.on_path_to_transfer(|path| path.into());
    transfer.on_transfer_to_path(|data| data.plain_text().unwrap_or_default());

    let pending_for_drop = pending_drop_target.clone();
    ui.on_source_drop_target(move |_index: i32, _data: slint::DataTransfer| {
        *pending_for_drop.borrow_mut() = Some(_index as usize);
    });

    let pending_for_drag = pending_drop_target.clone();
    let weak_drag = ui.as_weak();
    ui.on_source_drag_finished(
        move |source: i32, action: slint::private_unstable_api::re_exports::DragAction| {
            if action == slint::private_unstable_api::re_exports::DragAction::Move {
                let target = pending_for_drag.borrow_mut().take();
                if let Some(target) = target
                    && let Some(ui) = weak_drag.upgrade()
                    && !ui.get_busy()
                {
                    let mut current = source_list(&ui);
                    let src = source as usize;
                    if src < current.len() && target < current.len() && src != target {
                        let item = current.remove(src);
                        let insert_at = target;
                        if insert_at <= current.len() {
                            current.insert(insert_at, item);
                        }
                        set_source_list(&ui, current);
                    }
                }
            }
        },
    );
}

fn register_output_actions(
    ui: &MainWindow,
    toaster: Toaster,
    report_dialog: Rc<RefCell<Option<ReportDialog>>>,
) {
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
        if ui.get_busy() {
            return;
        }
        let output_path = ui.get_output_path();
        let raw = output_path.as_str();
        if raw.trim().is_empty() {
            open_file_toaster.show(
                "Aucun fichier de sortie selectionne.".to_owned(),
                ToastLevel::Warning,
            );
            return;
        }
        let path = resolve_output_path(raw);
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
        if ui.get_busy() {
            return;
        }
        let output_path = ui.get_output_path();
        let raw = output_path.as_str();
        if raw.trim().is_empty() {
            open_folder_toaster.show(
                "Aucun fichier de sortie selectionne.".to_owned(),
                ToastLevel::Warning,
            );
            return;
        }
        let folder = output_folder(Path::new(raw));
        if let Err(error) = open_path(&folder) {
            open_folder_toaster.show(error.to_string(), ToastLevel::Error);
        }
    });

    let weak = ui.as_weak();
    let report_open_toaster = toaster.clone();
    let report_for_open = report_dialog;
    ui.on_open_report(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        if ui.get_busy() {
            return;
        }
        let report_text = ui.get_report_text();
        let mut guard = report_for_open.borrow_mut();
        if guard.is_none() {
            match create_report_dialog(report_for_open.clone()) {
                Ok(dialog) => *guard = Some(dialog),
                Err(error) => {
                    report_open_toaster.show(
                        format!("impossible d'ouvrir le rapport : {error}"),
                        ToastLevel::Error,
                    );
                    return;
                }
            }
        }
        if let Some(dialog) = guard.as_ref() {
            dialog.set_report_text(report_text);
            let _ = dialog.window().show();
        }
    });
}

fn create_report_dialog(
    handle: Rc<RefCell<Option<ReportDialog>>>,
) -> Result<ReportDialog, slint::PlatformError> {
    let dialog = ReportDialog::new()?;
    let closer = handle.clone();
    dialog.on_ok_clicked(move || {
        if let Some(dialog) = closer.borrow().as_ref() {
            let _ = dialog.window().hide();
        }
        *closer.borrow_mut() = None;
    });
    Ok(dialog)
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

        let output = resolve_output_path(ui.get_output_path().as_str());
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
                            let output_path = std::fs::canonicalize(&summary.output)
                                .unwrap_or_else(|_| {
                                    if summary.output.is_relative() {
                                        std::env::current_dir()
                                            .unwrap_or_default()
                                            .join(&summary.output)
                                    } else {
                                        summary.output.clone()
                                    }
                                });
                            ui.set_output_path(output_path.display().to_string().into());
                            ui.set_report_text(summary.report_text.clone().into());
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
                            ui.set_report_text(String::new().into());
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

fn output_folder(path: &Path) -> PathBuf {
    let candidate = resolve_output_path(path.to_string_lossy().as_ref());
    let resolved = std::fs::canonicalize(&candidate).unwrap_or_else(|_| {
        if candidate.is_relative() {
            std::env::current_dir().unwrap_or_default().join(&candidate)
        } else {
            candidate.clone()
        }
    });
    match resolved.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => resolved,
    }
}

fn desktop_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let desktop = PathBuf::from(home).join("Desktop");
        if desktop.is_dir() {
            return desktop;
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let desktop = PathBuf::from(profile).join("Desktop");
        if desktop.is_dir() {
            return desktop;
        }
    }
    if let (Ok(drive), Ok(homepath)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        let desktop = PathBuf::from(format!("{drive}{homepath}")).join("Desktop");
        if desktop.is_dir() {
            return desktop;
        }
    }
    if let Ok(desktop) = std::env::var("XDG_DESKTOP_DIR") {
        let desktop = PathBuf::from(desktop);
        if desktop.is_dir() {
            return desktop;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn default_output_path() -> PathBuf {
    desktop_dir().join("combinator.pdf")
}

fn resolve_output_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if raw.trim().is_empty() {
        return default_output_path();
    }
    if path
        .parent()
        .map(|parent| parent.as_os_str().is_empty())
        .unwrap_or(true)
    {
        desktop_dir().join(path.file_name().unwrap_or_default())
    } else {
        path
    }
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
        .map(|value| value.path.to_string())
        .collect()
}

fn set_source_list(ui: &MainWindow, sources: Vec<String>) {
    let items: Vec<SourceItem> = sources
        .into_iter()
        .map(|path| {
            let name = std::path::Path::new(&path)
                .file_name()
                .and_then(|os| os.to_str())
                .unwrap_or(path.as_str())
                .to_owned();
            let directory = std::path::Path::new(&path)
                .parent()
                .map(|parent| parent.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut directory = if directory.is_empty() {
                ".".to_owned()
            } else {
                directory
            };
            if !directory.ends_with(std::path::MAIN_SEPARATOR) {
                directory.push(std::path::MAIN_SEPARATOR);
            }
            SourceItem {
                path: path.clone().into(),
                name: name.clone().into(),
                display_name: truncate_filename(&name, 50).into(),
                display_path: truncate_path(&directory, 75).into(),
            }
        })
        .collect();
    ui.set_source_items(ModelRc::new(VecModel::from(items)));
}

fn truncate_filename(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }

    if let Some(dot_index) = text.rfind('.') {
        let extension: String = text[dot_index..].chars().collect();
        let extension_chars = extension.chars().count();
        let stem_limit = max.saturating_sub(extension_chars).saturating_sub(3);
        if stem_limit > 0 && extension_chars + 4 <= max {
            let stem: String = text[..dot_index].chars().take(stem_limit).collect();
            return format!("{stem}...{extension}");
        }
    }

    let limit = max.saturating_sub(3);
    format!("{}...", chars.into_iter().take(limit).collect::<String>())
}

fn truncate_path(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    if max <= 3 {
        return ".".repeat(max);
    }

    let kept = max - 3;
    let left_count = kept / 2;
    let right_count = kept - left_count;
    let left: String = chars[..left_count].iter().collect();
    let right: String = chars[chars.len() - right_count..].iter().collect();
    format!("{left}...{right}")
}
