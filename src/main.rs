mod audio;
mod chart;
mod editor;
mod i18n;
mod ui;

use audio::controller::AudioController;
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::{I18n, Language, TextKey};
use macroquad::prelude::*;
use ui::fonts::{init_egui_fonts, load_macroquad_cjk_font};
use ui::info_toast::InfoToastManager;
use ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, draw_note_selector_panel, draw_snap_slider_panel};
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::{BASE_HEIGHT, BASE_WIDTH, ui_scale_factor};
use ui::input_state::{set_pointer_blocked, safe_mouse_wheel, free_mouse_wheel};
use ui::top_menu::{TopMenuAction, TopMenuResult, draw_top_menu};
use ui::settings_window::{SettingsCategory, draw_settings_window};

const TOP_BAR_HEIGHT: f32 = 32.0;
const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;
// const DEFAULT_CHART_PATH: &str = "songs/alamode/alamode3.spc";
// const DEFAULT_AUDIO_TRACK_PATH: &str = "songs/alamode/music.ogg";

const DEFAULT_CHART_PATH: &str = "grievouslady2.spc";
const DEFAULT_AUDIO_TRACK_PATH: &str = "music.ogg";

// const DEFAULT_CHART_PATH: &str = "astralquant_infalsus/astralquant2.spc";
// const DEFAULT_AUDIO_TRACK_PATH: &str = "astralquant_infalsus/music.ogg";


