use crate::editor::falling::{
    FallingGroundEditor, PlaceEventType, PlaceNoteType,
};
use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;

pub const NOTE_PANEL_BASE_WIDTH_POINTS: f32 = 280.0;

fn tool_caption(tool: PlaceNoteType) -> &'static str {
    match tool {
        PlaceNoteType::Tap => "Tap",
        PlaceNoteType::Hold => "Hold",
        PlaceNoteType::Flick => "Flick",
        PlaceNoteType::SkyArea => "SkyArea",
    }
}

fn event_caption(tool: PlaceEventType) -> &'static str {
    match tool {
        PlaceEventType::Bpm => "Bpm",
        PlaceEventType::Track => "Track",
        PlaceEventType::Lane => "Lane",
    }
}

fn draw_tool_row(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let h = 34.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::click());

    let fill = if selected {
        egui::Color32::from_rgba_unmultiplied(106, 168, 255, 70)
    } else if response.hovered() {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(
        rect.shrink2(egui::vec2(0.0, 1.0)),
        egui::CornerRadius::same(6),
        fill,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        egui::Color32::from_rgb(236, 236, 242),
    );

    response
}

pub fn draw_note_selector_panel(
    ctx: &egui::Context,
    i18n: &I18n,
    editor: &mut FallingGroundEditor,
) -> f32 {
    let ppp = ctx.pixels_per_point().max(0.000_1);
    let panel_width_points = NOTE_PANEL_BASE_WIDTH_POINTS;

    let panel = egui::SidePanel::right("note_selector_panel")
        .resizable(false)
        .min_width(panel_width_points)
        .max_width(panel_width_points)
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(10, 10, 14, 236))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                ))
                .inner_margin(egui::Margin::same(8)),
        )
        .show(ctx, |ui| {
            ui.label("Note");

            let current_note = editor.place_note_type();
            let current_event = editor.place_event_type();
            for tool in [
                PlaceNoteType::Tap,
                PlaceNoteType::Hold,
                PlaceNoteType::Flick,
                PlaceNoteType::SkyArea,
            ] {
                let response = draw_tool_row(ui, tool_caption(tool), current_note == Some(tool));
                if response.clicked() {
                    editor.set_place_note_type(Some(tool));
                }
                if response.secondary_clicked() {
                    editor.set_place_note_type(None);
                }
            }

            ui.separator();
            ui.label("Event");
            for tool in [
                PlaceEventType::Bpm,
                PlaceEventType::Track,
                PlaceEventType::Lane,
            ] {
                let response = draw_tool_row(ui, event_caption(tool), current_event == Some(tool));
                if response.clicked() {
                    editor.set_place_event_type(Some(tool));
                }
                if response.secondary_clicked() {
                    editor.set_place_event_type(None);
                }
            }

            ui.separator();
            let mode_text = current_note
                .map(tool_caption)
                .or_else(|| current_event.map(event_caption))
                .unwrap_or("None");
            ui.label(format!("Current: {mode_text}"));
            if let Some(head_time_ms) = editor.pending_hold_head_time_ms() {
                ui.label(format!("Hold head: {:.0}ms", head_time_ms));
                ui.label("Click again to set tail");
            }
            if let Some(head_time_ms) = editor.pending_skyarea_head_time_ms() {
                ui.label(format!("SkyArea head: {:.0}ms", head_time_ms));
                ui.label("Click again to set tail");
            }

            // Bottom status line
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Speed: {:.2} H/s   Snap: {}x",
                        editor.scroll_speed(),
                        editor.snap_division(),
                    ))
                    .color(egui::Color32::from_rgb(160, 160, 170))
                    .size(12.0),
                );
                ui.separator();
                let mut enabled = editor.track_speed_enabled();
                let track_speed_label = egui::RichText::new(i18n.t(TextKey::NotePanelRenderSpeedEvents))
                    .size(14.0)
                    .color(egui::Color32::from_rgb(220, 220, 230));
                if ui
                    .add(egui::Checkbox::new(&mut enabled, track_speed_label))
                    .changed()
                {
                    editor.set_track_speed_enabled(enabled);
                }
                ui.separator();
            });
        });

    if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
        let cancel_on_panel = ctx.input(|i| i.pointer.secondary_clicked())
            && panel.response.rect.contains(pointer_pos);
        if cancel_on_panel {
            editor.set_place_note_type(None);
            editor.set_place_event_type(None);
        }
    }

    panel.response.rect.width() * ppp
}

/// Draw a narrow vertical snap slider panel to the left of the note panel.
/// Uses a fixed area so it only occupies the editor region (not the top progress band).
/// Returns the panel width in physical pixels.
pub fn draw_snap_slider_panel(
    ctx: &egui::Context,
    editor: &mut FallingGroundEditor,
    note_panel_width_px: f32,
    top_offset_px: f32,
) -> f32 {
    use crate::ui::snap_slider::draw_snap_slider_vertical;

    let ppp = ctx.pixels_per_point().max(0.000_1);
    let panel_w: f32 = 56.0;
    let note_panel_width = note_panel_width_px / ppp;
    let top_offset = top_offset_px / ppp;
    let screen_rect = ctx.input(|i| i.screen_rect());

    let panel_x = screen_rect.right() - note_panel_width - panel_w;
    let panel_y = screen_rect.top() + top_offset;
    let panel_h = (screen_rect.bottom() - panel_y - 4.0).max(100.0);

    egui::Area::new(egui::Id::new("snap_slider_panel_area"))
        .fixed_pos(egui::pos2(panel_x, panel_y))
        .show(ctx, |ui| {
            ui.set_min_width(panel_w);
            ui.set_max_width(panel_w);
            if let Some(new_div) = draw_snap_slider_vertical(ui, editor.snap_division(), panel_h) {
                editor.set_snap_division(new_div);
            }
        });

    panel_w * ppp
}
