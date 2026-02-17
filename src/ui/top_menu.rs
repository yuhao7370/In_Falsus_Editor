use crate::i18n::{I18n, Language, TextKey};
use egui_macroquad::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopMenuAction {
    CreateProject,
    OpenProject,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SetLanguage(Language),
    SetVolume(f32),
    SetDebugHitbox(bool),
    SetAutoPlay(bool),
    SetShowSpectrum(bool),
    SetMinimapVisible(bool),
}

const TOP_MENU_BUTTON_WIDTH: f32 = 83.0;
const TOP_MENU_BUTTON_HEIGHT: f32 = 28.0;
const POPUP_ITEM_HEIGHT: f32 = 30.0;

fn draw_popup_row(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let row_width = ui.available_width().max(1.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row_width, POPUP_ITEM_HEIGHT),
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

fn draw_top_menu_button(ui: &mut egui::Ui, text: &str, is_open: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(TOP_MENU_BUTTON_WIDTH, TOP_MENU_BUTTON_HEIGHT),
        egui::Sense::click(),
    );

    let is_hot = is_open || response.hovered();
    if is_hot {
        ui.painter().rect_filled(
            rect.shrink(1.0),
            egui::CornerRadius::same(6),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 22),
        );
    }

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::TextStyle::Button.resolve(ui.style()),
        egui::Color32::from_rgb(235, 235, 235),
    );

    response
}

fn draw_popup_item(
    ui: &mut egui::Ui,
    action: &mut Option<TopMenuAction>,
    item_action: TopMenuAction,
    text: &str,
) {
    if draw_popup_row(ui, text, false).clicked() {
        *action = Some(item_action);
        ui.memory_mut(|mem| mem.close_popup());
    }
}

fn draw_top_button_with_popup<R>(
    ui: &mut egui::Ui,
    id_source: &'static str,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) {
    let popup_id = ui.make_persistent_id(id_source);
    let is_open = ui.memory(|mem| mem.is_popup_open(popup_id));
    let response = draw_top_menu_button(ui, label, is_open);

    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    ui.scope(|ui| {
        let style = ui.style_mut();
        style.visuals.window_fill = egui::Color32::from_rgba_unmultiplied(8, 8, 8, 238);
        style.visuals.window_stroke = egui::Stroke::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.spacing.menu_margin = egui::Margin::same(0);

        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &response,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |popup_ui| {
                popup_ui.spacing_mut().item_spacing.y = 2.0;
                popup_ui.set_min_width(150.0);
                add_contents(popup_ui);
            },
        );
    });
}

/// Result of drawing the top menu: the optional action plus whether any popup is open.
pub struct TopMenuResult {
    pub action: Option<TopMenuAction>,
    pub any_popup_open: bool,
}

const POPUP_IDS: &[&str] = &[
    "top_menu_file",
    "top_menu_edit",
    "top_menu_select",
    "top_menu_settings",
];

pub fn draw_top_menu(
    ctx: &egui::Context,
    i18n: &I18n,
    current_volume: f32,
    volume_enabled: bool,
    current_debug_hitbox: bool,
    current_autoplay: bool,
    current_show_spectrum: bool,
    current_show_minimap: bool,
) -> TopMenuResult {
    let mut action = None;
    let mut any_popup_open = false;

    egui::TopBottomPanel::top("main_top_menu")
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(8, 8, 8))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(24, 24, 24)))
                .inner_margin(egui::Margin::same(2)),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.horizontal(|ui| {
                // Check if any popup was open at the start of this frame
                for &id_src in POPUP_IDS {
                    let pid = ui.make_persistent_id(id_src);
                    if ui.memory(|mem| mem.is_popup_open(pid)) {
                        any_popup_open = true;
                        break;
                    }
                }

                draw_top_button_with_popup(ui, "top_menu_file", i18n.t(TextKey::MenuFile), |ui| {
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::CreateProject,
                        i18n.t(TextKey::FileCreateProject),
                    );
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::OpenProject,
                        i18n.t(TextKey::FileOpenProject),
                    );
                });

                draw_top_button_with_popup(ui, "top_menu_edit", i18n.t(TextKey::MenuEdit), |ui| {
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::Undo,
                        i18n.t(TextKey::EditUndo),
                    );
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::Redo,
                        i18n.t(TextKey::EditRedo),
                    );
                    ui.separator();
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::Cut,
                        i18n.t(TextKey::EditCut),
                    );
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::Copy,
                        i18n.t(TextKey::EditCopy),
                    );
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::Paste,
                        i18n.t(TextKey::EditPaste),
                    );
                });

                draw_top_button_with_popup(
                    ui,
                    "top_menu_select",
                    i18n.t(TextKey::MenuSelect),
                    |_ui| {},
                );

                draw_top_button_with_popup(
                    ui,
                    "top_menu_settings",
                    i18n.t(TextKey::MenuSettings),
                    |ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(210, 210, 210),
                            i18n.t(TextKey::SettingsLanguage),
                        );
                        ui.separator();

                        let zh_selected = i18n.language() == Language::ZhCn;
                        if draw_popup_row(ui, i18n.t(TextKey::LanguageChinese), zh_selected)
                            .clicked()
                        {
                            action = Some(TopMenuAction::SetLanguage(Language::ZhCn));
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        let en_selected = i18n.language() == Language::EnUs;
                        if draw_popup_row(ui, i18n.t(TextKey::LanguageEnglish), en_selected)
                            .clicked()
                        {
                            action = Some(TopMenuAction::SetLanguage(Language::EnUs));
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        ui.separator();
                        ui.colored_label(
                            egui::Color32::from_rgb(210, 210, 210),
                            i18n.t(TextKey::PlayerLabelVolume),
                        );
                        let mut volume = current_volume.clamp(0.0, 1.0);
                        let slider = egui::Slider::new(&mut volume, 0.0..=1.0)
                            .show_value(true)
                            .text("");
                        let response = ui.add_enabled(volume_enabled, slider);
                        if response.changed() && volume_enabled {
                            action = Some(TopMenuAction::SetVolume(volume));
                        }

                        ui.separator();
                        if draw_popup_row(ui, i18n.t(TextKey::SettingsAutoPlay), current_autoplay)
                            .clicked()
                        {
                            action = Some(TopMenuAction::SetAutoPlay(!current_autoplay));
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        if draw_popup_row(ui, i18n.t(TextKey::SettingsShowSpectrum), current_show_spectrum)
                            .clicked()
                        {
                            action = Some(TopMenuAction::SetShowSpectrum(!current_show_spectrum));
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        ui.separator();
                        let debug_selected = current_debug_hitbox;
                        if draw_popup_row(ui, i18n.t(TextKey::SettingsDebugHitbox), debug_selected)
                            .clicked()
                        {
                            action = Some(TopMenuAction::SetDebugHitbox(!current_debug_hitbox));
                            ui.memory_mut(|mem| mem.close_popup());
                        }

                        let minimap_selected = current_show_minimap;
                        if draw_popup_row(
                            ui,
                            i18n.t(TextKey::SettingsShowMinimap),
                            minimap_selected,
                        )
                        .clicked()
                        {
                            action = Some(TopMenuAction::SetMinimapVisible(!current_show_minimap));
                            ui.memory_mut(|mem| mem.close_popup());
                        }
                    },
                );
            });
        });

    TopMenuResult {
        action,
        any_popup_open,
    }
}
