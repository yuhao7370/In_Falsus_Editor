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
use ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, PropertyEditState, draw_note_selector_panel, draw_snap_slider_panel};
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::{BASE_HEIGHT, BASE_WIDTH, ui_scale_factor};
use ui::input_state::{set_pointer_blocked, safe_mouse_wheel, free_mouse_wheel};
use ui::top_menu::{TopMenuAction, TopMenuResult, draw_top_menu};
use ui::settings_window::{SettingsCategory, draw_settings_window};
use ui::open_project_window::{OpenProjectState, draw_open_project_window};

const TOP_BAR_HEIGHT: f32 = 32.0;
const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;
const DEFAULT_CHART_PATH: &str = "songs/alamode/alamode3.spc";
const DEFAULT_AUDIO_TRACK_PATH: &str = "songs/alamode/music.ogg";

// const DEFAULT_CHART_PATH: &str = "testchart/grievouslady2.spc";
// const DEFAULT_AUDIO_TRACK_PATH: &str = "testchart/music.ogg";

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
    info_toasts: &mut InfoToastManager,
) {
    match action {
        TopMenuAction::CreateProject => {
            audio.status = i18n.t(TextKey::ActionCreateProject).to_owned();
        }
        TopMenuAction::OpenProject => {
            // Handled separately in main loop (opens window)
            audio.status.clear();
        }
        TopMenuAction::SaveChart => {
            match editor.save_chart() {
                Ok(()) => {
                    audio.status = format!("谱面已保存: {}", editor.chart_path());
                }
                Err(e) => {
                    audio.status = format!("保存失败: {e}");
                }
            }
        }
        TopMenuAction::Undo => {
            if !editor.undo() {
                info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo));
            }
            audio.status = i18n.t(TextKey::ActionUndo).to_owned();
        }
        TopMenuAction::Redo => {
            if !editor.redo() {
                info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo));
            }
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
        TopMenuAction::SetXSplit(value) => {
            editor.set_x_split(value);
            audio.status = format!("{}: {}", i18n.t(TextKey::SettingsXSplit), value);
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
    let mut open_project_state = OpenProjectState::new();
    let mut prop_edit_state = PropertyEditState::default();
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
        let mut open_project_result: Option<(String, String)> = None;
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
                    editor.x_split(),
                ) {
                    top_menu_result.action = Some(settings_action);
                }
            }
            note_panel_width_px = draw_note_selector_panel(ctx, &i18n, &mut editor, &mut prop_edit_state, &mut info_toasts);
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
            // Draw open project window (if open)
            open_project_result = draw_open_project_window(ctx, &i18n, &mut open_project_state);
            // Check if pointer is over egui widgets/panels.
            let raw_egui_pointer = ctx.is_using_pointer()
                || ctx.is_pointer_over_area()
                || top_menu_result.any_popup_open;
            egui_wants_pointer = raw_egui_pointer;
        });
        set_pointer_blocked(egui_wants_pointer);

        // Handle OpenProject action: open the project window
        if top_menu_result.action == Some(TopMenuAction::OpenProject) {
            open_project_state.open = true;
            open_project_state.chart_path = None;
            open_project_state.audio_path = None;
            top_menu_result.action = None; // consume it
        }

        if let Some(action) = top_menu_result.action {
            audio.status.clear();
            handle_top_menu_action(action, &mut editor, &mut audio, &mut i18n, &mut info_toasts);
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        // Handle open project result
        if let Some((chart_path, audio_path)) = open_project_result {
            let font_backup = macroquad_font.clone();
            editor = FallingGroundEditor::from_chart_path(&chart_path);
            editor.set_text_font(font_backup);
            audio.load_audio_file(&audio_path, &i18n);
            info_toasts.push(format!("项目已加载: {}", chart_path));
        }

        // Ctrl+S: save chart, Ctrl+Z: undo, Ctrl+Y: redo
        {
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::S) {
                match editor.save_chart() {
                    Ok(()) => {
                        info_toasts.push(format!("谱面已保存: {}", editor.chart_path()));
                    }
                    Err(e) => {
                        info_toasts.push(format!("保存失败: {e}"));
                    }
                }
            }
            if ctrl && is_key_pressed(KeyCode::Z) {
                if editor.undo() {
                    info_toasts.push(i18n.t(TextKey::ActionUndo));
                } else {
                    info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo));
                }
            }
            if ctrl && is_key_pressed(KeyCode::Y) {
                if editor.redo() {
                    info_toasts.push(i18n.t(TextKey::ActionRedo));
                } else {
                    info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo));
                }
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
