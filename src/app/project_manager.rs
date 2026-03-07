use crate::app::setup::apply_settings_to_editor;
use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::I18n;
use crate::ui::current_project_window::CurrentProjectAction;
use crate::ui::info_toast::InfoToastManager;
use crate::ui::loading_status::{LoadAction, ProjectLoader};
use macroquad::prelude::Font;
use std::path::Path;

use super::ui_orchestrator::UiOutput;

/// 封装 ProjectLoader + 项目加载/创建的完整流程
pub struct ProjectManager {
    loader: ProjectLoader,
    pending_music_time_ms: Option<f32>,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            loader: ProjectLoader::new(),
            pending_music_time_ms: None,
        }
    }

    /// 保存当前音乐时间到工程文件
    pub fn save_music_time_to_project(&self, editor: &FallingGroundEditor, audio: &AudioController) -> Result<(), String> {
        // Find the project file by looking for .iffproj file in the same directory as the chart
        let chart_path_str = editor.chart_path();
        let chart_path = Path::new(&chart_path_str);
        let chart_dir = chart_path.parent().ok_or("无法获取谱面目录")?;
        let chart_name = chart_path.file_stem().ok_or("无法获取谱面名称")?;
        let proj_path = chart_dir.join(format!("{}.iffproj", chart_name.to_string_lossy()));

        // Read existing project file
        let content = std::fs::read_to_string(&proj_path)
            .map_err(|e| format!("读取工程文件失败: {e}"))?;

        // Parse JSON
        let mut json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("解析工程文件失败: {e}"))?;
        
        // Get current music time in milliseconds
        let current_time_ms = audio.current_sec() * 1000.0;

        // Update last_music_time_ms
        json["last_music_time_ms"] = serde_json::Value::Number(
            serde_json::Number::from_f64(current_time_ms as f64)
                .ok_or("无法转换播放时间为数字")?
        );
        
        // Write back
        let updated_content = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("序列化工程文件失败: {e}"))?;

        std::fs::write(&proj_path, updated_content)
            .map_err(|e| format!("写入工程文件失败: {e}"))?;

        Ok(())
    }

    /// 根据 UiOutput 中的动作启动异步加载
    pub fn handle_ui_actions(
        &mut self,
        ui: &UiOutput,
        info_toasts: &mut InfoToastManager,
    ) {
        // 打开项目
        if let Some((chart_path, audio_path, last_music_time_ms)) = &ui.open_project {
            if !self.loader.is_loading() {
                self.loader.start_open_project(chart_path.clone(), audio_path.clone(), *last_music_time_ms);
                info_toasts.pin(self.loader.status_text());
            }
        }

        // 当前项目窗口动作
        if let Some(ref cp_action) = ui.current_project_action {
            match cp_action {
                CurrentProjectAction::LoadChart(chart_path) => {
                    let audio_path = ui.current_project_audio_path.clone();
                    if !self.loader.is_loading() {
                        self.loader.start_open_project(chart_path.clone(), audio_path, 0.0);
                        info_toasts.pin(self.loader.status_text());
                    }
                }
                CurrentProjectAction::LoadAudio(audio_path) => {
                    let chart_path = ui.current_project_chart_path.clone();
                    if !self.loader.is_loading() {
                        self.loader.start_open_project(chart_path, audio_path.clone(), 0.0);
                        info_toasts.pin(self.loader.status_text());
                    }
                }
            }
        }

        // 创建项目
        if let Some(ref params) = ui.create_project {
            if !self.loader.is_loading() {
                self.loader.start_create_project(
                    params.name.clone(),
                    params.source_audio.clone(),
                    params.bpm,
                    params.bpl,
                );
                info_toasts.pin(self.loader.status_text());
            }
        }
    }

    /// 每帧推进 ProjectLoader 状态机，处理 LoadAction
    pub fn tick_and_apply(
        &mut self,
        editor: &mut FallingGroundEditor,
        audio: &mut AudioController,
        i18n: &I18n,
        info_toasts: &mut InfoToastManager,
        macroquad_font: &Option<Font>,
    ) {
        let prev_status = self.loader.status_text().to_owned();
        let action = self.loader.tick();
        let new_status = self.loader.status_text();
        if new_status != prev_status {
            if new_status.is_empty() {
                info_toasts.dismiss_pinned();
            } else {
                info_toasts.pin(new_status);
            }
        }
        match action {
            LoadAction::None => {}
            LoadAction::LoadChart { chart_path, audio_path, last_music_time_ms } => {
                let font_backup = macroquad_font.clone();
                *editor = FallingGroundEditor::from_chart_path(&chart_path);
                editor.set_text_font(font_backup);
                apply_settings_to_editor(editor, audio, i18n);
                
                // Store the music time to set after audio is installed
                self.pending_music_time_ms = Some(last_music_time_ms);
                
                self.loader.advance_after_chart_load(chart_path, audio_path);
                info_toasts.pin(self.loader.status_text());
            }
            LoadAction::InstallAudio { clip, chart_path, audio_path } => {
                audio.install_decoded_audio(clip, &audio_path, i18n);
                
                // Set audio position to saved time after audio is installed (convert ms to seconds)
                if let Some(last_music_time_ms) = self.pending_music_time_ms.take() {
                    if last_music_time_ms > 0.0 {
                        let target_sec = last_music_time_ms / 1000.0;
                        audio.seek_to(target_sec, i18n);
                    }
                }
                
                self.loader.finish();
                info_toasts.dismiss_pinned();
                info_toasts.push(format!("项目已加载: {}", chart_path));
            }
            LoadAction::Error(e) => {
                self.loader.finish();
                info_toasts.dismiss_pinned();
                info_toasts.push_warn(format!("加载失败: {}", e));
            }
        }
    }
}
