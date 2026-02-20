mod app;
mod audio;
mod chart;
mod editor;
mod i18n;
mod settings;
mod shortcuts;
mod ui;

use app::constants::*;
use app::input_handler;
use app::menu_actions::handle_top_menu_action;
use app::project_manager::ProjectManager;
use app::render_mode::RenderModePresenter;
use app::setup::{apply_settings_to_editor, window_conf};
use app::ui_orchestrator::UiOrchestrator;
use audio::controller::AudioController;
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::I18n;
use macroquad::prelude::*;
use settings::settings;
use ui::fonts::load_macroquad_cjk_font;
use ui::info_toast::InfoToastManager;
use ui::input_state::{
    free_key_pressed, safe_key_pressed, set_keyboard_blocked, set_pointer_blocked,
};
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::refresh_ui_scale;

#[macroquad::main(window_conf)]
async fn main() {
    let mut i18n = I18n::from_settings(&settings().language);

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
    let mut render_mode = RenderModePresenter::new();
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    apply_settings_to_editor(&mut editor, &mut audio, &i18n);
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }

    loop {
        refresh_ui_scale();
        render_mode.begin_frame_capture();
        clear_background(Color::from_rgba(7, 7, 10, 255));

        // 1. Tick audio
        audio.tick(&i18n);

        if render_mode.is_render_3d() {
            let back_to_2d_requested = free_key_pressed(KeyCode::P);
            // 3D mode is render-only: no egui pipeline or editor input.
            set_pointer_blocked(true);
            set_keyboard_blocked(true);

            let _ = audio.handle_keyboard(&i18n);
            project_mgr.tick_and_apply(
                &mut editor,
                &mut audio,
                &i18n,
                &mut info_toasts,
                &macroquad_font,
            );

            let frame_ctx = audio.frame_snapshot();
            let ui_scale = ui::scale::ui_scale_factor();
            let edge_pad = 18.0 * ui_scale;
            let editor_rect = Rect::new(
                edge_pad,
                edge_pad,
                (screen_width() - edge_pad * 2.0).max(360.0),
                (screen_height() - edge_pad * 2.0).max(140.0),
            );
            for action in editor.draw(editor_rect, &frame_ctx) {
                match action {
                    FallingEditorAction::MinimapSeekTo(sec) => {
                        audio.handle_editor_seek(sec, &i18n);
                    }
                }
            }
            for (msg, is_warn) in editor.drain_toasts() {
                if is_warn {
                    info_toasts.push_warn(&msg);
                } else {
                    info_toasts.push(&msg);
                }
            }

            {
                let note_heads = editor.note_head_times();
                audio.trigger_hitsounds(&note_heads);
            }
            info_toasts.draw(ui_scale, 0.0, macroquad_font.as_ref());

            render_mode.present();
            if back_to_2d_requested {
                let mode = render_mode.toggle_mode();
                info_toasts.push(format!("view mode: {}", mode.label()));
            }
            next_frame().await;
            continue;
        }

        // 2. UI draw (egui)
        let ui_output = ui.draw(&mut editor, &mut audio, &i18n, &mut info_toasts);
        let keyboard_shortcut_blocked =
            ui_output.egui_wants_keyboard || ui_output.shortcut_capture_active;
        let switch_to_3d_requested = !keyboard_shortcut_blocked && safe_key_pressed(KeyCode::P);
        let space_consumed = if keyboard_shortcut_blocked {
            false
        } else {
            audio.handle_keyboard(&i18n)
        };

        // 3. Menu actions
        if let Some(ref action) = ui_output.menu_action {
            audio.status.clear();
            handle_top_menu_action(
                action.clone(),
                &mut editor,
                &mut audio,
                &mut i18n,
                &mut info_toasts,
            );
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        // 4. Project loading
        project_mgr.handle_ui_actions(&ui_output, &mut info_toasts);
        project_mgr.tick_and_apply(
            &mut editor,
            &mut audio,
            &i18n,
            &mut info_toasts,
            &macroquad_font,
        );

        // 5. Shortcuts & wheel
        if !keyboard_shortcut_blocked {
            input_handler::handle_shortcuts(&mut editor, &mut audio, &i18n, &mut info_toasts);
        }
        input_handler::handle_wheel(
            &mut editor,
            &mut audio,
            &i18n,
            &mut info_toasts,
            space_consumed,
            &ui_output,
        );

        // 6. Read audio frame context and compute layout
        let mut frame_ctx = audio.frame_snapshot();
        let ui_scale = ui_output.ui_scale;
        let menu_height = ui_output.menu_height;
        let top_bar_height = ui_output.top_bar_height;
        let panel_pad = 10.0 * ui_scale;
        let editor_gap = 12.0 * ui_scale;
        let editor_bottom_pad = 8.0 * ui_scale;
        let editor_width =
            (screen_width() - panel_pad * 2.0 - ui_output.total_right_panels_px - editor_gap)
                .max(360.0);

        // 7. Top progress bar
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
        set_pointer_blocked(ui_output.egui_wants_pointer || progress_output.blocks_editor_pointer);

        // 8. Editor draw
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - editor_bottom_pad).max(140.0),
        );
        for action in editor.draw(editor_rect, &frame_ctx) {
            match action {
                FallingEditorAction::MinimapSeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                }
            }
        }
        for (msg, is_warn) in editor.drain_toasts() {
            if is_warn {
                info_toasts.push_warn(&msg);
            } else {
                info_toasts.push(&msg);
            }
        }

        // 9. Hitsound
        {
            let note_heads = editor.note_head_times();
            audio.trigger_hitsounds(&note_heads);
        }

        // 10. Toast
        info_toasts.draw(
            ui_scale,
            menu_height + top_bar_height,
            macroquad_font.as_ref(),
        );

        render_mode.present();
        egui_macroquad::draw();
        if switch_to_3d_requested {
            let mode = render_mode.toggle_mode();
            info_toasts.push(format!("view mode: {}", mode.label()));
        }
        next_frame().await;
    }
}
