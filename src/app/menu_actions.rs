use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::{I18n, TextKey};
use crate::settings::{modify_settings, modify_settings_nosave};
use crate::ui::info_toast::InfoToastManager;
use crate::ui::top_menu::TopMenuAction;

pub fn handle_top_menu_action(
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
        TopMenuAction::OpenProject | TopMenuAction::CurrentProject => {
            audio.status.clear();
        }
        TopMenuAction::SaveChart => {
            match editor.save_chart() {
                Ok(()) => audio.status = format!("谱面已保存: {}", editor.chart_path()),
                Err(e) => audio.status = format!("保存失败: {e}"),
            }
        }
        TopMenuAction::HotReloadChart => {
            match editor.reload_chart() {
                Ok(true) => audio.status = i18n.t(TextKey::ActionHotReloadChart).to_owned(),
                Ok(false) => audio.status = i18n.t(TextKey::ActionHotReloadChartNoChange).to_owned(),
                Err(e) => audio.status = format!("{}: {e}", i18n.t(TextKey::ActionHotReloadChartFailed)),
            }
        }
        TopMenuAction::Undo => {
            if !editor.undo() { info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo)); }
            audio.status = i18n.t(TextKey::ActionUndo).to_owned();
        }
        TopMenuAction::Redo => {
            if !editor.redo() { info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo)); }
            audio.status = i18n.t(TextKey::ActionRedo).to_owned();
        }
        TopMenuAction::Cut => audio.status = i18n.t(TextKey::ActionCut).to_owned(),
        TopMenuAction::Copy => audio.status = i18n.t(TextKey::ActionCopy).to_owned(),
        TopMenuAction::Paste => audio.status = i18n.t(TextKey::ActionPaste).to_owned(),
        TopMenuAction::SetLanguage(language) => {
            i18n.set_language(language.clone());
            editor.set_i18n(i18n.clone());
            modify_settings(|s| s.set_language_from(&language));
            audio.status = i18n.t(TextKey::ActionLanguageSwitched)
                .replace("{lang}", i18n.language_display_name(&language));
        }
        TopMenuAction::SetMasterVolume(vol) => {
            audio.set_master_volume(vol, i18n);
            modify_settings(|s| s.master_volume = vol);
            audio.status.clear();
        }
        TopMenuAction::SetMusicVolume(vol) => {
            audio.set_music_volume(vol, i18n);
            modify_settings(|s| s.music_volume = vol);
            audio.status.clear();
        }
        TopMenuAction::SetAutoPlay(enabled) => {
            editor.set_autoplay_enabled(enabled);
            modify_settings(|s| s.autoplay = enabled);
        }
        TopMenuAction::SetShowSpectrum(enabled) => {
            editor.set_show_spectrum(enabled);
            modify_settings(|s| s.show_spectrum = enabled);
        }
        TopMenuAction::SetDebugHitbox(enabled) => {
            editor.set_debug_show_hitboxes(enabled);
            modify_settings(|s| s.debug_hitbox = enabled);
            audio.status = if enabled {
                i18n.t(TextKey::ActionDebugHitboxOn).to_owned()
            } else {
                i18n.t(TextKey::ActionDebugHitboxOff).to_owned()
            };
        }
        TopMenuAction::SetMinimapVisible(enabled) => {
            editor.set_show_minimap(enabled);
            modify_settings(|s| s.show_minimap = enabled);
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
            modify_settings_nosave(|s| s.scroll_speed = speed);
            audio.status.clear();
        }
        TopMenuAction::SetScrollSpeedFinal(speed) => {
            editor.set_scroll_speed(speed);
            modify_settings(|s| s.scroll_speed = speed);
            audio.status = format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), speed);
        }
        TopMenuAction::SetSnapDivision(division) => {
            editor.set_snap_division(division);
            modify_settings_nosave(|s| s.snap_division = division);
            audio.status.clear();
        }
        TopMenuAction::SetSnapDivisionFinal(division) => {
            editor.set_snap_division(division);
            modify_settings(|s| s.snap_division = division);
            audio.status = format!("{}: {}x", i18n.t(TextKey::SettingsBarlineSnap), division);
        }
        TopMenuAction::SetXSplit(value) => {
            editor.set_x_split(value);
            modify_settings(|s| s.x_split = value);
            audio.status = format!("{}: {}", i18n.t(TextKey::SettingsXSplit), value);
        }
        TopMenuAction::SetXSplitEditable(enabled) => {
            editor.set_xsplit_editable(enabled);
            modify_settings(|s| s.xsplit_editable = enabled);
        }
        TopMenuAction::SetHitsoundEnabled(enabled) => {
            audio.set_hitsound_enabled(enabled);
            modify_settings(|s| s.hitsound_enabled = enabled);
        }
        TopMenuAction::SetHitsoundTapVolume(vol) => {
            audio.set_hitsound_tap_volume(vol);
            modify_settings(|s| s.hitsound_tap_volume = vol);
        }
        TopMenuAction::SetHitsoundArcVolume(vol) => {
            audio.set_hitsound_arc_volume(vol);
            modify_settings(|s| s.hitsound_arc_volume = vol);
        }
        TopMenuAction::SetHitsoundDelay(ms) => {
            audio.set_hitsound_delay_ms(ms);
            modify_settings(|s| s.hitsound_delay_ms = ms);
        }
        TopMenuAction::SetDebugAudio(enabled) => {
            modify_settings(|s| s.debug_audio = enabled);
        }
    }
}
