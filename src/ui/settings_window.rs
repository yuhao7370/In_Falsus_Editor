use crate::i18n::{I18n, TextKey};
use crate::settings::settings;
use crate::shortcuts::{ShortcutAction, detect_key_chord};
use crate::ui::input_state::{free_key_down, free_key_pressed};
use crate::ui::snap_slider::draw_snap_slider;
use crate::ui::top_menu::{SettingsAction, TopMenuAction};
use egui_macroquad::egui;
use macroquad::prelude::KeyCode;

const CATEGORY_ITEM_HEIGHT: f32 = 32.0;
const SETTING_ROW_HEIGHT: f32 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    Display,
    Audio,
    Language,
    Shortcuts,
    Debug,
}

impl SettingsCategory {
    pub const ALL: &'static [SettingsCategory] = &[
        SettingsCategory::Display,
        SettingsCategory::Audio,
        SettingsCategory::Language,
        SettingsCategory::Shortcuts,
        SettingsCategory::Debug,
    ];

    pub fn label<'a>(&self, i18n: &'a I18n) -> &'a str {
        match self {
            SettingsCategory::Display => i18n.t(TextKey::SettingsCategoryDisplay),
            SettingsCategory::Audio => i18n.t(TextKey::SettingsCategoryAudio),
            SettingsCategory::Language => i18n.t(TextKey::SettingsCategoryLanguage),
            SettingsCategory::Shortcuts => i18n.t(TextKey::SettingsCategoryShortcuts),
            SettingsCategory::Debug => i18n.t(TextKey::SettingsCategoryDebug),
        }
    }
}

fn shortcut_action_text_key(action: ShortcutAction) -> TextKey {
    match action {
        ShortcutAction::SaveChart => TextKey::ShortcutActionSaveChart,
        ShortcutAction::Undo => TextKey::ShortcutActionUndo,
        ShortcutAction::Redo => TextKey::ShortcutActionRedo,
        ShortcutAction::Cut => TextKey::ShortcutActionCut,
        ShortcutAction::Copy => TextKey::ShortcutActionCopy,
        ShortcutAction::Paste => TextKey::ShortcutActionPaste,
        ShortcutAction::ToggleHitsound => TextKey::ShortcutActionToggleHitsound,
    }
}

fn draw_setting_row(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let row_width = ui.available_width().max(1.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row_width, SETTING_ROW_HEIGHT),
        egui::Sense::click(),
    );

    let is_hot = selected || response.hovered();
    if is_hot {
        let alpha = if selected { 44 } else { 28 };
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(1.0, 0.0)),
            egui::CornerRadius::same(5),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
        );
    }

    let text_color = if selected {
        egui::Color32::from_rgb(252, 252, 252)
    } else {
        egui::Color32::from_rgb(228, 228, 228)
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        egui::TextStyle::Button.resolve(ui.style()),
        text_color,
    );

    response
}

fn draw_category_item(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let row_width = ui.available_width().max(1.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row_width, CATEGORY_ITEM_HEIGHT),
        egui::Sense::click(),
    );

    if selected {
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(2.0, 1.0)),
            egui::CornerRadius::same(5),
            egui::Color32::from_rgba_unmultiplied(106, 168, 255, 50),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect.shrink2(egui::vec2(2.0, 1.0)),
            egui::CornerRadius::same(5),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
        );
    }

    let text_color = if selected {
        egui::Color32::from_rgb(255, 255, 255)
    } else {
        egui::Color32::from_rgb(200, 200, 200)
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        egui::TextStyle::Button.resolve(ui.style()),
        text_color,
    );

    response
}

