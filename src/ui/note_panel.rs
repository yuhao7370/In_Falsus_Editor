use crate::editor::falling::{
    FallingGroundEditor, NotePropertyData, EventPropertyData, PlaceEventType, PlaceNoteType,
};
use crate::i18n::{I18n, TextKey};
use crate::ui::info_toast::InfoToastManager;
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
    pub note_data: Option<NotePropertyData>,
    pub event_data: Option<EventPropertyData>,
    editing_note_id: Option<u64>,
    editing_event_id: Option<u64>,
    pub last_time_edit: LastTimeEdit,
}

pub const NOTE_PANEL_BASE_WIDTH_POINTS: f32 = 280.0;

const EASE_LABELS: [&str; 3] = ["Linear", "SineOut", "SineIn"];
const BTN_MIN_SIZE: egui::Vec2 = egui::vec2(110.0, 30.0);
const LABEL_SIZE: f32 = 13.5;
const FIELD_LABEL_COLOR: egui::Color32 = egui::Color32::from_rgb(180, 180, 190);
const INPUT_FONT_SIZE: f32 = 14.0;
const INPUT_BOX_W: f32 = 190.0;
const TITLE_SIZE: f32 = 16.0;
const SUB_TITLE_SIZE: f32 = 14.0;
const PM_BTN_SIZE: egui::Vec2 = egui::vec2(24.0, 24.0);
const PM_BTN_FONT: f32 = 14.0;
const SWITCH_BTN_SIZE: egui::Vec2 = egui::vec2(24.0, 24.0);
const COMBO_WIDTH: f32 = 100.0;

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

fn prop_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(LABEL_SIZE).color(FIELD_LABEL_COLOR));
}

/// Fixed-size numeric text input with buffered editing.
/// Text is only parsed & applied when the input loses focus (Enter / click away).
/// While focused the user can type freely without the value being reformatted.
fn num_input_f32(ui: &mut egui::Ui, id_salt: &str, val: &mut f32, min: f32, max: f32, decimals: usize) -> bool {
    let box_h = PM_BTN_SIZE.y;
    let id = ui.id().with(id_salt);
    let formatted = format!("{:.*}", decimals, *val);
    let mut buf: String = ui.data(|d| d.get_temp::<String>(id).unwrap_or_else(|| formatted.clone()));
    let r = ui.add_sized(
        egui::vec2(INPUT_BOX_W, box_h),
        egui::TextEdit::singleline(&mut buf)
            .font(egui::FontId::proportional(INPUT_FONT_SIZE))
            .horizontal_align(egui::Align::RIGHT),
    );
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf));
        return false;
    }
    let mut changed = false;
    if r.lost_focus() {
        if let Ok(v) = buf.parse::<f32>() {
            let clamped = v.clamp(min, max);
            if (*val - clamped).abs() > f32::EPSILON { changed = true; }
            *val = clamped;
        }
    }
    ui.data_mut(|d| d.insert_temp(id, format!("{:.*}", decimals, *val)));
    changed
}

fn num_input_f64(ui: &mut egui::Ui, id_salt: &str, val: &mut f64, min: f64, max: f64, decimals: usize) -> bool {
    let box_h = PM_BTN_SIZE.y;
    let id = ui.id().with(id_salt);
    let formatted = format!("{:.*}", decimals, *val);
    let mut buf: String = ui.data(|d| d.get_temp::<String>(id).unwrap_or_else(|| formatted.clone()));
    let r = ui.add_sized(
        egui::vec2(INPUT_BOX_W, box_h),
        egui::TextEdit::singleline(&mut buf)
            .font(egui::FontId::proportional(INPUT_FONT_SIZE))
            .horizontal_align(egui::Align::RIGHT),
    );
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf));
        return false;
    }
    let mut changed = false;
    if r.lost_focus() {
        if let Ok(v) = buf.parse::<f64>() {
            let clamped = v.clamp(min, max);
            if (*val - clamped).abs() > f64::EPSILON { changed = true; }
            *val = clamped;
        }
    }
    ui.data_mut(|d| d.insert_temp(id, format!("{:.*}", decimals, *val)));
    changed
}

