use crate::app::setup::apply_settings_to_editor;
use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::I18n;
use crate::ui::current_project_window::CurrentProjectAction;
use crate::ui::info_toast::InfoToastManager;
use crate::ui::loading_status::{LoadAction, ProjectLoader};
use macroquad::prelude::Font;

use super::ui_orchestrator::UiOutput;

/// 封装 ProjectLoader + 项目加载/创建的完整流程
pub struct ProjectManager {
    loader: ProjectLoader,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            loader: ProjectLoader::new(),
        }
    }

    /// 根据 UiOutput 中的动作启动异步加载
    pub fn handle_ui_actions(
        &mut self,
        ui: &UiOutput,
        info_toasts: &mut InfoToastManager,
    ) {
        // 打开项目
        if let Some((chart_path, audio_path)) = &ui.open_project {
            if !self.loader.is_loading() {
                self.loader.start_open_project(chart_path.clone(), audio_path.clone());
                info_toasts.pin(self.loader.status_text());
            }
        }

        // 当前项目窗口动作
        if let Some(ref cp_action) = ui.current_project_action {
            match cp_action {
                CurrentProjectAction::LoadChart(chart_path) => {
                    let audio_path = ui.current_project_audio_path.clone();
                    if !self.loader.is_loading() {
                        self.loader.start_open_project(chart_path.clone(), audio_path);
                        info_toasts.pin(self.loader.status_text());
                    }
                }
                CurrentProjectAction::LoadAudio(audio_path) => {
                    let chart_path = ui.current_project_chart_path.clone();
                    if !self.loader.is_loading() {
                        self.loader.start_open_project(chart_path, audio_path.clone());
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
            LoadAction::LoadChart { chart_path, audio_path } => {
                let font_backup = macroquad_font.clone();
                *editor = FallingGroundEditor::from_chart_path(&chart_path);
                editor.set_text_font(font_backup);
                apply_settings_to_editor(editor, audio, i18n);
                self.loader.advance_after_chart_load(chart_path, audio_path);
                info_toasts.pin(self.loader.status_text());
            }
            LoadAction::InstallAudio { clip, chart_path, audio_path } => {
                audio.install_decoded_audio(clip, &audio_path, i18n);
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
