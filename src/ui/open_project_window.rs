use crate::i18n::I18n;
use egui_macroquad::egui;

/// State for the "Open Project" window.
#[derive(Debug, Clone)]
pub struct OpenProjectState {
    pub open: bool,
    pub chart_path: Option<String>,
    pub audio_path: Option<String>,
}

/// Result from the open project window: Some((chart, audio)) when user clicks Load.
pub type OpenProjectResult = Option<(String, String)>;

impl OpenProjectState {
    pub fn new() -> Self {
        Self {
            open: false,
            chart_path: None,
            audio_path: None,
        }
    }
}

pub fn draw_open_project_window(
    ctx: &egui::Context,
    _i18n: &I18n,
    state: &mut OpenProjectState,
) -> OpenProjectResult {
    if !state.open {
        return None;
    }

    let mut result = None;
    let mut should_close = false;

    egui::Window::new("Open Project")
        .collapsible(false)
        .resizable(false)
        .min_width(400.0)
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

            // Chart file row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("谱面文件 (.spc)")
                        .color(egui::Color32::from_rgb(200, 200, 210))
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("选择...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SPC Chart", &["spc"])
                            .pick_file()
                        {
                            state.chart_path = Some(path.to_string_lossy().to_string());
                        }
                    }
                });
            });
            if let Some(ref path) = state.chart_path {
                ui.label(
                    egui::RichText::new(path)
                        .color(egui::Color32::from_rgb(140, 200, 140))
                        .size(12.0),
                );
            } else {
                ui.label(
                    egui::RichText::new("未选择")
                        .color(egui::Color32::from_rgb(160, 160, 170))
                        .size(12.0),
                );
            }

            ui.separator();

            // Audio file row
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("音频文件 (.ogg/.mp3/.wav)")
                        .color(egui::Color32::from_rgb(200, 200, 210))
                        .size(14.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("选择...").clicked() {
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
                    egui::RichText::new("未选择")
                        .color(egui::Color32::from_rgb(160, 160, 170))
                        .size(12.0),
                );
            }

            ui.separator();

            // Buttons
            ui.horizontal(|ui| {
                let both_selected = state.chart_path.is_some() && state.audio_path.is_some();
                ui.add_enabled_ui(both_selected, |ui| {
                    if ui.button("  加载  ").clicked() {
                        if let (Some(chart), Some(audio)) =
                            (state.chart_path.clone(), state.audio_path.clone())
                        {
                            result = Some((chart, audio));
                            should_close = true;
                        }
                    }
                });
                if ui.button("  取消  ").clicked() {
                    should_close = true;
                }
            });
        });

    if should_close {
        state.open = false;
    }

    result
}
