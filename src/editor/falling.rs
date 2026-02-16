use crate::chart::{Chart, ChartEvent, Ease, FlickType};
use macroquad::prelude::*;
use sasa::AudioClip;

const LANE_COUNT: usize = 6;
const DEFAULT_CHART_PATH: &str = "songs/alamode/alamode3.spc";
const DEFAULT_SKYAREA_MS: f32 = 800.0;
const DEFAULT_AIR_WIDTH_NORM: f32 = 0.5;
const DEFAULT_SCROLL_SPEED: f32 = 1.25;
const MIN_SCROLL_SPEED: f32 = 0.2;
const MAX_SCROLL_SPEED: f32 = 4.0;
const SCROLL_SPEED_STEP: f32 = 0.1;
const AIR_SKYAREA_HEAD_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.84);
const AIR_SKYAREA_BODY_COLOR: Color = Color::new(0.72, 0.60, 0.98, 0.42);
const PORTRAIT_SCREEN_RATIO: f32 = 10.0 / 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceNoteType {
    Tap,
    Hold,
    Flick,
    SkyArea,
}

impl PlaceNoteType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tap => "Tap",
            Self::Hold => "Hold",
            Self::Flick => "Flick",
            Self::SkyArea => "SkyArea",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderScope {
    Split,
    Both,
}

impl RenderScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Split => "Split",
            Self::Both => "Both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundNoteKind {
    Tap,
    Hold,
    Flick,
    SkyArea,
}

fn is_ground_kind(kind: GroundNoteKind) -> bool {
    matches!(kind, GroundNoteKind::Tap | GroundNoteKind::Hold)
}

fn is_air_kind(kind: GroundNoteKind) -> bool {
    matches!(kind, GroundNoteKind::Flick | GroundNoteKind::SkyArea)
}

fn is_ground_tool(tool: PlaceNoteType) -> bool {
    matches!(tool, PlaceNoteType::Tap | PlaceNoteType::Hold)
}

fn is_air_tool(tool: PlaceNoteType) -> bool {
    matches!(tool, PlaceNoteType::Flick | PlaceNoteType::SkyArea)
}

#[derive(Debug, Clone, Copy)]
pub enum FallingEditorAction {
    SeekTo(f32),
}

#[derive(Debug, Clone)]
struct GroundNote {
    id: u64,
    kind: GroundNoteKind,
    lane: usize,
    time_ms: f32,
    duration_ms: f32,
    width: f32,
    flick_right: bool,
    skyarea_shape: Option<SkyAreaShape>,
}

impl GroundNote {
    fn has_tail(&self) -> bool {
        matches!(self.kind, GroundNoteKind::Hold | GroundNoteKind::SkyArea) && self.duration_ms > 1.0
    }

    fn end_time_ms(&self) -> f32 {
        self.time_ms + self.duration_ms.max(0.0)
    }

}

#[derive(Debug, Clone, Copy)]
struct SkyAreaShape {
    start_left_norm: f32,
    start_right_norm: f32,
    end_left_norm: f32,
    end_right_norm: f32,
    left_ease: Ease,
    right_ease: Ease,
}

#[derive(Debug, Clone, Copy)]
struct BpmPoint {
    time_ms: f32,
    bpm: f32,
    beats_per_measure: f32,
    start_beat: f32,
}

#[derive(Debug, Clone, Copy)]
struct BarLine {
    time_ms: f32,
    is_major: bool,
}

#[derive(Debug, Clone)]
struct BpmTimeline {
    points: Vec<BpmPoint>,
}

impl BpmTimeline {
    fn from_chart(chart: &Chart) -> Self {
        let mut base_bpm = 120.0_f32;
        let mut base_beats = 4.0_f32;

        for event in &chart.events {
            if let ChartEvent::Chart { bpm, beats } = event {
                base_bpm = *bpm as f32;
                base_beats = (*beats as f32).max(1.0);
                break;
            }
        }

        let mut points = vec![BpmPoint {
            time_ms: 0.0,
            bpm: base_bpm,
            beats_per_measure: base_beats,
            start_beat: 0.0,
        }];

        let mut bpm_events = Vec::new();
        for event in &chart.events {
            if let ChartEvent::Bpm {
                time,
                bpm,
                beats,
                ..
            } = event
            {
                bpm_events.push((*time as f32, *bpm as f32, (*beats as f32).max(1.0)));
            }
        }

        bpm_events.sort_by(|a, b| a.0.total_cmp(&b.0));

        for (time_ms, bpm, beats_per_measure) in bpm_events {
            if time_ms <= 0.0 {
                points[0].bpm = bpm;
                points[0].beats_per_measure = beats_per_measure;
                continue;
            }

            if let Some(last) = points.last_mut() {
                if (last.time_ms - time_ms).abs() < 0.000_1 {
                    last.bpm = bpm;
                    last.beats_per_measure = beats_per_measure;
                    continue;
                }
            }

            points.push(BpmPoint {
                time_ms,
                bpm,
                beats_per_measure,
                start_beat: 0.0,
            });
        }

        points.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));

        for idx in 1..points.len() {
            let previous = points[idx - 1];
            let dt_ms = (points[idx].time_ms - previous.time_ms).max(0.0);
            let bpm = previous.bpm.abs().max(0.001);
            points[idx].start_beat = previous.start_beat + dt_ms / 60_000.0 * bpm;
        }

        Self { points }
    }

    fn point_at_time(&self, time_ms: f32) -> BpmPoint {
        let mut active = self.points[0];
        for point in &self.points {
            if point.time_ms <= time_ms {
                active = *point;
            } else {
                break;
            }
        }
        active
    }

    fn time_to_beat(&self, time_ms: f32) -> f32 {
        let point = self.point_at_time(time_ms);
        let bpm = point.bpm.abs().max(0.001);
        point.start_beat + (time_ms - point.time_ms) / 60_000.0 * bpm
    }

    fn beat_to_time(&self, beat: f32) -> f32 {
        for idx in 0..self.points.len() {
            let point = self.points[idx];
            let next_beat = if idx + 1 < self.points.len() {
                self.points[idx + 1].start_beat
            } else {
                f32::INFINITY
            };
            if beat < next_beat {
                let bpm = point.bpm.abs().max(0.001);
                return point.time_ms + (beat - point.start_beat) * 60_000.0 / bpm;
            }
        }

        let point = *self.points.last().unwrap_or(&BpmPoint {
            time_ms: 0.0,
            bpm: 120.0,
            beats_per_measure: 4.0,
            start_beat: 0.0,
        });
        let bpm = point.bpm.abs().max(0.001);
        point.time_ms + (beat - point.start_beat) * 60_000.0 / bpm
    }

    fn snap_time_ms(&self, time_ms: f32, division: u32) -> f32 {
        if division == 0 {
            return time_ms.max(0.0);
        }
        let beat = self.time_to_beat(time_ms);
        let snapped = (beat * division as f32).round() / division as f32;
        self.beat_to_time(snapped).max(0.0)
    }

    fn visible_barlines(&self, current_ms: f32, ahead_ms: f32, behind_ms: f32) -> Vec<BarLine> {
        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;
        let mut output = Vec::new();

        for idx in 0..self.points.len() {
            let point = self.points[idx];
            let segment_start = point.time_ms;
            let segment_end = if idx + 1 < self.points.len() {
                self.points[idx + 1].time_ms
            } else {
                end_ms + 60_000.0
            };

            let visible_start = segment_start.max(start_ms);
            let visible_end = segment_end.min(end_ms);
            if visible_end < visible_start {
                continue;
            }

            let bpm = point.bpm.abs().max(0.001);
            let beat_ms = 60_000.0 / bpm;
            let beats_per_measure = point.beats_per_measure.max(1.0);

            let n_start = ((visible_start - segment_start) / beat_ms).floor() as i32 - 2;
            let n_end = ((visible_end - segment_start) / beat_ms).ceil() as i32 + 2;

            for n in n_start..=n_end {
                if n < 0 {
                    continue;
                }
                let line_time_ms = segment_start + n as f32 * beat_ms;
                if line_time_ms < visible_start - 0.001 || line_time_ms > visible_end + 0.001 {
                    continue;
                }

                let beat = point.start_beat + n as f32;
                let measure_phase = beat / beats_per_measure;
                let is_major = (measure_phase - measure_phase.round()).abs() < 0.001;

                output.push(BarLine {
                    time_ms: line_time_ms,
                    is_major,
                });
            }
        }

        output.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));
        output
    }
}

#[derive(Debug, Clone)]
struct Waveform {
    path: String,
    peaks: Vec<f32>,
    duration_sec: f32,
}

