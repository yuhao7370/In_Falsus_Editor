mod audio;
mod chart;
mod editor;
mod i18n;
mod ui;

use audio::player::{PlaybackState, PlayerError, PlayerEvent, SongPlayer, StopReason};
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::{I18n, Language, TextKey};
use macroquad::prelude::*;
use ui::fonts::init_egui_fonts;
use ui::note_panel::{draw_note_selector_panel, NOTE_PANEL_BASE_WIDTH_POINTS};
use ui::top_menu::{draw_top_menu, TopMenuAction};

const BASE_WIDTH: f32 = 1366.0;
const BASE_HEIGHT: f32 = 768.0;
const TOP_BAR_HEIGHT: f32 = 56.0;
const EGUI_MENU_BASE_HEIGHT: f32 = 42.0;
const WHEEL_SEEK_DIV_DEFAULT: f32 = 12_000.0;
const WHEEL_SEEK_DIV_CTRL: f32 = 60_000.0;
const WHEEL_SEEK_DIV_ALT: f32 = 3_000.0;
const WHEEL_SEEK_SPEED_MULT: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
enum PlayerUiAction {
    TogglePlayPause,
    Seek(f32),
    SetVolume(f32),
}

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

fn format_player_error(error: &PlayerError, i18n: &I18n) -> String {
    match error {
        PlayerError::BackendInit(_) => i18n.with_detail(TextKey::StatusInitAudioFailed, error),
        PlayerError::Io { .. } => i18n.with_detail(TextKey::StatusReadAudioFailed, error),
        PlayerError::Decode(_) => i18n.with_detail(TextKey::StatusDecodeAudioFailed, error),
        PlayerError::CreateMusic(_) => i18n.with_detail(TextKey::StatusCreateMusicFailed, error),
        PlayerError::StartPlayback(_) | PlayerError::PausePlayback(_) => {
            i18n.with_detail(TextKey::StatusStartPlaybackFailed, error)
        }
        PlayerError::Seek(_) | PlayerError::InvalidSeek { .. } => {
            i18n.with_detail(TextKey::StatusSeekFailed, error)
        }
        PlayerError::NoTrackLoaded | PlayerError::SetVolume(_) => {
            i18n.with_detail(TextKey::StatusAudioUnavailable, error)
        }
        PlayerError::BackendRecover(_) => i18n.with_detail(TextKey::StatusBackendError, error),
    }
}

fn format_player_event(event: PlayerEvent, i18n: &I18n) -> String {
    match event {
        PlayerEvent::Loaded { path, duration_sec } => {
            format!("{}: {} ({duration_sec:.2}s)", i18n.t(TextKey::StatusLoaded), path)
        }
        PlayerEvent::Started => i18n.t(TextKey::StatusPlaying).to_owned(),
        PlayerEvent::Paused => i18n.t(TextKey::StatusPaused).to_owned(),
        PlayerEvent::Stopped(StopReason::User) => i18n.t(TextKey::StatusStopped).to_owned(),
        PlayerEvent::Stopped(StopReason::EndOfTrack) => {
            i18n.t(TextKey::StatusPlaybackEnded).to_owned()
        }
        PlayerEvent::BackendRecovered => i18n.t(TextKey::StatusBackendRecovered).to_owned(),
        PlayerEvent::Error(error) => format_player_error(&error, i18n),
    }
}

fn apply_player_action(player: &mut SongPlayer, action: PlayerUiAction, i18n: &I18n) -> Option<String> {
    let result = match action {
        PlayerUiAction::TogglePlayPause => player.toggle_play_pause(),
        PlayerUiAction::Seek(position_sec) => player.seek_to(position_sec),
        PlayerUiAction::SetVolume(volume) => player.set_volume(volume),
    };

    match result {
        Ok(()) => match action {
            PlayerUiAction::SetVolume(volume) => Some(format!(
                "{}: {:.0}%",
                i18n.t(TextKey::StatusVolumeUpdated),
                volume.clamp(0.0, 1.0) * 100.0
            )),
            _ => None,
        },
        Err(error) => Some(format_player_error(&error, i18n)),
    }
}

