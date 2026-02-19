mod audio;
mod chart;
mod editor;
mod i18n;
mod settings;
mod ui;

use audio::controller::AudioController;
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::{I18n, Language, TextKey};
use macroquad::prelude::*;
use ui::fonts::{init_egui_fonts, load_macroquad_cjk_font};
use ui::info_toast::InfoToastManager;
use ui::note_panel::{NOTE_PANEL_BASE_WIDTH_POINTS, PropertyEditState, draw_note_selector_panel, draw_snap_slider_panel};
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::{BASE_HEIGHT, BASE_WIDTH, ui_scale_factor};
use ui::input_state::{set_pointer_blocked, set_keyboard_blocked, safe_mouse_wheel, free_mouse_wheel};
use ui::top_menu::{TopMenuAction, TopMenuResult, draw_top_menu};
use ui::settings_window::{SettingsCategory, draw_settings_window};
use ui::audio_debug_window::draw_audio_debug_window;
use ui::current_project_window::{CurrentProjectAction, CurrentProjectState, draw_current_project_window};
use ui::create_project_window::{CreateProjectParams, CreateProjectState, draw_create_project_window};
use ui::loading_status::{LoadAction, ProjectLoader};
use settings::AppSettings;

const TOP_BAR_HEIGHT: f32 = 32.0;
const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;

/// 开发模式：设为 true 时启动自动加载下方指定的谱面和音频，方便调试。
const DEV_MODE: bool = true;
// const DEV_CHART_PATH: &str = "songs/alamode/alamode3.spc";
// const DEV_AUDIO_PATH: &str = "songs/alamode/music.ogg";
const DEV_CHART_PATH: &str = "testchart/grievouslady2.spc";
const DEV_AUDIO_PATH: &str = "testchart/music.ogg";

