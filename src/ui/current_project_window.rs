use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;
use std::path::Path;

/// State for the "Current Project" window.
#[derive(Debug, Clone)]
pub struct CurrentProjectState {
    pub open: bool,
    pub chart_path: String,
    pub audio_path: String,
    /// Project directory (e.g. "projects/alamode"), used for copying files into.
    pub project_dir: String,
}

/// Action returned from the current project window.
#[derive(Debug, Clone)]
pub enum CurrentProjectAction {
    /// User loaded a new chart file; (new_chart_path)
    LoadChart(String),
    /// User loaded a new audio file; (new_audio_path)
    LoadAudio(String),
}

impl CurrentProjectState {
    pub fn new() -> Self {
        Self {
            open: false,
            chart_path: String::new(),
            audio_path: String::new(),
            project_dir: String::new(),
        }
    }
}

pub fn draw_current_project_window(
    ctx: &egui::Context,
    i18n: &I18n,
    state: &mut CurrentProjectState,
) -> Option<CurrentProjectAction> {
    if !state.open {
        return None;
    }

    let mut result = None;
    let mut should_close = false;

    egui::Window::new(i18n.t(TextKey::CurrentProjectTitle))
        .collapsible(false)
        .resizable(false)
        .min_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(egui::Color32::from_rgba_unmultiplied(16, 16, 22, 245))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                ))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            let label_color = egui::Color32::from_rgb(200, 200, 210);
            let path_color = egui::Color32::from_rgb(140, 200, 140);
            let hint_color = egui::Color32::from_rgb(160, 160, 170);
            let missing_color = egui::Color32::from_rgb(255, 160, 120);

            let has_project = !state.chart_path.is_empty() || !state.audio_path.is_empty();

            if !has_project {
                ui.label(
                    egui::RichText::new(i18n.t(TextKey::CurrentProjectNoProject))
                        .color(hint_color)
                        .size(14.0),
                );
            } else {
                // ── Chart file row ──
                let chart_exists = !state.chart_path.is_empty()
                    && Path::new(&state.chart_path).exists();

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(i18n.t(TextKey::CurrentProjectChart))
                            .color(label_color)
                            .size(14.0),
                    );
                    if !chart_exists {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .button(i18n.t(TextKey::CurrentProjectLoadChart))
                                    .clicked()
                                {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("SPC Chart", &["spc"])
                                        .pick_file()
                                    {
                                        let src = path.to_string_lossy().to_string();
                                        match copy_file_to_project(&src, &state.project_dir) {
                                            Ok(dest) => {
                                                state.chart_path = dest.clone();
                                                result = Some(CurrentProjectAction::LoadChart(dest));
                                            }
                                            Err(_) => {
                                                // fallback: use original path
                                                state.chart_path = src.clone();
                                                result = Some(CurrentProjectAction::LoadChart(src));
                                            }
                                        }
                                    }
                                }
                            },
                        );
                    }
                });
                if state.chart_path.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("—")
                                .color(hint_color)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(i18n.t(TextKey::CurrentProjectMissing))
                                .color(missing_color)
                                .size(12.0),
                        );
                    });
                } else if !chart_exists {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&state.chart_path)
                                .color(missing_color)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(i18n.t(TextKey::CurrentProjectMissing))
                                .color(missing_color)
                                .size(12.0),
                        );
                    });
                } else {
                    ui.label(
                        egui::RichText::new(&state.chart_path)
                            .color(path_color)
                            .size(12.0),
                    );
                }

                ui.separator();

                // ── Audio file row ──
                let audio_exists = !state.audio_path.is_empty()
                    && Path::new(&state.audio_path).exists();

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(i18n.t(TextKey::CurrentProjectAudio))
                            .color(label_color)
                            .size(14.0),
                    );
                    if !audio_exists {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .button(i18n.t(TextKey::CurrentProjectLoadAudio))
                                    .clicked()
                                {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Audio", &["ogg", "mp3", "wav", "flac"])
                                        .pick_file()
                                    {
                                        let src = path.to_string_lossy().to_string();
                                        match copy_file_to_project(&src, &state.project_dir) {
                                            Ok(dest) => {
                                                state.audio_path = dest.clone();
                                                result = Some(CurrentProjectAction::LoadAudio(dest));
                                            }
                                            Err(_) => {
                                                state.audio_path = src.clone();
                                                result = Some(CurrentProjectAction::LoadAudio(src));
                                            }
                                        }
                                    }
                                }
                            },
                        );
                    }
                });
                if state.audio_path.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("—")
                                .color(hint_color)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(i18n.t(TextKey::CurrentProjectMissing))
                                .color(missing_color)
                                .size(12.0),
                        );
                    });
                } else if !audio_exists {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&state.audio_path)
                                .color(missing_color)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(i18n.t(TextKey::CurrentProjectMissing))
                                .color(missing_color)
                                .size(12.0),
                        );
                    });
                } else {
                    ui.label(
                        egui::RichText::new(&state.audio_path)
                            .color(path_color)
                            .size(12.0),
                    );
                }
            }

            ui.separator();

            // Close button
            if ui.button(i18n.t(TextKey::CurrentProjectClose)).clicked() {
                should_close = true;
            }
        });

    if should_close {
        state.open = false;
    }

    result
}

/// Copy a file into the project directory, preserving its filename.
/// Returns the destination path on success.
fn copy_file_to_project(src: &str, project_dir: &str) -> Result<String, String> {
    if project_dir.is_empty() {
        return Err("No project directory".to_string());
    }
    let src_path = Path::new(src);
    let filename = src_path
        .file_name()
        .ok_or_else(|| "Invalid source filename".to_string())?;
    let dest = Path::new(project_dir).join(filename);
    // Create project dir if needed
    std::fs::create_dir_all(project_dir)
        .map_err(|e| format!("Failed to create project dir: {e}"))?;
    std::fs::copy(src, &dest)
        .map_err(|e| format!("Failed to copy file: {e}"))?;
    Ok(dest.to_string_lossy().to_string())
}
