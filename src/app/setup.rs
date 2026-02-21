use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::I18n;
use crate::settings::settings;
use crate::ui::scale::{BASE_HEIGHT, BASE_WIDTH};
use macroquad::prelude::Conf;

pub fn window_conf() -> Conf {
    Conf {
        window_title: "In Falsus Editor".to_owned(),
        window_width: BASE_WIDTH as i32,
        window_height: BASE_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

/// 将全局设置应用到 editor 和 audio
pub fn apply_settings_to_editor(editor: &mut FallingGroundEditor, audio: &mut AudioController, i18n: &I18n) {
    let s = settings();
    editor.set_scroll_speed(s.scroll_speed);
    editor.set_snap_division(s.snap_division);
    editor.set_autoplay_enabled(s.autoplay);
    editor.set_show_spectrum(s.show_spectrum);
    editor.set_show_barlines(s.show_barlines);
    editor.set_color_barlines(s.color_barlines);
    editor.set_show_minimap(s.show_minimap);
    editor.set_x_split(s.x_split);
    editor.set_xsplit_editable(s.xsplit_editable);
    editor.set_debug_show_hitboxes(s.debug_hitbox);
    editor.set_debug_skyarea_body_only(s.debug_skyarea_body_only);
    editor.set_i18n(i18n.clone());
    audio.set_master_volume(s.master_volume, i18n);
    audio.set_music_volume(s.music_volume, i18n);
    audio.set_hitsound_enabled(s.hitsound_enabled);
    audio.set_hitsound_tap_volume(s.hitsound_tap_volume);
    audio.set_hitsound_arc_volume(s.hitsound_arc_volume);
    audio.set_hitsound_max_voices(s.hitsound_max_voices);
    audio.set_hitsound_delay_ms(s.hitsound_delay_ms);
}
