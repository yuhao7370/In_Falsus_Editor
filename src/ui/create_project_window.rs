use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;
use std::path::Path;

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

/// Result from the create project window: Some((chart_path, audio_path)) when project is created.
pub type CreateProjectResult = Option<(String, String)>;

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

/// Create the project on disk:
/// 1. Create projects/{name}/ directory
/// 2. Copy audio file into it
/// 3. Create .spc with chart(bpm, bpl)
/// 4. Create .iffproj with audio_path and chart_path
/// Returns (chart_path, audio_path_in_project) on success.
fn create_project_on_disk(
    name: &str,
    source_audio: &str,
    bpm: f64,
    bpl: f64,
) -> Result<(String, String), String> {
    let project_dir = format!("projects/{}", name);
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("创建项目目录失败: {e}"))?;

    // Copy audio file
    let audio_source = Path::new(source_audio);
    let audio_ext = audio_source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("ogg");
    let audio_filename = format!("music.{}", audio_ext);
    let audio_dest = format!("{}/{}", project_dir, audio_filename);
    std::fs::copy(source_audio, &audio_dest)
        .map_err(|e| format!("复制音频文件失败: {e}"))?;

    // Create .spc file
    let chart_filename = format!("{}.spc", name);
    let chart_path = format!("{}/{}", project_dir, chart_filename);
    let spc_content = format!("chart({:.2},{:.2})\n", bpm, bpl);
    std::fs::write(&chart_path, &spc_content)
        .map_err(|e| format!("创建谱面文件失败: {e}"))?;

    // Create .iffproj file
    let proj_path = format!("{}/{}.iffproj", project_dir, name);
    let proj_json = serde_json::json!({
        "audio_path": audio_dest,
        "chart_path": chart_path,
    });
    let proj_content = serde_json::to_string_pretty(&proj_json)
        .map_err(|e| format!("序列化项目文件失败: {e}"))?;
    std::fs::write(&proj_path, &proj_content)
        .map_err(|e| format!("创建项目文件失败: {e}"))?;

    Ok((chart_path, audio_dest))
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

                        match create_project_on_disk(&name, &audio, bpm, bpl) {
                            Ok((chart_path, audio_path)) => {
                                result = Some((chart_path, audio_path));
                                should_close = true;
                            }
                            Err(e) => {
                                state.error_msg = Some(e);
                            }
                        }
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
