mod app;
mod audio;
mod chart;
mod editor;
mod i18n;
mod shortcuts;
mod settings;
mod ui;

use app::constants::*;
use app::input_handler;
use app::menu_actions::handle_top_menu_action;
use app::project_manager::ProjectManager;
use app::setup::{apply_settings_to_editor, window_conf};
use app::ui_orchestrator::UiOrchestrator;
use audio::controller::AudioController;
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::I18n;
use macroquad::prelude::*;
use settings::settings;
use ui::fonts::load_macroquad_cjk_font;
use ui::info_toast::InfoToastManager;
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::refresh_ui_scale;

#[macroquad::main(window_conf)]
async fn main() {
    let mut i18n = I18n::from_settings(&settings().language);

    // DEV_MODE: 自动加载指定谱面和音频；否则启动空编辑器
    let (mut editor, mut audio) = if DEV_MODE {
        (
            FallingGroundEditor::new(DEV_CHART_PATH),
            AudioController::new(&i18n, DEV_AUDIO_PATH),
        )
    } else {
        (
            FallingGroundEditor::from_chart_path(""),
            AudioController::new_empty(&i18n),
        )
    };

    let mut info_toasts = InfoToastManager::new();
    let mut top_progress_state = TopProgressBarState::new();
    let mut ui = UiOrchestrator::new();
    let mut project_mgr = ProjectManager::new();
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    apply_settings_to_editor(&mut editor, &mut audio, &i18n);
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }

    loop {
        clear_background(Color::from_rgba(7, 7, 10, 255));
        refresh_ui_scale();

        // 1. Tick audio
        audio.tick(&i18n);
        let space_consumed = audio.handle_keyboard(&i18n);

        // 2. UI 绘制（egui）
        let ui_output = ui.draw(&mut editor, &mut audio, &i18n, &mut info_toasts);

        // 3. 菜单动作
        if let Some(ref action) = ui_output.menu_action {
            audio.status.clear();
            handle_top_menu_action(action.clone(), &mut editor, &mut audio, &mut i18n, &mut info_toasts);
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        // 4. 项目加载
        project_mgr.handle_ui_actions(&ui_output, &mut info_toasts);
        project_mgr.tick_and_apply(&mut editor, &mut audio, &i18n, &mut info_toasts, &macroquad_font);

        // 5. 快捷键 & 滚轮
        input_handler::handle_shortcuts(&mut editor, &mut audio, &i18n, &mut info_toasts);
        input_handler::handle_wheel(&mut editor, &mut audio, &i18n, &mut info_toasts, space_consumed, &ui_output);

        // 6. 读取音频快照，计算布局
        let mut frame_ctx = audio.frame_snapshot();
        let ui_scale = ui_output.ui_scale;
        let menu_height = ui_output.menu_height;
        let top_bar_height = ui_output.top_bar_height;
        let panel_pad = 10.0 * ui_scale;
        let editor_gap = 12.0 * ui_scale;
        let editor_bottom_pad = 8.0 * ui_scale;
        let editor_width =
            (screen_width() - panel_pad * 2.0 - ui_output.total_right_panels_px - editor_gap).max(360.0);

        // 7. 顶部进度条
        let progress_output = draw_top_progress_bar(
            ui_scale,
            menu_height,
            top_bar_height,
            ui_output.note_panel_width_px,
            &frame_ctx,
            macroquad_font.as_ref(),
            &mut top_progress_state,
        );
        frame_ctx.current_sec = progress_output.display_sec;
        if let Some(seek_sec) = progress_output.seek_to_sec {
            audio.handle_editor_seek(seek_sec, &i18n);
            frame_ctx.current_sec = audio.current_sec();
        }

        // 8. 编辑器绘制
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - editor_bottom_pad).max(140.0),
        );
        for action in editor.draw(editor_rect, &frame_ctx) {
            match action {
                FallingEditorAction::SeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                    info_toasts.push(format!("seek {:.2}s", sec));
                }
                FallingEditorAction::MinimapSeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                }
            }
        }
        for (msg, is_warn) in editor.drain_toasts() {
            if is_warn { info_toasts.push_warn(&msg); } else { info_toasts.push(&msg); }
        }

        // 9. Hitsound
        {
            let note_heads = editor.note_head_times();
            audio.trigger_hitsounds(&note_heads);
        }

        // 10. Toast 通知
        info_toasts.draw(ui_scale, menu_height + top_bar_height, macroquad_font.as_ref());

        egui_macroquad::draw();
        next_frame().await;
    }
}