impl Waveform {
    fn from_audio_file(path: &str, bucket_count: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|err| format!("failed to read audio: {err}"))?;
        let clip = AudioClip::new(bytes).map_err(|err| format!("failed to decode audio: {err}"))?;
        let frames = clip.frames();
        let bucket_count = bucket_count.max(256);
        let mut peaks = vec![0.0_f32; bucket_count];

        if !frames.is_empty() {
            let frame_count = frames.len();
            for (idx, frame) in frames.iter().enumerate() {
                let bucket = idx * bucket_count / frame_count;
                let amp = frame.avg().abs().min(1.0);
                if amp > peaks[bucket] {
                    peaks[bucket] = amp;
                }
            }
        }

        Ok(Self {
            path: path.to_owned(),
            peaks,
            duration_sec: clip.length(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct DragState {
    note_id: u64,
    time_offset_ms: f32,
}

#[derive(Debug, Clone, Copy)]
struct PendingHoldPlacement {
    lane: usize,
    start_time_ms: f32,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    time_ms: f32,
    label: String,
    color: Color,
}

pub struct FallingGroundEditor {
    chart_path: String,
    notes: Vec<GroundNote>,
    next_note_id: u64,
    selected_note_id: Option<u64>,
    drag_state: Option<DragState>,
    timeline: BpmTimeline,
    timeline_events: Vec<TimelineEvent>,
    snap_enabled: bool,
    snap_division: u32,
    scroll_speed: f32,
    render_scope: RenderScope,
    place_note_type: Option<PlaceNoteType>,
    pending_hold: Option<PendingHoldPlacement>,
    waveform: Option<Waveform>,
    waveform_error: Option<String>,
    waveform_seek_active: bool,
    waveform_seek_sec: f32,
    status: String,
}

impl FallingGroundEditor {
    pub fn new() -> Self {
        Self::from_chart_path(DEFAULT_CHART_PATH)
    }

    pub fn from_chart_path(path: &str) -> Self {
        let (notes, timeline, timeline_events, status) = match Chart::from_file(path) {
            Ok(chart) => {
                (
                    extract_ground_notes(&chart),
                    BpmTimeline::from_chart(&chart),
                    extract_timeline_events(&chart),
                    format!("chart loaded: {path}"),
                )
            }
            Err(err) => {
                (
                    Vec::new(),
                    BpmTimeline {
                        points: vec![BpmPoint {
                            time_ms: 0.0,
                            bpm: 120.0,
                            beats_per_measure: 4.0,
                            start_beat: 0.0,
                        }],
                    },
                    vec![TimelineEvent {
                        time_ms: 0.0,
                        label: "chart 120.00/4.00".to_owned(),
                        color: Color::from_rgba(140, 214, 255, 255),
                    }],
                    format!("failed to load chart: {err}"),
                )
            }
        };

        let next_note_id = notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        Self {
            chart_path: path.to_owned(),
            notes,
            next_note_id,
            selected_note_id: None,
            drag_state: None,
            timeline,
            timeline_events,
            snap_enabled: true,
            snap_division: 4,
            scroll_speed: DEFAULT_SCROLL_SPEED,
            render_scope: RenderScope::Both,
            place_note_type: None,
            pending_hold: None,
            waveform: None,
            waveform_error: None,
            waveform_seek_active: false,
            waveform_seek_sec: 0.0,
            status,
        }
    }

    pub fn place_note_type(&self) -> Option<PlaceNoteType> {
        self.place_note_type
    }

    pub fn render_scope(&self) -> RenderScope {
        self.render_scope
    }

    pub fn set_render_scope(&mut self, scope: RenderScope) {
        self.render_scope = scope;
        self.pending_hold = None;
        self.status = format!("render scope: {}", scope.label());
    }

    pub fn set_place_note_type(&mut self, note_type: Option<PlaceNoteType>) {
        self.place_note_type = note_type;
        self.pending_hold = None;
        self.status = match note_type {
            Some(kind) => format!("place mode: {}", kind.label()),
            None => "place mode cleared".to_owned(),
        };
    }

    pub fn pending_hold_head_time_ms(&self) -> Option<f32> {
        self.pending_hold.map(|pending| pending.start_time_ms)
    }

    pub fn draw(
        &mut self,
        area: Rect,
        current_sec: f32,
        audio_duration_sec: f32,
        audio_path: Option<&str>,
    ) -> Vec<FallingEditorAction> {
        self.sync_waveform(audio_path);
        let mut actions = Vec::new();
        let current_ms = current_sec * 1000.0;

        let header_h = 34.0;
        let footer_h = 22.0;
        let header_rect = Rect::new(area.x, area.y, area.w, header_h);
        let content_rect = Rect::new(
            area.x + 8.0,
            area.y + header_h + 6.0,
            (area.w - 16.0).max(40.0),
            (area.h - header_h - footer_h - 10.0).max(40.0),
        );
        let (left_screen, right_screen) = self.split_portrait_screens(content_rect);

        let left_inner = Rect::new(
            left_screen.x + 8.0,
            left_screen.y + 8.0,
            (left_screen.w - 16.0).max(8.0),
            (left_screen.h - 16.0).max(8.0),
        );
        let right_inner = Rect::new(
            right_screen.x + 8.0,
            right_screen.y + 8.0,
            (right_screen.w - 16.0).max(8.0),
            (right_screen.h - 16.0).max(8.0),
        );

        let progress_w = (right_inner.w * 0.08).clamp(14.0, 24.0);
        let progress_rect = Rect::new(
            right_inner.x + right_inner.w - progress_w,
            right_inner.y,
            progress_w,
            right_inner.h,
        );
        let lanes_rect = Rect::new(
            right_inner.x,
            right_inner.y,
            (right_inner.w - progress_w - 8.0).max(8.0),
            right_inner.h,
        );

        draw_rectangle(area.x, area.y, area.w, area.h, Color::from_rgba(10, 10, 12, 255));
        draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, Color::from_rgba(44, 44, 52, 255));

        draw_rectangle(
            left_screen.x,
            left_screen.y,
            left_screen.w,
            left_screen.h,
            Color::from_rgba(12, 12, 18, 255),
        );
        draw_rectangle_lines(
            left_screen.x,
            left_screen.y,
            left_screen.w,
            left_screen.h,
            1.0,
            Color::from_rgba(56, 62, 86, 255),
        );
        draw_rectangle(
            right_screen.x,
            right_screen.y,
            right_screen.w,
            right_screen.h,
            Color::from_rgba(12, 12, 18, 255),
        );
        draw_rectangle_lines(
            right_screen.x,
            right_screen.y,
            right_screen.w,
            right_screen.h,
            1.0,
            Color::from_rgba(56, 62, 86, 255),
        );

        self.draw_header(header_rect);
        self.handle_scroll_speed_controls(header_rect);
        self.handle_vertical_progress_seek(progress_rect, audio_duration_sec, &mut actions);
        self.draw_vertical_progress(progress_rect, current_sec, audio_duration_sec);

        let (ground_rect, air_rect) = match self.render_scope {
            RenderScope::Both => {
                self.draw_event_view(left_inner, current_ms);
                (Some(lanes_rect), Some(lanes_rect))
            }
            RenderScope::Split => (Some(lanes_rect), Some(left_inner)),
        };

        if is_mouse_button_pressed(MouseButton::Right)
            && (self.place_note_type.is_some() || self.pending_hold.is_some())
        {
            self.place_note_type = None;
            self.pending_hold = None;
            self.drag_state = None;
            self.status = "place mode cleared".to_owned();
        }

        if let Some(rect) = ground_rect {
            self.handle_ground_input(rect, current_ms);
            self.draw_ground_view(rect, current_ms, true);
        }
        if let Some(rect) = air_rect {
            self.handle_air_input(rect, current_ms);
            self.draw_air_view(
                rect,
                current_ms,
                self.render_scope == RenderScope::Both,
                self.render_scope != RenderScope::Both,
            );
        }

        let (mx, my) = mouse_position();
        let using_note_cursor = match self.place_note_type {
            Some(tool) if is_ground_tool(tool) => {
                ground_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false)
            }
            Some(tool) if is_air_tool(tool) => air_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false),
            _ => false,
        };
        show_mouse(!using_note_cursor);
        if using_note_cursor {
            match self.place_note_type {
                Some(tool) if is_ground_tool(tool) => {
                    if let Some(rect) = ground_rect {
                        self.draw_place_cursor(rect, current_ms);
                    }
                }
                Some(tool) if is_air_tool(tool) => {
                    if let Some(rect) = air_rect {
                        self.draw_place_cursor(rect, current_ms);
                    }
                }
                _ => {}
            }
        }

        if let Some(error) = &self.waveform_error {
            draw_text_ex(
                error,
                area.x + 12.0,
                area.y + area.h - 6.0,
                TextParams {
                    font_size: 18,
                    color: Color::from_rgba(255, 100, 100, 255),
                    ..Default::default()
                },
            );
        } else {
            draw_text_ex(
                &self.status,
                area.x + 12.0,
                area.y + area.h - 6.0,
                TextParams {
                    font_size: 18,
                    color: Color::from_rgba(176, 210, 255, 255),
                    ..Default::default()
                },
            );
        }

        actions
    }

    fn split_portrait_screens(&self, rect: Rect) -> (Rect, Rect) {
        let gap = (rect.w * 0.04).clamp(10.0, 28.0);
        let max_h_by_width = ((rect.w - gap).max(10.0)) / (2.0 * PORTRAIT_SCREEN_RATIO);
        let screen_h = rect.h.min(max_h_by_width).max(20.0);
        let screen_w = (screen_h * PORTRAIT_SCREEN_RATIO).max(12.0);
        let pair_w = screen_w * 2.0 + gap;
        let start_x = rect.x + (rect.w - pair_w) * 0.5;
        let y = rect.y + (rect.h - screen_h) * 0.5;
        (
            Rect::new(start_x, y, screen_w, screen_h),
            Rect::new(start_x + screen_w + gap, y, screen_w, screen_h),
        )
    }

    fn draw_event_view(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 || rect.w <= 8.0 {
            return;
        }

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(9, 11, 19, 255));
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(44, 58, 86, 255),
        );

