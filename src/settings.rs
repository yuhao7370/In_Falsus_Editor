use crate::i18n::Language;
use serde::{Deserialize, Serialize};

const SETTINGS_FILE: &str = "settings.json";

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
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
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

    pub fn language_enum(&self) -> Language {
        match self.language.as_str() {
            "en-us" => Language::EnUs,
            _ => Language::ZhCn,
        }
    }

    pub fn set_language_from(&mut self, lang: Language) {
        self.language = match lang {
            Language::ZhCn => "zh-cn".to_owned(),
            Language::EnUs => "en-us".to_owned(),
        };
    }
}