fn window_conf() -> Conf {
    Conf {
        window_title: "In Falsus Editor".to_owned(),
        window_width: BASE_WIDTH as i32,
        window_height: BASE_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

fn handle_top_menu_action(
    action: TopMenuAction,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &mut I18n,
    info_toasts: &mut InfoToastManager,
    app_settings: &mut AppSettings,
) {
    match action {
        TopMenuAction::CreateProject => {
            audio.status = i18n.t(TextKey::ActionCreateProject).to_owned();
        }
        TopMenuAction::OpenProject | TopMenuAction::CurrentProject => {
            // Handled separately in main loop
            audio.status.clear();
        }
        TopMenuAction::SaveChart => {
            match editor.save_chart() {
                Ok(()) => {
                    audio.status = format!("谱面已保存: {}", editor.chart_path());
                }
                Err(e) => {
                    audio.status = format!("保存失败: {e}");
                }
            }
        }
        TopMenuAction::HotReloadChart => {
            match editor.reload_chart() {
                Ok(true) => {
                    audio.status = i18n.t(TextKey::ActionHotReloadChart).to_owned();
                }
                Ok(false) => {
                    audio.status = i18n.t(TextKey::ActionHotReloadChartNoChange).to_owned();
                }
                Err(e) => {
                    audio.status = format!("{}: {e}", i18n.t(TextKey::ActionHotReloadChartFailed));
                }
            }
        }
        TopMenuAction::Undo => {
            if !editor.undo() {
                info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo));
            }
            audio.status = i18n.t(TextKey::ActionUndo).to_owned();
        }
        TopMenuAction::Redo => {
            if !editor.redo() {
                info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo));
            }
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
            app_settings.set_language_from(language);
            app_settings.save();
            audio.status = match language {
                Language::ZhCn => i18n.t(TextKey::ActionSetLanguageZh).to_owned(),
                Language::EnUs => i18n.t(TextKey::ActionSetLanguageEn).to_owned(),
            };
        }
        TopMenuAction::SetMasterVolume(vol) => {
            audio.set_master_volume(vol, i18n);
            app_settings.master_volume = vol;
            app_settings.save();
            audio.status.clear();
        }
        TopMenuAction::SetMusicVolume(vol) => {
            audio.set_music_volume(vol, i18n);
            app_settings.music_volume = vol;
            app_settings.save();
            audio.status.clear();
        }
        TopMenuAction::SetAutoPlay(enabled) => {
            editor.set_autoplay_enabled(enabled);
            app_settings.autoplay = enabled;
            app_settings.save();
        }
        TopMenuAction::SetShowSpectrum(enabled) => {
            editor.set_show_spectrum(enabled);
            app_settings.show_spectrum = enabled;
            app_settings.save();
        }
        TopMenuAction::SetDebugHitbox(enabled) => {
            editor.set_debug_show_hitboxes(enabled);
            app_settings.debug_hitbox = enabled;
            app_settings.save();
            audio.status = if enabled {
                i18n.t(TextKey::ActionDebugHitboxOn).to_owned()
            } else {
                i18n.t(TextKey::ActionDebugHitboxOff).to_owned()
            };
        }
        TopMenuAction::SetMinimapVisible(enabled) => {
            editor.set_show_minimap(enabled);
            app_settings.show_minimap = enabled;
            app_settings.save();
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
            // Slider dragging — no toast
            audio.status.clear();
        }
        TopMenuAction::SetScrollSpeedFinal(speed) => {
            editor.set_scroll_speed(speed);
            app_settings.scroll_speed = speed;
            app_settings.save();
            audio.status = format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), speed);
        }
        TopMenuAction::SetSnapDivision(division) => {
            editor.set_snap_division(division);
            // Dragging — no toast
            audio.status.clear();
        }
        TopMenuAction::SetSnapDivisionFinal(division) => {
            editor.set_snap_division(division);
            app_settings.snap_division = division;
            app_settings.save();
            audio.status = format!("{}: {}x", i18n.t(TextKey::SettingsBarlineSnap), division);
        }
        TopMenuAction::SetXSplit(value) => {
            editor.set_x_split(value);
            app_settings.x_split = value;
            app_settings.save();
            audio.status = format!("{}: {}", i18n.t(TextKey::SettingsXSplit), value);
        }
        TopMenuAction::SetXSplitEditable(enabled) => {
            editor.set_xsplit_editable(enabled);
            app_settings.xsplit_editable = enabled;
            app_settings.save();
        }
        TopMenuAction::SetHitsoundEnabled(enabled) => {
            audio.set_hitsound_enabled(enabled);
            app_settings.hitsound_enabled = enabled;
            app_settings.save();
        }
        TopMenuAction::SetHitsoundTapVolume(vol) => {
            audio.set_hitsound_tap_volume(vol);
            app_settings.hitsound_tap_volume = vol;
            app_settings.save();
        }
        TopMenuAction::SetHitsoundArcVolume(vol) => {
            audio.set_hitsound_arc_volume(vol);
            app_settings.hitsound_arc_volume = vol;
            app_settings.save();
        }
        TopMenuAction::SetHitsoundDelay(ms) => {
            audio.set_hitsound_delay_ms(ms);
            app_settings.hitsound_delay_ms = ms;
            app_settings.save();
        }
        TopMenuAction::SetDebugAudio(enabled) => {
            app_settings.debug_audio = enabled;
            app_settings.save();
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app_settings = AppSettings::load();
    let mut i18n = I18n::new(app_settings.language_enum());
    let mut egui_fonts_ready = false;

    // DEV_MODE: 自动加载指定谱面和音频；否则启动空编辑器
    let (mut editor, mut audio) = if DEV_MODE {
        (
            FallingGroundEditor::new(DEV_CHART_PATH),
            AudioController::new(&i18n, DEV_AUDIO_PATH),
        )
    } else {
        (
            FallingGroundEditor::from_chart_path(""),
            AudioController::new_empty(&i18n),
        )
    };
    let mut top_progress_state = TopProgressBarState::new();
    let mut settings_open = false;
    let mut settings_category = SettingsCategory::Display;
    let mut info_toasts = InfoToastManager::new();
    let mut create_project_state = CreateProjectState::new();
    let mut current_project_state = CurrentProjectState::new();
    let mut prop_edit_state = PropertyEditState::default();
    let mut project_loader = ProjectLoader::new();
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    // Apply saved settings
    editor.set_scroll_speed(app_settings.scroll_speed);
    editor.set_snap_division(app_settings.snap_division);
    editor.set_autoplay_enabled(app_settings.autoplay);
    editor.set_show_spectrum(app_settings.show_spectrum);
    editor.set_show_minimap(app_settings.show_minimap);
    editor.set_x_split(app_settings.x_split);
    editor.set_xsplit_editable(app_settings.xsplit_editable);
    editor.set_debug_show_hitboxes(app_settings.debug_hitbox);
    audio.set_master_volume(app_settings.master_volume, &i18n);
    audio.set_music_volume(app_settings.music_volume, &i18n);
    audio.set_hitsound_enabled(app_settings.hitsound_enabled);
    audio.set_hitsound_tap_volume(app_settings.hitsound_tap_volume);
    audio.set_hitsound_arc_volume(app_settings.hitsound_arc_volume);
    audio.set_hitsound_max_voices(app_settings.hitsound_max_voices);
    audio.set_hitsound_delay_ms(app_settings.hitsound_delay_ms);
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }
    // info_toasts.push("Info toasts enabled. Press F8 for multi-toast test.");

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
        let editor_bottom_pad = 8.0 * ui_scale;

        let mut top_menu_result = TopMenuResult { action: None, any_popup_open: false };
        let mut egui_wants_pointer = false;
        let mut egui_wants_keyboard = false;
        let mut total_right_panels_px = note_panel_width_px;
        let mut open_project_result: Option<(String, String)> = None;
        let mut create_project_result: Option<CreateProjectParams> = None;
        let mut current_project_action: Option<CurrentProjectAction> = None;
        egui_macroquad::ui(|ctx| {
            if !egui_fonts_ready {
                let _ = init_egui_fonts(ctx);
                egui_fonts_ready = true;
            }
            ctx.set_pixels_per_point(ui_scale);
            let master_volume = audio.master_volume();
            let music_volume = audio.music_volume();
            top_menu_result = draw_top_menu(
                ctx,
                &i18n,
                editor.render_scope(),
                &mut settings_open,
            );
            // Draw settings window (if open)
            if settings_open {
                if let Some(settings_action) = draw_settings_window(
                    ctx,
                    &i18n,
                    &mut settings_open,
                    &mut settings_category,
                    master_volume,
                    music_volume,
                    audio.has_player(),
                    editor.debug_show_hitboxes(),
                    editor.autoplay_enabled(),
                    editor.show_spectrum(),
                    editor.show_minimap(),
                    editor.scroll_speed(),
                    editor.min_scroll_speed(),
                    editor.max_scroll_speed(),
                    editor.scroll_speed_step(),
                    editor.snap_division(),
                    editor.x_split(),
                    editor.xsplit_editable(),
                    audio.hitsound_enabled(),
                    audio.hitsound_tap_volume(),
                    audio.hitsound_arc_volume(),
                    audio.hitsound_delay_ms(),
                    app_settings.debug_audio,
                ) {
                    top_menu_result.action = Some(settings_action);
                }
            }
            note_panel_width_px = draw_note_selector_panel(ctx, &i18n, &mut editor, &mut prop_edit_state, &mut info_toasts);
            let snap_panel_px = draw_snap_slider_panel(
                ctx,
                &mut editor,
                note_panel_width_px,
                menu_height + top_bar_height + 4.0 * ui_scale,
            );
            // note_panel_width_px is for progress bar (excludes snap panel).
            // total_right_panels_px includes snap panel for editor width.
            total_right_panels_px = note_panel_width_px + snap_panel_px;
            egui_wheel_y = ctx.input(|i| i.raw_scroll_delta.y);
            // Draw create project window (if open)
            create_project_result = draw_create_project_window(ctx, &i18n, &mut create_project_state);
            // Draw current project window (if open)
            current_project_action = draw_current_project_window(ctx, &i18n, &mut current_project_state);
            // Draw audio debug window (if enabled)
            if app_settings.debug_audio {
                let snapshot = audio.debug_snapshot();
                draw_audio_debug_window(ctx, &mut app_settings.debug_audio, &snapshot);
            }
            // Check if pointer is over egui widgets/panels.
            let raw_egui_pointer = ctx.is_using_pointer()
                || ctx.is_pointer_over_area()
                || top_menu_result.any_popup_open;
            egui_wants_pointer = raw_egui_pointer;
            // 键盘拦截：仅当 egui 文本框获得焦点或弹窗/窗口打开时阻断键盘快捷键
            egui_wants_keyboard = ctx.wants_keyboard_input()
                || top_menu_result.any_popup_open;
        });
        set_pointer_blocked(egui_wants_pointer);
        set_keyboard_blocked(egui_wants_keyboard);

        // Handle CreateProject action: open the create project window
        if top_menu_result.action == Some(TopMenuAction::CreateProject) {
            create_project_state.reset();
            create_project_state.open = true;
            top_menu_result.action = None; // consume it
        }

        // Handle CurrentProject action: 打开当前项目信息窗口
        if top_menu_result.action == Some(TopMenuAction::CurrentProject) {
            let cp = editor.chart_path().to_string();
            current_project_state.chart_path = cp.clone();
            current_project_state.audio_path = audio.track_path().unwrap_or("").to_string();
            // Derive project_dir from chart_path parent
            current_project_state.project_dir = std::path::Path::new(&cp)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            current_project_state.open = true;
            top_menu_result.action = None; // consume it
        }

        // Handle OpenProject action: 直接弹出文件选择器选 .iffproj
        if top_menu_result.action == Some(TopMenuAction::OpenProject) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("IFF Project", &["iffproj"])
                .pick_file()
            {
                // 获取 iffproj 文件所在目录，用于解析相对路径
                let proj_dir = path.parent().unwrap_or(std::path::Path::new("."));
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(json) => {
                                let chart = json.get("chart_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                                let audio_val = json.get("audio_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                                if let (Some(cp_raw), Some(ap_raw)) = (chart, audio_val) {
                                    // 将相对路径解析为基于 iffproj 目录的绝对路径
                                    let cp_path = std::path::Path::new(&cp_raw);
                                    let ap_path = std::path::Path::new(&ap_raw);
                                    let cp = if cp_path.is_absolute() { cp_raw } else { proj_dir.join(cp_path).to_string_lossy().to_string() };
                                    let ap = if ap_path.is_absolute() { ap_raw } else { proj_dir.join(ap_path).to_string_lossy().to_string() };
                                    open_project_result = Some((cp, ap));
                                } else {
                                    info_toasts.push_warn("iffproj 文件缺少 chart_path 或 audio_path 字段");
                                }
                            }
                            Err(e) => {
                                info_toasts.push_warn(format!("解析 iffproj 失败: {e}"));
                            }
                        }
                    }
                    Err(e) => {
                        info_toasts.push_warn(format!("读取 iffproj 失败: {e}"));
                    }
                }
            }
            top_menu_result.action = None; // consume it
        }

        if let Some(action) = top_menu_result.action {
            audio.status.clear();
            handle_top_menu_action(action, &mut editor, &mut audio, &mut i18n, &mut info_toasts, &mut app_settings);
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        // Handle open project result → 启动异步加载
        if let Some((chart_path, audio_path)) = open_project_result {
            if !project_loader.is_loading() {
                project_loader.start_open_project(chart_path, audio_path);
                info_toasts.pin(project_loader.status_text());
            }
        }

        // Handle current project window actions (load missing chart/audio)
        if let Some(cp_action) = current_project_action {
            match cp_action {
                CurrentProjectAction::LoadChart(chart_path) => {
                    let audio_path = current_project_state.audio_path.clone();
                    if !project_loader.is_loading() {
                        project_loader.start_open_project(chart_path, audio_path);
                        info_toasts.pin(project_loader.status_text());
                    }
                }
                CurrentProjectAction::LoadAudio(audio_path) => {
                    let chart_path = current_project_state.chart_path.clone();
                    if !project_loader.is_loading() {
                        project_loader.start_open_project(chart_path, audio_path);
                        info_toasts.pin(project_loader.status_text());
                    }
                }
            }
        }

        // Handle create project result → 启动异步创建+加载
        if let Some(params) = create_project_result {
            if !project_loader.is_loading() {
                project_loader.start_create_project(
                    params.name,
                    params.source_audio,
                    params.bpm,
                    params.bpl,
                );
                info_toasts.pin(project_loader.status_text());
            }
        }

        // Tick ProjectLoader 状态机
        {
            let prev_status = project_loader.status_text().to_owned();
            let action = project_loader.tick();
            // 状态文本变化时更新 pinned toast
            let new_status = project_loader.status_text();
            if new_status != prev_status {
                if new_status.is_empty() {
                    info_toasts.dismiss_pinned();
                } else {
                    info_toasts.pin(new_status);
                }
            }
            match action {
                LoadAction::None => {}
                LoadAction::LoadChart { chart_path, audio_path } => {
                    let font_backup = macroquad_font.clone();
                    editor = FallingGroundEditor::from_chart_path(&chart_path);
                    editor.set_text_font(font_backup);
                    // 应用已保存的编辑器设置
                    editor.set_scroll_speed(app_settings.scroll_speed);
                    editor.set_snap_division(app_settings.snap_division);
                    editor.set_autoplay_enabled(app_settings.autoplay);
                    editor.set_show_spectrum(app_settings.show_spectrum);
                    editor.set_show_minimap(app_settings.show_minimap);
                    editor.set_x_split(app_settings.x_split);
                    editor.set_xsplit_editable(app_settings.xsplit_editable);
                    editor.set_debug_show_hitboxes(app_settings.debug_hitbox);
                    // 进入下一阶段：读取音频字节
                    project_loader.advance_after_chart_load(chart_path, audio_path);
                    info_toasts.pin(project_loader.status_text());
                }
                LoadAction::InstallAudio { clip, chart_path, audio_path } => {
                    audio.install_decoded_audio(clip, &audio_path, &i18n);
                    project_loader.finish();
                    info_toasts.dismiss_pinned();
                    info_toasts.push(format!("项目已加载: {}", chart_path));
                }
                LoadAction::Error(e) => {
                    project_loader.finish();
                    info_toasts.dismiss_pinned();
                    info_toasts.push_warn(format!("加载失败: {}", e));
                }
            }
        }

        // Ctrl+S: save chart, Ctrl+Z: undo, Ctrl+Y: redo
        {
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::S) {
                match editor.save_chart() {
                    Ok(()) => {
                        info_toasts.push(format!("谱面已保存: {}", editor.chart_path()));
                    }
                    Err(e) => {
                        info_toasts.push(format!("保存失败: {e}"));
                    }
                }
            }
            if ctrl && is_key_pressed(KeyCode::Z) {
                if editor.undo() {
                    info_toasts.push(i18n.t(TextKey::ActionUndo));
                } else {
                    info_toasts.push_warn(i18n.t(TextKey::ActionNothingToUndo));
                }
            }
            if ctrl && is_key_pressed(KeyCode::Y) {
                if editor.redo() {
                    info_toasts.push(i18n.t(TextKey::ActionRedo));
                } else {
                    info_toasts.push_warn(i18n.t(TextKey::ActionNothingToRedo));
                }
            }
        }

        // if is_key_pressed(KeyCode::F8) {
        //     info_toasts.push("Info A: multi-toast test");
        //     info_toasts.push("Info B: animation should be smooth");
        //     info_toasts.push("Info C: dismisses in queue order");
        // }

        // 4. Wheel: Ctrl+wheel = flow speed (free, ignores egui block), otherwise seek
        let (_, free_wheel_y) = free_mouse_wheel();
        let (_, mq_wheel_y) = safe_mouse_wheel();
        let ctrl_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if egui_wants_pointer {
            egui_wheel_y = 0.0;
        }
        let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        if ctrl_down && free_wheel_y.abs() > f32::EPSILON {
            let step = editor.scroll_speed_step();
            let delta = if free_wheel_y > 0.0 { step } else { -step };
            editor.nudge_scroll_speed(delta);
            app_settings.scroll_speed = editor.scroll_speed();
            app_settings.save();
            info_toasts.push(format!("{}: {:.2} H/s", i18n.t(TextKey::SettingsFlowSpeed), editor.scroll_speed()));
        } else if shift_down && free_wheel_y.abs() > f32::EPSILON && !audio.is_playing() && audio.duration_sec() > 0.0 {
            let forward = free_wheel_y > 0.0;
            let current_ms = audio.current_sec() * 1000.0;
            let target_ms = editor.snap_seek_ms(current_ms, forward);
            let target_sec = (target_ms / 1000.0).clamp(0.0, audio.duration_sec());
            audio.handle_editor_seek(target_sec, &i18n);
        } else {
            audio.handle_wheel_seek(mq_wheel_y, egui_wheel_y, space_consumed, &i18n);
        }

        // Read snapshot values after input mutations this frame
        let mut current_sec = audio.current_sec();
        let duration_sec = audio.duration_sec();
        let track_path = audio.track_path().map(|s| s.to_owned());
        let is_playing = audio.is_playing();
        let editor_width =
            (screen_width() - panel_pad * 2.0 - total_right_panels_px - editor_gap).max(360.0);
        // 5. Top progress bar
        let progress_output = draw_top_progress_bar(
            ui_scale,
            menu_height,
            top_bar_height,
            note_panel_width_px,
            current_sec,
            duration_sec,
            is_playing,
            macroquad_font.as_ref(),
            &mut top_progress_state,
        );
        current_sec = progress_output.display_sec;
        if let Some(seek_sec) = progress_output.seek_to_sec {
            audio.handle_editor_seek(seek_sec, &i18n);
            current_sec = audio.current_sec();
        }

        // 6. Editor
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - editor_bottom_pad).max(140.0),
        );

        for action in editor.draw(
            editor_rect,
            current_sec,
            duration_sec,
            track_path.as_deref(),
            is_playing,
        ) {
            match action {
                FallingEditorAction::SeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                    info_toasts.push(format!("seek {:.2}s", sec));
                }
                FallingEditorAction::MinimapSeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                }
            }
        }

        // 7. Hitsound triggering
        {
            let note_heads = editor.note_head_times();
            audio.trigger_hitsounds(&note_heads);
        }

        // 8. Toast 通知
        info_toasts.draw(
            ui_scale,
            menu_height + top_bar_height,
            macroquad_font.as_ref(),
        );

        egui_macroquad::draw();
        next_frame().await;
    }
}