        draw_text_ex(
            "EVENTS",
            rect.x + 8.0,
            rect.y + 20.0,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(198, 218, 250, 255),
                ..Default::default()
            },
        );

        let judge_y = rect.y + rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let ahead_ms = ((judge_y - rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((rect.y + rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);

        for barline in self.timeline.visible_barlines(current_ms, ahead_ms, behind_ms) {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 22.0 || y > rect.y + rect.h + 1.0 {
                continue;
            }
            let color = if barline.is_major {
                Color::from_rgba(92, 122, 166, 170)
            } else {
                Color::from_rgba(62, 82, 118, 130)
            };
            let thickness = if barline.is_major { 1.4 } else { 1.0 };
            draw_line(rect.x + 6.0, y, rect.x + rect.w - 6.0, y, thickness, color);
        }

        draw_line(
            rect.x + 6.0,
            judge_y,
            rect.x + rect.w - 6.0,
            judge_y,
            2.2,
            Color::from_rgba(255, 146, 114, 240),
        );

        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;
        let mut drawn = 0_u32;
        let mut last_y = f32::NEG_INFINITY;
        for event in &self.timeline_events {
            if event.time_ms < start_ms - 0.001 || event.time_ms > end_ms + 0.001 {
                continue;
            }
            let y = self.time_to_y(event.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 28.0 || y > rect.y + rect.h - 4.0 {
                continue;
            }
            if y - last_y < 12.0 {
                continue;
            }
            draw_circle(rect.x + 10.0, y, 2.8, event.color);
            draw_line(rect.x + 14.0, y, rect.x + 26.0, y, 1.4, event.color);
            draw_text_ex(
                &event.label,
                rect.x + 30.0,
                y + 5.0,
                TextParams {
                    font_size: 16,
                    color: event.color,
                    ..Default::default()
                },
            );
            last_y = y;
            drawn += 1;
            if drawn >= 90 {
                break;
            }
        }
    }

    fn draw_header(&self, rect: Rect) {
        let ground_count = self
            .notes
            .iter()
            .filter(|note| is_ground_kind(note.kind))
            .count();
        let air_count = self
            .notes
            .iter()
            .filter(|note| is_air_kind(note.kind))
            .count();
        draw_text_ex(
            &format!(
                "Falling | chart={} | G:{} A:{} | view={} | tool={} | snap={} 1/{} | speed={:.2}H/s",
                self.chart_path,
                ground_count,
                air_count,
                self.render_scope.label(),
                self.place_note_type
                    .map(PlaceNoteType::label)
                    .unwrap_or("None"),
                if self.snap_enabled { "on" } else { "off" },
                self.snap_division,
                self.scroll_speed
            ),
            rect.x + 10.0,
            rect.y + 24.0,
            TextParams {
                font_size: 22,
                color: WHITE,
                ..Default::default()
            },
        );
    }

    fn handle_scroll_speed_controls(&mut self, header_rect: Rect) {
        let panel_w = 224.0;
        let panel_h = (header_rect.h - 8.0).max(24.0);
        let panel_rect = Rect::new(
            header_rect.x + header_rect.w - panel_w - 10.0,
            header_rect.y + 4.0,
            panel_w,
            panel_h,
        );
        draw_rectangle(
            panel_rect.x,
            panel_rect.y,
            panel_rect.w,
            panel_rect.h,
            Color::from_rgba(18, 18, 28, 232),
        );
        draw_rectangle_lines(
            panel_rect.x,
            panel_rect.y,
            panel_rect.w,
            panel_rect.h,
            1.0,
            Color::from_rgba(78, 78, 96, 255),
        );

        let minus_rect = Rect::new(panel_rect.x + 6.0, panel_rect.y + 3.0, 28.0, panel_rect.h - 6.0);
        let plus_rect = Rect::new(
            panel_rect.x + panel_rect.w - 34.0,
            panel_rect.y + 3.0,
            28.0,
            panel_rect.h - 6.0,
        );

        if draw_small_button(minus_rect, "-") {
            self.adjust_scroll_speed(-SCROLL_SPEED_STEP);
        }
        if draw_small_button(plus_rect, "+") {
            self.adjust_scroll_speed(SCROLL_SPEED_STEP);
        }

        let (mx, my) = mouse_position();
        if point_in_rect(mx, my, panel_rect) {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y.abs() > f32::EPSILON {
                self.adjust_scroll_speed(wheel_y * SCROLL_SPEED_STEP);
            }
        }

        draw_text_ex(
            &format!("Flow {:.2}H/s", self.scroll_speed),
            panel_rect.x + 42.0,
            panel_rect.y + panel_rect.h * 0.72,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(220, 226, 240, 255),
                ..Default::default()
            },
        );
    }

    fn draw_vertical_progress(&self, rect: Rect, current_sec: f32, duration_sec: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(12, 16, 24, 255));
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(44, 54, 84, 255),
        );

        let duration = self.estimate_duration(duration_sec).max(0.001);
        let progress = (current_sec / duration).clamp(0.0, 1.0);
        let fill_h = rect.h * progress;
        if fill_h > 0.5 {
            draw_rectangle(
                rect.x + 2.0,
                rect.y,
                (rect.w - 4.0).max(1.0),
                fill_h,
                Color::from_rgba(74, 134, 210, 165),
            );
        }

        let playhead_y = rect.y + progress * rect.h;
        draw_line(
            rect.x,
            playhead_y,
            rect.x + rect.w,
            playhead_y,
            2.0,
            Color::from_rgba(255, 96, 96, 255),
        );
        if self.waveform_seek_active {
            let seek_progress = (self.waveform_seek_sec / duration).clamp(0.0, 1.0);
            let seek_y = rect.y + seek_progress * rect.h;
            draw_line(
                rect.x,
                seek_y,
                rect.x + rect.w,
                seek_y,
                1.6,
                Color::from_rgba(255, 220, 80, 255),
            );
        }

        draw_text_ex(
            "AUDIO",
            rect.x + 2.0,
            rect.y + 16.0,
            TextParams {
                font_size: 14,
                color: Color::from_rgba(176, 200, 236, 255),
                ..Default::default()
            },
        );
    }

    fn draw_falling_spectrum(&self, rect: Rect, current_ms: f32, judge_y: f32, tint: Color) {
        let Some(waveform) = &self.waveform else {
            return;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 || rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }

        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let mut y = rect.y;
        while y <= rect.y + rect.h {
            let dt_ms = (judge_y - y) / pixels_per_sec * 1000.0;
            let time_sec = (current_ms + dt_ms).max(0.0) / 1000.0;
            let amp = self.sample_waveform_amp(time_sec).powf(0.82);
            if amp > 0.015 {
                let alpha = (amp * 116.0).clamp(10.0, 128.0) as u8;
                let main_color = Color::new(tint.r, tint.g, tint.b, alpha as f32 / 255.0);
                let edge = (rect.w * (0.5 - 0.46 * amp)).clamp(0.0, rect.w * 0.45);
                draw_line(
                    rect.x + edge,
                    y,
                    rect.x + rect.w - edge,
                    y,
                    1.0,
                    main_color,
                );
            }
            y += 2.0;
        }
    }

    fn sample_waveform_amp(&self, sec: f32) -> f32 {
        let Some(waveform) = &self.waveform else {
            return 0.0;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 {
            return 0.0;
        }
        let len = waveform.peaks.len();
        if len == 1 {
            return waveform.peaks[0].clamp(0.0, 1.0);
        }
        let t = (sec / waveform.duration_sec).clamp(0.0, 1.0);
        let pos = t * (len as f32 - 1.0);
        let i0 = pos.floor() as usize;
        let i1 = (i0 + 1).min(len - 1);
        let f = pos - i0 as f32;
        lerp(waveform.peaks[i0], waveform.peaks[i1], f).clamp(0.0, 1.0)
    }

    fn draw_ground_view(&self, rect: Rect, current_ms: f32, show_spectrum: bool) {
        if rect.h <= 8.0 {
            return;
        }

        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let ahead_ms = ((judge_y - rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((rect.y + rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);

        for lane in 0..LANE_COUNT {
            let x = rect.x + lane as f32 * lane_w;
            let bg = if lane % 2 == 0 {
                Color::from_rgba(18, 18, 22, 255)
            } else {
                Color::from_rgba(22, 22, 28, 255)
            };
            draw_rectangle(x, rect.y, lane_w, rect.h, bg);
            draw_line(x, rect.y, x, rect.y + rect.h, 1.0, Color::from_rgba(36, 36, 48, 255));
            draw_text_ex(
                &format!("L{lane}"),
                x + 8.0,
                rect.y + 20.0,
                TextParams {
                    font_size: 18,
                    color: Color::from_rgba(170, 170, 180, 255),
                    ..Default::default()
                },
            );
        }
        draw_line(
            rect.x + rect.w,
            rect.y,
            rect.x + rect.w,
            rect.y + rect.h,
            1.0,
            Color::from_rgba(36, 36, 48, 255),
        );

        if show_spectrum {
            self.draw_falling_spectrum(
                rect,
                current_ms,
                judge_y,
                Color::from_rgba(86, 176, 255, 255),
            );
        }

        for barline in self.timeline.visible_barlines(current_ms, ahead_ms, behind_ms) {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = if barline.is_major {
                (2.0, Color::from_rgba(170, 205, 255, 210))
            } else {
                (1.0, Color::from_rgba(90, 120, 150, 170))
            };
            draw_line(rect.x, y, rect.x + rect.w, y, thickness, color);
        }

        draw_line(
            rect.x,
            judge_y,
            rect.x + rect.w,
            judge_y,
            3.0,
            Color::from_rgba(255, 120, 96, 255),
        );
        draw_text_ex(
            "JUDGE",
            rect.x + 8.0,
            judge_y - 6.0,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(255, 170, 140, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            "GROUND",
            rect.x + rect.w - 112.0,
            rect.y + 22.0,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(185, 198, 224, 255),
                ..Default::default()
            },
        );

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let lane_x = rect.x + lane_w * note.lane as f32;
            let note_w = note_head_width(note, lane_w);
            let note_x = lane_x + (lane_w - note_w) * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let selected = self.selected_note_id == Some(note.id);
            let palette = lane_note_palette(note.lane);

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                if y2 >= rect.y && y1 <= rect.y + rect.h {
                    let (body_x, body_w, body_color) = match note.kind {
                        GroundNoteKind::Hold => (
                            note_x + note_w * 0.04,
                            note_w * 0.92,
                            palette.hold_body,
                        ),
                        GroundNoteKind::SkyArea => (
                            note_x + note_w * 0.02,
                            note_w * 0.96,
                            palette.skyarea_body,
                        ),
                        _ => (
                            note_x + note_w * 0.35,
                            note_w * 0.3,
                            palette.hold_body,
                        ),
                    };
                    let body_y = y1.max(rect.y);
                    let body_h = (y2.min(rect.y + rect.h) - body_y).max(1.0);
                    draw_rectangle(body_x, body_y, body_w, body_h, body_color);
                }
            }

            if head_y >= rect.y - 28.0 && head_y <= rect.y + rect.h + 28.0 {
                let head_color = match note.kind {
                    GroundNoteKind::Tap => palette.tap,
                    GroundNoteKind::Hold => palette.hold_head,
                    _ => palette.tap,
                };

                draw_rectangle(note_x, head_y - 8.0, note_w, 16.0, head_color);
                draw_rectangle(
                    note_x + 1.5,
                    head_y - 7.0,
                    (note_w - 3.0).max(1.0),
                    5.0,
                    Color::from_rgba(255, 255, 255, 34),
                );

                if selected {
                    draw_rectangle_lines(
                        note_x - 1.0,
                        head_y - 9.0,
                        note_w + 2.0,
                        18.0,
                        2.0,
                        Color::from_rgba(255, 220, 96, 255),
                    );
                }
            }
        }

    }

    fn draw_air_view(&self, rect: Rect, current_ms: f32, overlay_mode: bool, show_spectrum: bool) {
        if rect.h <= 8.0 {
            return;
        }
        let split_rect = air_split_rect(rect);

        if overlay_mode {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(48, 40, 78, 28));
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::from_rgba(86, 94, 124, 120));
        } else {
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(14, 18, 26, 255));
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::from_rgba(44, 58, 84, 255));
        }

        for i in 0..=4 {
            let x = split_rect.x + split_rect.w * (i as f32 / 4.0);
            let color = if i == 0 || i == 4 {
                if overlay_mode {
                    Color::from_rgba(136, 152, 196, 180)
                } else {
                    Color::from_rgba(56, 76, 110, 255)
                }
            } else {
                if overlay_mode {
                    Color::from_rgba(102, 118, 160, 138)
                } else {
                    Color::from_rgba(42, 56, 84, 220)
                }
            };
            draw_line(x, rect.y, x, rect.y + rect.h, 1.0, color);
        }

        let judge_y = rect.y + rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let ahead_ms = ((judge_y - rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((rect.y + rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);

        if show_spectrum {
            self.draw_falling_spectrum(
                split_rect,
                current_ms,
                judge_y,
                Color::from_rgba(178, 196, 255, 255),
            );
        }

        for barline in self.timeline.visible_barlines(current_ms, ahead_ms, behind_ms) {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = if barline.is_major {
                (2.0, Color::from_rgba(164, 198, 255, 210))
            } else {
                (1.0, Color::from_rgba(82, 112, 150, 170))
            };
            draw_line(split_rect.x, y, split_rect.x + split_rect.w, y, thickness, color);
        }

        draw_line(
            split_rect.x,
            judge_y,
            split_rect.x + split_rect.w,
            judge_y,
            3.0,
            if overlay_mode {
                Color::from_rgba(170, 206, 255, 220)
            } else {
                Color::from_rgba(132, 196, 255, 255)
            },
        );
        draw_text_ex(
            "SKY",
            rect.x + 8.0,
            rect.y + 22.0,
            TextParams {
                font_size: 18,
                color: if overlay_mode {
                    Color::from_rgba(214, 226, 250, 230)
                } else {
                    Color::from_rgba(190, 216, 255, 255)
                },
                ..Default::default()
            },
        );

        for note in &self.notes {
            if !is_air_kind(note.kind) {
                continue;
            }
            let x_norm = lane_to_air_x_norm(note.lane);
            let center_x = split_rect.x + x_norm * split_rect.w;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let selected = self.selected_note_id == Some(note.id);
            let lane_for_palette = note.lane.clamp(0, LANE_COUNT - 1);
            let palette = lane_note_palette(lane_for_palette);

            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    self.draw_skyarea_shape(
                        split_rect,
                        current_ms,
                        judge_y,
                        rect.h,
                        note,
                        shape,
                        selected,
                    );
                    continue;
                }
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                if y2 >= rect.y && y1 <= rect.y + rect.h {
                    let body_y = y1.max(rect.y);
                    let body_h = (y2.min(rect.y + rect.h) - body_y).max(1.0);
                    let body_color = match note.kind {
                        GroundNoteKind::SkyArea => AIR_SKYAREA_BODY_COLOR,
                        _ => palette.hold_body,
                    };
                    draw_rectangle(note_x, body_y, note_w, body_h, body_color);
                }
            }

            if head_y >= rect.y - 24.0 && head_y <= rect.y + rect.h + 24.0 {
                if note.kind == GroundNoteKind::Flick {
                    draw_flick_curve_shape(note, note_x, note_w, head_y);
                    if selected {
                        let bounds = flick_shape_bounds(note, note_x, note_w, head_y);
                        draw_rectangle_lines(
                            bounds.x - 1.0,
                            bounds.y - 1.0,
                            bounds.w + 2.0,
                            bounds.h + 2.0,
                            2.0,
                            Color::from_rgba(255, 220, 96, 255),
                        );
                    }
                } else {
                    let head_color = match note.kind {
                        GroundNoteKind::SkyArea => AIR_SKYAREA_HEAD_COLOR,
                        _ => palette.tap,
                    };
                    draw_rectangle(note_x, head_y - 8.0, note_w, 16.0, head_color);
                    draw_rectangle(
                        note_x + 1.0,
                        head_y - 7.0,
                        (note_w - 2.0).max(1.0),
                        5.0,
                        Color::from_rgba(255, 255, 255, 34),
                    );

                    if selected {
                    draw_rectangle_lines(
                        note_x - 1.0,
                        head_y - 9.0,
                        note_w + 2.0,
                        18.0,
                        2.0,
                        Color::from_rgba(255, 220, 96, 255),
                    );
                    }
                }
            }
        }
    }

    fn draw_place_cursor(&self, rect: Rect, current_ms: f32) {
        let Some(place_type) = self.place_note_type else {
            return;
        };
        if rect.h <= 8.0 {
            return;
        }

        let (mx, my) = mouse_position();
        if !point_in_rect(mx, my, rect) {
            return;
        }

        if is_ground_tool(place_type) {
            let lane_w = rect.w / LANE_COUNT as f32;
            let judge_y = rect.y + rect.h * 0.82;
            let lane = lane_from_x(mx, rect.x, lane_w);
            let palette = lane_note_palette(lane);
            match place_type {
                PlaceNoteType::Hold => {
                    if let Some(pending) = self.pending_hold {
                        let lane_x = rect.x + lane_w * pending.lane as f32;
                        let note_w = lane_w * 0.94;
                        let note_x = lane_x + (lane_w - note_w) * 0.5;
                        let start_y = self.time_to_y(pending.start_time_ms, current_ms, judge_y, rect.h);
                        let y1 = start_y.min(my);
                        let y2 = start_y.max(my);

                        draw_rectangle(
                            note_x + note_w * 0.04,
                            y1,
                            note_w * 0.92,
                            (y2 - y1).max(1.0),
                            Color::from_rgba(236, 204, 120, 116),
                        );
                        draw_rectangle(
                            note_x,
                            start_y - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 222, 140, 220),
                        );
                        draw_rectangle(
                            note_x,
                            my - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 236, 170, 220),
                        );
                    } else {
                        let lane_x = rect.x + lane_w * lane as f32;
                        let note_w = lane_w * 0.94;
                        let note_x = lane_x + (lane_w - note_w) * 0.5;
                        draw_rectangle(
                            note_x,
                            my - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 222, 140, 220),
                        );
                    }
                }
                PlaceNoteType::Tap => {
                    let lane_x = rect.x + lane_w * lane as f32;
                    let note_w = lane_w * 0.78;
                    let note_x = lane_x + (lane_w - note_w) * 0.5;
                    draw_rectangle(
                        note_x,
                        my - 8.0,
                        note_w,
                        16.0,
                        Color::new(palette.tap.r, palette.tap.g, palette.tap.b, 0.82),
                    );
                }
                _ => {}
            }
        } else if is_air_tool(place_type) {
            let split_rect = air_split_rect(rect);
            if !point_in_rect(mx, my, split_rect) {
                return;
            }
            let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
            let lane = air_x_to_lane(x_norm);
            let center_x = split_rect.x + x_norm * split_rect.w;
            let note_w = match place_type {
                PlaceNoteType::SkyArea => split_rect.w * DEFAULT_AIR_WIDTH_NORM,
                _ => split_rect.w * DEFAULT_AIR_WIDTH_NORM,
            };
            let note_x = center_x - note_w * 0.5;
            let preview = GroundNote {
                id: 0,
                kind: GroundNoteKind::Flick,
                lane,
                time_ms: 0.0,
                duration_ms: 0.0,
                width: 1.0,
                flick_right: true,
                skyarea_shape: None,
            };
            if place_type == PlaceNoteType::Flick {
                draw_flick_curve_shape(&preview, note_x, note_w, my);
            } else {
                let color = match place_type {
                    PlaceNoteType::SkyArea => AIR_SKYAREA_HEAD_COLOR,
                    _ => WHITE,
                };
                draw_rectangle(note_x, my - 8.0, note_w, 16.0, color);
            }
        }
    }

    fn draw_skyarea_shape(
        &self,
        split_rect: Rect,
        current_ms: f32,
        judge_y: f32,
        lane_h: f32,
        note: &GroundNote,
        shape: SkyAreaShape,
        selected: bool,
    ) {
        if note.duration_ms <= 1.0 {
            return;
        }
        let clip_top = split_rect.y;
        let clip_bottom = split_rect.y + split_rect.h;

        let seg_count = 20;
        for i in 0..seg_count {
            let p0 = i as f32 / seg_count as f32;
            let p1 = (i + 1) as f32 / seg_count as f32;
            let t0 = note.time_ms + note.duration_ms * p0;
            let t1 = note.time_ms + note.duration_ms * p1;
            let y0_raw = self.time_to_y(t0, current_ms, judge_y, lane_h);
            let y1_raw = self.time_to_y(t1, current_ms, judge_y, lane_h);
            if (y0_raw < clip_top && y1_raw < clip_top) || (y0_raw > clip_bottom && y1_raw > clip_bottom)
            {
                continue;
            }
            let y0 = y0_raw.clamp(clip_top, clip_bottom);
            let y1 = y1_raw.clamp(clip_top, clip_bottom);

            let left0 = lerp(
                shape.start_left_norm,
                shape.end_left_norm,
                ease_progress(shape.left_ease, p0),
            );
            let right0 = lerp(
                shape.start_right_norm,
                shape.end_right_norm,
                ease_progress(shape.right_ease, p0),
            );
            let left1 = lerp(
                shape.start_left_norm,
                shape.end_left_norm,
                ease_progress(shape.left_ease, p1),
            );
            let right1 = lerp(
                shape.start_right_norm,
                shape.end_right_norm,
                ease_progress(shape.right_ease, p1),
            );

            let lx0 = split_rect.x + left0.clamp(0.0, 1.0) * split_rect.w;
            let rx0 = split_rect.x + right0.clamp(0.0, 1.0) * split_rect.w;
            let lx1 = split_rect.x + left1.clamp(0.0, 1.0) * split_rect.w;
            let rx1 = split_rect.x + right1.clamp(0.0, 1.0) * split_rect.w;

            draw_triangle(
                Vec2::new(lx0, y0),
                Vec2::new(rx0, y0),
                Vec2::new(rx1, y1),
                AIR_SKYAREA_BODY_COLOR,
            );
            draw_triangle(
                Vec2::new(lx0, y0),
                Vec2::new(rx1, y1),
                Vec2::new(lx1, y1),
                AIR_SKYAREA_BODY_COLOR,
            );
        }

        let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, lane_h);
        if head_y < clip_top - 18.0 || head_y > clip_bottom + 18.0 {
            return;
        }
        let head_w = (head_right - head_left).max(2.0);
        draw_rectangle(head_left, head_y - 8.0, head_w, 16.0, AIR_SKYAREA_HEAD_COLOR);

        if selected {
            draw_rectangle_lines(
                head_left - 1.0,
                head_y - 9.0,
                head_w + 2.0,
                18.0,
                2.0,
                Color::from_rgba(255, 220, 96, 255),
            );
        }
    }

    fn handle_vertical_progress_seek(
        &mut self,
        rect: Rect,
        audio_duration_sec: f32,
        actions: &mut Vec<FallingEditorAction>,
    ) {
        let (mx, my) = mouse_position();
        let inside = point_in_rect(mx, my, rect);
        let duration = self.estimate_duration(audio_duration_sec);

        if is_mouse_button_pressed(MouseButton::Left) && inside {
            self.waveform_seek_active = true;
            self.waveform_seek_sec = y_to_time_sec(my, rect, duration);
        }

        if self.waveform_seek_active && is_mouse_button_down(MouseButton::Left) {
            self.waveform_seek_sec = y_to_time_sec(my, rect, duration);
        }

        if self.waveform_seek_active && is_mouse_button_released(MouseButton::Left) {
            self.waveform_seek_active = false;
            actions.push(FallingEditorAction::SeekTo(self.waveform_seek_sec));
            self.status = format!("seek to {:.2}s", self.waveform_seek_sec);
        }
    }

    fn handle_ground_input(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 {
            return;
        }
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = mouse_position();
        let inside = point_in_rect(mx, my, rect);

        if is_mouse_button_pressed(MouseButton::Left) && inside {
            if let Some(tool) = self.place_note_type {
                if !is_ground_tool(tool) {
                    return;
                }
                let lane = lane_from_x(mx, rect.x, lane_w);
                let time_ms = self.apply_snap(
                    self.pointer_to_time(my, current_ms, judge_y, rect.h)
                        .max(0.0),
                );

                match tool {
                    PlaceNoteType::Tap => {
                        self.push_note(GroundNote {
                            id: self.next_note_id,
                            kind: GroundNoteKind::Tap,
                            lane,
                            time_ms,
                            duration_ms: 0.0,
                            width: 1.0,
                            flick_right: true,
                            skyarea_shape: None,
                        });
                        self.status = "new tap created".to_owned();
                    }
                    PlaceNoteType::Hold => {
                        if let Some(pending) = self.pending_hold.take() {
                            let start = pending.start_time_ms.min(time_ms);
                            let end = pending.start_time_ms.max(time_ms);
                            let duration = (end - start).max(30.0);
                            self.push_note(GroundNote {
                                id: self.next_note_id,
                                kind: GroundNoteKind::Hold,
                                lane: pending.lane,
                                time_ms: start,
                                duration_ms: duration,
                                width: 1.0,
                                flick_right: true,
                                skyarea_shape: None,
                            });
                            self.status = format!(
                                "new hold created lane={} {}ms -> {}ms",
                                pending.lane,
                                start.round(),
                                end.round()
                            );
                        } else {
                            self.pending_hold = Some(PendingHoldPlacement {
                                lane,
                                start_time_ms: time_ms,
                            });
                            self.status = format!("hold head set: lane={} time={:.0}ms", lane, time_ms);
                        }
                    }
                    _ => {}
                }
            } else if let Some(note_id) = self.hit_test_ground(mx, my, rect, current_ms) {
                self.selected_note_id = Some(note_id);
                let time_ms = self.pointer_to_time(my, current_ms, judge_y, rect.h);
                if let Some(note) = self.notes.iter().find(|note| note.id == note_id) {
                    self.drag_state = Some(DragState {
                        note_id,
                        time_offset_ms: note.time_ms - time_ms,
                    });
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) && inside && self.place_note_type.is_none() {
            self.selected_note_id = None;
            self.drag_state = None;
        }

        if let Some(drag) = self.drag_state {
            if is_mouse_button_down(MouseButton::Left) {
                let lane = lane_from_x(mx, rect.x, lane_w);
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                if let Some(note) = self
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_ground_kind(note.kind))
                {
                    note.lane = lane;
                    note.time_ms = snapped_time;
                    self.status = format!("dragging lane={} time={:.0}ms", lane, note.time_ms);
                }
            } else {
                self.drag_state = None;
                self.sort_notes();
            }
        }
    }

    fn handle_air_input(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 {
            return;
        }
        let split_rect = air_split_rect(rect);
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = mouse_position();
        let inside = point_in_rect(mx, my, split_rect);

        if is_mouse_button_pressed(MouseButton::Left) && inside {
            if let Some(tool) = self.place_note_type {
                if !is_air_tool(tool) {
                    return;
                }
                let time_ms = self.apply_snap(
                    self.pointer_to_time(my, current_ms, judge_y, rect.h)
                        .max(0.0),
                );
                let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                let lane = air_x_to_lane(x_norm);

                match tool {
                    PlaceNoteType::Flick => {
                        self.push_note(GroundNote {
                            id: self.next_note_id,
                            kind: GroundNoteKind::Flick,
                            lane,
                            time_ms,
                            duration_ms: 0.0,
                            width: DEFAULT_AIR_WIDTH_NORM,
                            flick_right: true,
                            skyarea_shape: None,
                        });
                        self.status = "new flick created".to_owned();
                    }
                    PlaceNoteType::SkyArea => {
                        let half = DEFAULT_AIR_WIDTH_NORM * 0.5;
                        let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                        let left = (x_norm - half).clamp(0.0, 1.0);
                        let right = (x_norm + half).clamp(0.0, 1.0);
                        self.push_note(GroundNote {
                            id: self.next_note_id,
                            kind: GroundNoteKind::SkyArea,
                            lane,
                            time_ms,
                            duration_ms: DEFAULT_SKYAREA_MS,
                            width: DEFAULT_AIR_WIDTH_NORM,
                            flick_right: true,
                            skyarea_shape: Some(SkyAreaShape {
                                start_left_norm: left,
                                start_right_norm: right,
                                end_left_norm: left,
                                end_right_norm: right,
                                left_ease: Ease::Linear,
                                right_ease: Ease::Linear,
                            }),
                        });
                        self.status = "new skyarea created".to_owned();
                    }
                    _ => {}
                }
            } else if let Some(note_id) = self.hit_test_air(mx, my, rect, current_ms) {
                self.selected_note_id = Some(note_id);
                let time_ms = self.pointer_to_time(my, current_ms, judge_y, rect.h);
                if let Some(note) = self.notes.iter().find(|note| note.id == note_id) {
                    self.drag_state = Some(DragState {
                        note_id,
                        time_offset_ms: note.time_ms - time_ms,
                    });
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) && inside && self.place_note_type.is_none() {
            self.selected_note_id = None;
            self.drag_state = None;
        }

        if let Some(drag) = self.drag_state {
            if is_mouse_button_down(MouseButton::Left) {
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                if let Some(note) = self
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_air_kind(note.kind))
                {
                    note.lane = air_x_to_lane(x_norm);
                    if note.kind == GroundNoteKind::SkyArea {
                        if let Some(shape) = note.skyarea_shape.as_mut() {
                            let start_w = (shape.start_right_norm - shape.start_left_norm)
                                .abs()
                                .clamp(0.02, 1.0);
                            let end_w = (shape.end_right_norm - shape.end_left_norm)
                                .abs()
                                .clamp(0.02, 1.0);
                            let start_half = start_w * 0.5;
                            let end_half = end_w * 0.5;
                            let start_center = x_norm.clamp(start_half, 1.0 - start_half);
                            let end_center = x_norm.clamp(end_half, 1.0 - end_half);
                            shape.start_left_norm = start_center - start_half;
                            shape.start_right_norm = start_center + start_half;
                            shape.end_left_norm = end_center - end_half;
                            shape.end_right_norm = end_center + end_half;
                        }
                    }
                    note.time_ms = snapped_time;
                    self.status = format!("air drag lane={} time={:.0}ms", note.lane, note.time_ms);
                }
            } else {
                self.drag_state = None;
                self.sort_notes();
            }
        }
    }

    fn hit_test_air(&self, mx: f32, my: f32, rect: Rect, current_ms: f32) -> Option<u64> {
        let judge_y = rect.y + rect.h * 0.82;
        let split_rect = air_split_rect(rect);
        if !point_in_rect(mx, my, split_rect) {
            return None;
        }
        for note in self.notes.iter().rev() {
            if !is_air_kind(note.kind) {
                continue;
            }
            let center_x = split_rect.x + lane_to_air_x_norm(note.lane) * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let min_left = shape
                        .start_left_norm
                        .min(shape.end_left_norm)
                        .clamp(0.0, 1.0);
                    let max_right = shape
                        .start_right_norm
                        .max(shape.end_right_norm)
                        .clamp(0.0, 1.0);
                    let x1 = split_rect.x + min_left * split_rect.w;
                    let x2 = split_rect.x + max_right * split_rect.w;
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                    let y1 = head_y.min(tail_y);
                    let y2 = head_y.max(tail_y);
                    if point_in_rect(mx, my, Rect::new(x1, y1, (x2 - x1).max(1.0), (y2 - y1).max(1.0)))
                    {
                        return Some(note.id);
                    }
                    continue;
                }
            }
            if point_in_rect(mx, my, Rect::new(note_x, head_y - 10.0, note_w, 20.0)) {
                return Some(note.id);
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                if point_in_rect(mx, my, Rect::new(note_x, y1, note_w, (y2 - y1).max(1.0))) {
                    return Some(note.id);
                }
            }
        }
        None
    }

    fn hit_test_ground(&self, mx: f32, my: f32, rect: Rect, current_ms: f32) -> Option<u64> {
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;

        for note in self.notes.iter().rev() {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let lane_x = rect.x + lane_w * note.lane as f32;
            let note_w = note_head_width(note, lane_w);
            let note_x = lane_x + (lane_w - note_w) * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);

            if point_in_rect(mx, my, Rect::new(note_x, head_y - 10.0, note_w, 20.0)) {
                return Some(note.id);
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let (body_x, body_w) = match note.kind {
                    GroundNoteKind::Hold => (note_x + note_w * 0.04, note_w * 0.92),
                    GroundNoteKind::SkyArea => (note_x + note_w * 0.02, note_w * 0.96),
                    _ => (note_x + note_w * 0.34, note_w * 0.32),
                };
                if point_in_rect(mx, my, Rect::new(body_x, y1, body_w, (y2 - y1).max(1.0))) {
                    return Some(note.id);
                }
            }
        }
        None
    }

    fn pointer_to_time(&self, mouse_y: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        current_ms + (judge_y - mouse_y) / (self.scroll_speed * lane_h).max(1.0) * 1000.0
    }

    fn time_to_y(&self, note_time_ms: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        judge_y - (note_time_ms - current_ms) / 1000.0 * (self.scroll_speed * lane_h)
    }

    fn apply_snap(&self, time_ms: f32) -> f32 {
        if self.snap_enabled {
            self.timeline.snap_time_ms(time_ms, self.snap_division)
        } else {
            time_ms.max(0.0)
        }
    }

    fn adjust_scroll_speed(&mut self, delta: f32) {
        let old_speed = self.scroll_speed;
        let new_speed = (self.scroll_speed + delta).clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED);
        self.scroll_speed = new_speed;
        if (old_speed - new_speed).abs() > 0.01 {
            self.status = format!("scroll speed set to {:.2}H/s", self.scroll_speed);
        }
    }

    fn push_note(&mut self, note: GroundNote) {
        self.next_note_id = self.next_note_id.saturating_add(1);
        self.selected_note_id = Some(note.id);
        self.notes.push(note);
        self.sort_notes();
    }

    fn sort_notes(&mut self) {
        self.notes.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.lane.cmp(&b.lane))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn sync_waveform(&mut self, audio_path: Option<&str>) {
        let Some(path) = audio_path else {
            return;
        };
        let changed = self
            .waveform
            .as_ref()
            .map(|wave| wave.path.as_str() != path)
            .unwrap_or(true);
        if !changed {
            return;
        }

        match Waveform::from_audio_file(path, 4096) {
            Ok(waveform) => {
                self.waveform = Some(waveform);
                self.waveform_error = None;
                self.status = format!("waveform loaded: {path}");
            }
            Err(err) => {
                self.waveform = None;
                self.waveform_error = Some(err);
            }
        }
    }

    fn estimate_duration(&self, audio_duration_sec: f32) -> f32 {
        if audio_duration_sec > 0.0 {
            return audio_duration_sec;
        }
        self.waveform
            .as_ref()
            .map(|waveform| waveform.duration_sec)
            .unwrap_or(1.0)
            .max(1.0)
    }
}