fn handle_top_menu_action(
    action: TopMenuAction,
    i18n: &mut I18n,
) -> (Option<String>, Option<PlayerUiAction>) {
    match action {
        TopMenuAction::CreateProject => (Some(i18n.t(TextKey::ActionCreateProject).to_owned()), None),
        TopMenuAction::OpenProject => (Some(i18n.t(TextKey::ActionOpenProject).to_owned()), None),
        TopMenuAction::Undo => (Some(i18n.t(TextKey::ActionUndo).to_owned()), None),
        TopMenuAction::Redo => (Some(i18n.t(TextKey::ActionRedo).to_owned()), None),
        TopMenuAction::Cut => (Some(i18n.t(TextKey::ActionCut).to_owned()), None),
        TopMenuAction::Copy => (Some(i18n.t(TextKey::ActionCopy).to_owned()), None),
        TopMenuAction::Paste => (Some(i18n.t(TextKey::ActionPaste).to_owned()), None),
        TopMenuAction::SetLanguage(language) => {
            i18n.set_language(language);
            let msg = match language {
                Language::ZhCn => i18n.t(TextKey::ActionSetLanguageZh).to_owned(),
                Language::EnUs => i18n.t(TextKey::ActionSetLanguageEn).to_owned(),
            };
            (Some(msg), None)
        }
        TopMenuAction::SetVolume(volume) => (
            Some(format!(
                "{}: {:.0}%",
                i18n.t(TextKey::StatusVolumeUpdated),
                volume.clamp(0.0, 1.0) * 100.0
            )),
            Some(PlayerUiAction::SetVolume(volume)),
        ),
    }
}

fn ui_scale_factor() -> f32 {
    (screen_width() / BASE_WIDTH)
        .min(screen_height() / BASE_HEIGHT)
        .clamp(0.75, 2.0)
}

