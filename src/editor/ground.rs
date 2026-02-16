use crate::chart::{Chart, ChartEvent};
use egui_macroquad::egui;
use sasa::AudioClip;

const DEFAULT_CHART_PATH: &str = "grievouslady2.spc";
const DEFAULT_HOLD_MS: f64 = 500.0;
const WAVEFORM_HEIGHT: f32 = 110.0;
const LANE_COUNT: usize = 6;
const LANE_HEIGHT: f32 = 38.0;
const LANE_GAP: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub enum GroundEditorAction {
    SeekTo(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteKind {
    Tap,
    Hold,
}

#[derive(Debug, Clone)]
struct GroundNote {
    id: u64,
    lane: usize,
    time_ms: f64,
    duration_ms: f64,
}

impl GroundNote {
    fn kind(&self) -> NoteKind {
        if self.duration_ms > 1.0 {
            NoteKind::Hold
        } else {
            NoteKind::Tap
        }
    }

    fn end_ms(&self) -> f64 {
        self.time_ms + self.duration_ms.max(0.0)
    }
}

#[derive(Debug, Clone)]
struct DragState {
    note_id: u64,
    pointer_origin: egui::Pos2,
    note_origin_time_ms: f64,
    note_origin_lane: usize,
}

#[derive(Debug, Clone)]
struct WaveformData {
    peaks: Vec<f32>,
    duration_sec: f32,
}

impl WaveformData {
    fn from_audio_file(path: &str, peak_count: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|err| format!("failed to read audio: {err}"))?;
        let clip = AudioClip::new(bytes).map_err(|err| format!("failed to decode audio: {err}"))?;
        let frames = clip.frames();
        let duration_sec = clip.length();
        let peak_count = peak_count.max(256);
        let mut peaks = vec![0.0_f32; peak_count];

        if !frames.is_empty() {
            let frame_count = frames.len();
            for (index, frame) in frames.iter().enumerate() {
                let bucket = index * peak_count / frame_count;
                let amp = frame.avg().abs().min(1.0);
                if amp > peaks[bucket] {
                    peaks[bucket] = amp;
                }
            }
        }

        Ok(Self {
            peaks,
            duration_sec,
        })
    }
}

pub struct GroundEditor {
    notes: Vec<GroundNote>,
    next_note_id: u64,
    selected_note_id: Option<u64>,
    drag_state: Option<DragState>,
    chart_path: String,
    status_message: String,
    base_bpm: f64,
    pixels_per_second: f32,
    snap_enabled: bool,
    snap_division: u32,
    waveform: Option<WaveformData>,
    waveform_error: Option<String>,
    last_audio_path: Option<String>,
}

impl GroundEditor {
    pub fn new() -> Self {
        let mut editor = Self {
            notes: Vec::new(),
            next_note_id: 1,
            selected_note_id: None,
            drag_state: None,
            chart_path: DEFAULT_CHART_PATH.to_owned(),
            status_message: String::new(),
            base_bpm: 120.0,
            pixels_per_second: 120.0,
            snap_enabled: true,
            snap_division: 4,
            waveform: None,
            waveform_error: None,
            last_audio_path: None,
        };
        editor.load_chart(DEFAULT_CHART_PATH);
        editor
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        playhead_sec: f32,
        audio_duration_sec: f32,
        audio_path: Option<&str>,
    ) -> Vec<GroundEditorAction> {
        self.sync_waveform(audio_path);
        let mut actions = Vec::new();

        ui.horizontal(|ui| {
            ui.strong("6K Ground Editor");
            ui.separator();
            ui.label(format!("Chart: {}", self.chart_path));
            ui.separator();
            ui.label(format!("Notes: {}", self.notes.len()));
            ui.separator();
            ui.label(format!("BPM: {:.2}", self.base_bpm));
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut self.pixels_per_second, 40.0..=320.0)
                    .text("Zoom(px/s)"),
            );
            ui.checkbox(&mut self.snap_enabled, "Snap");

            egui::ComboBox::from_id_salt("snap_division")
                .selected_text(format!("1/{0}", self.snap_division))
                .show_ui(ui, |ui| {
                    for division in [1_u32, 2, 4, 8, 16] {
                        ui.selectable_value(
                            &mut self.snap_division,
                            division,
                            format!("1/{division}"),
                        );
                    }
                });

            ui.separator();
            ui.label("LMB empty: add Tap");
            ui.label("Shift+LMB: add Hold");
            ui.label("LMB drag: move note");
            ui.label("RMB: delete note");
        });

        if !self.status_message.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(176, 214, 255), &self.status_message);
        }
        if let Some(error) = &self.waveform_error {
            ui.colored_label(egui::Color32::from_rgb(255, 128, 128), error);
        }

        let timeline_duration_sec = self
            .max_note_end_sec()
            .max(audio_duration_sec.max(0.0))
            .max(8.0);
        let timeline_width = (timeline_duration_sec * self.pixels_per_second).max(ui.available_width());
        let lanes_total_height = (LANE_HEIGHT + LANE_GAP) * LANE_COUNT as f32;
        let timeline_height = WAVEFORM_HEIGHT + 18.0 + lanes_total_height;

        egui::ScrollArea::both().id_salt("ground_editor_scroll").show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(timeline_width, timeline_height),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let wave_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.right(), rect.top() + WAVEFORM_HEIGHT),
            );
            let lane_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), wave_rect.bottom() + 18.0),
                rect.max,
            );

            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(8, 8, 8));
            self.draw_second_grid(&painter, rect, timeline_duration_sec);
            self.draw_waveform(&painter, wave_rect);
            self.draw_lanes(&painter, lane_rect);
            self.draw_notes(&painter, lane_rect);
            self.draw_playhead(&painter, rect, playhead_sec, timeline_duration_sec);

            self.handle_pointer(
                ui,
                &response,
                rect,
                wave_rect,
                lane_rect,
                timeline_duration_sec,
                &mut actions,
            );
        });

        actions
    }

    fn handle_pointer(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        timeline_rect: egui::Rect,
        wave_rect: egui::Rect,
        lane_rect: egui::Rect,
        timeline_duration_sec: f32,
        actions: &mut Vec<GroundEditorAction>,
    ) {
        let pointer = response.interact_pointer_pos();
        let modifiers = ui.input(|input| input.modifiers);

        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = pointer {
                if wave_rect.contains(pointer_pos) {
                    let seek = self.pointer_to_time(pointer_pos.x, timeline_rect, timeline_duration_sec);
                    actions.push(GroundEditorAction::SeekTo(seek));
                } else if lane_rect.contains(pointer_pos) {
                    if let Some(hit_note) = self.hit_test(pointer_pos, lane_rect, timeline_rect) {
                        self.selected_note_id = Some(hit_note);
                        if let Some(note) = self.notes.iter().find(|note| note.id == hit_note) {
                            self.drag_state = Some(DragState {
                                note_id: note.id,
                                pointer_origin: pointer_pos,
                                note_origin_time_ms: note.time_ms,
                                note_origin_lane: note.lane,
                            });
                        }
                    } else {
                        self.create_note(pointer_pos, lane_rect, timeline_rect, timeline_duration_sec, modifiers.shift);
                    }
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(pointer_pos) = pointer {
                if lane_rect.contains(pointer_pos) {
                    if let Some(hit_note) = self.hit_test(pointer_pos, lane_rect, timeline_rect) {
                        self.notes.retain(|note| note.id != hit_note);
                        if self.selected_note_id == Some(hit_note) {
                            self.selected_note_id = None;
                        }
                        self.drag_state = None;
                        self.status_message = "note deleted".to_owned();
                    }
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            if let (Some(pointer_pos), Some(drag)) = (pointer, self.drag_state.clone()) {
                let delta_x = pointer_pos.x - drag.pointer_origin.x;
                let delta_time_ms = (delta_x / self.pixels_per_second * 1000.0) as f64;
                let new_time = self.snap_time((drag.note_origin_time_ms + delta_time_ms).max(0.0));
                let lane_delta = ((pointer_pos.y - drag.pointer_origin.y) / (LANE_HEIGHT + LANE_GAP))
                    .round() as i32;
                let new_lane = (drag.note_origin_lane as i32 + lane_delta)
                    .clamp(0, (LANE_COUNT as i32) - 1) as usize;

                if let Some(note) = self.notes.iter_mut().find(|note| note.id == drag.note_id) {
                    note.time_ms = new_time;
                    note.lane = new_lane;
                    self.status_message =
                        format!("dragging: lane={} time={:.0}ms", note.lane, note.time_ms);
                }
            } else if let Some(pointer_pos) = pointer {
                if wave_rect.contains(pointer_pos) {
                    let seek = self.pointer_to_time(pointer_pos.x, timeline_rect, timeline_duration_sec);
                    actions.push(GroundEditorAction::SeekTo(seek));
                }
            }
        }

        if response.drag_stopped() {
            self.drag_state = None;
            self.sort_notes();
        }
    }

    fn draw_second_grid(&self, painter: &egui::Painter, rect: egui::Rect, duration_sec: f32) {
        let seconds = duration_sec.ceil() as i32;
        for second in 0..=seconds {
            let x = rect.left() + second as f32 * self.pixels_per_second;
            let strong = second % 5 == 0;
            let color = if strong {
                egui::Color32::from_rgb(50, 50, 50)
            } else {
                egui::Color32::from_rgb(28, 28, 28)
            };
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, color),
            );
        }
    }

    fn draw_waveform(&self, painter: &egui::Painter, rect: egui::Rect) {
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(12, 14, 18));
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 52)),
        );

        let Some(waveform) = &self.waveform else {
            return;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 {
            return;
        }

        let peak_count = waveform.peaks.len();
        for (index, peak) in waveform.peaks.iter().enumerate() {
            let t = index as f32 / (peak_count - 1) as f32;
            let x = rect.left() + t * waveform.duration_sec * self.pixels_per_second;
            if x < rect.left() || x > rect.right() {
                continue;
            }

            let amp = (peak * (rect.height() * 0.45)).max(1.0);
            painter.line_segment(
                [
                    egui::pos2(x, rect.center().y - amp),
                    egui::pos2(x, rect.center().y + amp),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(86, 171, 255)),
            );
        }
    }

    fn draw_lanes(&self, painter: &egui::Painter, lane_rect: egui::Rect) {
        for lane in 0..LANE_COUNT {
            let top = lane_rect.top() + lane as f32 * (LANE_HEIGHT + LANE_GAP);
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(lane_rect.left(), top),
                egui::pos2(lane_rect.right(), top + LANE_HEIGHT),
            );
            let bg = if lane % 2 == 0 {
                egui::Color32::from_rgb(15, 15, 15)
            } else {
                egui::Color32::from_rgb(18, 18, 18)
            };
            painter.rect_filled(row_rect, 0.0, bg);

            painter.text(
                egui::pos2(row_rect.left() + 6.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("L{lane}"),
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(160, 160, 160),
            );
        }
    }

    fn draw_notes(&self, painter: &egui::Painter, lane_rect: egui::Rect) {
        for note in &self.notes {
            let Some(rect) = self.note_rect(note, lane_rect) else {
                continue;
            };

            let is_selected = self.selected_note_id == Some(note.id);
            let base_color = match note.kind() {
                NoteKind::Tap => egui::Color32::from_rgb(76, 185, 255),
                NoteKind::Hold => egui::Color32::from_rgb(120, 220, 120),
            };
            let color = if is_selected {
                egui::Color32::from_rgb(255, 206, 86)
            } else {
                base_color
            };

            painter.rect_filled(rect, 3.0, color);
            if note.kind() == NoteKind::Hold {
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), rect.center().y),
                        egui::pos2(rect.right(), rect.center().y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(36, 80, 36)),
                );
            }
        }
    }

    fn draw_playhead(&self, painter: &egui::Painter, rect: egui::Rect, playhead_sec: f32, duration_sec: f32) {
        if duration_sec <= 0.0 {
            return;
        }
        let x = rect.left() + playhead_sec.max(0.0) * self.pixels_per_second;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 106, 106)),
        );
    }

    fn create_note(
        &mut self,
        pointer: egui::Pos2,
        lane_rect: egui::Rect,
        timeline_rect: egui::Rect,
        timeline_duration_sec: f32,
        make_hold: bool,
    ) {
        let lane = self.pointer_to_lane(pointer.y, lane_rect);
        let time_sec = self.pointer_to_time(pointer.x, timeline_rect, timeline_duration_sec);
        let time_ms = self.snap_time(time_sec as f64 * 1000.0);
        let duration_ms = if make_hold { DEFAULT_HOLD_MS } else { 0.0 };

        let note = GroundNote {
            id: self.next_note_id,
            lane,
            time_ms,
            duration_ms,
        };
        self.next_note_id += 1;
        self.selected_note_id = Some(note.id);
        self.notes.push(note);
        self.sort_notes();
        self.status_message = if make_hold {
            format!("new Hold: lane={lane}, time={time_ms:.0}ms")
        } else {
            format!("new Tap: lane={lane}, time={time_ms:.0}ms")
        };
    }

    fn pointer_to_time(&self, x: f32, timeline_rect: egui::Rect, duration_sec: f32) -> f32 {
        let time_sec = (x - timeline_rect.left()) / self.pixels_per_second;
        time_sec.clamp(0.0, duration_sec.max(0.0))
    }

    fn pointer_to_lane(&self, y: f32, lane_rect: egui::Rect) -> usize {
        let lane = ((y - lane_rect.top()) / (LANE_HEIGHT + LANE_GAP)).floor() as i32;
        lane.clamp(0, (LANE_COUNT as i32) - 1) as usize
    }

    fn hit_test(
        &self,
        pointer: egui::Pos2,
        lane_rect: egui::Rect,
        timeline_rect: egui::Rect,
    ) -> Option<u64> {
        for note in self.notes.iter().rev() {
            let Some(rect) = self.note_rect(note, lane_rect) else {
                continue;
            };
            if rect.contains(pointer) {
                return Some(note.id);
            }

            if note.kind() == NoteKind::Tap {
                let center = egui::pos2(
                    timeline_rect.left() + (note.time_ms as f32 / 1000.0) * self.pixels_per_second,
                    rect.center().y,
                );
                if center.distance(pointer) <= 8.0 {
                    return Some(note.id);
                }
            }
        }
        None
    }

    fn note_rect(&self, note: &GroundNote, lane_rect: egui::Rect) -> Option<egui::Rect> {
        if note.lane >= LANE_COUNT {
            return None;
        }
        let lane_top = lane_rect.top() + note.lane as f32 * (LANE_HEIGHT + LANE_GAP);
        let y1 = lane_top + 4.0;
        let y2 = lane_top + LANE_HEIGHT - 4.0;
        let x1 = lane_rect.left() + (note.time_ms as f32 / 1000.0) * self.pixels_per_second;
        let x2 = if note.kind() == NoteKind::Hold {
            lane_rect.left() + (note.end_ms() as f32 / 1000.0) * self.pixels_per_second
        } else {
            x1 + 8.0
        };
        Some(egui::Rect::from_min_max(
            egui::pos2(x1, y1),
            egui::pos2((x2).max(x1 + 6.0), y2),
        ))
    }

    fn snap_time(&self, time_ms: f64) -> f64 {
        if !self.snap_enabled {
            return time_ms.max(0.0);
        }
        let bpm = self.base_bpm.max(1e-6);
        let division = self.snap_division.max(1) as f64;
        let step = (60000.0 / bpm) / division;
        if step <= 0.0 {
            return time_ms.max(0.0);
        }
        (time_ms / step).round() * step
    }

    fn sort_notes(&mut self) {
        self.notes.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.lane.cmp(&b.lane))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn max_note_end_sec(&self) -> f32 {
        self.notes
            .iter()
            .map(|note| note.end_ms() as f32 / 1000.0)
            .fold(0.0, f32::max)
    }

    fn sync_waveform(&mut self, audio_path: Option<&str>) {
        let Some(path) = audio_path else {
            return;
        };
        let changed = self
            .last_audio_path
            .as_ref()
            .map(|last| last != path)
            .unwrap_or(true);
        if !changed {
            return;
        }

        self.last_audio_path = Some(path.to_owned());
        match WaveformData::from_audio_file(path, 4096) {
            Ok(data) => {
                self.waveform = Some(data);
                self.waveform_error = None;
                self.status_message = format!("waveform loaded: {path}");
            }
            Err(err) => {
                self.waveform = None;
                self.waveform_error = Some(err);
            }
        }
    }

    fn load_chart(&mut self, path: &str) {
        self.chart_path = path.to_owned();
        match Chart::from_file(path) {
            Ok(chart) => {
                self.base_bpm = chart.chart_info().map(|(bpm, _)| bpm).unwrap_or(120.0);
                self.notes = Self::extract_ground_notes(&chart);
                self.next_note_id = self
                    .notes
                    .iter()
                    .map(|note| note.id)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                self.status_message =
                    format!("chart loaded: {} ground notes", self.notes.len());
            }
            Err(err) => {
                self.notes.clear();
                self.next_note_id = 1;
                self.status_message = format!("failed to read chart: {err}");
            }
        }
    }

    fn extract_ground_notes(chart: &Chart) -> Vec<GroundNote> {
        let mut notes = Vec::new();
        let mut next_note_id = 1_u64;
        for event in &chart.events {
            match event {
                ChartEvent::Tap { time, lane, .. } => {
                    if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                        notes.push(GroundNote {
                            id: next_note_id,
                            lane: *lane as usize,
                            time_ms: *time,
                            duration_ms: 0.0,
                        });
                        next_note_id += 1;
                    }
                }
                ChartEvent::Hold {
                    time,
                    lane,
                    duration,
                    ..
                } => {
                    if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                        notes.push(GroundNote {
                            id: next_note_id,
                            lane: *lane as usize,
                            time_ms: *time,
                            duration_ms: (*duration).max(1.0),
                        });
                        next_note_id += 1;
                    }
                }
                _ => {}
            }
        }
        notes.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));
        notes
    }
}