fn lane_from_x(x: f32, lanes_x: f32, lane_w: f32) -> usize {
    ((x - lanes_x) / lane_w).floor().clamp(0.0, (LANE_COUNT as f32) - 1.0) as usize
}

#[derive(Debug, Clone, Copy)]
struct LaneNotePalette {
    tap: Color,
    hold_head: Color,
    hold_body: Color,
    flick_head: Color,
    flick_arrow: Color,
    skyarea_head: Color,
    skyarea_body: Color,
}

fn lane_note_palette(lane: usize) -> LaneNotePalette {
    match lane {
        0 => LaneNotePalette {
            tap: Color::from_rgba(174, 118, 255, 255),
            hold_head: Color::from_rgba(202, 156, 255, 255),
            hold_body: Color::from_rgba(124, 84, 192, 212),
            flick_head: Color::from_rgba(192, 138, 255, 255),
            flick_arrow: Color::from_rgba(248, 224, 255, 255),
            skyarea_head: Color::from_rgba(160, 110, 238, 255),
            skyarea_body: Color::from_rgba(120, 84, 182, 124),
        },
        1 => LaneNotePalette {
            tap: Color::from_rgba(100, 206, 255, 255),
            hold_head: Color::from_rgba(129, 220, 255, 255),
            hold_body: Color::from_rgba(73, 145, 186, 212),
            flick_head: Color::from_rgba(128, 220, 255, 255),
            flick_arrow: Color::from_rgba(220, 245, 255, 255),
            skyarea_head: Color::from_rgba(82, 186, 236, 255),
            skyarea_body: Color::from_rgba(52, 130, 170, 124),
        },
        2 => LaneNotePalette {
            tap: Color::from_rgba(108, 220, 255, 255),
            hold_head: Color::from_rgba(138, 232, 255, 255),
            hold_body: Color::from_rgba(77, 156, 190, 212),
            flick_head: Color::from_rgba(140, 233, 255, 255),
            flick_arrow: Color::from_rgba(226, 248, 255, 255),
            skyarea_head: Color::from_rgba(90, 194, 238, 255),
            skyarea_body: Color::from_rgba(58, 136, 174, 124),
        },
        3 => LaneNotePalette {
            tap: Color::from_rgba(120, 216, 255, 255),
            hold_head: Color::from_rgba(149, 228, 255, 255),
            hold_body: Color::from_rgba(84, 153, 188, 212),
            flick_head: Color::from_rgba(148, 228, 255, 255),
            flick_arrow: Color::from_rgba(226, 248, 255, 255),
            skyarea_head: Color::from_rgba(96, 191, 238, 255),
            skyarea_body: Color::from_rgba(64, 134, 172, 124),
        },
        4 => LaneNotePalette {
            tap: Color::from_rgba(131, 205, 255, 255),
            hold_head: Color::from_rgba(161, 218, 255, 255),
            hold_body: Color::from_rgba(92, 142, 184, 212),
            flick_head: Color::from_rgba(162, 220, 255, 255),
            flick_arrow: Color::from_rgba(226, 244, 255, 255),
            skyarea_head: Color::from_rgba(106, 181, 232, 255),
            skyarea_body: Color::from_rgba(72, 122, 168, 124),
        },
        _ => LaneNotePalette {
            tap: Color::from_rgba(255, 112, 108, 255),
            hold_head: Color::from_rgba(255, 142, 138, 255),
            hold_body: Color::from_rgba(194, 82, 78, 212),
            flick_head: Color::from_rgba(255, 134, 128, 255),
            flick_arrow: Color::from_rgba(255, 228, 226, 255),
            skyarea_head: Color::from_rgba(238, 100, 94, 255),
            skyarea_body: Color::from_rgba(176, 72, 68, 124),
        },
    }
}

