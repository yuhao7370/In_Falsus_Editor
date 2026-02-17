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
use ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, draw_note_selector_panel};
use ui::top_menu::{TopMenuAction, draw_top_menu};

const BASE_WIDTH: f32 = 1366.0;
const BASE_HEIGHT: f32 = 768.0;
const TOP_BAR_HEIGHT: f32 = 56.0;
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

fn format_time(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00.00".to_owned();
    }
    let minutes = (seconds / 60.0).floor() as i32;
    let sec = seconds % 60.0;
    format!("{minutes:02}:{sec:05.2}")
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

fn ui_scale_factor() -> f32 {
    (screen_width() / BASE_WIDTH)
        .min(screen_height() / BASE_HEIGHT)
        .clamp(0.75, 3.5)
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut editor = FallingGroundEditor::new();
    let mut i18n = I18n::detect();
    let mut egui_fonts_ready = false;
    let mut audio = AudioController::new(&i18n);
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }

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
        let status_bottom_pad = 8.0 * ui_scale;

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
        }

        // 4. Wheel seek
        let (_, mq_wheel_y) = mouse_wheel();
        audio.handle_wheel_seek(mq_wheel_y, egui_wheel_y, space_consumed, &i18n);

        // Read final snapshot values AFTER all mutations this frame
        let current_sec = audio.current_sec();
        let duration_sec = audio.duration_sec();
        let track_path = audio.track_path().map(|s| s.to_owned());

        // 5. Top bar
        let top_bar = Rect::new(0.0, menu_height, screen_width(), top_bar_height);
        draw_rectangle(
            top_bar.x,
            top_bar.y,
            top_bar.w,
            top_bar.h,
            Color::from_rgba(15, 15, 20, 255),
        );
        draw_line(
            0.0,
            top_bar.y + top_bar.h,
            screen_width(),
            top_bar.y + top_bar.h,
            (1.0 * ui_scale).max(1.0),
            Color::from_rgba(40, 40, 50, 255),
        );

        draw_text_ex(
            &format!(
                "{} / {}",
                format_time(current_sec),
                format_time(duration_sec)
            ),
            panel_pad,
            menu_height + 33.0 * ui_scale,
            TextParams {
                font: macroquad_font.as_ref(),
                font_size: (22.0 * ui_scale).round().clamp(14.0, 84.0) as u16,
                color: Color::from_rgba(236, 236, 242, 255),
                ..Default::default()
            },
        );

        draw_text_ex(
            "Keys: Space Play/Pause | <- -> seek 1s | Up/Down seek 0.1s | Wheel seek | Ctrl=finer",
            panel_pad,
            menu_height + 52.0 * ui_scale,
            TextParams {
                font: macroquad_font.as_ref(),
                font_size: (16.0 * ui_scale).round().clamp(11.0, 60.0) as u16,
                color: Color::from_rgba(170, 170, 185, 255),
                ..Default::default()
            },
        );

        // 6. Editor
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_width =
            (screen_width() - panel_pad * 2.0 - note_panel_width_px - editor_gap).max(360.0);
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - 28.0 * ui_scale).max(140.0),
        );

        for action in editor.draw(
            editor_rect,
            current_sec,
            duration_sec,
            track_path.as_deref(),
            audio.is_playing(),
        ) {
            match action {
                FallingEditorAction::SeekTo(sec) => audio.handle_editor_seek(sec, &i18n),
            }
        }

        // 7. Status bar
        draw_text_ex(
            &audio.status,
            panel_pad,
            screen_height() - status_bottom_pad,
            TextParams {
                font: macroquad_font.as_ref(),
                font_size: (18.0 * ui_scale).round().clamp(12.0, 64.0) as u16,
                color: Color::from_rgba(170, 205, 255, 255),
                ..Default::default()
            },
        );

        egui_macroquad::draw();
        next_frame().await;
    }
}
