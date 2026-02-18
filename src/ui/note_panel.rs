use crate::editor::falling::{
    FallingGroundEditor, NotePropertyData, EventPropertyData, PlaceEventType, PlaceNoteType,
};
use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;

/// Persistent state for the property editing panel.
#[derive(Default)]
pub struct PropertyEditState {
    /// Editing note data (live copy for UI sliders).
    pub note_data: Option<NotePropertyData>,
    /// Editing event data.
    pub event_data: Option<EventPropertyData>,
    /// The id we started editing — used to detect selection changes.
    editing_note_id: Option<u64>,
    editing_event_id: Option<u64>,
}

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
    prop_state: &mut PropertyEditState,
) -> f32 {
    let ppp = ctx.pixels_per_point().max(0.000_1);
    let panel_width_points = NOTE_PANEL_BASE_WIDTH_POINTS;

    // Detect selection changes and manage edit lifecycle
    let sel_note = editor.selected_note_properties();
    let sel_event = editor.selected_event_properties();
    let sel_note_id = sel_note.as_ref().map(|n| n.id);
    let sel_event_id = sel_event.as_ref().map(|e| e.id);

    // If selection changed while editing, cancel the old edit
    if prop_state.editing_note_id.is_some() && prop_state.editing_note_id != sel_note_id {
        editor.cancel_note_edit();
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    }
    if prop_state.editing_event_id.is_some() && prop_state.editing_event_id != sel_event_id {
        editor.cancel_event_edit();
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    }

    // Auto-begin editing when a note/event is newly selected
    if sel_note_id.is_some() && prop_state.editing_note_id != sel_note_id {
        prop_state.editing_note_id = sel_note_id;
        prop_state.note_data = sel_note.clone();
        editor.begin_note_edit();
    }
    if sel_event_id.is_some() && prop_state.editing_event_id != sel_event_id {
        prop_state.editing_event_id = sel_event_id;
        prop_state.event_data = sel_event.clone();
        editor.begin_event_edit();
    }
    // Clear edit state when nothing is selected
    if sel_note_id.is_none() && prop_state.editing_note_id.is_some() {
        editor.cancel_note_edit();
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    }
    if sel_event_id.is_none() && prop_state.editing_event_id.is_some() {
        editor.cancel_event_edit();
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    }

    let show_note_props = prop_state.note_data.is_some();
    let show_event_props = prop_state.event_data.is_some();

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
            if show_note_props {
                draw_note_property_editor(ui, editor, prop_state);
            } else if show_event_props {
                draw_event_property_editor(ui, editor, prop_state);
            } else {
                draw_tool_selector(ui, i18n, editor);
            }
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

fn draw_tool_selector(ui: &mut egui::Ui, i18n: &I18n, editor: &mut FallingGroundEditor) {
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
        if response.clicked() { editor.set_place_note_type(Some(tool)); }
        if response.secondary_clicked() { editor.set_place_note_type(None); }
    }
    ui.separator();
    ui.label("Event");
    for tool in [PlaceEventType::Bpm, PlaceEventType::Track, PlaceEventType::Lane] {
        let response = draw_tool_row(ui, event_caption(tool), current_event == Some(tool));
        if response.clicked() { editor.set_place_event_type(Some(tool)); }
        if response.secondary_clicked() { editor.set_place_event_type(None); }
    }
    ui.separator();
    let mode_text = current_note.map(tool_caption)
        .or_else(|| current_event.map(event_caption))
        .unwrap_or("None");
    ui.label(format!("Current: {mode_text}"));
    if let Some(t) = editor.pending_hold_head_time_ms() {
        ui.label(format!("Hold head: {:.0}ms", t));
        ui.label("Click again to set tail");
    }
    if let Some(t) = editor.pending_skyarea_head_time_ms() {
        ui.label(format!("SkyArea head: {:.0}ms", t));
        ui.label("Click again to set tail");
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "Speed: {:.2} H/s   Snap: {}x",
                editor.scroll_speed(), editor.snap_division(),
            )).color(egui::Color32::from_rgb(160, 160, 170)).size(12.0),
        );
        ui.separator();
        let mut enabled = editor.track_speed_enabled();
        let label = egui::RichText::new(i18n.t(TextKey::NotePanelRenderSpeedEvents))
            .size(14.0).color(egui::Color32::from_rgb(220, 220, 230));
        if ui.add(egui::Checkbox::new(&mut enabled, label)).changed() {
            editor.set_track_speed_enabled(enabled);
        }
        ui.separator();
    });
}

fn draw_note_property_editor(
    ui: &mut egui::Ui,
    editor: &mut FallingGroundEditor,
    prop_state: &mut PropertyEditState,
) {
    let Some(data) = prop_state.note_data.as_mut() else { return };
    let mut changed = false;

    ui.label(egui::RichText::new(format!("Edit Note: {}", data.kind))
        .size(16.0).color(egui::Color32::from_rgb(255, 220, 120)));
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Time (ms):");
        let r = ui.add(egui::DragValue::new(&mut data.time_ms).speed(1.0).range(0.0..=600000.0));
        if r.changed() { changed = true; }
    });

    if data.kind == "Tap" || data.kind == "Hold" {
        ui.horizontal(|ui| {
            ui.label("Lane:");
            let r = ui.add(egui::DragValue::new(&mut data.lane).speed(0.1).range(0..=7));
            if r.changed() { changed = true; }
        });
    }

    if data.kind == "Hold" || data.kind == "SkyArea" {
        ui.horizontal(|ui| {
            ui.label("Duration (ms):");
            let r = ui.add(egui::DragValue::new(&mut data.duration_ms).speed(1.0).range(0.0..=600000.0));
            if r.changed() { changed = true; }
        });
    }

    ui.horizontal(|ui| {
        ui.label("Width:");
        let r = ui.add(egui::DragValue::new(&mut data.width).speed(0.01).range(0.05..=8.0));
        if r.changed() { changed = true; }
    });

    if data.kind == "Flick" {
        ui.horizontal(|ui| {
            ui.label("Direction:");
            let r = ui.add(egui::Checkbox::new(&mut data.flick_right, "Right"));
            if r.changed() { changed = true; }
        });
    }

    if changed {
        editor.preview_note_properties(data);
    }

    ui.add_space(12.0);
    let apply = ui.button("✔ Apply").clicked();
    let cancel = ui.button("✖ Cancel").clicked();

    if apply {
        editor.apply_note_properties(data);
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    } else if cancel {
        editor.cancel_note_edit();
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    }
}

fn draw_event_property_editor(
    ui: &mut egui::Ui,
    editor: &mut FallingGroundEditor,
    prop_state: &mut PropertyEditState,
) {
    let Some(data) = prop_state.event_data.as_mut() else { return };
    let mut changed = false;

    ui.label(egui::RichText::new(format!("Edit Event: {}", data.kind))
        .size(16.0).color(egui::Color32::from_rgb(120, 220, 255)));
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Time (ms):");
        let r = ui.add(egui::DragValue::new(&mut data.time_ms).speed(1.0).range(0.0..=600000.0));
        if r.changed() { changed = true; }
    });

    ui.horizontal(|ui| {
        ui.label("Label:");
        let r = ui.add(egui::TextEdit::singleline(&mut data.label).desired_width(160.0));
        if r.changed() { changed = true; }
    });

    if changed {
        editor.preview_event_properties(data);
    }

    ui.add_space(12.0);
    let apply = ui.button("✔ Apply").clicked();
    let cancel = ui.button("✖ Cancel").clicked();

    if apply {
        editor.apply_event_properties(data);
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    } else if cancel {
        editor.cancel_event_edit();
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    }
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