fn note_head_width(note: &GroundNote, lane_w: f32) -> f32 {
    match note.kind {
        GroundNoteKind::Hold | GroundNoteKind::SkyArea => lane_w * 0.94,
        GroundNoteKind::Tap | GroundNoteKind::Flick => lane_w * (0.78 * note.width.clamp(0.5, 1.2)),
    }
}

fn flick_direction_shape_colors(flick_right: bool) -> (Color, Color) {
    if flick_right {
        (
            Color::from_rgba(74, 216, 136, 136),
            Color::from_rgba(154, 255, 190, 242),
        )
    } else {
        (
            Color::from_rgba(238, 214, 84, 128),
            Color::from_rgba(255, 246, 154, 242),
        )
    }
}

fn draw_flick_curve_shape(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32) {
    let (fill_color, edge_color) = flick_direction_shape_colors(note.flick_right);
    let stroke = (note_w * 0.05).clamp(1.0, 2.2);
    let base_head_h = (note_w * 0.23).clamp(6.0, 13.0);
    let side_h = base_head_h * 3.0;
    let y_bottom = head_y + 7.0;
    let y_top = y_bottom - side_h;
    let y_tip_bottom = y_bottom;
    let y_tip_top = y_bottom - (base_head_h * 0.08).clamp(0.6, 1.8);

    let (x_start, x_tip) = if note.flick_right {
        (note_x + note_w * 0.08, note_x + note_w * 0.98)
    } else {
        (note_x + note_w * 0.92, note_x + note_w * 0.02)
    };

    let mut top_curve = Vec::with_capacity(25);
    for i in 0..=24 {
        let t = i as f32 / 24.0;
        let x = lerp(x_start, x_tip, t);
        let eased = ease_progress(Ease::SineOut, t);
        let y = lerp(y_top, y_tip_top, eased);
        top_curve.push(Vec2::new(x, y));
    }

    let mut polygon = Vec::with_capacity(28);
    polygon.push(Vec2::new(x_start, y_bottom));
    polygon.extend_from_slice(&top_curve);
    polygon.push(Vec2::new(x_tip, y_tip_bottom));

    for i in 1..(polygon.len() - 1) {
        draw_triangle(polygon[0], polygon[i], polygon[i + 1], fill_color);
    }

    for i in 0..(top_curve.len() - 1) {
        let a = top_curve[i];
        let b = top_curve[i + 1];
        draw_line(a.x, a.y, b.x, b.y, stroke, edge_color);
    }
    draw_line(x_start, y_bottom, x_tip, y_tip_bottom, stroke, edge_color);
    draw_line(x_start, y_bottom, x_start, y_top, stroke, edge_color);
}