/// Draw the settings window with left-right split layout.
/// Returns an optional action.
pub fn draw_settings_window(
    ctx: &egui::Context,
    i18n: &I18n,
    open: &mut bool,
    selected_category: &mut SettingsCategory,
    recording_shortcut: &mut Option<ShortcutAction>,
    volume_enabled: bool,
    min_scroll_speed: f32,
    max_scroll_speed: f32,
    scroll_speed_step: f32,
) -> Option<TopMenuAction> {
    let mut action = None;

    // 从全局设置读取当前值（短暂持锁后立即释放）
    let s = settings();
    let current_master_volume = s.master_volume;
    let current_music_volume = s.music_volume;
    let current_debug_hitbox = s.debug_hitbox;
    let current_autoplay = s.autoplay;
    let current_show_spectrum = s.show_spectrum;
    let current_show_barlines = s.show_barlines;
    let current_show_minimap = s.show_minimap;
    let current_scroll_speed = s.scroll_speed;
    let current_snap_division = s.snap_division;
    let current_x_split = s.x_split;
    let current_xsplit_editable = s.xsplit_editable;
    let current_hitsound_enabled = s.hitsound_enabled;
    let current_hitsound_tap_volume = s.hitsound_tap_volume;
    let current_hitsound_arc_volume = s.hitsound_arc_volume;
    let current_hitsound_delay_ms = s.hitsound_delay_ms;
    let current_debug_audio = s.debug_audio;
    let current_shortcuts = s.shortcuts.clone();
    drop(s);

    let mut is_open = *open;
    egui::Window::new(i18n.t(TextKey::MenuSettings))
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .fixed_size(egui::vec2(620.0, 420.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(18, 18, 22, 245))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 55)))
                .inner_margin(egui::Margin::symmetric(12, 0))
                .corner_radius(egui::CornerRadius::same(8)),
        )
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(540.0, 400.0));
            ui.horizontal(|ui| {
                // ── Left: category list ──
                ui.allocate_ui_with_layout(
                    egui::vec2(140.0, 400.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(8.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for &cat in SettingsCategory::ALL {
                            let is_sel = *selected_category == cat;
                            if draw_category_item(ui, cat.label(i18n), is_sel).clicked() {
                                *selected_category = cat;
                            }
                        }
                    },
                );

                // ── Vertical separator ──
                ui.separator();

                // ── Right: settings for selected category ──
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.set_min_width(260.0);
                    ui.set_max_width(420.0);

                    // Category title
                    ui.colored_label(
                        egui::Color32::from_rgb(180, 200, 255),
                        egui::RichText::new(selected_category.label(i18n)).size(15.0),
                    );
                    ui.separator();
                    ui.add_space(4.0);
                    if *selected_category != SettingsCategory::Shortcuts {
                        *recording_shortcut = None;
                    }

                    match *selected_category {
                        SettingsCategory::Language => {
                            for lang in i18n.available_languages() {
                                let is_sel = i18n.language() == &lang;
                                let display = i18n.language_display_name(&lang);
                                if draw_setting_row(ui, display, is_sel).clicked() {
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetLanguage(lang)));
                                }
                            }
                        }
                        SettingsCategory::Audio => {
                            // Master volume
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsMasterVolume))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                            let mut master_vol = current_master_volume.clamp(0.0, 1.0);
                            let master_slider = egui::Slider::new(&mut master_vol, 0.0..=1.0)
                                .show_value(true)
                                .text("");
                            if ui.add_enabled(volume_enabled, master_slider).changed() && volume_enabled {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetMasterVolume(master_vol)));
                            }

                            // Music volume
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::PlayerLabelVolume))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                            let mut music_vol = current_music_volume.clamp(0.0, 1.0);
                            let music_slider = egui::Slider::new(&mut music_vol, 0.0..=1.0)
                                .show_value(true)
                                .text("");
                            if ui.add_enabled(volume_enabled, music_slider).changed() && volume_enabled {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetMusicVolume(music_vol)));
                            }

                            ui.add_space(8.0);
                            // Hitsound toggle
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsHitsoundEnabled), current_hitsound_enabled).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetHitsoundEnabled(!current_hitsound_enabled)));
                            }

                            // Tap hitsound volume (0% ~ 200%)
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsHitsoundTapVolume))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                            let mut tap_vol = current_hitsound_tap_volume;
                            let tap_slider = egui::Slider::new(&mut tap_vol, 0.0..=2.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .show_value(true)
                                .text("");
                            if ui.add_enabled(current_hitsound_enabled, tap_slider).changed() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetHitsoundTapVolume(tap_vol)));
                            }

                            // Arc hitsound volume (0% ~ 200%)
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsHitsoundArcVolume))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                            let mut arc_vol = current_hitsound_arc_volume;
                            let arc_slider = egui::Slider::new(&mut arc_vol, 0.0..=2.0)
                                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                                .show_value(true)
                                .text("");
                            if ui.add_enabled(current_hitsound_enabled, arc_slider).changed() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetHitsoundArcVolume(arc_vol)));
                            }

                            // Hitsound delay (-100ms ~ +100ms)
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsHitsoundDelay))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                            let mut delay = current_hitsound_delay_ms;
                            let delay_slider = egui::Slider::new(&mut delay, -100..=100)
                                .custom_formatter(|v, _| format!("{:+.0} ms", v))
                                .show_value(true)
                                .text("");
                            if ui.add_enabled(current_hitsound_enabled, delay_slider).changed() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetHitsoundDelay(delay)));
                            }
                        }
                        SettingsCategory::Display => {
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsAutoPlay), current_autoplay).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetAutoPlay(!current_autoplay)));
                            }
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsShowSpectrum), current_show_spectrum).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetShowSpectrum(!current_show_spectrum)));
                            }
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsShowBarlines), current_show_barlines).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetShowBarlines(!current_show_barlines)));
                            }
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsShowMinimap), current_show_minimap).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetMinimapVisible(!current_show_minimap)));
                            }
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsFlowSpeed))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new("-").min_size(egui::vec2(28.0, 26.0))).clicked() {
                                    let new_speed = (current_scroll_speed - scroll_speed_step).max(min_scroll_speed);
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetScrollSpeedFinal(new_speed)));
                                }
                                ui.spacing_mut().slider_width = (ui.available_width() - 108.0).max(60.0);
                                let mut speed = current_scroll_speed;
                                let slider = egui::Slider::new(&mut speed, min_scroll_speed..=max_scroll_speed)
                                    .step_by(scroll_speed_step as f64)
                                    .show_value(true)
                                    .text("H/s");
                                let response = ui.add(slider);
                                if response.changed() {
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetScrollSpeed(speed)));
                                }
                                if response.drag_stopped() {
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetScrollSpeedFinal(speed)));
                                }
                                if ui.add(egui::Button::new("+").min_size(egui::vec2(28.0, 26.0))).clicked() {
                                    let new_speed = (current_scroll_speed + scroll_speed_step).min(max_scroll_speed);
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetScrollSpeedFinal(new_speed)));
                                }
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(i18n.t(TextKey::SettingsBarlineSnap))
                                        .color(egui::Color32::from_rgb(210, 210, 210)),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{}x", current_snap_division))
                                        .color(egui::Color32::from_rgb(106, 168, 255)),
                                );
                            });
                            let snap_width = (ui.available_width() - 4.0).max(80.0);
                            let (changed, finished) = draw_snap_slider(ui, current_snap_division, snap_width);
                            if let Some(new_div) = changed {
                                // While dragging, update value silently (no toast)
                                action = Some(TopMenuAction::Settings(SettingsAction::SetSnapDivision(new_div)));
                            }
                            if finished {
                                // On release / click, trigger toast
                                action = Some(TopMenuAction::Settings(SettingsAction::SetSnapDivisionFinal(current_snap_division)));
                            }
                            // If both changed and finished on the same frame, final wins
                            if changed.is_some() && finished {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetSnapDivisionFinal(changed.unwrap())));
                            }
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsXSplit))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(60.0);
                                let mut x_split = current_x_split;
                                let slider = egui::Slider::new(&mut x_split, 1.0..=1024.0)
                                    .logarithmic(true)
                                    .show_value(true)
                                    .text("");
                                let response = ui.add(slider);
                                if response.changed() || response.drag_stopped() {
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetXSplit(x_split)));
                                }
                            });
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsXSplitEditable), current_xsplit_editable).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetXSplitEditable(!current_xsplit_editable)));
                            }
                        }
                        SettingsCategory::Shortcuts => {
                            if let Some(capturing_action) = *recording_shortcut {
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 220, 120),
                                    i18n.t(TextKey::SettingsShortcutPressAnyKey),
                                );
                                if free_key_pressed(KeyCode::Escape) {
                                    *recording_shortcut = None;
                                } else if let Some(chord) = detect_key_chord(free_key_pressed, free_key_down) {
                                    action = Some(TopMenuAction::Settings(SettingsAction::SetShortcut(
                                        capturing_action,
                                        chord,
                                    )));
                                    *recording_shortcut = None;
                                }
                                ui.add_space(4.0);
                            }

                            ui.label(
                                egui::RichText::new(i18n.t(TextKey::SettingsShortcutEditable))
                                    .color(egui::Color32::from_rgb(210, 210, 210)),
                            );
                            ui.add_space(2.0);
                            for shortcut_action in ShortcutAction::ALL {
                                if !shortcut_action.is_editable() {
                                    continue;
                                }
                                let is_recording = *recording_shortcut == Some(shortcut_action);
                                let chord_text = if is_recording {
                                    i18n.t(TextKey::SettingsShortcutPressAnyKey).to_owned()
                                } else {
                                    current_shortcuts.chord_for(shortcut_action).display()
                                };
                                let is_default =
                                    current_shortcuts.chord_for(shortcut_action)
                                        == shortcut_action.default_chord();

                                ui.horizontal(|ui| {
                                    ui.label(i18n.t(shortcut_action_text_key(shortcut_action)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let btn_label = if is_recording {
                                            i18n.t(TextKey::SettingsShortcutCancel)
                                        } else {
                                            i18n.t(TextKey::SettingsShortcutChange)
                                        };
                                        if ui.button(btn_label).clicked() {
                                            if is_recording {
                                                *recording_shortcut = None;
                                            } else {
                                                *recording_shortcut = Some(shortcut_action);
                                            }
                                        }
                                        if ui
                                            .add_enabled(
                                                !is_default,
                                                egui::Button::new(i18n.t(TextKey::SettingsShortcutReset)),
                                            )
                                            .clicked()
                                        {
                                            action = Some(TopMenuAction::Settings(
                                                SettingsAction::ResetShortcut(shortcut_action),
                                            ));
                                        }
                                        ui.label(
                                            egui::RichText::new(chord_text)
                                                .monospace()
                                                .color(egui::Color32::from_rgb(160, 200, 255)),
                                        );
                                    });
                                });
                            }
                        }
                        SettingsCategory::Debug => {
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsDebugHitbox), current_debug_hitbox).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetDebugHitbox(!current_debug_hitbox)));
                            }
                            if draw_setting_row(ui, i18n.t(TextKey::SettingsDebugAudio), current_debug_audio).clicked() {
                                action = Some(TopMenuAction::Settings(SettingsAction::SetDebugAudio(!current_debug_audio)));
                            }
                        }
                    }

                    ui.add_space(8.0);
                });
            });
        });
    *open = is_open;
    if !is_open {
        *recording_shortcut = None;
    }

    action
}
