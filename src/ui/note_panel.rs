use crate::editor::falling::{
    FallingGroundEditor, PlaceNoteType, RenderScope, SNAP_DIVISION_OPTIONS,
};
use egui_macroquad::egui;

pub const NOTE_PANEL_BASE_WIDTH_POINTS: f32 = 280.0;

fn tool_caption(tool: PlaceNoteType) -> &'static str {
    match tool {
        PlaceNoteType::Tap => "Tap (Ground)",
        PlaceNoteType::Hold => "Hold (Ground, 2 Clicks)",
        PlaceNoteType::Flick => "Flick (Air)",
        PlaceNoteType::SkyArea => "SkyArea (Air, 2 Clicks)",
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

pub fn draw_note_selector_panel(ctx: &egui::Context, editor: &mut FallingGroundEditor) -> f32 {
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
            ui.heading("Note Tool");
            ui.label("LMB: Select  RMB: Clear");
            ui.separator();
            ui.label("Render");
            let scope = editor.render_scope();
            for s in [RenderScope::Both, RenderScope::Split] {
                if draw_tool_row(ui, s.label(), scope == s).clicked() {
                    editor.set_render_scope(s);
                }
            }

            ui.separator();
            ui.label("Barline Snap");
            ui.horizontal_wrapped(|ui| {
                let current = editor.snap_division();
                for division in SNAP_DIVISION_OPTIONS {
                    let selected = current == division;
                    let button = egui::Button::new(format!("{division}x"))
                        .min_size(egui::vec2(48.0, 26.0))
                        .fill(if selected {
                            egui::Color32::from_rgba_unmultiplied(106, 168, 255, 76)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 8)
                        });
                    if ui.add(button).clicked() {
                        editor.set_snap_division(division);
                    }
                }
            });

            ui.separator();
            ui.label("Flow Speed");
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("-").min_size(egui::vec2(28.0, 26.0)))
                    .clicked()
                {
                    editor.nudge_scroll_speed(-editor.scroll_speed_step());
                }

                let mut speed = editor.scroll_speed();
                let slider = egui::Slider::new(
                    &mut speed,
                    editor.min_scroll_speed()..=editor.max_scroll_speed(),
                )
                .text("H/s")
                .clamping(egui::SliderClamping::Always);
                if ui.add(slider).changed() {
                    editor.set_scroll_speed(speed);
                }

                if ui
                    .add(egui::Button::new("+").min_size(egui::vec2(28.0, 26.0)))
                    .clicked()
                {
                    editor.nudge_scroll_speed(editor.scroll_speed_step());
                }
            });
            ui.label(format!("Current: {:.2}H/s", editor.scroll_speed()));

            ui.separator();
            ui.label("Place");

            let current = editor.place_note_type();
            for tool in [
                PlaceNoteType::Tap,
                PlaceNoteType::Hold,
                PlaceNoteType::Flick,
                PlaceNoteType::SkyArea,
            ] {
                let response = draw_tool_row(ui, tool_caption(tool), current == Some(tool));
                if response.clicked() {
                    editor.set_place_note_type(Some(tool));
                }
                if response.secondary_clicked() {
                    editor.set_place_note_type(None);
                }
            }

            ui.separator();
            let mode_text = current.map(tool_caption).unwrap_or("None");
            ui.label(format!("Current: {mode_text}"));
            if let Some(head_time_ms) = editor.pending_hold_head_time_ms() {
                ui.label(format!("Hold head: {:.0}ms", head_time_ms));
                ui.label("Click again to set tail");
            }
            if let Some(head_time_ms) = editor.pending_skyarea_head_time_ms() {
                ui.label(format!("SkyArea head: {:.0}ms", head_time_ms));
                ui.label("Click again to set tail");
            }
        });

    if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
        let cancel_on_panel = ctx.input(|i| i.pointer.secondary_clicked())
            && panel.response.rect.contains(pointer_pos);
        if cancel_on_panel {
            editor.set_place_note_type(None);
        }
    }

    panel.response.rect.width() * ppp
}
