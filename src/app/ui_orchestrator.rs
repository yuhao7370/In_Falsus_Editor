use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::I18n;
use crate::settings::{modify_settings, settings};
use crate::shortcuts::ShortcutAction;
use crate::ui::audio_debug_window::draw_audio_debug_window;
use crate::ui::create_project_window::{CreateProjectParams, CreateProjectState, draw_create_project_window};
use crate::ui::current_project_window::{CurrentProjectAction, CurrentProjectState, copy_file_to_project, draw_current_project_window};
use crate::ui::docs_window::{DocsCategory, draw_docs_window};
use crate::ui::fonts::init_egui_fonts;
use crate::ui::info_toast::InfoToastManager;
use crate::ui::input_state::{set_keyboard_blocked, set_pointer_blocked};
use crate::ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, PropertyEditState, draw_note_selector_panel, draw_snap_slider_panel};
use crate::ui::scale::ui_scale_factor;
use crate::ui::settings_window::{SettingsCategory, draw_settings_window};
use crate::ui::top_menu::{FileAction, TopMenuAction, TopMenuResult, draw_top_menu};

use super::constants::*;

/// egui UI 每帧绘制后的输出
pub struct UiOutput {
    pub menu_action: Option<TopMenuAction>,
    pub open_project: Option<(String, String)>,
    pub create_project: Option<CreateProjectParams>,
    pub current_project_action: Option<CurrentProjectAction>,
    pub egui_wants_pointer: bool,
    pub note_panel_width_px: f32,
    pub total_right_panels_px: f32,
    pub egui_wheel_y: f32,
    pub ui_scale: f32,
    pub menu_height: f32,
    pub top_bar_height: f32,
    /// 当前项目窗口中的 chart_path（供 ProjectManager 补全 CurrentProjectAction）
    pub current_project_chart_path: String,
    /// 当前项目窗口中的 audio_path（供 ProjectManager 补全 CurrentProjectAction）
    pub current_project_audio_path: String,
}

/// 持有所有 egui UI 状态，负责每帧绘制
pub struct UiOrchestrator {
    pub settings_open: bool,
    pub settings_category: SettingsCategory,
    pub settings_recording_shortcut: Option<ShortcutAction>,
    pub docs_open: bool,
    pub docs_category: DocsCategory,
    pub create_project_state: CreateProjectState,
    pub current_project_state: CurrentProjectState,
    pub prop_edit_state: PropertyEditState,
    egui_fonts_ready: bool,
}

impl UiOrchestrator {
    pub fn new() -> Self {
        Self {
            settings_open: false,
            settings_category: SettingsCategory::Display,
            settings_recording_shortcut: None,
            docs_open: false,
            docs_category: DocsCategory::Operations,
            create_project_state: CreateProjectState::new(),
            current_project_state: CurrentProjectState::new(),
            prop_edit_state: PropertyEditState::default(),
            egui_fonts_ready: false,
        }
    }