fn flick_shape_bounds(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32) -> Rect {
    let base_head_h = (note_w * 0.23).clamp(6.0, 13.0);
    let side_h = base_head_h * 3.0;
    let y_bottom = head_y + 7.0;
    let y_top = y_bottom - side_h;
    let (x_start, x_tip) = if note.flick_right {
        (note_x + note_w * 0.08, note_x + note_w * 0.98)
    } else {
        (note_x + note_w * 0.92, note_x + note_w * 0.02)
    };
    let x1 = x_start.min(x_tip);
    let x2 = x_start.max(x_tip);
    Rect::new(x1, y_top, (x2 - x1).max(1.0), (y_bottom - y_top).max(1.0))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_progress(ease: Ease, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        Ease::Linear => t,
        Ease::SineOut => (t * std::f32::consts::FRAC_PI_2).sin(),
        Ease::SineIn => 1.0 - (t * std::f32::consts::FRAC_PI_2).cos(),
    }
}

fn y_to_time_sec(y: f32, rect: Rect, duration_sec: f32) -> f32 {
    let t = ((y - rect.y) / rect.h).clamp(0.0, 1.0);
    t * duration_sec
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn draw_small_button(rect: Rect, text: &str) -> bool {
    let (mx, my) = mouse_position();
    let hovered = point_in_rect(mx, my, rect);
    let bg = if hovered {
        Color::from_rgba(104, 108, 138, 255)
    } else {
        Color::from_rgba(64, 68, 92, 255)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(150, 154, 186, 255),
    );

    let metrics = measure_text(text, None, 24, 1.0);
    draw_text_ex(
        text,
        rect.x + (rect.w - metrics.width) * 0.5,
        rect.y + rect.h * 0.72,
        TextParams {
            font_size: 24,
            color: Color::from_rgba(235, 238, 255, 255),
            ..Default::default()
        },
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn extract_timeline_events(chart: &Chart) -> Vec<TimelineEvent> {
    let mut events = Vec::new();

    for event in &chart.events {
        match event {
            ChartEvent::Chart { bpm, beats } => events.push(TimelineEvent {
                time_ms: 0.0,
                label: format!("chart {:.2}/{:.2}", bpm, beats),
                color: Color::from_rgba(126, 210, 255, 255),
            }),
            ChartEvent::Bpm { time, bpm, beats, .. } => events.push(TimelineEvent {
                time_ms: *time as f32,
                label: format!("bpm {:.2} (beats {:.2})", bpm, beats),
                color: Color::from_rgba(124, 226, 255, 255),
            }),
            ChartEvent::Track { time, speed } => {
                let color = if *speed >= 0.0 {
                    Color::from_rgba(150, 240, 170, 255)
                } else {
                    Color::from_rgba(255, 168, 128, 255)
                };
                events.push(TimelineEvent {
                    time_ms: *time as f32,
                    label: format!("track x{:.2}", speed),
                    color,
                });
            }
            ChartEvent::Lane { time, lane, enable } => events.push(TimelineEvent {
                time_ms: *time as f32,
                label: format!("lane {} {}", lane, if *enable { "on" } else { "off" }),
                color: Color::from_rgba(232, 198, 124, 255),
            }),
            _ => {}
        }
    }

    events.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms).then_with(|| a.label.cmp(&b.label)));
    events
}

fn extract_ground_notes(chart: &Chart) -> Vec<GroundNote> {
    let mut notes = Vec::new();
    let mut next_id = 1_u64;

    for event in &chart.events {
        match event {
            ChartEvent::Tap { time, width, lane } => {
                if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                    notes.push(GroundNote {
                        id: next_id,
                        kind: GroundNoteKind::Tap,
                        lane: *lane as usize,
                        time_ms: *time as f32,
                        duration_ms: 0.0,
                        width: (*width as f32).max(0.4),
                        flick_right: true,
                        skyarea_shape: None,
                    });
                    next_id += 1;
                }
            }
            ChartEvent::Hold {
                time,
                lane,
                width,
                duration,
            } => {
                if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                    notes.push(GroundNote {
                        id: next_id,
                        kind: GroundNoteKind::Hold,
                        lane: *lane as usize,
                        time_ms: *time as f32,
                        duration_ms: (*duration as f32).max(1.0),
                        width: (*width as f32).max(0.4),
                        flick_right: true,
                        skyarea_shape: None,
                    });
                    next_id += 1;
                }
            }
            ChartEvent::Flick {
                time,
                x,
                x_split,
                width,
                flick_type,
            } => {
                notes.push(GroundNote {
                    id: next_id,
                    kind: GroundNoteKind::Flick,
                    lane: lane_from_normalized_x((*x as f32) / (*x_split as f32).max(1.0)),
                    time_ms: *time as f32,
                    duration_ms: 0.0,
                    width: normalized_width_to_air_ratio((*width as f32) / (*x_split as f32).max(1.0)),
                    flick_right: !matches!(flick_type, FlickType::Left),
                    skyarea_shape: None,
                });
                next_id += 1;
            }
            ChartEvent::SkyArea {
                time,
                start_x,
                start_x_split,
                start_width,
                end_x,
                end_x_split,
                end_width,
                left_ease,
                right_ease,
                duration,
                ..
            } => {
                let start_split = (*start_x_split as f32).max(1.0);
                let end_split = (*end_x_split as f32).max(1.0);

                // skyarea 的 X 语义为中心点：left/right 由中心点和宽度对称展开。
                let start_center = (*start_x as f32) / start_split;
                let end_center = (*end_x as f32) / end_split;
                let start_half = ((*start_width as f32) / start_split).abs() * 0.5;
                let end_half = ((*end_width as f32) / end_split).abs() * 0.5;

                let start_left = (start_center - start_half).clamp(0.0, 1.0);
                let start_right = (start_center + start_half).clamp(0.0, 1.0);
                let end_left = (end_center - end_half).clamp(0.0, 1.0);
                let end_right = (end_center + end_half).clamp(0.0, 1.0);

                let avg_width_norm = (((*start_width as f32) / start_split).abs()
                    + ((*end_width as f32) / end_split).abs())
                    * 0.5;
                notes.push(GroundNote {
                    id: next_id,
                    kind: GroundNoteKind::SkyArea,
                    lane: lane_from_normalized_x((start_center + end_center) * 0.5),
                    time_ms: *time as f32,
                    duration_ms: (*duration as f32).max(30.0),
                    width: normalized_width_to_air_ratio(avg_width_norm),
                    flick_right: true,
                    skyarea_shape: Some(SkyAreaShape {
                        start_left_norm: start_left,
                        start_right_norm: start_right,
                        end_left_norm: end_left,
                        end_right_norm: end_right,
                        left_ease: *left_ease,
                        right_ease: *right_ease,
                    }),
                });
                next_id += 1;
            }
            _ => {}
        }
    }

    notes.sort_by(|a, b| {
        a.time_ms
            .total_cmp(&b.time_ms)
            .then_with(|| a.lane.cmp(&b.lane))
            .then_with(|| a.id.cmp(&b.id))
    });
    notes
}

