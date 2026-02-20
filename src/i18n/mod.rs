use std::collections::HashMap;
use std::sync::Arc;

const ZH_CN_FALLBACK_JSON: &str = include_str!("../../i18n/zh-CN.json");
const EN_US_FALLBACK_JSON: &str = include_str!("../../i18n/en-US.json");
const I18N_DIR: &str = "i18n";

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

/// Scan the i18n directory and load all .json files.
/// Returns a map of language code (e.g. "zh-CN") to Messages.
fn discover_languages() -> HashMap<String, Messages> {
    let mut all = HashMap::new();

    // Always include compiled-in fallbacks
    if let Some(msgs) = parse_messages(ZH_CN_FALLBACK_JSON) {
        all.insert("zh-CN".to_owned(), msgs);
    }
    if let Some(msgs) = parse_messages(EN_US_FALLBACK_JSON) {
        all.insert("en-US".to_owned(), msgs);
    }

    // Scan i18n/ directory for additional or overriding language files
    if let Ok(entries) = std::fs::read_dir(I18N_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let lang_code = stem.to_owned();
                    if let Some(msgs) = load_messages_from_disk(&path.to_string_lossy()) {
                        all.insert(lang_code, msgs);
                    }
                }
            }
        }
    }

    all
}

/// Language identifier — a thin wrapper around a locale string like "zh-CN", "en-US", "ja-JP".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Language(pub String);

impl Language {
    pub const ZH_CN: &'static str = "zh-CN";
    pub const EN_US: &'static str = "en-US";

    pub fn zh_cn() -> Self {
        Self(Self::ZH_CN.to_owned())
    }

    pub fn en_us() -> Self {
        Self(Self::EN_US.to_owned())
    }

    pub fn code(&self) -> &str {
        &self.0
    }