fn normalize_wheel_units(raw: f32) -> f32 {
    // RotaenoChartTool_rs 使用的是“120 为一格”的轮值语义。
    // macroquad 在不同平台可能返回 1 或 120，这里做兼容归一。
    if raw.abs() <= 10.0 {
        raw * 120.0
    } else {
        raw
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut editor = FallingGroundEditor::new();
    let mut i18n = I18n::detect();
    let mut egui_fonts_ready = false;

    let (mut player, mut status_message) = match SongPlayer::new() {
        Ok(mut player) => {
            let status = if let Err(error) = player.load_default(false) {
                format_player_error(&error, &i18n)
            } else {
                format!(
                    "{}: {}",
                    i18n.t(TextKey::StatusLoaded),
                    SongPlayer::default_track_path()
                )
            };
            (Some(player), status)
        }
        Err(error) => (None, format_player_error(&error, &i18n)),
    };

    loop {
        clear_background(Color::from_rgba(7, 7, 10, 255));

        if let Some(player_ref) = player.as_mut() {
            if let Some(event) = player_ref.update() {
                status_message = format_player_event(event, &i18n);
            }
        }

        let snapshot = player.as_mut().map(SongPlayer::snapshot);
        let current_sec = snapshot.as_ref().map(|s| s.position_sec).unwrap_or(0.0);
        let duration_sec = snapshot.as_ref().map(|s| s.duration_sec).unwrap_or(0.0);
        let track_path = snapshot.as_ref().and_then(|s| s.track_path.as_deref());

        let mut actions = Vec::new();

        if is_key_pressed(KeyCode::Space) {
            actions.push(PlayerUiAction::TogglePlayPause);
        }
        if let Some(s) = snapshot.as_ref() {
            if is_key_pressed(KeyCode::Left) {
                actions.push(PlayerUiAction::Seek((s.position_sec - 1.0).max(0.0)));
            }
            if is_key_pressed(KeyCode::Right) {
                actions.push(PlayerUiAction::Seek((s.position_sec + 1.0).min(s.duration_sec)));
            }
            if is_key_pressed(KeyCode::Up) {
                actions.push(PlayerUiAction::Seek((s.position_sec - 0.1).max(0.0)));
            }
            if is_key_pressed(KeyCode::Down) {
                actions.push(PlayerUiAction::Seek((s.position_sec + 0.1).min(s.duration_sec)));
            }
        }

        let ui_scale = ui_scale_factor();
        let menu_height = EGUI_MENU_BASE_HEIGHT * ui_scale;
        let mut note_panel_width_px = NOTE_PANEL_BASE_WIDTH_POINTS * ui_scale;
        let mut egui_wheel_y = 0.0_f32;

        let mut top_menu_action = None;
        egui_macroquad::ui(|ctx| {
            if !egui_fonts_ready {
                let _ = init_egui_fonts(ctx);
                egui_fonts_ready = true;
            }
            ctx.set_pixels_per_point(ui_scale);
            let volume = snapshot.as_ref().map(|s| s.volume).unwrap_or(1.0);
            top_menu_action = draw_top_menu(ctx, &i18n, volume, player.is_some());
            note_panel_width_px = draw_note_selector_panel(ctx, &mut editor);
            egui_wheel_y = ctx.input(|i| i.raw_scroll_delta.y);
        });

        if let Some(action) = top_menu_action {
            let (message, player_action) = handle_top_menu_action(action, &mut i18n);
            if let Some(player_action) = player_action {
                actions.push(player_action);
            }
            if let Some(message) = message {
                status_message = message;
            }
        }

        if let Some(s) = snapshot.as_ref() {
            let (_, mq_wheel_y) = mouse_wheel();
            let wheel_units = if mq_wheel_y.abs() > f32::EPSILON {
                normalize_wheel_units(mq_wheel_y)
            } else {
                normalize_wheel_units(egui_wheel_y)
            };
            if s.can_seek
                && s.state != PlaybackState::Playing
                && wheel_units.abs() > f32::EPSILON
            {
                let denom = if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
                    WHEEL_SEEK_DIV_CTRL
                } else if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
                    WHEEL_SEEK_DIV_ALT
                } else {
                    WHEEL_SEEK_DIV_DEFAULT
                };
                let delta = (wheel_units / denom) * WHEEL_SEEK_SPEED_MULT;
                let target = (s.position_sec + delta).clamp(0.0, s.duration_sec);
                actions.push(PlayerUiAction::Seek(target));
            }
        }

        let top_bar = Rect::new(0.0, menu_height, screen_width(), TOP_BAR_HEIGHT);
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
            1.0,
            Color::from_rgba(40, 40, 50, 255),
        );

        draw_text_ex(
            &format!(
                "{} / {}",
                format_time(current_sec),
                format_time(duration_sec)
            ),
            10.0,
            menu_height + 33.0,
            TextParams {
                font_size: 22,
                color: Color::from_rgba(236, 236, 242, 255),
                ..Default::default()
            },
        );

        draw_text_ex(
            "Keys: Space Play/Pause | <- -> seek 1s | Up/Down seek 0.1s | Wheel seek | Ctrl=finer",
            10.0,
            menu_height + 52.0,
            TextParams {
                font_size: 16,
                color: Color::from_rgba(170, 170, 185, 255),
                ..Default::default()
            },
        );

        let editor_y = menu_height + TOP_BAR_HEIGHT + 8.0;
        let editor_gap = 12.0;
        let editor_width = (screen_width() - 20.0 - note_panel_width_px - editor_gap).max(360.0);
        let editor_rect = Rect::new(
            10.0,
            editor_y,
            editor_width,
            (screen_height() - editor_y - 28.0).max(140.0),
        );

        for action in editor.draw(editor_rect, current_sec, duration_sec, track_path) {
            match action {
                FallingEditorAction::SeekTo(sec) => actions.push(PlayerUiAction::Seek(sec)),
            }
        }

        if let Some(player_ref) = player.as_mut() {
            for action in actions {
                if let Some(message) = apply_player_action(player_ref, action, &i18n) {
                    status_message = message;
                }
                if let Some(event) = player_ref.update() {
                    status_message = format_player_event(event, &i18n);
                }
            }
        }

        draw_text_ex(
            &status_message,
            10.0,
            screen_height() - 8.0,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(170, 205, 255, 255),
                ..Default::default()
            },
        );

        egui_macroquad::draw();
        next_frame().await;
    }
}
