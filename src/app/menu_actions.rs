use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::{I18n, TextKey};
use crate::settings::{modify_settings, modify_settings_nosave};
use crate::ui::info_toast::InfoToastManager;
use crate::ui::top_menu::{EditAction, FileAction, SettingsAction, TopMenuAction};

pub fn handle_top_menu_action(
    action: TopMenuAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &mut I18n,
    info_toasts: &mut InfoToastManager,
) {
    match action {
        TopMenuAction::File(fa) => handle_file_action(fa, editor, audio, i18n),
        TopMenuAction::Edit(ea) => handle_edit_action(ea, editor, audio, i18n, info_toasts),
        TopMenuAction::Settings(sa) => handle_settings_action(sa, editor, audio, i18n),
    }
}

fn handle_file_action(
    action: FileAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &I18n,
) {
    match action {
        FileAction::CreateProject => {
            audio.status = i18n.t(TextKey::ActionCreateProject).to_owned();
        }
        FileAction::OpenProject | FileAction::CurrentProject => {
            audio.status.clear();
        }
        FileAction::SaveChart => {
            match editor.save_chart() {
                Ok(()) => {
                    audio.status = format!(
                        "{}: {}",
                        i18n.t(TextKey::ActionSaveChartSuccess),
                        editor.chart_path()
                    )
                }
                Err(e) => {
                    audio.status = format!("{}: {e}", i18n.t(TextKey::ActionSaveChartFailed))
                }
            }
        }
        FileAction::HotReloadChart => {
            match editor.reload_chart() {
                Ok(true) => audio.status = i18n.t(TextKey::ActionHotReloadChart).to_owned(),
                Ok(false) => audio.status = i18n.t(TextKey::ActionHotReloadChartNoChange).to_owned(),
                Err(e) => audio.status = format!("{}: {e}", i18n.t(TextKey::ActionHotReloadChartFailed)),
            }
        }
    }
}

fn handle_edit_action(
    action: EditAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &I18n,
    info_toasts: &mut InfoToastManager,
) {
    match action {
        EditAction::Undo => {
            if !editor.undo() { info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo)); }
            audio.status = i18n.t(TextKey::ActionUndo).to_owned();
        }
        EditAction::Redo => {
            if !editor.redo() { info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo)); }
            audio.status = i18n.t(TextKey::ActionRedo).to_owned();
        }
        EditAction::Cut => {
            editor.cut_selection();
            audio.status.clear();
        }
        EditAction::Copy => {
            editor.copy_selection();
            audio.status.clear();
        }
        EditAction::Paste => {
            editor.enter_normal_paste_mode();
            audio.status.clear();
        }
        EditAction::MirrorPaste => {
            editor.enter_mirrored_paste_mode();
            audio.status.clear();
        }
        EditAction::MirrorSelected => {
            editor.mirror_selection();
            audio.status.clear();
        }
        EditAction::CopyMirrorSelected => {
            editor.copy_and_mirror_selection();
            audio.status.clear();
        }
    }
}

fn handle_settings_action(
    action: SettingsAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &mut I18n,
) {
    match action {
        SettingsAction::SetLanguage(language) => {
            i18n.set_language(language.clone());
            editor.set_i18n(i18n.clone());
            modify_settings(|s| s.set_language_from(&language));
            audio.status = i18n.t(TextKey::ActionLanguageSwitched)
                .replace("{lang}", i18n.language_display_name(&language));
        }
        SettingsAction::SetMasterVolume(vol) => {
            audio.set_master_volume(vol, i18n);
            modify_settings(|s| s.master_volume = vol);
            audio.status.clear();
        }
        SettingsAction::SetMusicVolume(vol) => {
            audio.set_music_volume(vol, i18n);
            modify_settings(|s| s.music_volume = vol);
            audio.status.clear();
        }
        SettingsAction::SetAutoPlay(enabled) => {
            editor.set_autoplay_enabled(enabled);
            modify_settings(|s| s.autoplay = enabled);
        }
        SettingsAction::SetShowSpectrum(enabled) => {
            editor.set_show_spectrum(enabled);
            modify_settings(|s| s.show_spectrum = enabled);
        }
        SettingsAction::SetDebugHitbox(enabled) => {
            editor.set_debug_show_hitboxes(enabled);
            modify_settings(|s| s.debug_hitbox = enabled);
            audio.status = if enabled {
                i18n.t(TextKey::ActionDebugHitboxOn).to_owned()
            } else {
                i18n.t(TextKey::ActionDebugHitboxOff).to_owned()
            };
        }
        SettingsAction::SetMinimapVisible(enabled) => {
            editor.set_show_minimap(enabled);
            modify_settings(|s| s.show_minimap = enabled);
            audio.status = if enabled {
                i18n.t(TextKey::ActionMinimapOn).to_owned()
            } else {
                i18n.t(TextKey::ActionMinimapOff).to_owned()
            };
        }
        SettingsAction::SetRenderScope(scope) => {
            editor.set_render_scope(scope);
        }
        SettingsAction::SetScrollSpeed(speed) => {
            editor.set_scroll_speed(speed);
            modify_settings_nosave(|s| s.scroll_speed = speed);
            audio.status.clear();
        }
        SettingsAction::SetScrollSpeedFinal(speed) => {
            editor.set_scroll_speed(speed);
            modify_settings(|s| s.scroll_speed = speed);
            audio.status = format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), speed);
        }
        SettingsAction::SetSnapDivision(division) => {
            editor.set_snap_division(division);
            modify_settings_nosave(|s| s.snap_division = division);
            audio.status.clear();
        }
        SettingsAction::SetSnapDivisionFinal(division) => {
            editor.set_snap_division(division);
            modify_settings(|s| s.snap_division = division);
            audio.status = format!("{}: {}x", i18n.t(TextKey::SettingsBarlineSnap), division);
        }
        SettingsAction::SetXSplit(value) => {
            editor.set_x_split(value);
            modify_settings(|s| s.x_split = value);
            audio.status = format!("{}: {}", i18n.t(TextKey::SettingsXSplit), value);
        }
        SettingsAction::SetXSplitEditable(enabled) => {
            editor.set_xsplit_editable(enabled);
            modify_settings(|s| s.xsplit_editable = enabled);
        }
        SettingsAction::SetHitsoundEnabled(enabled) => {
            audio.set_hitsound_enabled(enabled);
            modify_settings(|s| s.hitsound_enabled = enabled);
        }
        SettingsAction::SetHitsoundTapVolume(vol) => {
            audio.set_hitsound_tap_volume(vol);
            modify_settings(|s| s.hitsound_tap_volume = vol);
        }
        SettingsAction::SetHitsoundArcVolume(vol) => {
            audio.set_hitsound_arc_volume(vol);
            modify_settings(|s| s.hitsound_arc_volume = vol);
        }
        SettingsAction::SetHitsoundDelay(ms) => {
            audio.set_hitsound_delay_ms(ms);
            modify_settings(|s| s.hitsound_delay_ms = ms);
        }
        SettingsAction::SetDebugAudio(enabled) => {
            modify_settings(|s| s.debug_audio = enabled);
        }
        SettingsAction::SetShortcut(action, chord) => {
            modify_settings(|s| {
                let _ = s.shortcuts.set_chord(action, chord);
            });
        }
        SettingsAction::ResetShortcut(action) => {
            modify_settings(|s| {
                let _ = s.shortcuts.reset_chord(action);
            });
        }
    }
}
