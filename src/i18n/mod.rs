use std::collections::HashMap;

const ZH_CN_FILE_PATH: &str = "i18n/zh-CN.json";
const EN_US_FILE_PATH: &str = "i18n/en-US.json";
const ZH_CN_FALLBACK_JSON: &str = include_str!("../../i18n/zh-CN.json");
const EN_US_FALLBACK_JSON: &str = include_str!("../../i18n/en-US.json");

type Messages = HashMap<String, String>;

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

fn parse_messages(json: &str) -> Option<Messages> {
    serde_json::from_str(json).ok()
}

fn load_messages_from_disk(path: &str) -> Option<Messages> {
    let bytes = std::fs::read(path).ok()?;
    let text = std::str::from_utf8(strip_utf8_bom(&bytes)).ok()?;
    parse_messages(text)
}

fn load_messages(path: &str, fallback_json: &str) -> Messages {
    load_messages_from_disk(path)
        .or_else(|| parse_messages(fallback_json))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ZhCn,
    EnUs,
}

impl Language {
    pub fn detect() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                let value = value.to_ascii_lowercase();
                if value.contains("zh") {
                    return Self::ZhCn;
                }
                if value.contains("en") {
                    return Self::EnUs;
                }
            }
        }
        Self::ZhCn
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    BannerTitle,
    PlayerWindowTitle,
    PlayerBtnLoad,
    PlayerBtnPlay,
    PlayerBtnPause,
    PlayerBtnStop,
    PlayerLabelVolume,
    MenuFile,
    MenuEdit,
    MenuSelect,
    MenuSettings,
    SettingsLanguage,
    SettingsDebugHitbox,
    LanguageChinese,
    LanguageEnglish,
    FileCreateProject,
    FileOpenProject,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    StatusPlaying,
    StatusInitAudioFailed,
    StatusReadAudioFailed,
    StatusDecodeAudioFailed,
    StatusCreateMusicFailed,
    StatusStartPlaybackFailed,
    StatusAudioUnavailable,
    StatusBackendError,
    StatusLoaded,
    StatusPaused,
    StatusStopped,
    StatusPlaybackEnded,
    StatusBackendRecovered,
    StatusSeekFailed,
    StatusVolumeUpdated,
    ActionCreateProject,
    ActionOpenProject,
    ActionUndo,
    ActionRedo,
    ActionCut,
    ActionCopy,
    ActionPaste,
    ActionSetLanguageZh,
    ActionSetLanguageEn,
    ActionDebugHitboxOn,
    ActionDebugHitboxOff,
}

impl TextKey {
    pub fn as_str(self) -> &'static str {
        match self {
            TextKey::BannerTitle => "banner_title",
            TextKey::PlayerWindowTitle => "player_window_title",
            TextKey::PlayerBtnLoad => "player_btn_load",
            TextKey::PlayerBtnPlay => "player_btn_play",
            TextKey::PlayerBtnPause => "player_btn_pause",
            TextKey::PlayerBtnStop => "player_btn_stop",
            TextKey::PlayerLabelVolume => "player_label_volume",
            TextKey::MenuFile => "menu_file",
            TextKey::MenuEdit => "menu_edit",
            TextKey::MenuSelect => "menu_select",
            TextKey::MenuSettings => "menu_settings",
            TextKey::SettingsLanguage => "settings_language",
            TextKey::SettingsDebugHitbox => "settings_debug_hitbox",
            TextKey::LanguageChinese => "language_chinese",
            TextKey::LanguageEnglish => "language_english",
            TextKey::FileCreateProject => "file_create_project",
            TextKey::FileOpenProject => "file_open_project",
            TextKey::EditUndo => "edit_undo",
            TextKey::EditRedo => "edit_redo",
            TextKey::EditCut => "edit_cut",
            TextKey::EditCopy => "edit_copy",
            TextKey::EditPaste => "edit_paste",
            TextKey::StatusPlaying => "status_playing",
            TextKey::StatusInitAudioFailed => "status_init_audio_failed",
            TextKey::StatusReadAudioFailed => "status_read_audio_failed",
            TextKey::StatusDecodeAudioFailed => "status_decode_audio_failed",
            TextKey::StatusCreateMusicFailed => "status_create_music_failed",
            TextKey::StatusStartPlaybackFailed => "status_start_playback_failed",
            TextKey::StatusAudioUnavailable => "status_audio_unavailable",
            TextKey::StatusBackendError => "status_backend_error",
            TextKey::StatusLoaded => "status_loaded",
            TextKey::StatusPaused => "status_paused",
            TextKey::StatusStopped => "status_stopped",
            TextKey::StatusPlaybackEnded => "status_playback_ended",
            TextKey::StatusBackendRecovered => "status_backend_recovered",
            TextKey::StatusSeekFailed => "status_seek_failed",
            TextKey::StatusVolumeUpdated => "status_volume_updated",
            TextKey::ActionCreateProject => "action_create_project",
            TextKey::ActionOpenProject => "action_open_project",
            TextKey::ActionUndo => "action_undo",
            TextKey::ActionRedo => "action_redo",
            TextKey::ActionCut => "action_cut",
            TextKey::ActionCopy => "action_copy",
            TextKey::ActionPaste => "action_paste",
            TextKey::ActionSetLanguageZh => "action_set_language_zh",
            TextKey::ActionSetLanguageEn => "action_set_language_en",
            TextKey::ActionDebugHitboxOn => "action_debug_hitbox_on",
            TextKey::ActionDebugHitboxOff => "action_debug_hitbox_off",
        }
    }
}

#[derive(Debug, Clone)]
pub struct I18n {
    language: Language,
    zh_cn: Messages,
    en_us: Messages,
}

impl I18n {
    pub fn detect() -> Self {
        Self::new(Language::detect())
    }

    pub fn new(language: Language) -> Self {
        Self {
            language,
            zh_cn: load_messages(ZH_CN_FILE_PATH, ZH_CN_FALLBACK_JSON),
            en_us: load_messages(EN_US_FILE_PATH, EN_US_FALLBACK_JSON),
        }
    }

    fn active_messages(&self) -> &Messages {
        match self.language {
            Language::ZhCn => &self.zh_cn,
            Language::EnUs => &self.en_us,
        }
    }

    fn fallback_messages(&self) -> &Messages {
        match self.language {
            Language::ZhCn => &self.en_us,
            Language::EnUs => &self.zh_cn,
        }
    }

    pub fn t(&self, key: TextKey) -> &str {
        let key_name = key.as_str();
        self.active_messages()
            .get(key_name)
            .or_else(|| self.fallback_messages().get(key_name))
            .map(String::as_str)
            .unwrap_or(key_name)
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    pub fn with_detail(&self, key: TextKey, detail: impl std::fmt::Display) -> String {
        format!("{}: {}", self.t(key), detail)
    }
}