fn num_input_usize(ui: &mut egui::Ui, id_salt: &str, val: &mut usize, min: usize, max: usize) -> bool {
    let box_h = PM_BTN_SIZE.y;
    let id = ui.id().with(id_salt);
    let formatted = format!("{}", *val);
    let mut buf: String = ui.data(|d| d.get_temp::<String>(id).unwrap_or_else(|| formatted.clone()));
    let r = ui.add_sized(
        egui::vec2(INPUT_BOX_W, box_h),
        egui::TextEdit::singleline(&mut buf)
            .font(egui::FontId::proportional(INPUT_FONT_SIZE))
            .horizontal_align(egui::Align::RIGHT),
    );
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf));
        return false;
    }
    let mut changed = false;
    if r.lost_focus() {
        if let Ok(v) = buf.parse::<usize>() {
            let clamped = v.clamp(min, max);
            if *val != clamped { changed = true; }
            *val = clamped;
        }
    }
    ui.data_mut(|d| d.insert_temp(id, format!("{}", *val)));
    changed
}

fn num_input_i32(ui: &mut egui::Ui, id_salt: &str, val: &mut i32, min: i32, max: i32) -> bool {
    let box_h = PM_BTN_SIZE.y;
    let id = ui.id().with(id_salt);
    let formatted = format!("{}", *val);
    let mut buf: String = ui.data(|d| d.get_temp::<String>(id).unwrap_or_else(|| formatted.clone()));
    let r = ui.add_sized(
        egui::vec2(INPUT_BOX_W, box_h),
        egui::TextEdit::singleline(&mut buf)
            .font(egui::FontId::proportional(INPUT_FONT_SIZE))
            .horizontal_align(egui::Align::RIGHT),
    );
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, buf));
        return false;
    }
    let mut changed = false;
    if r.lost_focus() {
        if let Ok(v) = buf.parse::<i32>() {
            let clamped = v.clamp(min, max);
            if *val != clamped { changed = true; }
            *val = clamped;
        }
    }
    ui.data_mut(|d| d.insert_temp(id, format!("{}", *val)));
    changed
}

fn pm_btn(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add_sized(PM_BTN_SIZE, egui::Button::new(
        egui::RichText::new(label).size(PM_BTN_FONT)
    )).clicked()
}

fn switch_btn(ui: &mut egui::Ui) -> bool {
    ui.add_sized(SWITCH_BTN_SIZE, egui::Button::new(
        egui::RichText::new("\u{27F3}").size(PM_BTN_FONT)
    )).clicked()
}

/// 与 ground_note_effective_width 保持一致的最大宽度计算。
/// Lane 0 和 5 锁定为 1，Lane 1-4 的 max = 5 - lane。
fn panel_max_width(lane: usize) -> f32 {
    if lane == 0 || lane >= 5 {
        1.0
    } else {
        (5 - lane) as f32
    }
}

/// Flick 宽度在当前中心点下的最大可用值（raw 坐标）。
/// 约束：x - width/2 >= 0 且 x + width/2 <= x_split。
fn flick_width_max_for_center(x: f64, x_split: f64) -> f64 {
    if x_split <= 0.0 {
        return 0.0;
    }
    let left_room = x.max(0.0);
    let right_room = (x_split - x).max(0.0);
    (left_room.min(right_room) * 2.0).max(0.0)
}

const EASE_COMBO_FONT: f32 = 16.0;
const EASE_COMBO_WIDTH: f32 = 120.0;
const EASE_SWITCH_SIZE: egui::Vec2 = egui::vec2(28.0, 28.0);

/// Ease combo with switch button. Returns true if changed.
fn ease_combo_with_switch(ui: &mut egui::Ui, id_salt: &str, value: &mut i32) -> bool {
    let idx = (*value as usize).min(2);
    let mut changed = false;
    let old_interact_h = ui.spacing().interact_size.y;
    ui.spacing_mut().interact_size.y = 28.0;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(egui::RichText::new(EASE_LABELS[idx]).size(EASE_COMBO_FONT))
        .width(EASE_COMBO_WIDTH)
        .show_ui(ui, |ui| {
            for (i, label) in EASE_LABELS.iter().enumerate() {
                let r = ui.selectable_value(value, i as i32,
                    egui::RichText::new(*label).size(EASE_COMBO_FONT));
                if r.changed() { changed = true; }
            }
        });
    ui.spacing_mut().interact_size.y = old_interact_h;
    if ui.add_sized(EASE_SWITCH_SIZE, egui::Button::new(
        egui::RichText::new("\u{27F3}").size(EASE_COMBO_FONT)
    )).clicked() {
        *value = (*value + 1) % 3;
        changed = true;
    }
    changed
}

