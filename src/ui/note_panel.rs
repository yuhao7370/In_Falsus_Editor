use crate::editor::falling::{
    FallingGroundEditor, NotePropertyData, EventPropertyData, PlaceEventType, PlaceNoteType,
};
use crate::i18n::{I18n, TextKey};
use egui_macroquad::egui;

/// Which field was last edited for time/beat sync.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum LastTimeEdit {
    #[default]
    Time,
    Beat,
}

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
    /// Track which field was last edited for time↔beat sync.
    pub last_time_edit: LastTimeEdit,
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

    // If selection changed while editing, restore the old backup without deselecting
    // (deselecting would clear overlap_cycle and break double-click cycling)
    if prop_state.editing_note_id.is_some() && prop_state.editing_note_id != sel_note_id {
        editor.restore_note_edit_backup();
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    }
    if prop_state.editing_event_id.is_some() && prop_state.editing_event_id != sel_event_id {
        editor.restore_event_edit_backup();
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

const EASE_LABELS: [&str; 3] = ["Linear", "SineOut", "SineIn"];
const BTN_MIN_SIZE: egui::Vec2 = egui::vec2(110.0, 30.0);
const LABEL_SIZE: f32 = 13.5;
const FIELD_LABEL_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 180, 190);

fn prop_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(LABEL_SIZE).color(FIELD_LABEL_COLOR));
}

fn ease_combo(ui: &mut egui::Ui, id_salt: &str, value: &mut i32) -> bool {
    let idx = (*value as usize).min(2);
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(EASE_LABELS[idx])
        .width(100.0)
        .show_ui(ui, |ui| {
            for (i, label) in EASE_LABELS.iter().enumerate() {
                if ui.selectable_value(&mut (*value), i as i32, *label).changed() {
                    changed = true;
                }
            }
        });
    changed
}

