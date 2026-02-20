use crate::i18n::{I18n, TextKey};
use crate::settings::settings;
use crate::shortcuts::ShortcutAction;
use egui_macroquad::egui;

const CATEGORY_ITEM_HEIGHT: f32 = 32.0;
const SHORTCUT_ROW_HEIGHT: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsCategory {
    Shortcuts,
}

impl DocsCategory {
    pub const ALL: &'static [DocsCategory] = &[DocsCategory::Shortcuts];

    pub fn label<'a>(&self, i18n: &'a I18n) -> &'a str {
        match self {
            DocsCategory::Shortcuts => i18n.t(TextKey::DocsCategoryShortcuts),
        }
    }
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

fn draw_shortcut_item(ui: &mut egui::Ui, key: &str, description: &str) {
    let row_width = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, SHORTCUT_ROW_HEIGHT),
        egui::Sense::hover(),
    );

    ui.painter().text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        key,
        egui::TextStyle::Monospace.resolve(ui.style()),
        egui::Color32::from_rgb(160, 200, 255),
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(120.0, 0.0),
        egui::Align2::LEFT_CENTER,
        description,
        egui::TextStyle::Body.resolve(ui.style()),
        egui::Color32::from_rgb(225, 225, 225),
    );
}

fn draw_shortcut_section_title(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.colored_label(
        egui::Color32::from_rgb(180, 200, 255),
        egui::RichText::new(text).size(14.0),
    );
    ui.separator();
}

pub fn draw_docs_window(
    ctx: &egui::Context,
    i18n: &I18n,
    open: &mut bool,
    selected_category: &mut DocsCategory,
) {
    let shortcuts = {
        let s = settings();
        s.shortcuts.clone()
    };

    let mut is_open = *open;
    egui::Window::new(i18n.t(TextKey::MenuDocs))
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .fixed_size(egui::vec2(760.0, 520.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(18, 18, 22, 245))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 55)))
                .inner_margin(egui::Margin::symmetric(12, 0))
                .corner_radius(egui::CornerRadius::same(8)),
        )
        .show(ctx, |ui| {
            ui.set_min_size(egui::vec2(680.0, 470.0));
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(160.0, 460.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(8.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for &cat in DocsCategory::ALL {
                            let is_sel = *selected_category == cat;
                            if draw_category_item(ui, cat.label(i18n), is_sel).clicked() {
                                *selected_category = cat;
                            }
                        }
                    },
                );

                ui.separator();

                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.set_min_width(420.0);

                    ui.colored_label(
                        egui::Color32::from_rgb(180, 200, 255),
                        egui::RichText::new(selected_category.label(i18n)).size(15.0),
                    );
                    ui.separator();

                    match *selected_category {
                        DocsCategory::Shortcuts => {
                            draw_shortcut_section_title(ui, i18n.t(TextKey::DocsSectionGlobal));
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::SaveChart).display(),
                                i18n.t(TextKey::DocsShortcutSaveChart),
                            );
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::Undo).display(),
                                i18n.t(TextKey::DocsShortcutUndo),
                            );
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::Redo).display(),
                                i18n.t(TextKey::DocsShortcutRedo),
                            );
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::ToggleHitsound).display(),
                                i18n.t(TextKey::DocsShortcutToggleHitsound),
                            );
                            draw_shortcut_item(ui, "Space", i18n.t(TextKey::DocsShortcutPlayPause));

                            ui.add_space(8.0);
                            draw_shortcut_section_title(ui, i18n.t(TextKey::DocsSectionEditor));
                            draw_shortcut_item(ui, "Delete", i18n.t(TextKey::DocsShortcutDelete));
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::Copy).display(),
                                i18n.t(TextKey::DocsShortcutCopy),
                            );
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::Cut).display(),
                                i18n.t(TextKey::DocsShortcutCut),
                            );
                            draw_shortcut_item(
                                ui,
                                &shortcuts.chord_for(ShortcutAction::Paste).display(),
                                i18n.t(TextKey::DocsShortcutPasteMode),
                            );
                            draw_shortcut_item(ui, "Ctrl+B", i18n.t(TextKey::DocsShortcutMirror));
                            draw_shortcut_item(ui, "Ctrl+M", i18n.t(TextKey::DocsShortcutMirrorCopy));
                            draw_shortcut_item(ui, "Ctrl+V / Ctrl+B", i18n.t(TextKey::DocsShortcutPasteModeSwitch));

                            ui.add_space(8.0);
                            draw_shortcut_section_title(ui, i18n.t(TextKey::DocsSectionWheel));
                            draw_shortcut_item(ui, "Ctrl+Wheel", i18n.t(TextKey::DocsShortcutWheelSpeed));
                            draw_shortcut_item(ui, "Shift+Wheel", i18n.t(TextKey::DocsShortcutWheelSnapSeek));
                            draw_shortcut_item(ui, "Wheel", i18n.t(TextKey::DocsShortcutWheelSeek));
                            draw_shortcut_item(ui, "Alt+Wheel", i18n.t(TextKey::DocsShortcutWheelSeekAlt));
                        }
                    }
                });
            });
        });

    *open = is_open;
}