fn window_conf() -> Conf {
    Conf {
        window_title: "In Falsus Editor".to_owned(),
        window_width: BASE_WIDTH as i32,
        window_height: BASE_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

fn handle_top_menu_action(
    action: TopMenuAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &mut I18n,
) {
    match action {
        TopMenuAction::CreateProject => {
            audio.status = i18n.t(TextKey::ActionCreateProject).to_owned();
        }
        TopMenuAction::OpenProject => {
            audio.status = i18n.t(TextKey::ActionOpenProject).to_owned();
        }
        TopMenuAction::Undo => {
            audio.status = i18n.t(TextKey::ActionUndo).to_owned();
        }
        TopMenuAction::Redo => {
            audio.status = i18n.t(TextKey::ActionRedo).to_owned();
        }
        TopMenuAction::Cut => {
            audio.status = i18n.t(TextKey::ActionCut).to_owned();
        }
        TopMenuAction::Copy => {
            audio.status = i18n.t(TextKey::ActionCopy).to_owned();
        }
        TopMenuAction::Paste => {
            audio.status = i18n.t(TextKey::ActionPaste).to_owned();
        }
        TopMenuAction::SetLanguage(language) => {
            i18n.set_language(language);
            audio.status = match language {
                Language::ZhCn => i18n.t(TextKey::ActionSetLanguageZh).to_owned(),
                Language::EnUs => i18n.t(TextKey::ActionSetLanguageEn).to_owned(),
            };
        }
        TopMenuAction::SetVolume(volume) => {
            audio.set_volume(volume, i18n);
            // Don't trigger toast for continuous volume slider drags
            audio.status.clear();
        }
        TopMenuAction::SetAutoPlay(enabled) => {
            editor.set_autoplay_enabled(enabled);
        }
        TopMenuAction::SetShowSpectrum(enabled) => {
            editor.set_show_spectrum(enabled);
        }
        TopMenuAction::SetDebugHitbox(enabled) => {
            editor.set_debug_show_hitboxes(enabled);
            audio.status = if enabled {
                i18n.t(TextKey::ActionDebugHitboxOn).to_owned()
            } else {
                i18n.t(TextKey::ActionDebugHitboxOff).to_owned()
            };
        }
        TopMenuAction::SetMinimapVisible(enabled) => {
            editor.set_show_minimap(enabled);
            audio.status = if enabled {
                i18n.t(TextKey::ActionMinimapOn).to_owned()
            } else {
                i18n.t(TextKey::ActionMinimapOff).to_owned()
            };
        }
        TopMenuAction::SetRenderScope(scope) => {
            editor.set_render_scope(scope);
        }
        TopMenuAction::SetScrollSpeed(speed) => {
            editor.set_scroll_speed(speed);
            // Slider dragging — no toast
            audio.status.clear();
        }
        TopMenuAction::SetScrollSpeedFinal(speed) => {
            editor.set_scroll_speed(speed);
            audio.status = format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), speed);
        }
        TopMenuAction::SetSnapDivision(division) => {
            editor.set_snap_division(division);
            // Dragging — no toast
            audio.status.clear();
        }
        TopMenuAction::SetSnapDivisionFinal(division) => {
            editor.set_snap_division(division);
            audio.status = format!("{}: {}x", i18n.t(TextKey::SettingsBarlineSnap), division);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut editor = FallingGroundEditor::new(DEFAULT_CHART_PATH);
    let mut i18n = I18n::detect();
    let mut egui_fonts_ready = false;
    let mut audio = AudioController::new(&i18n, DEFAULT_AUDIO_TRACK_PATH);
    let mut top_progress_state = TopProgressBarState::new();
    let mut settings_open = false;
    let mut settings_category = SettingsCategory::Display;
    let mut info_toasts = InfoToastManager::new();
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }
    info_toasts.push("Info toasts enabled. Press F8 for multi-toast test.");

    loop {
        clear_background(Color::from_rgba(7, 7, 10, 255));

        // 1. Tick audio (poll events, refresh snapshot)
        audio.tick(&i18n);

        // 2. Keyboard input (snapshot refreshed inside each action)
        let space_consumed = audio.handle_keyboard(&i18n);

        // 3. UI
        let ui_scale = ui_scale_factor();
        let menu_height = EGUI_MENU_BASE_HEIGHT * ui_scale;
        let mut note_panel_width_px = NOTE_PANEL_BASE_WIDTH_POINTS * ui_scale;
        let mut egui_wheel_y = 0.0_f32;
        let top_bar_height = TOP_BAR_HEIGHT * ui_scale;
        let panel_pad = 10.0 * ui_scale;
        let editor_gap = 12.0 * ui_scale;
        let editor_bottom_pad = 8.0 * ui_scale;

        let mut top_menu_result = TopMenuResult { action: None, any_popup_open: false };
        let mut egui_wants_pointer = false;
        let mut total_right_panels_px = note_panel_width_px;
        egui_macroquad::ui(|ctx| {
            if !egui_fonts_ready {
                let _ = init_egui_fonts(ctx);
                egui_fonts_ready = true;
            }
            ctx.set_pixels_per_point(ui_scale);
            let volume = audio.volume();
            top_menu_result = draw_top_menu(
                ctx,
                &i18n,
                editor.render_scope(),
                &mut settings_open,
            );
            // Draw settings window (if open)
            if settings_open {
                if let Some(settings_action) = draw_settings_window(
                    ctx,
                    &i18n,
                    &mut settings_open,
                    &mut settings_category,
                    volume,
                    audio.has_player(),
                    editor.debug_show_hitboxes(),
                    editor.autoplay_enabled(),
                    editor.show_spectrum(),
                    editor.show_minimap(),
                    editor.scroll_speed(),
                    editor.min_scroll_speed(),
                    editor.max_scroll_speed(),
                    editor.scroll_speed_step(),
                    editor.snap_division(),
                ) {
                    top_menu_result.action = Some(settings_action);
                }
            }
            note_panel_width_px = draw_note_selector_panel(ctx, &i18n, &mut editor);
            let snap_panel_px = draw_snap_slider_panel(
                ctx,
                &mut editor,
                note_panel_width_px,
                menu_height + top_bar_height + 4.0 * ui_scale,
            );
            // note_panel_width_px is for progress bar (excludes snap panel).
            // total_right_panels_px includes snap panel for editor width.
            total_right_panels_px = note_panel_width_px + snap_panel_px;
            egui_wheel_y = ctx.input(|i| i.raw_scroll_delta.y);
            // Check if pointer is over egui widgets/panels.
            let raw_egui_pointer = ctx.is_using_pointer()
                || ctx.is_pointer_over_area()
                || top_menu_result.any_popup_open;
            egui_wants_pointer = raw_egui_pointer;
        });
        set_pointer_blocked(egui_wants_pointer);

        if let Some(action) = top_menu_result.action {
            audio.status.clear();
            handle_top_menu_action(action, &mut editor, &mut audio, &mut i18n);
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        if is_key_pressed(KeyCode::F8) {
            info_toasts.push("Info A: multi-toast test");
            info_toasts.push("Info B: animation should be smooth");
            info_toasts.push("Info C: dismisses in queue order");
        }

        // 4. Wheel: Ctrl+wheel = flow speed (free, ignores egui block), otherwise seek
        let (_, free_wheel_y) = free_mouse_wheel();
        let (_, mq_wheel_y) = safe_mouse_wheel();
        let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if egui_wants_pointer {
            egui_wheel_y = 0.0;
        }
        if ctrl_down && free_wheel_y.abs() > f32::EPSILON {
            let step = editor.scroll_speed_step();
            let delta = if free_wheel_y > 0.0 { step } else { -step };
            editor.nudge_scroll_speed(delta);
            info_toasts.push(format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), editor.scroll_speed()));
        } else {
            audio.handle_wheel_seek(mq_wheel_y, egui_wheel_y, space_consumed, &i18n);
        }

        // Read snapshot values after input mutations this frame
        let mut current_sec = audio.current_sec();
        let duration_sec = audio.duration_sec();
        let track_path = audio.track_path().map(|s| s.to_owned());
        let is_playing = audio.is_playing();
        let editor_width =
            (screen_width() - panel_pad * 2.0 - total_right_panels_px - editor_gap).max(360.0);
        // 5. Top progress bar
        let progress_output = draw_top_progress_bar(
            ui_scale,
            menu_height,
            top_bar_height,
            note_panel_width_px,
            current_sec,
            duration_sec,
            is_playing,
            macroquad_font.as_ref(),
            &mut top_progress_state,
        );
        current_sec = progress_output.display_sec;
        if let Some(seek_sec) = progress_output.seek_to_sec {
            audio.handle_editor_seek(seek_sec, &i18n);
            current_sec = audio.current_sec();
        }

        // 6. Editor
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - editor_bottom_pad).max(140.0),
        );

        for action in editor.draw(
            editor_rect,
            current_sec,
            duration_sec,
            track_path.as_deref(),
            is_playing,
        ) {
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

        info_toasts.draw(
            ui_scale,
            menu_height + top_bar_height,
            macroquad_font.as_ref(),
        );

        egui_macroquad::draw();
        next_frame().await;
    }
}
