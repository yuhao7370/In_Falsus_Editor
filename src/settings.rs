use crate::i18n::{I18n, Language};
use crate::shortcuts::ShortcutBindings;
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex, MutexGuard};

const SETTINGS_FILE: &str = "settings.json";

static SETTINGS: LazyLock<Mutex<AppSettings>> = LazyLock::new(|| {
    Mutex::new(AppSettings::load_from_file())
});

/// 获取设置的只读锁
pub fn settings() -> MutexGuard<'static, AppSettings> {
    SETTINGS.lock().unwrap()
}

/// 修改设置并自动保存
pub fn modify_settings(f: impl FnOnce(&mut AppSettings)) {
    let mut s = SETTINGS.lock().unwrap();
    f(&mut s);
    s.save();
}

/// 修改设置但不保存（用于拖拽中间态）
pub fn modify_settings_nosave(f: impl FnOnce(&mut AppSettings)) {
    let mut s = SETTINGS.lock().unwrap();
    f(&mut s);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_volume")]
    pub master_volume: f32,
    #[serde(default = "default_volume")]
    pub music_volume: f32,
    #[serde(default)]
    pub autoplay: bool,
    #[serde(default = "default_true")]
    pub show_spectrum: bool,
    #[serde(default)]
    pub show_minimap: bool,
    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: f32,
    #[serde(default = "default_snap_division")]
    pub snap_division: u32,
    #[serde(default = "default_x_split")]
    pub x_split: f64,
    #[serde(default)]
    pub xsplit_editable: bool,
    #[serde(default)]
    pub debug_hitbox: bool,
    #[serde(default)]
    pub debug_audio: bool,
    #[serde(default = "default_true")]
    pub hitsound_enabled: bool,
    #[serde(default = "default_volume")]
    pub hitsound_tap_volume: f32,
    #[serde(default = "default_volume")]
    pub hitsound_arc_volume: f32,
    #[serde(default = "default_hitsound_max_voices")]
    pub hitsound_max_voices: usize,
    #[serde(default)]
    pub hitsound_delay_ms: i32,
    #[serde(default)]
    pub shortcuts: ShortcutBindings,
}

fn default_language() -> String {
    "zh-cn".to_owned()
}
fn default_volume() -> f32 {
    1.0
}
fn default_true() -> bool {
    true
}
fn default_scroll_speed() -> f32 {
    1.0
}
fn default_snap_division() -> u32 {
    4
}
fn default_x_split() -> f64 {
    24.0
}
fn default_hitsound_max_voices() -> usize {
    8
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: default_language(),
            master_volume: default_volume(),
            music_volume: default_volume(),
            autoplay: false,
            show_spectrum: true,
            show_minimap: false,
            scroll_speed: default_scroll_speed(),
            snap_division: default_snap_division(),
            x_split: default_x_split(),
            xsplit_editable: false,
            debug_hitbox: false,
            debug_audio: false,
            hitsound_enabled: true,
            hitsound_tap_volume: default_volume(),
            hitsound_arc_volume: default_volume(),
            hitsound_max_voices: default_hitsound_max_voices(),
            hitsound_delay_ms: 0,
            shortcuts: ShortcutBindings::default(),
        }
    }
}

impl AppSettings {
    pub fn load_from_file() -> Self {
        match std::fs::read_to_string(SETTINGS_FILE) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(SETTINGS_FILE, json);
        }
    }

    pub fn language_enum(&self, i18n: &I18n) -> Language {
        Language::from_settings(&self.language, &i18n.available_languages())
    }

    pub fn set_language_from(&mut self, lang: &Language) {
        self.language = lang.code().to_ascii_lowercase();
    }
}