    /// 每帧调用，绘制所有 egui UI 并返回输出
    pub fn draw(
        &mut self,
        editor: &mut FallingGroundEditor,
        audio: &mut AudioController,
        i18n: &I18n,
        info_toasts: &mut InfoToastManager,
    ) -> UiOutput {
        let ui_scale = ui_scale_factor();
        let menu_height = EGUI_MENU_BASE_HEIGHT * ui_scale;
        let top_bar_height = TOP_BAR_HEIGHT * ui_scale;
        let mut note_panel_width_px = NOTE_PANEL_BASE_WIDTH_POINTS * ui_scale;
        let mut egui_wheel_y = 0.0_f32;
        let mut total_right_panels_px = note_panel_width_px;

        let mut top_menu_result = TopMenuResult { action: None, any_popup_open: false };
        let mut egui_wants_pointer = false;
        let mut egui_wants_keyboard = false;
        let mut open_project_result: Option<(String, String)> = None;
        let mut create_project_result: Option<CreateProjectParams> = None;
        let mut current_project_action: Option<CurrentProjectAction> = None;

        let settings_open = &mut self.settings_open;
        let settings_category = &mut self.settings_category;
        let settings_recording_shortcut = &mut self.settings_recording_shortcut;
        let docs_open = &mut self.docs_open;
        let docs_category = &mut self.docs_category;
        let create_project_state = &mut self.create_project_state;
        let current_project_state = &mut self.current_project_state;
        let prop_edit_state = &mut self.prop_edit_state;
        let egui_fonts_ready = &mut self.egui_fonts_ready;

        egui_macroquad::ui(|ctx| {
            if !*egui_fonts_ready {
                let _ = init_egui_fonts(ctx);
                *egui_fonts_ready = true;
            }
            ctx.set_pixels_per_point(ui_scale);
            top_menu_result = draw_top_menu(
                ctx,
                i18n,
                editor.render_scope(),
                settings_open,
                docs_open,
            );
            if *settings_open {
                if let Some(settings_action) = draw_settings_window(
                    ctx,
                    i18n,
                    settings_open,
                    settings_category,
                    settings_recording_shortcut,
                    audio.has_player(),
                    editor.min_scroll_speed(),
                    editor.max_scroll_speed(),
                    editor.scroll_speed_step(),
                ) {
                    top_menu_result.action = Some(settings_action);
                }
            }
            if *docs_open {
                draw_docs_window(ctx, i18n, docs_open, docs_category);
            }
            note_panel_width_px = draw_note_selector_panel(ctx, i18n, editor, prop_edit_state, info_toasts);
            let snap_slider_interactive = !*settings_open
                && !*docs_open
                && !create_project_state.open
                && !current_project_state.open
                && !top_menu_result.any_popup_open;
            let snap_panel_px = draw_snap_slider_panel(
                ctx,
                editor,
                note_panel_width_px,
                menu_height + top_bar_height + 4.0 * ui_scale,
                snap_slider_interactive,
            );
            total_right_panels_px = note_panel_width_px + snap_panel_px;
            egui_wheel_y = ctx.input(|i| i.raw_scroll_delta.y);
            create_project_result = draw_create_project_window(ctx, i18n, create_project_state);
            current_project_action = draw_current_project_window(ctx, i18n, current_project_state);
            {
                let mut debug_audio = settings().debug_audio;
                if debug_audio {
                    let snapshot = audio.debug_snapshot();
                    draw_audio_debug_window(ctx, &mut debug_audio, &snapshot);
                    if !debug_audio {
                        modify_settings(|s| s.debug_audio = false);
                    }
                }
            }
            let raw_egui_pointer = ctx.is_using_pointer()
                || ctx.is_pointer_over_area()
                || top_menu_result.any_popup_open;
            egui_wants_pointer = raw_egui_pointer;
            egui_wants_keyboard = ctx.wants_keyboard_input()
                || top_menu_result.any_popup_open;
        });
        set_pointer_blocked(egui_wants_pointer);
        set_keyboard_blocked(egui_wants_keyboard);

        // Handle CreateProject action
        if top_menu_result.action == Some(TopMenuAction::File(FileAction::CreateProject)) {
            self.create_project_state.reset();
            self.create_project_state.open = true;
            top_menu_result.action = None;
        }

        // Handle CurrentProject action
        if top_menu_result.action == Some(TopMenuAction::File(FileAction::CurrentProject)) {
            let cp = editor.chart_path().to_string();
            self.current_project_state.chart_path = cp.clone();
            self.current_project_state.audio_path = audio.track_path().unwrap_or("").to_string();
            self.current_project_state.project_dir = std::path::Path::new(&cp)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            self.current_project_state.open = true;
            top_menu_result.action = None;
        }

        // Handle OpenProject action (with audio pause/resume)
        if top_menu_result.action == Some(TopMenuAction::File(FileAction::OpenProject)) {
            let was_playing = audio.pause_if_playing(i18n);
            open_project_result = Self::pick_open_project(info_toasts);
            audio.resume_if_was_playing(was_playing, i18n);
            top_menu_result.action = None;
        }

        // Handle CreateProject browse audio request
        if self.create_project_state.browse_audio_requested {
            self.create_project_state.browse_audio_requested = false;
            let was_playing = audio.pause_if_playing(i18n);
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", &["ogg", "mp3", "wav", "flac"])
                .pick_file()
            {
                self.create_project_state.audio_path = Some(path.to_string_lossy().to_string());
            }
            audio.resume_if_was_playing(was_playing, i18n);
        }

        // Handle CurrentProject browse chart request
        if self.current_project_state.browse_chart_requested {
            self.current_project_state.browse_chart_requested = false;
            let was_playing = audio.pause_if_playing(i18n);
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("SPC Chart", &["spc"])
                .pick_file()
            {
                let src = path.to_string_lossy().to_string();
                match copy_file_to_project(&src, &self.current_project_state.project_dir) {
                    Ok(dest) => {
                        self.current_project_state.chart_path = dest.clone();
                        current_project_action = Some(CurrentProjectAction::LoadChart(dest));
                    }
                    Err(_) => {
                        self.current_project_state.chart_path = src.clone();
                        current_project_action = Some(CurrentProjectAction::LoadChart(src));
                    }
                }
            }
            audio.resume_if_was_playing(was_playing, i18n);
        }

        // Handle CurrentProject browse audio request
        if self.current_project_state.browse_audio_requested {
            self.current_project_state.browse_audio_requested = false;
            let was_playing = audio.pause_if_playing(i18n);
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", &["ogg", "mp3", "wav", "flac"])
                .pick_file()
            {
                let src = path.to_string_lossy().to_string();
                match copy_file_to_project(&src, &self.current_project_state.project_dir) {
                    Ok(dest) => {
                        self.current_project_state.audio_path = dest.clone();
                        current_project_action = Some(CurrentProjectAction::LoadAudio(dest));
                    }
                    Err(_) => {
                        self.current_project_state.audio_path = src.clone();
                        current_project_action = Some(CurrentProjectAction::LoadAudio(src));
                    }
                }
            }
            audio.resume_if_was_playing(was_playing, i18n);
        }

        UiOutput {
            menu_action: top_menu_result.action,
            open_project: open_project_result,
            create_project: create_project_result,
            current_project_action,
            egui_wants_pointer,
            note_panel_width_px,
            total_right_panels_px,
            egui_wheel_y,
            ui_scale,
            menu_height,
            top_bar_height,
            current_project_chart_path: self.current_project_state.chart_path.clone(),
            current_project_audio_path: self.current_project_state.audio_path.clone(),
        }
    }