    pub fn detect() -> Self {
        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                let value = value.to_ascii_lowercase();
                if value.contains("zh") {
                    return Self::zh_cn();
                }
                if value.contains("en") {
                    return Self::en_us();
                }
            }
        }
        Self::zh_cn()
    }

    /// Parse a settings string (e.g. "zh-cn") into a Language.
    /// Case-insensitive matching against available languages, falls back to zh-CN.
    pub fn from_settings(s: &str, available: &[Language]) -> Self {
        let lower = s.to_ascii_lowercase();
        for lang in available {
            if lang.0.to_ascii_lowercase() == lower {
                return lang.clone();
            }
        }
        // Partial match: "zh" → first zh-*, "en" → first en-*
        for lang in available {
            let lang_lower = lang.0.to_ascii_lowercase();
            if lang_lower.starts_with(&lower) || lower.starts_with(&lang_lower.split('-').next().unwrap_or("")) {
                return lang.clone();
            }
        }
        Self::zh_cn()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TextKey {
    PlayerLabelVolume,
    MenuFile,
    MenuEdit,
    MenuSelect,
    MenuSettings,
    MenuDocs,
    SettingsAutoPlay,
    SettingsShowSpectrum,
    SettingsDebugHitbox,
    SettingsShowMinimap,
    LanguageChinese,
    LanguageEnglish,
    FileCreateProject,
    FileOpenProject,
    FileSaveChart,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    EditMirrorPaste,
    EditMirrorSelected,
    EditCopyMirrorSelected,
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
    ActionUndo,
    ActionRedo,
    ActionCut,
    ActionCopy,
    ActionPaste,
    ActionSetLanguageZh,
    ActionSetLanguageEn,
    ActionDebugHitboxOn,
    ActionDebugHitboxOff,
    ActionMinimapOn,
    ActionMinimapOff,
    ActionHitsoundOn,
    ActionHitsoundOff,
    ActionNothingToUndo,
    ActionNothingToRedo,
    RenderMerge,
    RenderSplit,
    SettingsCategoryLanguage,
    SettingsCategoryAudio,
    SettingsCategoryDisplay,
    SettingsCategoryDebug,
    SettingsCategoryShortcuts,
    SettingsFlowSpeed,
    SettingsBarlineSnap,
    NotePanelRenderSpeedEvents,
    SettingsXSplit,
    SettingsXSplitEditable,
    ToastLaneWidthReject,
    ToastLaneWidthMax,
    SettingsMasterVolume,
    SettingsHitsoundEnabled,
    SettingsHitsoundTapVolume,
    SettingsHitsoundArcVolume,
    SettingsHitsoundDelay,
    SettingsShortcutEditable,
    SettingsShortcutChange,
    SettingsShortcutCancel,
    SettingsShortcutReset,
    SettingsShortcutPressAnyKey,
    ShortcutActionSaveChart,
    ShortcutActionUndo,
    ShortcutActionRedo,
    ShortcutActionCut,
    ShortcutActionCopy,
    ShortcutActionPaste,
    ShortcutActionToggleHitsound,
    DocsCategoryOperations,
    DocsCategoryShortcuts,
    DocsSectionShortcuts,
    DocsSectionOperations,
    DocsSectionFixedShortcuts,
    DocsSectionCustomShortcuts,
    DocsSectionGlobal,
    DocsSectionEditor,
    DocsSectionWheel,
    DocsShortcutSaveChart,
    DocsShortcutUndo,
    DocsShortcutRedo,
    DocsShortcutToggleHitsound,
    DocsShortcutPlayPause,
    DocsShortcutDelete,
    DocsShortcutCopy,
    DocsShortcutCut,
    DocsShortcutPasteMode,
    DocsShortcutMirror,
    DocsShortcutMirrorCopy,
    DocsShortcutPasteModeSwitch,
    DocsShortcutBoxSelect,
    DocsShortcutMultiSelect,
    DocsShortcutRightCancel,
    DocsShortcutWheelSpeed,
    DocsShortcutWheelSnapSeek,
    DocsShortcutWheelSeek,
    DocsShortcutWheelSeekAlt,
    CreateProjectTitle,
    CreateProjectName,
    CreateProjectNameHint,
    CreateProjectAudio,
    CreateProjectBrowse,
    CreateProjectNoFile,
    CreateProjectBpl,
    CreateProjectCreate,
    CreateProjectCancel,
    FileCurrentProject,
    CurrentProjectTitle,
    CurrentProjectChart,
    CurrentProjectAudio,
    CurrentProjectClose,
    CurrentProjectNoProject,
    CurrentProjectLoadChart,
    CurrentProjectLoadAudio,
    CurrentProjectMissing,
    FileHotReloadChart,
    ActionHotReloadChart,
    ActionHotReloadChartFailed,
    ActionHotReloadChartNoChange,
    ActionSaveChartSuccess,
    ActionSaveChartFailed,
    SettingsDebugAudio,
    // ── Editor internal messages ──
    ActionLanguageSwitched,
    EditorNothingToCopy,
    EditorCopiedNotes,
    EditorNothingToCut,
    EditorCutNotes,
    EditorNothingToMirror,
    EditorMirroredNotes,
    EditorCopyMirroredNotes,
    EditorPastedNotes,
    EditorMirrorPastedNotes,
    EditorCannotPasteWideSideLane,
    EditorCannotPasteExceedLane,
    EditorClipboardEmpty,
    EditorPlaceModeCleared,
    EditorSelectionCleared,
    EditorPasteModeNormal,
    EditorPasteModeMirrored,
    EditorPasteCancelled,
    SpectrumStillLoading,
    SpectrumLoadedOk,
}

impl TextKey {
    pub fn as_str(self) -> &'static str {
        match self {
            TextKey::PlayerLabelVolume => "player_label_volume",
            TextKey::MenuFile => "menu_file",
            TextKey::MenuEdit => "menu_edit",
            TextKey::MenuSelect => "menu_select",
            TextKey::MenuSettings => "menu_settings",
            TextKey::MenuDocs => "menu_docs",
            TextKey::SettingsAutoPlay => "settings_autoplay",
            TextKey::SettingsShowSpectrum => "settings_show_spectrum",
            TextKey::SettingsDebugHitbox => "settings_debug_hitbox",
            TextKey::SettingsShowMinimap => "settings_show_minimap",
            TextKey::LanguageChinese => "language_chinese",
            TextKey::LanguageEnglish => "language_english",
            TextKey::FileCreateProject => "file_create_project",
            TextKey::FileOpenProject => "file_open_project",
            TextKey::FileSaveChart => "file_save_chart",
            TextKey::EditUndo => "edit_undo",
            TextKey::EditRedo => "edit_redo",
            TextKey::EditCut => "edit_cut",
            TextKey::EditCopy => "edit_copy",
            TextKey::EditPaste => "edit_paste",
            TextKey::EditMirrorPaste => "edit_mirror_paste",
            TextKey::EditMirrorSelected => "edit_mirror_selected",
            TextKey::EditCopyMirrorSelected => "edit_copy_mirror_selected",
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
            TextKey::ActionUndo => "action_undo",
            TextKey::ActionRedo => "action_redo",
            TextKey::ActionCut => "action_cut",
            TextKey::ActionCopy => "action_copy",
            TextKey::ActionPaste => "action_paste",
            TextKey::ActionSetLanguageZh => "action_set_language_zh",
            TextKey::ActionSetLanguageEn => "action_set_language_en",
            TextKey::ActionDebugHitboxOn => "action_debug_hitbox_on",
            TextKey::ActionDebugHitboxOff => "action_debug_hitbox_off",
            TextKey::ActionMinimapOn => "action_minimap_on",
            TextKey::ActionMinimapOff => "action_minimap_off",
            TextKey::ActionHitsoundOn => "action_hitsound_on",
            TextKey::ActionHitsoundOff => "action_hitsound_off",
            TextKey::ActionNothingToUndo => "action_nothing_to_undo",
            TextKey::ActionNothingToRedo => "action_nothing_to_redo",
            TextKey::RenderMerge => "render_merge",
            TextKey::RenderSplit => "render_split",
            TextKey::SettingsCategoryLanguage => "settings_category_language",
            TextKey::SettingsCategoryAudio => "settings_category_audio",
            TextKey::SettingsCategoryDisplay => "settings_category_display",
            TextKey::SettingsCategoryDebug => "settings_category_debug",
            TextKey::SettingsCategoryShortcuts => "settings_category_shortcuts",
            TextKey::SettingsFlowSpeed => "settings_flow_speed",
            TextKey::SettingsBarlineSnap => "settings_barline_snap",
            TextKey::NotePanelRenderSpeedEvents => "note_panel_render_speed_events",
            TextKey::SettingsXSplit => "settings_x_split",
            TextKey::SettingsXSplitEditable => "settings_xsplit_editable",
            TextKey::ToastLaneWidthReject => "toast_lane_width_reject",
            TextKey::ToastLaneWidthMax => "toast_lane_width_max",
            TextKey::SettingsMasterVolume => "settings_master_volume",
            TextKey::SettingsHitsoundEnabled => "settings_hitsound_enabled",
            TextKey::SettingsHitsoundTapVolume => "settings_hitsound_tap_volume",
            TextKey::SettingsHitsoundArcVolume => "settings_hitsound_arc_volume",
            TextKey::SettingsHitsoundDelay => "settings_hitsound_delay",
            TextKey::SettingsShortcutEditable => "settings_shortcut_editable",
            TextKey::SettingsShortcutChange => "settings_shortcut_change",
            TextKey::SettingsShortcutCancel => "settings_shortcut_cancel",
            TextKey::SettingsShortcutReset => "settings_shortcut_reset",
            TextKey::SettingsShortcutPressAnyKey => "settings_shortcut_press_any_key",
            TextKey::ShortcutActionSaveChart => "shortcut_action_save_chart",
            TextKey::ShortcutActionUndo => "shortcut_action_undo",
            TextKey::ShortcutActionRedo => "shortcut_action_redo",
            TextKey::ShortcutActionCut => "shortcut_action_cut",
            TextKey::ShortcutActionCopy => "shortcut_action_copy",
            TextKey::ShortcutActionPaste => "shortcut_action_paste",
            TextKey::ShortcutActionToggleHitsound => "shortcut_action_toggle_hitsound",
            TextKey::DocsCategoryOperations => "docs_category_operations",
            TextKey::DocsCategoryShortcuts => "docs_category_shortcuts",
            TextKey::DocsSectionShortcuts => "docs_section_shortcuts",
            TextKey::DocsSectionOperations => "docs_section_operations",
            TextKey::DocsSectionFixedShortcuts => "docs_section_fixed_shortcuts",
            TextKey::DocsSectionCustomShortcuts => "docs_section_custom_shortcuts",
            TextKey::DocsSectionGlobal => "docs_section_global",
            TextKey::DocsSectionEditor => "docs_section_editor",
            TextKey::DocsSectionWheel => "docs_section_wheel",
            TextKey::DocsShortcutSaveChart => "docs_shortcut_save_chart",
            TextKey::DocsShortcutUndo => "docs_shortcut_undo",
            TextKey::DocsShortcutRedo => "docs_shortcut_redo",
            TextKey::DocsShortcutToggleHitsound => "docs_shortcut_toggle_hitsound",
            TextKey::DocsShortcutPlayPause => "docs_shortcut_play_pause",
            TextKey::DocsShortcutDelete => "docs_shortcut_delete",
            TextKey::DocsShortcutCopy => "docs_shortcut_copy",
            TextKey::DocsShortcutCut => "docs_shortcut_cut",
            TextKey::DocsShortcutPasteMode => "docs_shortcut_paste_mode",
            TextKey::DocsShortcutMirror => "docs_shortcut_mirror",
            TextKey::DocsShortcutMirrorCopy => "docs_shortcut_mirror_copy",
            TextKey::DocsShortcutPasteModeSwitch => "docs_shortcut_paste_mode_switch",
            TextKey::DocsShortcutBoxSelect => "docs_shortcut_box_select",
            TextKey::DocsShortcutMultiSelect => "docs_shortcut_multi_select",
            TextKey::DocsShortcutRightCancel => "docs_shortcut_right_cancel",
            TextKey::DocsShortcutWheelSpeed => "docs_shortcut_wheel_speed",
            TextKey::DocsShortcutWheelSnapSeek => "docs_shortcut_wheel_snap_seek",
            TextKey::DocsShortcutWheelSeek => "docs_shortcut_wheel_seek",
            TextKey::DocsShortcutWheelSeekAlt => "docs_shortcut_wheel_seek_alt",
            TextKey::CreateProjectTitle => "create_project_title",
            TextKey::CreateProjectName => "create_project_name",
            TextKey::CreateProjectNameHint => "create_project_name_hint",
            TextKey::CreateProjectAudio => "create_project_audio",
            TextKey::CreateProjectBrowse => "create_project_browse",
            TextKey::CreateProjectNoFile => "create_project_no_file",
            TextKey::CreateProjectBpl => "create_project_bpl",
            TextKey::CreateProjectCreate => "create_project_create",
            TextKey::CreateProjectCancel => "create_project_cancel",
            TextKey::FileCurrentProject => "file_current_project",
            TextKey::CurrentProjectTitle => "current_project_title",
            TextKey::CurrentProjectChart => "current_project_chart",
            TextKey::CurrentProjectAudio => "current_project_audio",
            TextKey::CurrentProjectClose => "current_project_close",
            TextKey::CurrentProjectNoProject => "current_project_no_project",
            TextKey::CurrentProjectLoadChart => "current_project_load_chart",
            TextKey::CurrentProjectLoadAudio => "current_project_load_audio",
            TextKey::CurrentProjectMissing => "current_project_missing",
            TextKey::FileHotReloadChart => "file_hot_reload_chart",
            TextKey::ActionHotReloadChart => "action_hot_reload_chart",
            TextKey::ActionHotReloadChartFailed => "action_hot_reload_chart_failed",
            TextKey::ActionHotReloadChartNoChange => "action_hot_reload_chart_no_change",
            TextKey::ActionSaveChartSuccess => "action_save_chart_success",
            TextKey::ActionSaveChartFailed => "action_save_chart_failed",
            TextKey::SettingsDebugAudio => "settings_debug_audio",
            // Editor internal
            TextKey::ActionLanguageSwitched => "action_language_switched",
            TextKey::EditorNothingToCopy => "editor_nothing_to_copy",
            TextKey::EditorCopiedNotes => "editor_copied_notes",
            TextKey::EditorNothingToCut => "editor_nothing_to_cut",
            TextKey::EditorCutNotes => "editor_cut_notes",
            TextKey::EditorNothingToMirror => "editor_nothing_to_mirror",
            TextKey::EditorMirroredNotes => "editor_mirrored_notes",
            TextKey::EditorCopyMirroredNotes => "editor_copy_mirrored_notes",
            TextKey::EditorPastedNotes => "editor_pasted_notes",
            TextKey::EditorMirrorPastedNotes => "editor_mirror_pasted_notes",
            TextKey::EditorCannotPasteWideSideLane => "editor_cannot_paste_wide_side_lane",
            TextKey::EditorCannotPasteExceedLane => "editor_cannot_paste_exceed_lane",
            TextKey::EditorClipboardEmpty => "editor_clipboard_empty",
            TextKey::EditorPlaceModeCleared => "editor_place_mode_cleared",
            TextKey::EditorSelectionCleared => "editor_selection_cleared",
            TextKey::EditorPasteModeNormal => "editor_paste_mode_normal",
            TextKey::EditorPasteModeMirrored => "editor_paste_mode_mirrored",
            TextKey::EditorPasteCancelled => "editor_paste_cancelled",
            TextKey::SpectrumStillLoading => "spectrum_still_loading",
            TextKey::SpectrumLoadedOk => "spectrum_loaded_ok",
        }
    }
}