fn lane_from_normalized_x(norm_x: f32) -> usize {
    let central = (norm_x.clamp(0.0, 0.999_9) * 4.0).floor() as usize;
    (central + 1).min(LANE_COUNT - 1)
}

fn normalized_width_to_air_ratio(width_norm: f32) -> f32 {
    width_norm.abs().clamp(0.05, 1.0)
}

fn lane_to_air_x_norm(lane: usize) -> f32 {
    let lane4 = lane.clamp(1, 4);
    ((lane4 as f32) - 0.5) / 4.0
}

fn air_x_to_lane(x_norm: f32) -> usize {
    ((x_norm.clamp(0.0, 0.999_9) * 4.0).floor() as usize + 1).clamp(1, 4)
}

fn air_split_rect(rect: Rect) -> Rect {
    let lane_w = rect.w / LANE_COUNT as f32;
    Rect::new(rect.x + lane_w, rect.y, lane_w * 4.0, rect.h)
}

fn air_note_width(note: &GroundNote, total_width: f32) -> f32 {
    let width_norm = match note.kind {
        GroundNoteKind::Flick => note.width.clamp(0.05, 1.0),
        GroundNoteKind::SkyArea => note.width.clamp(0.05, 1.0),
        _ => note.width.clamp(0.05, 1.0),
    };
    width_norm * total_width
}
