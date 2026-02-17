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
use ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, draw_note_selector_panel};
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::{BASE_HEIGHT, BASE_WIDTH, ui_scale_factor};
use ui::top_menu::{TopMenuAction, draw_top_menu};

const TOP_BAR_HEIGHT: f32 = 32.0;
const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;

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
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut editor = FallingGroundEditor::new();
    let mut i18n = I18n::detect();
    let mut egui_fonts_ready = false;
    let mut audio = AudioController::new(&i18n);
    let mut top_progress_state = TopProgressBarState::new();
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

        let mut top_menu_action = None;
        egui_macroquad::ui(|ctx| {
            if !egui_fonts_ready {
                let _ = init_egui_fonts(ctx);
                egui_fonts_ready = true;
            }
            ctx.set_pixels_per_point(ui_scale);
            let volume = audio.volume();
            top_menu_action = draw_top_menu(
                ctx,
                &i18n,
                volume,
                audio.has_player(),
                editor.debug_show_hitboxes(),
                editor.show_minimap(),
            );
            note_panel_width_px = draw_note_selector_panel(ctx, &mut editor);
            egui_wheel_y = ctx.input(|i| i.raw_scroll_delta.y);
        });

        if let Some(action) = top_menu_action {
            handle_top_menu_action(action, &mut editor, &mut audio, &mut i18n);
            info_toasts.push(audio.status.clone());
        }

        if is_key_pressed(KeyCode::F8) {
            info_toasts.push("Info A: multi-toast test");
            info_toasts.push("Info B: animation should be smooth");
            info_toasts.push("Info C: dismisses in queue order");
        }

        // 4. Wheel seek
        let (_, mq_wheel_y) = mouse_wheel();
        audio.handle_wheel_seek(mq_wheel_y, egui_wheel_y, space_consumed, &i18n);

        // Read snapshot values after input mutations this frame
        let mut current_sec = audio.current_sec();
        let duration_sec = audio.duration_sec();
        let track_path = audio.track_path().map(|s| s.to_owned());
        let is_playing = audio.is_playing();
        let editor_width =
            (screen_width() - panel_pad * 2.0 - note_panel_width_px - editor_gap).max(360.0);
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
