use crate::editor::falling::RenderScope;
use crate::i18n::{I18n, Language, TextKey};
use egui_macroquad::egui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopMenuAction {
    CreateProject,
    OpenProject,
    SaveChart,
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
    SetRenderScope(RenderScope),
    SetScrollSpeed(f32),
    SetScrollSpeedFinal(f32),
    SetSnapDivision(u32),
    SetSnapDivisionFinal(u32),
    SetXSplit(f64),
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
];

pub fn draw_top_menu(
    ctx: &egui::Context,
    i18n: &I18n,
    current_render_scope: RenderScope,
    settings_open: &mut bool,
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
                    ui.separator();
                    draw_popup_item(
                        ui,
                        &mut action,
                        TopMenuAction::SaveChart,
                        i18n.t(TextKey::FileSaveChart),
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

                // Settings button: toggle window instead of popup
                {
                    let btn_response = draw_top_menu_button(ui, i18n.t(TextKey::MenuSettings), *settings_open);
                    if btn_response.clicked() {
                        *settings_open = !*settings_open;
                    }
                }

                // ── Right-aligned render scope toggle switch (animated) ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label_both = i18n.t(TextKey::RenderMerge);
                    let label_split = i18n.t(TextKey::RenderSplit);
                    let is_split = current_render_scope == RenderScope::Split;

                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let both_galley = ui.painter().layout_no_wrap(label_both.to_owned(), font.clone(), egui::Color32::WHITE);
                    let split_galley = ui.painter().layout_no_wrap(label_split.to_owned(), font.clone(), egui::Color32::WHITE);
                    let text_h = both_galley.size().y.max(split_galley.size().y);

                    let half_w = both_galley.size().x.max(split_galley.size().x) + 16.0;
                    let gap = 2.0_f32;
                    let total_w = half_w * 2.0 + gap;
                    let h = (text_h + 8.0).max(TOP_MENU_BUTTON_HEIGHT);

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(total_w, h),
                        egui::Sense::click(),
                    );

                    // Animate t: 0.0 = Both (left), 1.0 = Split (right)
                    // Manual ease-out animation (fast start, slow end), ~100ms duration
                    let anim_id = ui.id().with("render_scope_toggle");
                    let target = if is_split { 1.0_f32 } else { 0.0_f32 };
                    let dt = ctx.input(|i| i.stable_dt).min(0.05);
                    let anim_duration = 0.10_f32; // 100ms
                    let speed = 1.0 / anim_duration;
                    let (raw_t, needs_repaint) = ctx.data_mut(|d| {
                        let current = d.get_temp_mut_or(anim_id, target);
                        // Move current toward target
                        if (*current - target).abs() < 0.001 {
                            *current = target;
                            (*current, false)
                        } else {
                            let dir = if target > *current { 1.0 } else { -1.0 };
                            *current += dir * speed * dt;
                            *current = if dir > 0.0 {
                                current.min(target)
                            } else {
                                current.max(target)
                            };
                            (*current, true)
                        }
                    });
                    if needs_repaint {
                        ctx.request_repaint();
                    }
                    // Ease-out cubic: 1 - (1 - x)^3
                    let linear_t = if target > 0.5 { raw_t } else { 1.0 - raw_t };
                    let eased = 1.0 - (1.0 - linear_t).powi(3);
                    let t = if target > 0.5 { eased } else { 1.0 - eased };

                    // Background pill
                    let rounding = egui::CornerRadius::same((h / 2.0) as u8);
                    ui.painter().rect_filled(
                        rect,
                        rounding,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                    );

                    // Animated highlight slider
                    let slider_x = rect.min.x + t * (half_w + gap);
                    let slider_rect = egui::Rect::from_min_size(
                        egui::pos2(slider_x, rect.min.y),
                        egui::vec2(half_w, h),
                    );
                    ui.painter().rect_filled(
                        slider_rect,
                        rounding,
                        egui::Color32::from_rgba_unmultiplied(106, 168, 255, 80),
                    );

                    // Text positions (fixed, not animated)
                    let left_center = egui::pos2(rect.min.x + half_w * 0.5, rect.center().y);
                    let right_center = egui::pos2(rect.min.x + half_w + gap + half_w * 0.5, rect.center().y);

                    // Interpolate text colors
                    let lerp_u8 = |a: u8, b: u8, f: f32| -> u8 {
                        (a as f32 + (b as f32 - a as f32) * f).round() as u8
                    };
                    let bright = 255_u8;
                    let dim = 160_u8;
                    let both_lum = lerp_u8(bright, dim, t);
                    let split_lum = lerp_u8(dim, bright, t);

                    ui.painter().text(
                        left_center,
                        egui::Align2::CENTER_CENTER,
                        label_both,
                        font.clone(),
                        egui::Color32::from_rgb(both_lum, both_lum, both_lum),
                    );
                    ui.painter().text(
                        right_center,
                        egui::Align2::CENTER_CENTER,
                        label_split,
                        font,
                        egui::Color32::from_rgb(split_lum, split_lum, split_lum),
                    );

                    // Click handling
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let mid_x = rect.min.x + half_w + gap * 0.5;
                            let clicked_scope = if pos.x < mid_x {
                                RenderScope::Both
                            } else {
                                RenderScope::Split
                            };
                            if clicked_scope != current_render_scope {
                                action = Some(TopMenuAction::SetRenderScope(clicked_scope));
                            }
                        }
                    }
                });
            });
        });

    if *settings_open {
        any_popup_open = true;
    }

    TopMenuResult {
        action,
        any_popup_open,
    }
}

