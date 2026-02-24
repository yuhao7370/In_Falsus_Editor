use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::{I18n, TextKey};
use crate::settings::{modify_settings, settings};
use crate::shortcuts::ShortcutAction;
use crate::ui::info_toast::InfoToastManager;
use crate::ui::input_state::{free_mouse_wheel, safe_key_down, safe_key_pressed, safe_mouse_wheel};
use macroquad::prelude::*;

use super::ui_orchestrator::UiOutput;

/// Global keyboard shortcuts.
pub fn handle_shortcuts(
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &I18n,
    info_toasts: &mut InfoToastManager,
) {
    let shortcut_bindings = {
        let s = settings();
        s.shortcuts.clone()
    };

    if shortcut_bindings.is_pressed(ShortcutAction::SaveChart, safe_key_pressed, safe_key_down) {
        match editor.save_chart() {
            Ok(()) => info_toasts.push(format!(
                "{}: {}",
                i18n.t(TextKey::ActionSaveChartSuccess),
                editor.chart_path()
            )),
            Err(e) => info_toasts.push(format!("{}: {e}", i18n.t(TextKey::ActionSaveChartFailed))),
        }
    }

    if shortcut_bindings.is_pressed(ShortcutAction::Undo, safe_key_pressed, safe_key_down) {
        if editor.undo() {
            info_toasts.push(i18n.t(TextKey::ActionUndo));
        } else {
            info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo));
        }
    }

    if shortcut_bindings.is_pressed(ShortcutAction::Redo, safe_key_pressed, safe_key_down) {
        if editor.redo() {
            info_toasts.push(i18n.t(TextKey::ActionRedo));
        } else {
            info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo));
        }
    }

    if shortcut_bindings.is_pressed(
        ShortcutAction::ToggleHitsound,
        safe_key_pressed,
        safe_key_down,
    ) {
        let enabled = !audio.hitsound_enabled();
        audio.set_hitsound_enabled(enabled);
        modify_settings(|s| s.hitsound_enabled = enabled);
        if enabled {
            info_toasts.push(i18n.t(TextKey::ActionHitsoundOn));
        } else {
            info_toasts.push(i18n.t(TextKey::ActionHitsoundOff));
        }
    }
}

/// Mouse wheel input handling:
/// Ctrl+Wheel adjusts speed, Shift+Wheel snap-seeks, otherwise free seek.
pub fn handle_wheel(
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &I18n,
    info_toasts: &mut InfoToastManager,
    space_consumed: bool,
    ui: &UiOutput,
) {
    // Policy:
    // - Ctrl/Shift wheel branches use raw wheel deltas, but only when pointer is not captured by egui.
    // - Otherwise fall back to `handle_wheel_seek`, which already respects safe wheel routing.
    let (_, free_wheel_y) = free_mouse_wheel();
    let (_, mq_wheel_y) = safe_mouse_wheel();
    let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

    let mut egui_wheel_y = ui.egui_wheel_y;
    if ui.egui_wants_pointer {
        egui_wheel_y = 0.0;
    }

    if !ui.egui_wants_pointer && ctrl_down && free_wheel_y.abs() > f32::EPSILON {
        let step = editor.scroll_speed_step();
        let delta = if free_wheel_y > 0.0 { step } else { -step };
        editor.nudge_scroll_speed(delta);
        let new_speed = editor.scroll_speed();
        modify_settings(|s| s.scroll_speed = new_speed);
        info_toasts.push(format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), new_speed));
    } else if !ui.egui_wants_pointer
        && !shift_down
        && free_wheel_y.abs() > f32::EPSILON
        && !audio.is_playing()
        && audio.duration_sec() > 0.0
    {
        let forward = free_wheel_y > 0.0;
        let current_ms = audio.current_sec() * 1000.0;
        let target_ms = editor.snap_seek_ms(current_ms, forward);
        let target_sec = (target_ms / 1000.0).clamp(0.0, audio.duration_sec());
        audio.handle_editor_seek(target_sec, i18n);
    } else {
        audio.handle_wheel_seek(mq_wheel_y, egui_wheel_y, space_consumed, i18n);
    }
}
