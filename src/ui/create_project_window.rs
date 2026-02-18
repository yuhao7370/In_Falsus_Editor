use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;

/// State for the "Create Project" window.
#[derive(Debug, Clone)]
pub struct CreateProjectState {
    pub open: bool,
    pub project_name: String,
    pub audio_path: Option<String>,
    pub bpm: String,
    pub bpl: String,
    pub error_msg: Option<String>,
}

/// 创建项目参数（不再直接执行磁盘操作，交给 ProjectLoader 异步处理）
#[derive(Debug, Clone)]
pub struct CreateProjectParams {
    pub name: String,
    pub source_audio: String,
    pub bpm: f64,
    pub bpl: f64,
}

/// Result from the create project window: Some(params) when user clicks Create.
pub type CreateProjectResult = Option<CreateProjectParams>;

impl CreateProjectState {
    pub fn new() -> Self {
        Self {
            open: false,
            project_name: String::new(),
            audio_path: None,
            bpm: "120".to_string(),
            bpl: "4".to_string(),
            error_msg: None,
        }
    }

    pub fn reset(&mut self) {
        self.project_name.clear();
        self.audio_path = None;
        self.bpm = "120".to_string();
        self.bpl = "4".to_string();
        self.error_msg = None;
    }
}


pub fn draw_create_project_window(
    ctx: &egui::Context,
    i18n: &I18n,
    state: &mut CreateProjectState,
) -> CreateProjectResult {
    if !state.open {
        return None;
    }

    let mut result = None;
    let mut should_close = false;

    egui::Window::new(i18n.t(TextKey::CreateProjectTitle))
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
            let hint_color = egui::Color32::from_rgb(160, 160, 170);

            // Project name
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n.t(TextKey::CreateProjectName))
                        .color(label_color)
                        .size(14.0),
                );
            });
            ui.add(
                egui::TextEdit::singleline(&mut state.project_name)
                    .desired_width(380.0)
                    .hint_text(i18n.t(TextKey::CreateProjectNameHint)),
            );

            ui.separator();

            // Audio file
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n.t(TextKey::CreateProjectAudio))
                        .color(label_color)
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(i18n.t(TextKey::CreateProjectBrowse)).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Audio", &["ogg", "mp3", "wav", "flac"])
                            .pick_file()
                        {
                            state.audio_path = Some(path.to_string_lossy().to_string());
                        }
                    }
                });
            });
            if let Some(ref path) = state.audio_path {
                ui.label(
                    egui::RichText::new(path)
                        .color(egui::Color32::from_rgb(140, 200, 140))
                        .size(12.0),
                );
            } else {
                ui.label(
                    egui::RichText::new(i18n.t(TextKey::CreateProjectNoFile))
                        .color(hint_color)
                        .size(12.0),
                );
            }

            ui.separator();

            // BPM + BPL row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("BPM")
                        .color(label_color)
                        .size(14.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.bpm)
                        .desired_width(80.0),
                );
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(i18n.t(TextKey::CreateProjectBpl))
                        .color(label_color)
                        .size(14.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.bpl)
                        .desired_width(60.0),
                );
            });

            ui.separator();

            // Error message
            if let Some(ref err) = state.error_msg {
                ui.label(
                    egui::RichText::new(err)
                        .color(egui::Color32::from_rgb(255, 120, 120))
                        .size(12.0),
                );
            }

            // Buttons
            ui.horizontal(|ui| {
                let name_ok = !state.project_name.trim().is_empty();
                let audio_ok = state.audio_path.is_some();
                let bpm_ok = state.bpm.trim().parse::<f64>().is_ok();
                let bpl_ok = state.bpl.trim().parse::<f64>().is_ok();
                let all_ok = name_ok && audio_ok && bpm_ok && bpl_ok;

                ui.add_enabled_ui(all_ok, |ui| {
                    if ui.button(i18n.t(TextKey::CreateProjectCreate)).clicked() {
                        let name = state.project_name.trim().to_string();
                        let audio = state.audio_path.clone().unwrap();
                        let bpm: f64 = state.bpm.trim().parse().unwrap();
                        let bpl: f64 = state.bpl.trim().parse().unwrap();

                        result = Some(CreateProjectParams {
                            name,
                            source_audio: audio,
                            bpm,
                            bpl,
                        });
                        should_close = true;
                    }
                });
                if ui.button(i18n.t(TextKey::CreateProjectCancel)).clicked() {
                    should_close = true;
                }
            });
        });

    if should_close {
        state.open = false;
    }

    result
}