#[derive(Debug, Clone)]
pub struct I18n {
    language: Language,
    /// All loaded language packs (Arc-shared to make clone cheap).
    packs: Arc<HashMap<String, Messages>>,
}

impl I18n {
    pub fn detect() -> Self {
        Self::new(Language::detect())
    }

    pub fn new(language: Language) -> Self {
        Self {
            language,
            packs: Arc::new(discover_languages()),
        }
    }

    /// Create from a settings string (e.g. "zh-cn").
    pub fn from_settings(lang_str: &str) -> Self {
        let packs = Arc::new(discover_languages());
        let available: Vec<Language> = packs.keys().map(|k| Language(k.clone())).collect();
        let language = Language::from_settings(lang_str, &available);
        Self { language, packs }
    }

    fn active_messages(&self) -> Option<&Messages> {
        self.packs.get(self.language.code())
    }

    fn fallback_messages(&self) -> Option<&Messages> {
        // Fallback to en-US, then zh-CN
        if self.language.code() != Language::EN_US {
            self.packs.get(Language::EN_US)
        } else {
            self.packs.get(Language::ZH_CN)
        }
    }

    pub fn t(&self, key: TextKey) -> &str {
        let key_name = key.as_str();
        self.active_messages()
            .and_then(|m| m.get(key_name))
            .or_else(|| self.fallback_messages().and_then(|m| m.get(key_name)))
            .map(String::as_str)
            .unwrap_or(key_name)
    }

    pub fn language(&self) -> &Language {
        &self.language
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// Returns all available language codes, sorted.
    pub fn available_languages(&self) -> Vec<Language> {
        let mut langs: Vec<Language> = self.packs.keys().map(|k| Language(k.clone())).collect();
        langs.sort_by(|a, b| a.0.cmp(&b.0));
        langs
    }

    /// Get the display name for a language (looks up "language_name" key in that language's pack).
    pub fn language_display_name<'a>(&'a self, lang: &'a Language) -> &'a str {
        self.packs
            .get(lang.code())
            .and_then(|m| m.get("language_name"))
            .map(String::as_str)
            .unwrap_or(lang.code())
    }

    pub fn with_detail(&self, key: TextKey, detail: impl std::fmt::Display) -> String {
        format!("{}: {}", self.t(key), detail)
    }
}