fn draw_note_property_editor(
    ui: &mut egui::Ui,
    editor: &mut FallingGroundEditor,
    prop_state: &mut PropertyEditState,
) {
    let Some(data) = prop_state.note_data.as_mut() else { return };
    let mut changed = false;
    let mut time_edited = false;
    let mut beat_edited = false;
    let mut dur_ms_edited = false;
    let mut dur_beat_edited = false;

    ui.label(egui::RichText::new(format!("Edit Note: {}", data.kind))
        .size(16.0).color(egui::Color32::from_rgb(255, 220, 120)));
    ui.add_space(6.0);

    // Time (ms)
    ui.horizontal(|ui| {
        prop_label(ui, "Time (ms)");
        let r = ui.add(egui::DragValue::new(&mut data.time_ms)
            .speed(1.0).range(0.0..=600000.0).min_decimals(1).max_decimals(1)
            .update_while_editing(false));
        if r.changed() { time_edited = true; changed = true; }
    });
    // Beat
    ui.horizontal(|ui| {
        prop_label(ui, "Beat");
        let r = ui.add(egui::DragValue::new(&mut data.beat)
            .speed(0.01).range(0.0..=f32::MAX).min_decimals(3).max_decimals(3)
            .update_while_editing(false));
        if r.changed() { beat_edited = true; changed = true; }
    });

    // Sync time↔beat
    if time_edited {
        data.beat = editor.time_to_beat(data.time_ms);
        prop_state.last_time_edit = LastTimeEdit::Time;
    } else if beat_edited {
        data.time_ms = editor.beat_to_time(data.beat);
        prop_state.last_time_edit = LastTimeEdit::Beat;
    }

    ui.add_space(4.0);

    // Lane (ground notes only, 0-5)
    if data.kind == "Tap" || data.kind == "Hold" {
        ui.horizontal(|ui| {
            prop_label(ui, "Lane");
            let r = ui.add(egui::DragValue::new(&mut data.lane).speed(0.1).range(0..=5));
            if r.changed() { changed = true; }
        });
    }

    // Duration ms + beat (Hold / SkyArea)
    if data.kind == "Hold" || data.kind == "SkyArea" {
        ui.horizontal(|ui| {
            prop_label(ui, "Dur (ms)");
            let r = ui.add(egui::DragValue::new(&mut data.duration_ms)
                .speed(1.0).range(0.0..=600000.0).min_decimals(1).max_decimals(1));
            if r.changed() { dur_ms_edited = true; changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "Dur (beat)");
            let r = ui.add(egui::DragValue::new(&mut data.duration_beat)
                .speed(0.01).range(0.0..=f32::MAX).min_decimals(3).max_decimals(3));
            if r.changed() { dur_beat_edited = true; changed = true; }
        });
        // Sync duration ms↔beat
        if dur_ms_edited {
            let end_beat = editor.time_to_beat(data.time_ms + data.duration_ms);
            let start_beat = editor.time_to_beat(data.time_ms);
            data.duration_beat = end_beat - start_beat;
        } else if dur_beat_edited {
            let start_beat = editor.time_to_beat(data.time_ms);
            let end_time = editor.beat_to_time(start_beat + data.duration_beat);
            data.duration_ms = (end_time - data.time_ms).max(0.0);
        }
    }

    // Width (Tap / Hold only — integer lane count)
    if data.kind == "Tap" || data.kind == "Hold" {
        let max_w: f32 = if data.lane == 0 || data.lane >= 5 {
            1.0
        } else {
            (5 - data.lane) as f32
        };
        // Clamp width to valid range when lane changes
        data.width = data.width.round().clamp(1.0, max_w);
        let is_locked = max_w <= 1.0;
        ui.horizontal(|ui| {
            prop_label(ui, "Width");
            if is_locked {
                ui.label(egui::RichText::new("1 (locked)").size(13.0).color(egui::Color32::from_rgb(140, 140, 150)));
            } else {
                let r = ui.add(egui::DragValue::new(&mut data.width)
                    .speed(0.1).range(1.0..=max_w).min_decimals(0).max_decimals(0));
                if r.changed() {
                    data.width = data.width.round().clamp(1.0, max_w);
                    changed = true;
                }
            }
        });
    }

    // Flick: X, Width (raw), XSplit (locked), direction
    if data.kind == "Flick" {
        ui.horizontal(|ui| {
            prop_label(ui, "X");
            let r = ui.add(egui::DragValue::new(&mut data.x)
                .speed(1.0).range(0.0..=data.x_split).min_decimals(0).max_decimals(0));
            if r.changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "Width");
            let mut w = data.width as f64;
            let r = ui.add(egui::DragValue::new(&mut w)
                .speed(1.0).range(0.0..=data.x_split).min_decimals(0).max_decimals(0));
            if r.changed() { data.width = w as f32; changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "XSplit");
            let old_xs = data.x_split;
            let r = ui.add(egui::DragValue::new(&mut data.x_split)
                .speed(1.0).range(1.0..=1024.0).min_decimals(0).max_decimals(0));
            if r.changed() && old_xs > 0.0 {
                let ratio = data.x_split / old_xs;
                data.x = (data.x as f64 * ratio) as f64;
                data.width = (data.width as f64 * ratio) as f32;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "Direction");
            let label = if data.flick_right { "Right →" } else { "Left ←" };
            egui::ComboBox::from_id_salt("flick_dir")
                .selected_text(label)
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut data.flick_right, true, "Right →").changed() { changed = true; }
                    if ui.selectable_value(&mut data.flick_right, false, "Left ←").changed() { changed = true; }
                });
        });
    }

    // SkyArea: raw X/Width/XSplit + ease
    if data.kind == "SkyArea" {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Start").size(14.0).color(egui::Color32::from_rgb(200, 200, 220)));
        ui.horizontal(|ui| {
            prop_label(ui, "X");
            let r = ui.add(egui::DragValue::new(&mut data.start_x)
                .speed(1.0).range(0.0..=data.start_x_split).min_decimals(0).max_decimals(0));
            if r.changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "Width");
            let r = ui.add(egui::DragValue::new(&mut data.start_width)
                .speed(1.0).range(0.0..=data.start_x_split).min_decimals(0).max_decimals(0));
            if r.changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "XSplit");
            let old_xs = data.start_x_split;
            let r = ui.add(egui::DragValue::new(&mut data.start_x_split)
                .speed(1.0).range(1.0..=1024.0).min_decimals(0).max_decimals(0));
            if r.changed() && old_xs > 0.0 {
                let ratio = data.start_x_split / old_xs;
                data.start_x *= ratio;
                data.start_width *= ratio;
                changed = true;
            }
        });
        ui.add_space(4.0);
        ui.label(egui::RichText::new("End").size(14.0).color(egui::Color32::from_rgb(200, 200, 220)));
        ui.horizontal(|ui| {
            prop_label(ui, "X");
            let r = ui.add(egui::DragValue::new(&mut data.end_x)
                .speed(1.0).range(0.0..=data.end_x_split).min_decimals(0).max_decimals(0));
            if r.changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "Width");
            let r = ui.add(egui::DragValue::new(&mut data.end_width)
                .speed(1.0).range(0.0..=data.end_x_split).min_decimals(0).max_decimals(0));
            if r.changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "XSplit");
            let old_xs = data.end_x_split;
            let r = ui.add(egui::DragValue::new(&mut data.end_x_split)
                .speed(1.0).range(1.0..=1024.0).min_decimals(0).max_decimals(0));
            if r.changed() && old_xs > 0.0 {
                let ratio = data.end_x_split / old_xs;
                data.end_x *= ratio;
                data.end_width *= ratio;
                changed = true;
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            prop_label(ui, "L Ease");
            if ease_combo(ui, "left_ease", &mut data.left_ease) { changed = true; }
        });
        ui.horizontal(|ui| {
            prop_label(ui, "R Ease");
            if ease_combo(ui, "right_ease", &mut data.right_ease) { changed = true; }
        });
    }

    if changed {
        editor.preview_note_properties(data);
    }

    ui.add_space(12.0);
    let apply = ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
        egui::RichText::new("✔ Apply").size(15.0))).clicked();
    ui.add_space(4.0);
    let cancel = ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
        egui::RichText::new("✖ Cancel").size(15.0))).clicked();

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
    let mut time_edited = false;
    let mut beat_edited = false;

    let title_color = match data.kind.as_str() {
        "Bpm" => egui::Color32::from_rgb(124, 226, 255),
        "Track" => egui::Color32::from_rgb(150, 240, 170),
        "Lane" => egui::Color32::from_rgb(232, 198, 124),
        _ => egui::Color32::from_rgb(120, 220, 255),
    };
    ui.label(egui::RichText::new(format!("Edit Event: {}", data.kind))
        .size(16.0).color(title_color));
    ui.add_space(6.0);

    // Time (ms)
    ui.horizontal(|ui| {
        prop_label(ui, "Time (ms)");
        let r = ui.add(egui::DragValue::new(&mut data.time_ms)
            .speed(1.0).range(0.0..=600000.0).min_decimals(1).max_decimals(1)
            .update_while_editing(false));
        if r.changed() { time_edited = true; changed = true; }
    });
    // Beat
    ui.horizontal(|ui| {
        prop_label(ui, "Beat");
        let r = ui.add(egui::DragValue::new(&mut data.beat)
            .speed(0.01).range(0.0..=f32::MAX).min_decimals(3).max_decimals(3)
            .update_while_editing(false));
        if r.changed() { beat_edited = true; changed = true; }
    });

    // Sync time↔beat
    if time_edited {
        data.beat = editor.time_to_beat(data.time_ms);
        prop_state.last_time_edit = LastTimeEdit::Time;
    } else if beat_edited {
        data.time_ms = editor.beat_to_time(data.beat);
        prop_state.last_time_edit = LastTimeEdit::Beat;
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    // Type-specific params
    match data.kind.as_str() {
        "Bpm" => {
            ui.horizontal(|ui| {
                prop_label(ui, "BPM");
                let r = ui.add(egui::DragValue::new(&mut data.bpm)
                    .speed(0.1).range(0.001..=9999.0).min_decimals(2).max_decimals(2));
                if r.changed() { changed = true; }
            });
            ui.horizontal(|ui| {
                prop_label(ui, "BPL");
                let r = ui.add(egui::DragValue::new(&mut data.beats_per_measure)
                    .speed(0.1).range(1.0..=64.0).min_decimals(1).max_decimals(2));
                if r.changed() { changed = true; }
            });
        }
        "Track" => {
            ui.horizontal(|ui| {
                prop_label(ui, "Speed");
                let r = ui.add(egui::DragValue::new(&mut data.speed)
                    .speed(0.01).range(-100.0..=100.0).min_decimals(2).max_decimals(2));
                if r.changed() { changed = true; }
            });
        }
        "Lane" => {
            ui.horizontal(|ui| {
                prop_label(ui, "Lane");
                let r = ui.add(egui::DragValue::new(&mut data.lane)
                    .speed(0.1).range(0..=5));
                if r.changed() { changed = true; }
            });
            ui.horizontal(|ui| {
                prop_label(ui, "Enable");
                if ui.checkbox(&mut data.enable, "").changed() { changed = true; }
            });
        }
        _ => {}
    }

    if changed {
        editor.preview_event_properties(data);
    }

    ui.add_space(12.0);
    let apply = ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
        egui::RichText::new("✔ Apply").size(15.0))).clicked();
    ui.add_space(4.0);
    let cancel = ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
        egui::RichText::new("✖ Cancel").size(15.0))).clicked();

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