// -- Panel entry point --

pub fn draw_note_selector_panel(
    ctx: &egui::Context,
    i18n: &I18n,
    editor: &mut FallingGroundEditor,
    prop_state: &mut PropertyEditState,
    toasts: &mut InfoToastManager,
) -> f32 {
    let ppp = ctx.pixels_per_point().max(0.000_1);
    let panel_width_points = NOTE_PANEL_BASE_WIDTH_POINTS;

    let multi_count = editor.selected_note_count();
    let sel_note = if multi_count <= 1 { editor.selected_note_properties() } else { None };
    let sel_event = editor.selected_event_properties();
    let sel_note_id = sel_note.as_ref().map(|n| n.id);
    let sel_event_id = sel_event.as_ref().map(|e| e.id);

    if prop_state.editing_note_id.is_some() && prop_state.editing_note_id != sel_note_id {
        // Auto-apply if there are unsaved changes, otherwise just clean up
        if let Some(data) = prop_state.note_data.take() {
            if editor.has_note_edit_changed() {
                editor.apply_note_properties(&data);
            } else {
                editor.restore_note_edit_backup();
            }
        } else {
            editor.restore_note_edit_backup();
        }
        prop_state.editing_note_id = None;
    }
    if prop_state.editing_event_id.is_some() && prop_state.editing_event_id != sel_event_id {
        if let Some(data) = prop_state.event_data.take() {
            if editor.has_event_edit_changed() {
                editor.apply_event_properties(&data);
            } else {
                editor.restore_event_edit_backup();
            }
        } else {
            editor.restore_event_edit_backup();
        }
        prop_state.editing_event_id = None;
    }
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
    if sel_note_id.is_none() && prop_state.editing_note_id.is_some() {
        // Auto-apply on deselection (e.g. right-click) if changed
        if let Some(data) = prop_state.note_data.take() {
            if editor.has_note_edit_changed() {
                editor.apply_note_properties(&data);
            } else {
                editor.cancel_note_edit();
            }
        } else {
            editor.cancel_note_edit();
        }
        prop_state.editing_note_id = None;
    }
    if sel_event_id.is_none() && prop_state.editing_event_id.is_some() {
        if let Some(data) = prop_state.event_data.take() {
            if editor.has_event_edit_changed() {
                editor.apply_event_properties(&data);
            } else {
                editor.cancel_event_edit();
            }
        } else {
            editor.cancel_event_edit();
        }
        prop_state.editing_event_id = None;
    }

    // 拖拽期间实时刷新属性面板数据
    if editor.is_dragging_note() {
        if let Some(fresh) = &sel_note {
            if prop_state.editing_note_id == Some(fresh.id) {
                prop_state.note_data = Some(fresh.clone());
            }
        }
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                if show_note_props {
                    draw_note_property_editor(ui, i18n, editor, prop_state, toasts);
                } else if show_event_props {
                    draw_event_property_editor(ui, editor, prop_state);
                } else {
                    draw_tool_selector(ui, i18n, editor);
                }
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

fn draw_tool_selector(ui: &mut egui::Ui, i18n: &I18n, editor: &mut FallingGroundEditor) {
    let multi_count = editor.selected_note_count();
    if multi_count > 1 {
        ui.label(egui::RichText::new(format!("Selected {} note(s)", multi_count))
            .size(TITLE_SIZE).color(egui::Color32::from_rgb(180, 220, 255)));
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Press Delete to remove")
            .size(12.0).color(egui::Color32::from_rgb(140, 140, 150)));
        ui.separator();
    }
    ui.label("Note");
    let current_note = editor.place_note_type();
    let current_event = editor.place_event_type();
    for tool in [PlaceNoteType::Tap, PlaceNoteType::Hold, PlaceNoteType::Flick, PlaceNoteType::SkyArea] {
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
    i18n: &I18n,
    editor: &mut FallingGroundEditor,
    prop_state: &mut PropertyEditState,
    toasts: &mut InfoToastManager,
) {
    let Some(data) = prop_state.note_data.as_mut() else { return };
    let mut changed = false;
    let mut time_edited = false;
    let mut beat_edited = false;
    let mut dur_ms_edited = false;
    let mut dur_beat_edited = false;
    let beat_step = 1.0 / editor.snap_division().max(1) as f32;

    ui.label(egui::RichText::new(format!("Edit Note: {}", data.kind))
        .size(TITLE_SIZE).color(egui::Color32::from_rgb(255, 220, 120)));
    ui.add_space(6.0);

    // Time (ms)
    prop_label(ui, "Time (ms)");
    ui.horizontal(|ui| {
        if pm_btn(ui, "-") { data.time_ms = (data.time_ms - 1.0).max(0.0).round(); time_edited = true; changed = true; }
        if num_input_f32(ui, "note_time_ms", &mut data.time_ms, 0.0, 600000.0, 0) { data.time_ms = data.time_ms.round(); time_edited = true; changed = true; }
        if pm_btn(ui, "+") { data.time_ms = (data.time_ms + 1.0).min(600000.0).round(); time_edited = true; changed = true; }
    });
    // Beat
    prop_label(ui, "Beat");
    ui.horizontal(|ui| {
        if pm_btn(ui, "-") { data.beat = (data.beat - beat_step).max(0.0); beat_edited = true; changed = true; }
        if num_input_f32(ui, "note_beat", &mut data.beat, 0.0, f32::MAX, 3) { beat_edited = true; changed = true; }
        if pm_btn(ui, "+") { data.beat += beat_step; beat_edited = true; changed = true; }
    });

    if time_edited {
        data.beat = editor.time_to_beat(data.time_ms);
        prop_state.last_time_edit = LastTimeEdit::Time;
    } else if beat_edited {
        data.time_ms = editor.beat_to_time(data.beat);
        prop_state.last_time_edit = LastTimeEdit::Beat;
    }

    ui.add_space(4.0);

    // Lane (ground notes) — width-priority: reject lane change if width won't fit
    // 与 ground_note_effective_width 保持一致：Lane 0/5 锁定 width=1，Lane 1-4 宽音符范围 [1, 5-eff_w]
    if data.kind == "Tap" || data.kind == "Hold" {
        prop_label(ui, "Lane");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") {
                if data.lane > 0 {
                    let candidate = data.lane - 1;
                    let new_max_w = panel_max_width(candidate);
                    if data.width > new_max_w {
                        toasts.push_warn(
                            i18n.t(TextKey::ToastLaneWidthReject)
                                .replace("{lane}", &candidate.to_string())
                                .replace("{width}", &(data.width as usize).to_string())
                                .replace("{max}", &(new_max_w as usize).to_string())
                        );
                    } else {
                        data.lane = candidate;
                        changed = true;
                    }
                }
            }
            let old_lane = data.lane;
            if num_input_usize(ui, "note_lane", &mut data.lane, 0, 5) {
                let new_max_w = panel_max_width(data.lane);
                if data.width > new_max_w {
                    toasts.push_warn(
                        i18n.t(TextKey::ToastLaneWidthReject)
                            .replace("{lane}", &data.lane.to_string())
                            .replace("{width}", &(data.width as usize).to_string())
                            .replace("{max}", &(new_max_w as usize).to_string())
                    );
                    data.lane = old_lane;
                } else {
                    changed = true;
                }
            }
            if pm_btn(ui, "+") {
                if data.lane < 5 {
                    let candidate = data.lane + 1;
                    let new_max_w = panel_max_width(candidate);
                    if data.width > new_max_w {
                        toasts.push_warn(
                            i18n.t(TextKey::ToastLaneWidthReject)
                                .replace("{lane}", &candidate.to_string())
                                .replace("{width}", &(data.width as usize).to_string())
                                .replace("{max}", &(new_max_w as usize).to_string())
                        );
                    } else {
                        data.lane = candidate;
                        changed = true;
                    }
                }
            }
        });
    }

    // Duration (Hold / SkyArea)
    if data.kind == "Hold" || data.kind == "SkyArea" {
        prop_label(ui, "Dur (ms)");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.duration_ms = (data.duration_ms - 1.0).max(0.0).round(); dur_ms_edited = true; changed = true; }
            if num_input_f32(ui, "note_dur_ms", &mut data.duration_ms, 0.0, 600000.0, 0) { data.duration_ms = data.duration_ms.round(); dur_ms_edited = true; changed = true; }
            if pm_btn(ui, "+") { data.duration_ms = (data.duration_ms + 1.0).round(); dur_ms_edited = true; changed = true; }
        });
        prop_label(ui, "Dur (beat)");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.duration_beat = (data.duration_beat - beat_step).max(0.0); dur_beat_edited = true; changed = true; }
            if num_input_f32(ui, "note_dur_beat", &mut data.duration_beat, 0.0, f32::MAX, 3) { dur_beat_edited = true; changed = true; }
            if pm_btn(ui, "+") { data.duration_beat += beat_step; dur_beat_edited = true; changed = true; }
        });
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

    // Width (Tap / Hold) — 与 ground_note_effective_width 一致
    if data.kind == "Tap" || data.kind == "Hold" {
        let max_w: f32 = panel_max_width(data.lane);
        data.width = data.width.round().clamp(1.0, max_w);
        let is_locked = data.lane == 0 || data.lane >= 5;
        prop_label(ui, "Width");
        if is_locked {
            ui.label(egui::RichText::new("1 (locked)").size(INPUT_FONT_SIZE).color(egui::Color32::from_rgb(140, 140, 150)));
        } else {
            ui.horizontal(|ui| {
                if pm_btn(ui, "-") { data.width = (data.width - 1.0).max(1.0); changed = true; }
                if num_input_f32(ui, "note_width", &mut data.width, 1.0, max_w, 0) { data.width = data.width.round().clamp(1.0, max_w); changed = true; }
                if pm_btn(ui, "+") {
                    let candidate = data.width + 1.0;
                    if candidate > max_w {
                        toasts.push_warn(
                            i18n.t(TextKey::ToastLaneWidthMax)
                                .replace("{lane}", &data.lane.to_string())
                                .replace("{max}", &(max_w as usize).to_string())
                                .replace("{current}", &(data.width as usize).to_string())
                        );
                    } else {
                        data.width = candidate;
                        changed = true;
                    }
                }
            });
        }
    }

    // Flick
    if data.kind == "Flick" {
        let xsplit_editable = editor.xsplit_editable();
        if !xsplit_editable {
            // Locked mode: show XSplit as read-only
            prop_label(ui, "XSplit");
            ui.label(egui::RichText::new(format!("{} (locked)", data.x_split as i64))
                .size(INPUT_FONT_SIZE).color(egui::Color32::from_rgb(140, 140, 150)));
        }
        prop_label(ui, "X");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.x = (data.x - 1.0).max(0.0); changed = true; }
            if num_input_f64(ui, "flick_x", &mut data.x, 0.0, data.x_split, 0) { changed = true; }
            if pm_btn(ui, "+") { data.x = (data.x + 1.0).min(data.x_split); changed = true; }
        });
        prop_label(ui, "Width");
        ui.horizontal(|ui| {
            let mut w = data.width as f64;
            let max_w = flick_width_max_for_center(data.x, data.x_split);
            if pm_btn(ui, "-") { w = (w - 1.0).max(0.0); data.width = w as f32; changed = true; }
            if num_input_f64(ui, "flick_w", &mut w, 0.0, max_w, 0) { data.width = w as f32; changed = true; }
            if pm_btn(ui, "+") {
                let candidate = w + 1.0;
                if candidate <= max_w {
                    w = candidate;
                    data.width = w as f32;
                    changed = true;
                }
            }
        });
        if xsplit_editable {
            // Editable mode: per-note XSplit
            prop_label(ui, "XSplit");
            ui.horizontal(|ui| {
                let old_xs = data.x_split;
                if pm_btn(ui, "-") && old_xs > 1.0 {
                    let new_xs = (old_xs - 1.0).max(1.0);
                    let ratio = new_xs / old_xs;
                    data.x_split = new_xs;
                    data.x *= ratio;
                    data.width = (data.width as f64 * ratio) as f32;
                    changed = true;
                }
                if num_input_f64(ui, "flick_xs", &mut data.x_split, 1.0, 1024.0, 0) && old_xs > 0.0 {
                    let ratio = data.x_split / old_xs;
                    data.x *= ratio;
                    data.width = (data.width as f64 * ratio) as f32;
                    changed = true;
                }
                if pm_btn(ui, "+") {
                    let new_xs = (old_xs + 1.0).min(1024.0);
                    let ratio = new_xs / old_xs;
                    data.x_split = new_xs;
                    data.x *= ratio;
                    data.width = (data.width as f64 * ratio) as f32;
                    changed = true;
                }
            });
        }
        prop_label(ui, "Direction");
        ui.horizontal(|ui| {
            let label = if data.flick_right { "Right \u{2192}" } else { "Left \u{2190}" };
            egui::ComboBox::from_id_salt("flick_dir")
                .selected_text(egui::RichText::new(label).size(INPUT_FONT_SIZE))
                .width(COMBO_WIDTH)
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut data.flick_right, true,
                        egui::RichText::new("Right \u{2192}").size(INPUT_FONT_SIZE)).changed() { changed = true; }
                    if ui.selectable_value(&mut data.flick_right, false,
                        egui::RichText::new("Left \u{2190}").size(INPUT_FONT_SIZE)).changed() { changed = true; }
                });
            if switch_btn(ui) { data.flick_right = !data.flick_right; changed = true; }
        });
    }

    // SkyArea
    if data.kind == "SkyArea" {
        let xsplit_editable = editor.xsplit_editable();
        if !xsplit_editable {
            // Locked mode: show XSplit as read-only (shared for start/end)
            prop_label(ui, "XSplit");
            ui.label(egui::RichText::new(format!("{} / {} (locked)", data.start_x_split as i64, data.end_x_split as i64))
                .size(INPUT_FONT_SIZE).color(egui::Color32::from_rgb(140, 140, 150)));
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Start").size(SUB_TITLE_SIZE).color(egui::Color32::from_rgb(200, 200, 220)));
        prop_label(ui, "X");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.start_x = (data.start_x - 1.0).max(0.0); changed = true; }
            if num_input_f64(ui, "sky_sx", &mut data.start_x, 0.0, data.start_x_split, 0) { changed = true; }
            if pm_btn(ui, "+") { data.start_x = (data.start_x + 1.0).min(data.start_x_split); changed = true; }
        });
        prop_label(ui, "Width");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.start_width = (data.start_width - 1.0).max(0.0); changed = true; }
            if num_input_f64(ui, "sky_sw", &mut data.start_width, 0.0, data.start_x_split, 0) { changed = true; }
            if pm_btn(ui, "+") { data.start_width = (data.start_width + 1.0).min(data.start_x_split); changed = true; }
        });
        if xsplit_editable {
            prop_label(ui, "XSplit");
            ui.horizontal(|ui| {
                let old_xs = data.start_x_split;
                if pm_btn(ui, "-") && old_xs > 1.0 {
                    let new_xs = (old_xs - 1.0).max(1.0);
                    let ratio = new_xs / old_xs;
                    data.start_x_split = new_xs;
                    data.start_x *= ratio;
                    data.start_width *= ratio;
                    changed = true;
                }
                if num_input_f64(ui, "sky_sxs", &mut data.start_x_split, 1.0, 1024.0, 0) && old_xs > 0.0 {
                    let ratio = data.start_x_split / old_xs;
                    data.start_x *= ratio;
                    data.start_width *= ratio;
                    changed = true;
                }
                if pm_btn(ui, "+") {
                    let new_xs = (old_xs + 1.0).min(1024.0);
                    let ratio = new_xs / old_xs;
                    data.start_x_split = new_xs;
                    data.start_x *= ratio;
                    data.start_width *= ratio;
                    changed = true;
                }
            });
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new("End").size(SUB_TITLE_SIZE).color(egui::Color32::from_rgb(200, 200, 220)));
        prop_label(ui, "X");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.end_x = (data.end_x - 1.0).max(0.0); changed = true; }
            if num_input_f64(ui, "sky_ex", &mut data.end_x, 0.0, data.end_x_split, 0) { changed = true; }
            if pm_btn(ui, "+") { data.end_x = (data.end_x + 1.0).min(data.end_x_split); changed = true; }
        });
        prop_label(ui, "Width");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.end_width = (data.end_width - 1.0).max(0.0); changed = true; }
            if num_input_f64(ui, "sky_ew", &mut data.end_width, 0.0, data.end_x_split, 0) { changed = true; }
            if pm_btn(ui, "+") { data.end_width = (data.end_width + 1.0).min(data.end_x_split); changed = true; }
        });
        if xsplit_editable {
            prop_label(ui, "XSplit");
            ui.horizontal(|ui| {
                let old_xs = data.end_x_split;
                if pm_btn(ui, "-") && old_xs > 1.0 {
                    let new_xs = (old_xs - 1.0).max(1.0);
                    let ratio = new_xs / old_xs;
                    data.end_x_split = new_xs;
                    data.end_x *= ratio;
                    data.end_width *= ratio;
                    changed = true;
                }
                if num_input_f64(ui, "sky_exs", &mut data.end_x_split, 1.0, 1024.0, 0) && old_xs > 0.0 {
                    let ratio = data.end_x_split / old_xs;
                    data.end_x *= ratio;
                    data.end_width *= ratio;
                    changed = true;
                }
                if pm_btn(ui, "+") {
                    let new_xs = (old_xs + 1.0).min(1024.0);
                    let ratio = new_xs / old_xs;
                    data.end_x_split = new_xs;
                    data.end_x *= ratio;
                    data.end_width *= ratio;
                    changed = true;
                }
            });
        }
        ui.add_space(4.0);
        prop_label(ui, "L Ease");
        ui.horizontal(|ui| { if ease_combo_with_switch(ui, "left_ease", &mut data.left_ease) { changed = true; } });
        prop_label(ui, "R Ease");
        ui.horizontal(|ui| { if ease_combo_with_switch(ui, "right_ease", &mut data.right_ease) { changed = true; } });
        ui.add_space(4.0);
        prop_label(ui, "Group ID");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.group_id = (data.group_id - 1).max(-1); changed = true; }
            if num_input_i32(ui, "sky_group_id", &mut data.group_id, -1, i32::MAX) { changed = true; }
            if pm_btn(ui, "+") { data.group_id = data.group_id.saturating_add(1); changed = true; }
        });
    }

    if changed {
        editor.preview_note_properties(data);
    }

    ui.add_space(12.0);
    let mut do_apply = false;
    let mut do_cancel = false;
    ui.horizontal(|ui| {
        if ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
            egui::RichText::new("\u{2714} Apply").size(SUB_TITLE_SIZE))).clicked() {
            do_apply = true;
        }
        if ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
            egui::RichText::new("\u{2716} Cancel").size(SUB_TITLE_SIZE))).clicked() {
            do_cancel = true;
        }
    });
    if do_apply {
        editor.apply_note_properties(data);
        prop_state.note_data = None;
        prop_state.editing_note_id = None;
    } else if do_cancel {
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
    let beat_step = 1.0 / editor.snap_division().max(1) as f32;
    let is_chart_header = data.kind == "Bpm" && data.is_chart_header;

    let title_color = match data.kind.as_str() {
        "Bpm" => egui::Color32::from_rgb(124, 226, 255),
        "Track" => egui::Color32::from_rgb(150, 240, 170),
        "Lane" => egui::Color32::from_rgb(232, 198, 124),
        _ => egui::Color32::from_rgb(120, 220, 255),
    };
    ui.label(egui::RichText::new(format!("Edit Event: {}", data.kind))
        .size(TITLE_SIZE).color(title_color));
    ui.add_space(6.0);

    if is_chart_header {
        prop_label(ui, "Time (ms)");
        ui.label(
            egui::RichText::new("0 (chart header fixed)")
                .size(INPUT_FONT_SIZE)
                .color(egui::Color32::from_rgb(140, 140, 150)),
        );
        prop_label(ui, "Beat");
        ui.label(
            egui::RichText::new("0 (derived from time)")
                .size(INPUT_FONT_SIZE)
                .color(egui::Color32::from_rgb(140, 140, 150)),
        );
        data.time_ms = 0.0;
        data.beat = 0.0;
    } else {
        // Time (ms)
        prop_label(ui, "Time (ms)");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.time_ms = (data.time_ms - 1.0).max(0.0).round(); time_edited = true; changed = true; }
            if num_input_f32(ui, "evt_time_ms", &mut data.time_ms, 0.0, 600000.0, 0) { data.time_ms = data.time_ms.round(); time_edited = true; changed = true; }
            if pm_btn(ui, "+") { data.time_ms = (data.time_ms + 1.0).min(600000.0).round(); time_edited = true; changed = true; }
        });
        // Beat
        prop_label(ui, "Beat");
        ui.horizontal(|ui| {
            if pm_btn(ui, "-") { data.beat = (data.beat - beat_step).max(0.0); beat_edited = true; changed = true; }
            if num_input_f32(ui, "evt_beat", &mut data.beat, 0.0, f32::MAX, 3) { beat_edited = true; changed = true; }
            if pm_btn(ui, "+") { data.beat += beat_step; beat_edited = true; changed = true; }
        });

        if time_edited {
            data.beat = editor.time_to_beat(data.time_ms);
            prop_state.last_time_edit = LastTimeEdit::Time;
        } else if beat_edited {
            data.time_ms = editor.beat_to_time(data.beat);
            prop_state.last_time_edit = LastTimeEdit::Beat;
        }
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    match data.kind.as_str() {
        "Bpm" => {
            prop_label(ui, "BPM");
            ui.horizontal(|ui| { if num_input_f32(ui, "evt_bpm", &mut data.bpm, 0.001, 9999.0, 2) { changed = true; } });
            prop_label(ui, "BPL");
            ui.horizontal(|ui| { if num_input_f32(ui, "evt_bpl", &mut data.beats_per_measure, 1.0, 64.0, 2) { changed = true; } });
        }
        "Track" => {
            prop_label(ui, "Speed");
            ui.horizontal(|ui| { if num_input_f32(ui, "evt_speed", &mut data.speed, -100.0, 100.0, 2) { changed = true; } });
        }
        "Lane" => {
            prop_label(ui, "Lane");
            ui.horizontal(|ui| {
                if pm_btn(ui, "-") { data.lane = (data.lane - 1).max(0); changed = true; }
                if num_input_i32(ui, "evt_lane", &mut data.lane, 0, 5) { changed = true; }
                if pm_btn(ui, "+") { data.lane = (data.lane + 1).min(5); changed = true; }
            });
            prop_label(ui, "Enable");
            ui.horizontal(|ui| {
                if ui.checkbox(&mut data.enable, "").changed() { changed = true; }
                if switch_btn(ui) { data.enable = !data.enable; changed = true; }
            });
        }
        _ => {}
    }

    if changed {
        editor.preview_event_properties(data);
    }

    ui.add_space(12.0);
    let mut do_apply = false;
    let mut do_cancel = false;
    ui.horizontal(|ui| {
        if ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
            egui::RichText::new("\u{2714} Apply").size(SUB_TITLE_SIZE))).clicked() {
            do_apply = true;
        }
        if ui.add_sized(BTN_MIN_SIZE, egui::Button::new(
            egui::RichText::new("\u{2716} Cancel").size(SUB_TITLE_SIZE))).clicked() {
            do_cancel = true;
        }
    });
    if do_apply {
        editor.apply_event_properties(data);
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    } else if do_cancel {
        editor.cancel_event_edit();
        prop_state.event_data = None;
        prop_state.editing_event_id = None;
    }
}

/// Draw a narrow vertical snap slider panel to the left of the note panel.
pub fn draw_snap_slider_panel(
    ctx: &egui::Context,
    editor: &mut FallingGroundEditor,
    note_panel_width_px: f32,
    top_offset_px: f32,
    interactive: bool,
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
        .order(egui::Order::Background)
        .fixed_pos(egui::pos2(panel_x, panel_y))
        .show(ctx, |ui| {
            ui.set_min_width(panel_w);
            ui.set_max_width(panel_w);
            if let Some(new_div) =
                draw_snap_slider_vertical(ui, editor.snap_division(), panel_h, interactive)
            {
                editor.set_snap_division(new_div);
            }
        });

    panel_w * ppp
}