    /// 打开项目文件对话框，解析 .iffproj
    fn pick_open_project(info_toasts: &mut InfoToastManager) -> Option<(String, String)> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("IFF Project", &["iffproj"])
            .pick_file()
        {
            let proj_dir = path.parent().unwrap_or(std::path::Path::new("."));
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(json) => {
                            let chart = json.get("chart_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let audio_val = json.get("audio_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                            if let (Some(cp_raw), Some(ap_raw)) = (chart, audio_val) {
                                let cp_path = std::path::Path::new(&cp_raw);
                                let ap_path = std::path::Path::new(&ap_raw);
                                let cp = if cp_path.is_absolute() { cp_raw } else { proj_dir.join(cp_path).to_string_lossy().to_string() };
                                let ap = if ap_path.is_absolute() { ap_raw } else { proj_dir.join(ap_path).to_string_lossy().to_string() };
                                return Some((cp, ap));
                            } else {
                                info_toasts.push_warn("iffproj 文件缺少 chart_path 或 audio_path 字段");
                            }
                        }
                        Err(e) => info_toasts.push_warn(format!("解析 iffproj 失败: {e}")),
                    }
                }
                Err(e) => info_toasts.push_warn(format!("读取 iffproj 失败: {e}")),
            }
        }
        None
    }
}
