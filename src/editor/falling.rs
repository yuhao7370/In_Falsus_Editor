use crate::chart::{Chart, ChartEvent, Ease, FlickType};
use macroquad::prelude::*;
use sasa::AudioClip;

const LANE_COUNT: usize = 6;
const REFERENCE_WIDTH: f32 = 1366.0;
const REFERENCE_HEIGHT: f32 = 768.0;
const DEFAULT_CHART_PATH: &str = "songs/alamode/alamode3.spc";
const DEFAULT_AIR_WIDTH_NORM: f32 = 0.5;
const DEFAULT_SKYAREA_WIDTH_NORM: f32 = 0.25;
const DEFAULT_SCROLL_SPEED: f32 = 1.25;
const MIN_SCROLL_SPEED: f32 = 0.2;
const MAX_SCROLL_SPEED: f32 = 4.0;
const SCROLL_SPEED_STEP: f32 = 0.1;
pub const SNAP_DIVISION_OPTIONS: [u32; 9] = [2, 3, 4, 6, 8, 12, 16, 24, 32];
const AIR_SKYAREA_HEAD_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.84);
const AIR_SKYAREA_BODY_COLOR: Color = Color::new(0.72, 0.60, 0.98, 0.42);
const AIR_SKYAREA_TAIL_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.34);
const DRAG_HOLD_TO_START_SEC: f64 = 0.22;
const SKYAREA_VERTICAL_DRAG_THRESHOLD_PX: f32 = 4.0;
const PORTRAIT_SCREEN_RATIO: f32 = 10.0 / 16.0;
const OVERLAP_CYCLE_ANCHOR_PX: f32 = 14.0;
const OVERLAP_DOUBLE_CLICK_SEC: f64 = 0.20;
const NOTE_HEAD_HIT_HALF_H: f32 = 9.0;
const NOTE_HEAD_HIT_PAD_X: f32 = 2.0;
const NOTE_BODY_HIT_PAD_X: f32 = 2.0;
const NOTE_BODY_EDGE_GAP_Y: f32 = 8.0;
const SELECTED_NOTE_DARKEN_ALPHA: u8 = 72;
const MINIMAP_DRAG_EMIT_EPS_SEC: f32 = 0.002;

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
        matches!(self.kind, GroundNoteKind::Hold | GroundNoteKind::SkyArea) && self.duration_ms > 0.0
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
    kind: BarLineKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarLineKind {
    Measure,
    Beat,
    Subdivision,
}

impl BarLineKind {
    fn priority(self) -> u8 {
        match self {
            Self::Measure => 3,
            Self::Beat => 2,
            Self::Subdivision => 1,
        }
    }
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

    fn visible_barlines(
        &self,
        current_ms: f32,
        ahead_ms: f32,
        behind_ms: f32,
        subdivision: u32,
    ) -> Vec<BarLine> {
        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;
        let mut output = Vec::new();
        let subdivision = subdivision.max(1);
        let subdivision_i = subdivision as i32;

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
            let sub_ms = beat_ms / subdivision as f32;
            let beats_per_measure = point.beats_per_measure.max(1.0);

            let n_start = ((visible_start - segment_start) / sub_ms).floor() as i32 - 2;
            let n_end = ((visible_end - segment_start) / sub_ms).ceil() as i32 + 2;

            for n in n_start..=n_end {
                if n < 0 {
                    continue;
                }
                let line_time_ms = segment_start + n as f32 * sub_ms;
                if line_time_ms < visible_start - 0.001 || line_time_ms > visible_end + 0.001 {
                    continue;
                }

                let beat = point.start_beat + n as f32 / subdivision as f32;
                let measure_phase = beat / beats_per_measure;
                let is_measure = (measure_phase - measure_phase.round()).abs() < 0.001;
                let is_beat = n % subdivision_i == 0;
                let kind = if is_measure {
                    BarLineKind::Measure
                } else if is_beat {
                    BarLineKind::Beat
                } else {
                    BarLineKind::Subdivision
                };

                output.push(BarLine { time_ms: line_time_ms, kind });
            }
        }

        output.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| b.kind.priority().cmp(&a.kind.priority()))
        });

        let mut deduped: Vec<BarLine> = Vec::with_capacity(output.len());
        for line in output {
            if let Some(last) = deduped.last_mut() {
                if (last.time_ms - line.time_ms).abs() < 0.001 {
                    if line.kind.priority() > last.kind.priority() {
                        last.kind = line.kind;
                    }
                    continue;
                }
            }
            deduped.push(line);
        }
        deduped
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
    start_time_sec: f64,
    start_mouse_x: f32,
    start_mouse_y: f32,
    sky_start_center_norm: f32,
    sky_end_center_norm: f32,
    sky_start_half_norm: f32,
    sky_end_half_norm: f32,
    air_target: AirDragTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AirDragTarget {
    Body,
    SkyHead,
    SkyTail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitScope {
    Ground,
    Air,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitPart {
    Head,
    Tail,
    Body,
}

#[derive(Debug, Clone, Copy)]
struct HitCandidate {
    note_id: u64,
    scope: HitScope,
    air_target: AirDragTarget,
    part: HitPart,
    distance_sq: f32,
    z_order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HitSignatureItem {
    note_id: u64,
    scope: HitScope,
    air_target: AirDragTarget,
    part: HitPart,
}

#[derive(Debug, Clone)]
struct OverlapCycleState {
    signature: Vec<HitSignatureItem>,
    current_index: usize,
    selected_item: HitSignatureItem,
    anchor_x: i32,
    anchor_y: i32,
    scope: HitScope,
    last_click_time_sec: f64,
    double_click_armed: bool,
}

#[derive(Debug, Clone, Copy)]
struct HoverOverlapHint {
    mouse_x: f32,
    mouse_y: f32,
    current_index: usize,
    total: usize,
}

#[derive(Debug, Clone, Copy)]
struct PendingHoldPlacement {
    lane: usize,
    start_time_ms: f32,
}

#[derive(Debug, Clone, Copy)]
struct PendingSkyAreaPlacement {
    start_time_ms: f32,
    start_center_norm: f32,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    time_ms: f32,
    label: String,
    color: Color,
}

#[derive(Debug, Clone, Copy)]
struct MinimapPageConfig {
    measures_per_page: u32,
    page_index: u32,
}

#[derive(Debug, Clone, Copy)]
struct TimeWindowMs {
    start_ms: f32,
    end_ms: f32,
    current_ms: f32,
}

#[derive(Debug, Clone, Copy)]
struct MinimapRenderInfo {
    content_rect: Rect,
    highlight_rect: Rect,
    seek_start_ms: f32,
    seek_end_ms: f32,
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
    pending_skyarea: Option<PendingSkyAreaPlacement>,
    overlap_cycle: Option<OverlapCycleState>,
    hover_overlap_hint: Option<HoverOverlapHint>,
    debug_show_hitboxes: bool,
    show_minimap: bool,
    waveform: Option<Waveform>,
    waveform_error: Option<String>,
    waveform_seek_active: bool,
    waveform_seek_sec: f32,
    minimap_drag_active: bool,
    minimap_drag_offset_ms: f32,
    minimap_last_emit_sec: Option<f32>,
    minimap_drag_target_sec: Option<f32>,
    minimap_page: Option<MinimapPageConfig>,
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
            pending_skyarea: None,
            overlap_cycle: None,
            hover_overlap_hint: None,
            debug_show_hitboxes: false,
            show_minimap: false,
            waveform: None,
            waveform_error: None,
            waveform_seek_active: false,
            waveform_seek_sec: 0.0,
            minimap_drag_active: false,
            minimap_drag_offset_ms: 0.0,
            minimap_last_emit_sec: None,
            minimap_drag_target_sec: None,
            minimap_page: None,
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
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.minimap_drag_active = false;
        self.minimap_drag_target_sec = None;
        self.minimap_last_emit_sec = None;
        self.status = format!("render scope: {}", scope.label());
    }

    pub fn set_place_note_type(&mut self, note_type: Option<PlaceNoteType>) {
        self.place_note_type = note_type;
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.status = match note_type {
            Some(kind) => format!("place mode: {}", kind.label()),
            None => "place mode cleared".to_owned(),
        };
    }

    pub fn snap_division(&self) -> u32 {
        self.snap_division
    }

    pub fn set_snap_division(&mut self, division: u32) {
        if SNAP_DIVISION_OPTIONS.contains(&division) {
            self.snap_division = division;
            self.status = format!("snap division: {}x", division);
        }
    }

    pub fn pending_hold_head_time_ms(&self) -> Option<f32> {
        self.pending_hold.map(|pending| pending.start_time_ms)
    }

    pub fn pending_skyarea_head_time_ms(&self) -> Option<f32> {
        self.pending_skyarea.map(|pending| pending.start_time_ms)
    }

    pub fn debug_show_hitboxes(&self) -> bool {
        self.debug_show_hitboxes
    }

    pub fn set_debug_show_hitboxes(&mut self, enabled: bool) {
        self.debug_show_hitboxes = enabled;
        self.status = format!("debug hitbox {}", if enabled { "on" } else { "off" });
    }

    pub fn show_minimap(&self) -> bool {
        self.show_minimap
    }

    pub fn set_show_minimap(&mut self, enabled: bool) {
        self.show_minimap = enabled;
        if !enabled {
            self.minimap_drag_active = false;
            self.minimap_drag_offset_ms = 0.0;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
        }
    }

    pub fn draw(
        &mut self,
        area: Rect,
        current_sec: f32,
        audio_duration_sec: f32,
        audio_path: Option<&str>,
        is_playing: bool,
    ) -> Vec<FallingEditorAction> {
        self.sync_waveform(audio_path);
        let mut actions = Vec::new();

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
        let minimap_screen = if self.show_minimap {
            self.minimap_screen_from_left_gap(content_rect, left_screen)
        } else {
            None
        };

        let inner_rect = |screen: Rect| {
            Rect::new(
                screen.x + 8.0,
                screen.y + 8.0,
                (screen.w - 16.0).max(8.0),
                (screen.h - 16.0).max(8.0),
            )
        };
        let minimap_inner = minimap_screen.map(inner_rect);
        let left_inner = inner_rect(left_screen);
        let right_inner = inner_rect(right_screen);

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

        if let Some(screen) = minimap_screen {
            draw_rectangle(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                Color::from_rgba(12, 12, 18, 255),
            );
            draw_rectangle_lines(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                1.0,
                Color::from_rgba(56, 62, 86, 255),
            );
        }
        for screen in [left_screen, right_screen] {
            draw_rectangle(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                Color::from_rgba(12, 12, 18, 255),
            );
            draw_rectangle_lines(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                1.0,
                Color::from_rgba(56, 62, 86, 255),
            );
        }

        self.draw_header(header_rect);
        self.handle_scroll_speed_controls(header_rect);
        self.handle_vertical_progress_seek(progress_rect, audio_duration_sec, is_playing, &mut actions);
        let duration_sec = self.estimate_duration(audio_duration_sec).max(0.001);
        let mut render_current_sec = if self.waveform_seek_active {
            self.waveform_seek_sec
                .clamp(0.0, duration_sec)
        } else {
            current_sec
        };
        if let Some(target_sec) = self.minimap_drag_target_sec {
            render_current_sec = target_sec.clamp(0.0, duration_sec);
        }
        let mut current_ms = render_current_sec * 1000.0;

        let visible_window = self.compute_visible_window_ms(lanes_rect, current_ms);
        if let Some(minimap_inner) = minimap_inner {
            let minimap_info = self.draw_minimap_view(minimap_inner, duration_sec, visible_window);
            self.handle_minimap_seek_drag(
                minimap_info,
                current_ms,
                duration_sec,
                is_playing,
                &mut actions,
            );
        } else {
            self.minimap_drag_active = false;
            self.minimap_drag_offset_ms = 0.0;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
        }
        if let Some(target_sec) = self.minimap_drag_target_sec {
            render_current_sec = target_sec.clamp(0.0, duration_sec);
            current_ms = render_current_sec * 1000.0;
        }
        self.draw_vertical_progress(progress_rect, render_current_sec, duration_sec);

        let (ground_rect, air_rect) = match self.render_scope {
            RenderScope::Both => {
                self.draw_event_view(left_inner, current_ms);
                (Some(lanes_rect), Some(lanes_rect))
            }
            RenderScope::Split => (Some(left_inner), Some(lanes_rect)),
        };

        let allow_editor_input = !self.minimap_drag_active;
        if allow_editor_input {
            if is_mouse_button_pressed(MouseButton::Right)
                && (self.place_note_type.is_some()
                    || self.pending_hold.is_some()
                    || self.pending_skyarea.is_some())
            {
                self.place_note_type = None;
                self.pending_hold = None;
                self.pending_skyarea = None;
                self.drag_state = None;
                self.overlap_cycle = None;
                self.hover_overlap_hint = None;
                self.status = "place mode cleared".to_owned();
            }

            if self.place_note_type.is_none() {
                self.handle_note_selection_click(ground_rect, air_rect, current_ms);
                self.update_hover_overlap_hint(ground_rect, air_rect, current_ms);
            } else {
                self.overlap_cycle = None;
                self.hover_overlap_hint = None;
            }
        } else {
            self.drag_state = None;
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
        }

        if allow_editor_input {
            if let Some(rect) = ground_rect {
                self.handle_ground_input(rect, current_ms);
            }
            if let Some(rect) = air_rect {
                self.handle_air_input(rect, current_ms);
            }
        }

        match self.render_scope {
            RenderScope::Both => {
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, true);
                }
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, true, false);
                }
            }
            RenderScope::Split => {
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, false, true);
                }
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, true);
                }
            }
        }

        let (mx, my) = mouse_position();
        let using_note_cursor = if allow_editor_input {
            match self.place_note_type {
                Some(tool) if is_ground_tool(tool) => {
                    ground_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false)
                }
                Some(tool) if is_air_tool(tool) => {
                    air_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false)
                }
                _ => false,
            }
        } else {
            false
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

        self.draw_overlap_hint();

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

    fn minimap_screen_from_left_gap(&self, content_rect: Rect, left_screen: Rect) -> Option<Rect> {
        let gap = (content_rect.w * 0.008).clamp(2.0, 6.0);
        let available_w = left_screen.x - content_rect.x - gap;
        if available_w < 34.0 {
            return None;
        }
        Some(Rect::new(
            content_rect.x,
            left_screen.y,
            available_w,
            left_screen.h,
        ))
    }

    fn compute_visible_window_ms(&self, render_rect: Rect, current_ms: f32) -> TimeWindowMs {
        if render_rect.h <= 1.0 {
            return TimeWindowMs {
                start_ms: current_ms.max(0.0),
                end_ms: current_ms.max(0.0),
                current_ms: current_ms.max(0.0),
            };
        }
        let judge_y = render_rect.y + render_rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * render_rect.h).max(1.0);
        let ahead_ms = ((judge_y - render_rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((render_rect.y + render_rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);
        let start_ms = (current_ms - behind_ms).max(0.0);
        let end_ms = (current_ms + ahead_ms).max(start_ms);
        TimeWindowMs {
            start_ms,
            end_ms,
            current_ms: current_ms.max(0.0),
        }
    }

    fn minimap_segment_time_to_y(
        &self,
        time_ms: f32,
        rect: Rect,
        seg_start_ms: f32,
        seg_end_ms: f32,
    ) -> f32 {
        let span = (seg_end_ms - seg_start_ms).max(0.001);
        let t = ((time_ms - seg_start_ms) / span).clamp(0.0, 1.0);
        rect.y + rect.h * (1.0 - t)
    }

    fn minimap_segment_y_to_time(
        &self,
        y: f32,
        rect: Rect,
        seg_start_ms: f32,
        seg_end_ms: f32,
    ) -> f32 {
        let span = (seg_end_ms - seg_start_ms).max(0.001);
        let t = ((y - rect.y) / rect.h.max(0.001)).clamp(0.0, 1.0);
        (1.0 - t) * span + seg_start_ms
    }

    fn draw_minimap_view(
        &self,
        rect: Rect,
        duration_sec: f32,
        visible: TimeWindowMs,
    ) -> MinimapRenderInfo {
        let duration_ms = (duration_sec.max(0.001)) * 1000.0;
        if rect.h <= 6.0 || rect.w <= 6.0 {
            return MinimapRenderInfo {
                content_rect: rect,
                highlight_rect: rect,
                seek_start_ms: 0.0,
                seek_end_ms: duration_ms,
            };
        }

        let ui = adaptive_ui_scale();
        let title_h = 20.0 * ui;
        let pad = 6.0 * ui;

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(8, 10, 17, 255));
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(42, 58, 90, 255),
        );
        draw_text_ex(
            "MINIMAP",
            rect.x + 8.0 * ui,
            rect.y + 16.0 * ui,
            TextParams {
                font_size: scaled_font_size(16.0, 11, 36),
                color: Color::from_rgba(210, 222, 246, 255),
                ..Default::default()
            },
        );

        let content = Rect::new(
            rect.x + pad,
            rect.y + title_h,
            (rect.w - pad * 2.0).max(4.0),
            (rect.h - title_h - pad).max(4.0),
        );
        draw_rectangle(
            content.x,
            content.y,
            content.w,
            content.h,
            Color::from_rgba(10, 14, 24, 255),
        );
        draw_rectangle_lines(
            content.x,
            content.y,
            content.w,
            content.h,
            1.0,
            Color::from_rgba(52, 72, 108, 255),
        );

        if content.h <= 2.0 || content.w <= 2.0 {
            return MinimapRenderInfo {
                content_rect: content,
                highlight_rect: content,
                seek_start_ms: 0.0,
                seek_end_ms: duration_ms,
            };
        }

        let half_ms = duration_ms * 0.5;
        let pair_gap = (2.0 * ui).clamp(1.0, 5.0);
        let group_gap = (5.0 * ui).clamp(3.0, 10.0);
        let total_gap = pair_gap * 2.0 + group_gap;
        let col_w = ((content.w - total_gap) / 4.0).max(2.0);

        let ground_rect_1 = Rect::new(content.x, content.y, col_w, content.h);
        let sky_rect_1 = Rect::new(ground_rect_1.x + col_w + pair_gap, content.y, col_w, content.h);
        let ground_rect_2 = Rect::new(sky_rect_1.x + col_w + group_gap, content.y, col_w, content.h);
        let sky_rect_2 = Rect::new(ground_rect_2.x + col_w + pair_gap, content.y, col_w, content.h);

        let left_group_rect = Rect::new(
            ground_rect_1.x,
            ground_rect_1.y,
            (sky_rect_1.x + sky_rect_1.w - ground_rect_1.x).max(1.0),
            ground_rect_1.h,
        );
        let right_group_rect = Rect::new(
            ground_rect_2.x,
            ground_rect_2.y,
            (sky_rect_2.x + sky_rect_2.w - ground_rect_2.x).max(1.0),
            ground_rect_2.h,
        );
        let active_right = visible.current_ms >= half_ms;
        let (active_group_rect, active_start_ms, active_end_ms) = if active_right {
            (right_group_rect, half_ms, duration_ms)
        } else {
            (left_group_rect, 0.0, half_ms)
        };

        for (g_rect, a_rect, g_label, a_label) in [
            (ground_rect_1, sky_rect_1, "G1", "A1"),
            (ground_rect_2, sky_rect_2, "G2", "A2"),
        ] {
            draw_rectangle(
                g_rect.x,
                g_rect.y,
                g_rect.w,
                g_rect.h,
                Color::from_rgba(12, 18, 28, 188),
            );
            draw_rectangle(
                a_rect.x,
                a_rect.y,
                a_rect.w,
                a_rect.h,
                Color::from_rgba(18, 14, 30, 188),
            );
            draw_rectangle_lines(
                g_rect.x,
                g_rect.y,
                g_rect.w,
                g_rect.h,
                1.0,
                Color::from_rgba(62, 86, 118, 144),
            );
            draw_rectangle_lines(
                a_rect.x,
                a_rect.y,
                a_rect.w,
                a_rect.h,
                1.0,
                Color::from_rgba(94, 84, 138, 144),
            );
            draw_text_ex(
                g_label,
                g_rect.x + 2.0 * ui,
                g_rect.y + 12.0 * ui,
                TextParams {
                    font_size: scaled_font_size(10.0, 8, 20),
                    color: Color::from_rgba(186, 216, 245, 196),
                    ..Default::default()
                },
            );
            draw_text_ex(
                a_label,
                a_rect.x + 2.0 * ui,
                a_rect.y + 12.0 * ui,
                TextParams {
                    font_size: scaled_font_size(10.0, 8, 20),
                    color: Color::from_rgba(214, 188, 246, 196),
                    ..Default::default()
                },
            );

            let ground_lane_w = g_rect.w / LANE_COUNT as f32;
            for lane in 1..LANE_COUNT {
                let x = g_rect.x + lane as f32 * ground_lane_w;
                draw_line(
                    x,
                    g_rect.y,
                    x,
                    g_rect.y + g_rect.h,
                    1.0,
                    Color::from_rgba(42, 56, 84, 132),
                );
            }
            for lane in 1..4 {
                let x = a_rect.x + lane as f32 * (a_rect.w / 4.0);
                draw_line(
                    x,
                    a_rect.y,
                    x,
                    a_rect.y + a_rect.h,
                    1.0,
                    Color::from_rgba(72, 64, 108, 128),
                );
            }
        }

        // Measure / beat / subdivision lines per half page.
        for (group_rect, page_start_ms, page_end_ms) in [
            (left_group_rect, 0.0_f32, half_ms.max(0.001)),
            (right_group_rect, half_ms, duration_ms.max(half_ms + 0.001)),
        ] {
            let center_ms = (page_start_ms + page_end_ms) * 0.5;
            let ahead_ms = (page_end_ms - center_ms).max(0.0);
            let behind_ms = (center_ms - page_start_ms).max(0.0);
            for barline in self.timeline.visible_barlines(center_ms, ahead_ms, behind_ms, 16) {
                if barline.time_ms < page_start_ms || barline.time_ms > page_end_ms {
                    continue;
                }
                let y = self.minimap_segment_time_to_y(
                    barline.time_ms,
                    group_rect,
                    page_start_ms,
                    page_end_ms,
                );
                let (thickness, color) = match barline.kind {
                    BarLineKind::Measure => (1.3 * ui, Color::from_rgba(168, 190, 236, 170)),
                    BarLineKind::Beat => (1.0 * ui, Color::from_rgba(108, 128, 170, 124)),
                    BarLineKind::Subdivision => (0.8 * ui, Color::from_rgba(76, 96, 132, 92)),
                };
                draw_line(
                    group_rect.x,
                    y,
                    group_rect.x + group_rect.w,
                    y,
                    thickness.max(1.0),
                    color,
                );
            }
        }

        // Notes compressed over full duration (rect/strip based, no circle markers).
        let thin = (1.05 * ui).max(1.0);
        let head_h = (2.8 * ui).clamp(1.0, 5.0);
        let flick_tip_h = (head_h * 0.35).max(0.8);
        for note in &self.notes {
            let note_time = note.time_ms.max(0.0);
            let on_right = note_time >= half_ms;
            let (ground_rect, sky_rect, page_start_ms, page_end_ms) = if on_right {
                (ground_rect_2, sky_rect_2, half_ms, duration_ms.max(half_ms + 0.001))
            } else {
                (ground_rect_1, sky_rect_1, 0.0_f32, half_ms.max(0.001))
            };
            let y_head = self.minimap_segment_time_to_y(
                note_time,
                ground_rect,
                page_start_ms,
                page_end_ms,
            );
            let lane_palette = lane_note_palette(note.lane.clamp(0, LANE_COUNT - 1));
            let ground_lane_w = ground_rect.w / LANE_COUNT as f32;
            let page_duration_ms = (page_end_ms - page_start_ms).max(0.001);
            match note.kind {
                GroundNoteKind::Tap => {
                    let lane_x = ground_rect.x + ground_lane_w * note.lane as f32;
                    let note_w = (ground_lane_w * 0.74).max(1.0);
                    let note_x = lane_x + (ground_lane_w - note_w) * 0.5;
                    draw_rectangle(note_x, y_head - head_h * 0.5, note_w, head_h, lane_palette.tap);
                }
                GroundNoteKind::Hold => {
                    let note_end = note.end_time_ms();
                    for (g_rect, start_ms, end_ms) in [
                        (ground_rect_1, 0.0_f32, half_ms.max(0.001)),
                        (ground_rect_2, half_ms, duration_ms.max(half_ms + 0.001)),
                    ] {
                        if note_end < start_ms || note_time > end_ms {
                            continue;
                        }
                        let lane_w = g_rect.w / LANE_COUNT as f32;
                        let lane_x = g_rect.x + lane_w * note.lane as f32;
                        let head_w = (lane_w * 0.82).max(1.0);
                        let head_x = lane_x + (lane_w - head_w) * 0.5;
                        let body_start = note_time.max(start_ms);
                        let body_end = note_end.min(end_ms);
                        let y0 =
                            self.minimap_segment_time_to_y(body_start, g_rect, start_ms, end_ms);
                        let y1 =
                            self.minimap_segment_time_to_y(body_end, g_rect, start_ms, end_ms);
                        let body_w = (head_w * 0.56).max(1.0);
                        let body_x = head_x + (head_w - body_w) * 0.5;
                        draw_rectangle(body_x, y0.min(y1), body_w, (y1 - y0).abs().max(1.0), lane_palette.hold_body);

                        if note_time >= start_ms && note_time <= end_ms {
                            let y_head_local =
                                self.minimap_segment_time_to_y(note_time, g_rect, start_ms, end_ms);
                            draw_rectangle(
                                head_x,
                                y_head_local - head_h * 0.55,
                                head_w,
                                head_h * 1.1,
                                lane_palette.hold_head,
                            );
                        }
                        if note_end >= start_ms && note_end <= end_ms {
                            let y_tail_local =
                                self.minimap_segment_time_to_y(note_end, g_rect, start_ms, end_ms);
                            draw_rectangle(
                                head_x,
                                y_tail_local - head_h * 0.45,
                                head_w,
                                head_h * 0.9,
                                Color::from_rgba(
                                    (lane_palette.hold_head.r * 255.0) as u8,
                                    (lane_palette.hold_head.g * 255.0) as u8,
                                    (lane_palette.hold_head.b * 255.0) as u8,
                                    190,
                                ),
                            );
                        }
                    }
                }
                GroundNoteKind::Flick => {
                    let center_x = sky_rect.x + lane_to_air_x_norm(note.lane.clamp(1, 4)) * sky_rect.w;
                    let air_lane_w = sky_rect.w / 4.0;
                    let note_w = air_note_width(note, sky_rect.w).clamp(air_lane_w * 0.22, air_lane_w * 0.98);
                    let flick_color = if note.flick_right {
                        Color::from_rgba(112, 228, 156, 230)
                    } else {
                        Color::from_rgba(246, 232, 122, 230)
                    };
                    let bpm = self.timeline.point_at_time(note_time).bpm.abs().max(0.001);
                    let subdiv_ms = 60_000.0 / bpm / 16.0;
                    let side_h = (subdiv_ms / page_duration_ms * sky_rect.h).max(head_h);
                    let side_x = if note.flick_right {
                        center_x + note_w * 0.46
                    } else {
                        center_x - note_w * 0.46
                    };
                    let tip_x = if note.flick_right {
                        center_x - note_w * 0.52
                    } else {
                        center_x + note_w * 0.52
                    };
                    let y_bottom = self.minimap_segment_time_to_y(
                        note_time,
                        sky_rect,
                        page_start_ms,
                        page_end_ms,
                    );
                    let y_top = y_bottom - side_h;
                    let y_tip_top = y_bottom - flick_tip_h;
                    let mut top_curve = Vec::with_capacity(17);
                    for i in 0..=16 {
                        let t = i as f32 / 16.0;
                        let eased = ease_progress(Ease::SineOut, t);
                        let x = lerp(side_x, tip_x, t);
                        let y = lerp(y_top, y_tip_top, eased);
                        top_curve.push(Vec2::new(x, y));
                    }
                    let mut polygon = Vec::with_capacity(22);
                    polygon.push(Vec2::new(side_x, y_bottom));
                    polygon.extend_from_slice(&top_curve);
                    polygon.push(Vec2::new(tip_x, y_bottom));
                    for i in 1..(polygon.len() - 1) {
                        draw_triangle(
                            polygon[0],
                            polygon[i],
                            polygon[i + 1],
                            Color::new(flick_color.r, flick_color.g, flick_color.b, 0.52),
                        );
                    }
                    for i in 0..(top_curve.len() - 1) {
                        let a = top_curve[i];
                        let b = top_curve[i + 1];
                        draw_line(a.x, a.y, b.x, b.y, thin, flick_color);
                    }
                    draw_line(side_x, y_bottom, tip_x, y_bottom, thin, flick_color);
                    draw_line(side_x, y_bottom, side_x, y_top, thin, flick_color);
                }
                GroundNoteKind::SkyArea => {
                    let note_end = note.end_time_ms();
                    for (s_rect, start_ms, end_ms) in [
                        (sky_rect_1, 0.0_f32, half_ms.max(0.001)),
                        (sky_rect_2, half_ms, duration_ms.max(half_ms + 0.001)),
                    ] {
                        if note_end < start_ms || note_time > end_ms {
                            continue;
                        }
                        if let Some(shape) = note.skyarea_shape {
                            let inter_start = note_time.max(start_ms);
                            let inter_end = note_end.min(end_ms);
                            if inter_end > inter_start + 0.000_1 && note.duration_ms > 0.0 {
                                let seg_count = 20;
                                for i in 0..seg_count {
                                    let s0 = i as f32 / seg_count as f32;
                                    let s1 = (i + 1) as f32 / seg_count as f32;
                                    let t0 = lerp(inter_start, inter_end, s0);
                                    let t1 = lerp(inter_start, inter_end, s1);
                                    let p0 = ((t0 - note_time) / note.duration_ms).clamp(0.0, 1.0);
                                    let p1 = ((t1 - note_time) / note.duration_ms).clamp(0.0, 1.0);

                                    let y0 =
                                        self.minimap_segment_time_to_y(t0, s_rect, start_ms, end_ms);
                                    let y1 =
                                        self.minimap_segment_time_to_y(t1, s_rect, start_ms, end_ms);
                                    let l0 = lerp(
                                        shape.start_left_norm,
                                        shape.end_left_norm,
                                        ease_progress(shape.left_ease, p0),
                                    )
                                    .clamp(0.0, 1.0);
                                    let r0 = lerp(
                                        shape.start_right_norm,
                                        shape.end_right_norm,
                                        ease_progress(shape.right_ease, p0),
                                    )
                                    .clamp(0.0, 1.0);
                                    let l1 = lerp(
                                        shape.start_left_norm,
                                        shape.end_left_norm,
                                        ease_progress(shape.left_ease, p1),
                                    )
                                    .clamp(0.0, 1.0);
                                    let r1 = lerp(
                                        shape.start_right_norm,
                                        shape.end_right_norm,
                                        ease_progress(shape.right_ease, p1),
                                    )
                                    .clamp(0.0, 1.0);

                                    let lx0 = s_rect.x + l0 * s_rect.w;
                                    let rx0 = s_rect.x + r0 * s_rect.w;
                                    let lx1 = s_rect.x + l1 * s_rect.w;
                                    let rx1 = s_rect.x + r1 * s_rect.w;
                                    draw_triangle(
                                        Vec2::new(lx0, y0),
                                        Vec2::new(rx0, y0),
                                        Vec2::new(rx1, y1),
                                        Color::new(
                                            AIR_SKYAREA_BODY_COLOR.r,
                                            AIR_SKYAREA_BODY_COLOR.g,
                                            AIR_SKYAREA_BODY_COLOR.b,
                                            0.30,
                                        ),
                                    );
                                    draw_triangle(
                                        Vec2::new(lx0, y0),
                                        Vec2::new(rx1, y1),
                                        Vec2::new(lx1, y1),
                                        Color::new(
                                            AIR_SKYAREA_BODY_COLOR.r,
                                            AIR_SKYAREA_BODY_COLOR.g,
                                            AIR_SKYAREA_BODY_COLOR.b,
                                            0.30,
                                        ),
                                    );
                                }
                            }

                            if note_time >= start_ms && note_time <= end_ms {
                                let y_head_local =
                                    self.minimap_segment_time_to_y(note_time, s_rect, start_ms, end_ms);
                                let head_left =
                                    s_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * s_rect.w;
                                let head_right =
                                    s_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * s_rect.w;
                                draw_rectangle(
                                    head_left,
                                    y_head_local - head_h * 0.5,
                                    (head_right - head_left).max(1.0),
                                    head_h,
                                    AIR_SKYAREA_HEAD_COLOR,
                                );
                            }
                            if note_end >= start_ms && note_end <= end_ms {
                                let y_tail_local =
                                    self.minimap_segment_time_to_y(note_end, s_rect, start_ms, end_ms);
                                let tail_left =
                                    s_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * s_rect.w;
                                let tail_right =
                                    s_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * s_rect.w;
                                draw_rectangle(
                                    tail_left,
                                    y_tail_local - head_h * 0.5,
                                    (tail_right - tail_left).max(1.0),
                                    head_h,
                                    AIR_SKYAREA_TAIL_COLOR,
                                );
                            }
                        } else {
                            let x = s_rect.x + lane_to_air_x_norm(note.lane.clamp(1, 4)) * s_rect.w;
                            let y_head_local =
                                self.minimap_segment_time_to_y(note_time, s_rect, start_ms, end_ms);
                            let y_tail_local =
                                self.minimap_segment_time_to_y(note_end, s_rect, start_ms, end_ms);
                            let head_w = (s_rect.w / 4.0 * 0.64).max(1.0);
                            draw_rectangle(
                                x - head_w * 0.5,
                                y_head_local - head_h * 0.5,
                                head_w,
                                head_h,
                                AIR_SKYAREA_HEAD_COLOR,
                            );
                            draw_rectangle(
                                x - head_w * 0.5,
                                y_tail_local - head_h * 0.5,
                                head_w,
                                head_h,
                                AIR_SKYAREA_TAIL_COLOR,
                            );
                            draw_line(x, y_head_local, x, y_tail_local, thin, AIR_SKYAREA_BODY_COLOR);
                        }
                    }
                }
            }
        }

        // Highlight current visible window on both halves.
        let mut active_highlight = Rect::new(
            active_group_rect.x,
            active_group_rect.y,
            active_group_rect.w,
            (2.0 * ui).max(1.0),
        );
        for (group_rect, page_start_ms, page_end_ms) in [
            (left_group_rect, 0.0_f32, half_ms.max(0.001)),
            (right_group_rect, half_ms, duration_ms.max(half_ms + 0.001)),
        ] {
            let overlap_start = visible.start_ms.max(page_start_ms).min(page_end_ms);
            let overlap_end = visible.end_ms.max(page_start_ms).min(page_end_ms);
            if overlap_end < overlap_start {
                continue;
            }
            let y_top =
                self.minimap_segment_time_to_y(overlap_end, group_rect, page_start_ms, page_end_ms);
            let y_bottom = self.minimap_segment_time_to_y(
                overlap_start,
                group_rect,
                page_start_ms,
                page_end_ms,
            );
            let highlight_h = (y_bottom - y_top).abs().max((2.0 * ui).max(1.0));
            let highlight = Rect::new(group_rect.x, y_top.min(y_bottom), group_rect.w, highlight_h);
            draw_rectangle(
                highlight.x,
                highlight.y,
                highlight.w,
                highlight.h,
                Color::from_rgba(255, 255, 255, 28),
            );
            draw_rectangle_lines(
                highlight.x,
                highlight.y,
                highlight.w,
                highlight.h,
                (1.2 * ui).max(1.0),
                Color::from_rgba(255, 255, 255, 214),
            );
            if (page_start_ms - active_start_ms).abs() < 0.5 {
                active_highlight = highlight;
            }
        }

        let current_y = self.minimap_segment_time_to_y(
            visible.current_ms.clamp(active_start_ms, active_end_ms),
            active_group_rect,
            active_start_ms,
            active_end_ms,
        );
        draw_line(
            active_group_rect.x,
            current_y,
            active_group_rect.x + active_group_rect.w,
            current_y,
            (1.0 * ui).max(1.0),
            Color::from_rgba(255, 238, 204, 182),
        );

        MinimapRenderInfo {
            content_rect: active_group_rect,
            highlight_rect: active_highlight,
            seek_start_ms: active_start_ms,
            seek_end_ms: active_end_ms,
        }
    }

    fn handle_minimap_seek_drag(
        &mut self,
        minimap: MinimapRenderInfo,
        render_current_ms: f32,
        duration_sec: f32,
        is_playing: bool,
        actions: &mut Vec<FallingEditorAction>,
    ) {
        let duration_ms = duration_sec.max(0.001) * 1000.0;
        if minimap.content_rect.w <= 2.0 || minimap.content_rect.h <= 2.0 {
            self.minimap_drag_active = false;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
            return;
        }

        let (mx, my) = mouse_position();
        let ui = adaptive_ui_scale();
        let min_hit_h = (26.0 * ui).max(minimap.highlight_rect.h);
        let cy = minimap.highlight_rect.y + minimap.highlight_rect.h * 0.5;
        let hit_top = (cy - min_hit_h * 0.5).max(minimap.content_rect.y);
        let hit_bottom = (cy + min_hit_h * 0.5).min(minimap.content_rect.y + minimap.content_rect.h);
        let hit_pad_x = (8.0 * ui).max(4.0);
        let hit_rect = Rect::new(
            minimap.content_rect.x - hit_pad_x,
            hit_top,
            minimap.content_rect.w + hit_pad_x * 2.0,
            (hit_bottom - hit_top).max(1.0),
        );
        let inside_highlight = point_in_rect(mx, my, hit_rect);

        if is_mouse_button_pressed(MouseButton::Left) && inside_highlight {
            if is_playing {
                self.status = "pause to scrub minimap".to_owned();
                return;
            }
            self.drag_state = None;
            self.waveform_seek_active = false;
            self.minimap_drag_active = true;
            let mouse_ms = self.minimap_segment_y_to_time(
                my,
                minimap.content_rect,
                minimap.seek_start_ms,
                minimap.seek_end_ms,
            );
            self.minimap_drag_offset_ms = render_current_ms - mouse_ms;
            self.minimap_last_emit_sec = None;
        }

        if !self.minimap_drag_active {
            return;
        }

        if is_playing {
            self.minimap_drag_active = false;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
            self.status = "pause to scrub minimap".to_owned();
            return;
        }

        if is_mouse_button_down(MouseButton::Left) {
            let mouse_ms = self.minimap_segment_y_to_time(
                my,
                minimap.content_rect,
                minimap.seek_start_ms,
                minimap.seek_end_ms,
            );
            let target_ms = (mouse_ms + self.minimap_drag_offset_ms).clamp(0.0, duration_ms);
            let target_sec = target_ms / 1000.0;
            self.minimap_drag_target_sec = Some(target_sec);
            self.waveform_seek_sec = target_sec;

            let should_emit = self
                .minimap_last_emit_sec
                .map(|last| (last - target_sec).abs() >= MINIMAP_DRAG_EMIT_EPS_SEC)
                .unwrap_or(true);
            if should_emit {
                actions.push(FallingEditorAction::SeekTo(target_sec));
                self.minimap_last_emit_sec = Some(target_sec);
                self.status = format!("minimap seek {:.2}s", target_sec);
            }
        } else {
            self.minimap_drag_active = false;
            self.minimap_drag_offset_ms = 0.0;
            self.minimap_last_emit_sec = None;
            self.minimap_drag_target_sec = None;
        }
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

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 22.0 || y > rect.y + rect.h + 1.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (1.5, Color::from_rgba(102, 134, 180, 180)),
                BarLineKind::Beat => (1.1, Color::from_rgba(78, 104, 146, 152)),
                BarLineKind::Subdivision => (0.8, Color::from_rgba(58, 78, 112, 112)),
            };
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
                "Falling | chart={} | G:{} A:{} | view={} | tool={} | snap={} {}x | speed={:.2}H/s | hitbox={}",
                self.chart_path,
                ground_count,
                air_count,
                self.render_scope.label(),
                self.place_note_type
                    .map(PlaceNoteType::label)
                    .unwrap_or("None"),
                if self.snap_enabled { "on" } else { "off" },
                self.snap_division,
                self.scroll_speed,
                if self.debug_show_hitboxes { "on" } else { "off" }
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
                rect.y + rect.h - fill_h,
                (rect.w - 4.0).max(1.0),
                fill_h,
                Color::from_rgba(74, 134, 210, 165),
            );
        }

        let playhead_y = rect.y + rect.h - progress * rect.h;
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
            let seek_y = rect.y + rect.h - seek_progress * rect.h;
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

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (2.1, Color::from_rgba(170, 205, 255, 210)),
                BarLineKind::Beat => (1.3, Color::from_rgba(112, 148, 192, 186)),
                BarLineKind::Subdivision => (0.9, Color::from_rgba(80, 108, 142, 142)),
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
                    if selected {
                        draw_selected_note_darken_rect(body_x, body_y, body_w, body_h);
                    }
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
                    draw_selected_note_darken_rect(note_x, head_y - 8.0, note_w, 16.0);
                    draw_selected_note_outline(note_x, head_y - 8.0, note_w, 16.0);
                }
            }
        }

        if self.debug_show_hitboxes {
            self.draw_ground_hitbox_overlay(rect, current_ms);
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

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (2.1, Color::from_rgba(164, 198, 255, 210)),
                BarLineKind::Beat => (1.3, Color::from_rgba(108, 140, 186, 182)),
                BarLineKind::Subdivision => (0.9, Color::from_rgba(74, 102, 136, 140)),
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
                    if selected {
                        draw_selected_note_darken_rect(note_x, body_y, note_w, body_h);
                    }
                }
            }

            if head_y >= rect.y - 24.0 && head_y <= rect.y + rect.h + 24.0 {
                if note.kind == GroundNoteKind::Flick {
                    let side_h = self.flick_side_height_px(note.time_ms, rect.h);
                    draw_flick_curve_shape(note, note_x, note_w, head_y, side_h);
                    if selected {
                        let bounds = flick_shape_bounds(note, note_x, note_w, head_y, side_h);
                        draw_selected_note_darken_rect(bounds.x, bounds.y, bounds.w, bounds.h);
                        draw_selected_note_outline(bounds.x, bounds.y, bounds.w, bounds.h);
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
                    draw_selected_note_darken_rect(note_x, head_y - 8.0, note_w, 16.0);
                    draw_selected_note_outline(note_x, head_y - 8.0, note_w, 16.0);
                    }
                }
            }
        }

        if self.debug_show_hitboxes {
            self.draw_air_hitbox_overlay(rect, current_ms);
        }
    }

    fn draw_ground_hitbox_overlay(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let head_color = Color::from_rgba(84, 230, 255, 232);
        let tail_color = Color::from_rgba(255, 164, 88, 228);
        let body_color = Color::from_rgba(138, 255, 152, 218);

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let lane_x = rect.x + lane_w * note.lane as f32;
            let note_w = note_head_width(note, lane_w);
            let note_x = lane_x + (lane_w - note_w) * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let head_rect = note_end_hit_rect(note_x, note_w, head_y);
            draw_debug_hitbox_rect(head_rect, rect, head_color, 1.3);

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let tail_rect = note_end_hit_rect(note_x, note_w, tail_y);
                draw_debug_hitbox_rect(tail_rect, rect, tail_color, 1.3);

                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let (body_x, body_w) = match note.kind {
                    GroundNoteKind::Hold => (note_x + note_w * 0.04, note_w * 0.92),
                    GroundNoteKind::SkyArea => (note_x + note_w * 0.02, note_w * 0.96),
                    _ => (note_x + note_w * 0.34, note_w * 0.32),
                };
                let body_rect = note_body_hit_rect(body_x, body_w, y1, y2);
                draw_debug_hitbox_rect(body_rect, rect, body_color, 1.2);
            }

            let label = format!("#{}", note.id);
            draw_debug_hitbox_label(head_rect, rect, &label, head_color);
        }
    }

    fn draw_air_hitbox_overlay(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }
        let split_rect = air_split_rect(rect);
        let clip_rect = rect;
        let judge_y = rect.y + rect.h * 0.82;
        let head_color = Color::from_rgba(116, 234, 255, 232);
        let tail_color = Color::from_rgba(246, 186, 114, 228);
        let body_color = Color::from_rgba(176, 144, 255, 214);

        for note in &self.notes {
            if !is_air_kind(note.kind) {
                continue;
            }

            let center_x = split_rect.x + lane_to_air_x_norm(note.lane) * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);
            let mut label_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_left = split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_right = split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);

                    let head_rect =
                        note_end_hit_rect(head_left, (head_right - head_left).max(2.0), head_y);
                    let tail_rect =
                        note_end_hit_rect(tail_left, (tail_right - tail_left).max(2.0), tail_y);
                    draw_debug_hitbox_rect(head_rect, clip_rect, head_color, 1.3);
                    draw_debug_hitbox_rect(tail_rect, clip_rect, tail_color, 1.3);
                    draw_debug_skyarea_body_hit_overlay(split_rect, shape, head_y, tail_y, body_color);
                    label_rect = head_rect;
                }
            } else {
                let head_rect = if note.kind == GroundNoteKind::Flick {
                    flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
                } else {
                    note_end_hit_rect(note_x, note_w, head_y)
                };
                draw_debug_hitbox_rect(head_rect, clip_rect, head_color, 1.3);
                label_rect = head_rect;
                if note.has_tail() {
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                    let body_rect = note_body_hit_rect(note_x, note_w, head_y.min(tail_y), head_y.max(tail_y));
                    draw_debug_hitbox_rect(body_rect, clip_rect, body_color, 1.2);
                }
            }

            let label = format!("#{}", note.id);
            draw_debug_hitbox_label(label_rect, clip_rect, &label, head_color);
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
            let preview_time =
                self.apply_snap(self.pointer_to_time(my, current_ms, judge_y, rect.h).max(0.0));
            let preview_y = self.time_to_y(preview_time, current_ms, judge_y, rect.h);
            let lane = lane_from_x(mx, rect.x, lane_w);
            let palette = lane_note_palette(lane);
            draw_line(
                rect.x,
                preview_y,
                rect.x + rect.w,
                preview_y,
                1.2,
                Color::from_rgba(255, 230, 132, 190),
            );
            match place_type {
                PlaceNoteType::Hold => {
                    if let Some(pending) = self.pending_hold {
                        let lane_x = rect.x + lane_w * pending.lane as f32;
                        let note_w = lane_w * 0.94;
                        let note_x = lane_x + (lane_w - note_w) * 0.5;
                        let start_y = self.time_to_y(pending.start_time_ms, current_ms, judge_y, rect.h);
                        let y1 = start_y.min(preview_y);
                        let y2 = start_y.max(preview_y);

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
                            preview_y - 8.0,
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
                            preview_y - 8.0,
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
                        preview_y - 8.0,
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
            let judge_y = rect.y + rect.h * 0.82;
            let preview_time =
                self.apply_snap(self.pointer_to_time(my, current_ms, judge_y, rect.h).max(0.0));
            let preview_y = self.time_to_y(preview_time, current_ms, judge_y, rect.h);
            draw_line(
                split_rect.x,
                preview_y,
                split_rect.x + split_rect.w,
                preview_y,
                1.2,
                Color::from_rgba(216, 232, 255, 188),
            );
            let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
            let lane = air_x_to_lane(x_norm);
            let center_norm = if place_type == PlaceNoteType::Flick {
                lane_to_air_x_norm(lane)
            } else {
                x_norm
            };
            let center_x = split_rect.x + center_norm * split_rect.w;
            let note_w = match place_type {
                PlaceNoteType::SkyArea => split_rect.w * DEFAULT_SKYAREA_WIDTH_NORM,
                _ => split_rect.w * DEFAULT_AIR_WIDTH_NORM,
            };
            let note_x = center_x - note_w * 0.5;
            let preview = GroundNote {
                id: 0,
                kind: GroundNoteKind::Flick,
                lane,
                time_ms: preview_time,
                duration_ms: 0.0,
                width: DEFAULT_AIR_WIDTH_NORM,
                flick_right: true,
                skyarea_shape: None,
            };
            if place_type == PlaceNoteType::Flick {
                let side_h = self.flick_side_height_px(preview.time_ms, rect.h);
                draw_flick_curve_shape(&preview, note_x, note_w, preview_y, side_h);
            } else {
                if let Some(pending) = self.pending_skyarea {
                    let half = DEFAULT_SKYAREA_WIDTH_NORM * 0.5;
                    let (start_time_ms, end_time_ms, start_center_norm, end_center_norm) =
                        if pending.start_time_ms <= preview_time {
                            (
                                pending.start_time_ms,
                                preview_time,
                                pending.start_center_norm,
                                x_norm,
                            )
                        } else {
                            (
                                preview_time,
                                pending.start_time_ms,
                                x_norm,
                                pending.start_center_norm,
                            )
                        };
                    let start_left = (start_center_norm - half).clamp(0.0, 1.0);
                    let start_right = (start_center_norm + half).clamp(0.0, 1.0);
                    let end_left = (end_center_norm - half).clamp(0.0, 1.0);
                    let end_right = (end_center_norm + half).clamp(0.0, 1.0);
                    let shape = SkyAreaShape {
                        start_left_norm: start_left,
                        start_right_norm: start_right,
                        end_left_norm: end_left,
                        end_right_norm: end_right,
                        left_ease: Ease::Linear,
                        right_ease: Ease::Linear,
                    };
                    let preview_note = GroundNote {
                        id: 0,
                        kind: GroundNoteKind::SkyArea,
                        lane: air_x_to_lane(((start_center_norm + end_center_norm) * 0.5).clamp(0.0, 1.0)),
                        time_ms: start_time_ms,
                        duration_ms: (end_time_ms - start_time_ms).max(0.0),
                        width: DEFAULT_SKYAREA_WIDTH_NORM,
                        flick_right: true,
                        skyarea_shape: Some(shape),
                    };
                    self.draw_skyarea_shape(
                        split_rect,
                        current_ms,
                        judge_y,
                        rect.h,
                        &preview_note,
                        shape,
                        false,
                    );
                } else {
                    draw_rectangle(note_x, preview_y - 8.0, note_w, 16.0, AIR_SKYAREA_HEAD_COLOR);
                }
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
        let clip_top = split_rect.y;
        let clip_bottom = split_rect.y + split_rect.h;
        let has_tail = note.duration_ms > 0.0;

        if has_tail {
            let seg_count = 20;
            for i in 0..seg_count {
                let p0 = i as f32 / seg_count as f32;
                let p1 = (i + 1) as f32 / seg_count as f32;
                let t0 = note.time_ms + note.duration_ms * p0;
                let t1 = note.time_ms + note.duration_ms * p1;
                let y0_raw = self.time_to_y(t0, current_ms, judge_y, lane_h);
                let y1_raw = self.time_to_y(t1, current_ms, judge_y, lane_h);
                if (y0_raw < clip_top && y1_raw < clip_top)
                    || (y0_raw > clip_bottom && y1_raw > clip_bottom)
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
                if selected {
                    let dark = Color::from_rgba(0, 0, 0, SELECTED_NOTE_DARKEN_ALPHA);
                    draw_triangle(
                        Vec2::new(lx0, y0),
                        Vec2::new(rx0, y0),
                        Vec2::new(rx1, y1),
                        dark,
                    );
                    draw_triangle(
                        Vec2::new(lx0, y0),
                        Vec2::new(rx1, y1),
                        Vec2::new(lx1, y1),
                        dark,
                    );
                }
            }
        }

        let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, lane_h);
        let tail_left = split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
        let tail_right = split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
        let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, lane_h);

        let head_w = (head_right - head_left).max(2.0);
        if head_y >= clip_top - 18.0 && head_y <= clip_bottom + 18.0 {
            draw_rectangle(head_left, head_y - 8.0, head_w, 16.0, AIR_SKYAREA_HEAD_COLOR);
            if selected {
                draw_selected_note_darken_rect(head_left, head_y - 8.0, head_w, 16.0);
                draw_selected_note_outline(head_left, head_y - 8.0, head_w, 16.0);
            }
        }

        let tail_w = (tail_right - tail_left).max(2.0);
        if has_tail && tail_y >= clip_top - 18.0 && tail_y <= clip_bottom + 18.0 {
            draw_rectangle(tail_left, tail_y - 8.0, tail_w, 16.0, AIR_SKYAREA_TAIL_COLOR);
            if selected {
                draw_selected_note_darken_rect(tail_left, tail_y - 8.0, tail_w, 16.0);
                draw_selected_note_outline(tail_left, tail_y - 8.0, tail_w, 16.0);
            }
        }
    }

    fn handle_vertical_progress_seek(
        &mut self,
        rect: Rect,
        audio_duration_sec: f32,
        is_playing: bool,
        actions: &mut Vec<FallingEditorAction>,
    ) {
        let (mx, my) = mouse_position();
        let inside = point_in_rect(mx, my, rect);
        let duration = self.estimate_duration(audio_duration_sec);

        if is_playing {
            self.waveform_seek_active = false;
            // While playing: allow click-to-seek, but do not allow drag-to-seek.
            if is_mouse_button_pressed(MouseButton::Left) && inside {
                self.waveform_seek_sec = y_to_time_sec(my, rect, duration);
                actions.push(FallingEditorAction::SeekTo(self.waveform_seek_sec));
                self.status = format!("seek to {:.2}s", self.waveform_seek_sec);
            }
            return;
        }

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
                            let duration = (end - start).max(0.0);
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
            }
        }

        if let Some(drag) = self.drag_state {
            if is_mouse_button_down(MouseButton::Left) {
                if get_time() - drag.start_time_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
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
                        let width_norm = DEFAULT_SKYAREA_WIDTH_NORM;
                        let half = width_norm * 0.5;
                        if let Some(pending) = self.pending_skyarea.take() {
                            let (start_time_ms, end_time_ms, start_center_norm, end_center_norm) =
                                if pending.start_time_ms <= time_ms {
                                    (pending.start_time_ms, time_ms, pending.start_center_norm, x_norm)
                                } else {
                                    (time_ms, pending.start_time_ms, x_norm, pending.start_center_norm)
                                };
                            let start_left = (start_center_norm - half).clamp(0.0, 1.0);
                            let start_right = (start_center_norm + half).clamp(0.0, 1.0);
                            let end_left = (end_center_norm - half).clamp(0.0, 1.0);
                            let end_right = (end_center_norm + half).clamp(0.0, 1.0);
                            self.push_note(GroundNote {
                                id: self.next_note_id,
                                kind: GroundNoteKind::SkyArea,
                                lane: air_x_to_lane(
                                    ((start_center_norm + end_center_norm) * 0.5).clamp(0.0, 1.0),
                                ),
                                time_ms: start_time_ms,
                                duration_ms: (end_time_ms - start_time_ms).max(0.0),
                                width: width_norm,
                                flick_right: true,
                                skyarea_shape: Some(SkyAreaShape {
                                    start_left_norm: start_left,
                                    start_right_norm: start_right,
                                    end_left_norm: end_left,
                                    end_right_norm: end_right,
                                    left_ease: Ease::Linear,
                                    right_ease: Ease::Linear,
                                }),
                            });
                            self.status = format!(
                                "new skyarea created {:.0}ms -> {:.0}ms",
                                start_time_ms.round(),
                                end_time_ms.round()
                            );
                        } else {
                            self.pending_skyarea = Some(PendingSkyAreaPlacement {
                                start_time_ms: time_ms,
                                start_center_norm: x_norm,
                            });
                            self.status = format!(
                                "skyarea head set x={:.3} time={:.0}ms",
                                x_norm,
                                time_ms.round()
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(drag) = self.drag_state {
            if is_mouse_button_down(MouseButton::Left) {
                if get_time() - drag.start_time_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                let dx = mx - drag.start_mouse_x;
                let dy = my - drag.start_mouse_y;
                let vertical_drag = dy.abs() >= scaled_px(SKYAREA_VERTICAL_DRAG_THRESHOLD_PX)
                    && dy.abs() > dx.abs();
                if let Some(note) = self
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_air_kind(note.kind))
                {
                    if note.kind == GroundNoteKind::SkyArea {
                        let old_tail = note.time_ms + note.duration_ms;
                        if let Some(shape) = note.skyarea_shape.as_mut() {
                            let start_half_now = ((shape.start_right_norm - shape.start_left_norm).abs() * 0.5)
                                .clamp(0.01, 0.5);
                            let end_half_now = ((shape.end_right_norm - shape.end_left_norm).abs() * 0.5)
                                .clamp(0.01, 0.5);

                            match drag.air_target {
                                AirDragTarget::Body => {
                                    // Body drag keeps skyarea easing shape, only translating start/end X together.
                                    // Use one shared delta and edge-based limits, so:
                                    // 1) head/tail widths stay unchanged
                                    // 2) head-tail X gap stays unchanged
                                    // 3) both head and tail stay in [0, 1]
                                    let start_half = drag.sky_start_half_norm.clamp(0.01, 0.5);
                                    let end_half = drag.sky_end_half_norm.clamp(0.01, 0.5);
                                    let start_left_0 = drag.sky_start_center_norm - start_half;
                                    let start_right_0 = drag.sky_start_center_norm + start_half;
                                    let end_left_0 = drag.sky_end_center_norm - end_half;
                                    let end_right_0 = drag.sky_end_center_norm + end_half;
                                    let delta_norm = (mx - drag.start_mouse_x) / split_rect.w.max(1.0);
                                    let delta_min = (-start_left_0).max(-end_left_0);
                                    let delta_max = (1.0 - start_right_0).min(1.0 - end_right_0);
                                    let delta = if delta_min <= delta_max {
                                        delta_norm.clamp(delta_min, delta_max)
                                    } else {
                                        0.0
                                    };
                                    shape.start_left_norm = start_left_0 + delta;
                                    shape.start_right_norm = start_right_0 + delta;
                                    shape.end_left_norm = end_left_0 + delta;
                                    shape.end_right_norm = end_right_0 + delta;

                                    note.time_ms = snapped_time;
                                }
                                AirDragTarget::SkyHead => {
                                    let start_center = x_norm.clamp(start_half_now, 1.0 - start_half_now);
                                    shape.start_left_norm = (start_center - start_half_now).clamp(0.0, 1.0);
                                    shape.start_right_norm = (start_center + start_half_now).clamp(0.0, 1.0);

                                    if vertical_drag {
                                        let new_start = snapped_time.min(old_tail);
                                        note.time_ms = new_start.max(0.0);
                                        note.duration_ms = (old_tail - note.time_ms).max(0.0);
                                    }
                                }
                                AirDragTarget::SkyTail => {
                                    let end_center = x_norm.clamp(end_half_now, 1.0 - end_half_now);
                                    shape.end_left_norm = (end_center - end_half_now).clamp(0.0, 1.0);
                                    shape.end_right_norm = (end_center + end_half_now).clamp(0.0, 1.0);

                                    if vertical_drag {
                                        let tail_time = snapped_time.max(note.time_ms);
                                        note.duration_ms = (tail_time - note.time_ms).max(0.0);
                                    }
                                }
                            }

                            let center_norm = ((shape.start_left_norm
                                + shape.start_right_norm
                                + shape.end_left_norm
                                + shape.end_right_norm)
                                * 0.25)
                                .clamp(0.0, 1.0);
                            let start_w = (shape.start_right_norm - shape.start_left_norm).abs().clamp(0.02, 1.0);
                            let end_w = (shape.end_right_norm - shape.end_left_norm).abs().clamp(0.02, 1.0);
                            note.lane = air_x_to_lane(center_norm);
                            note.width = ((start_w + end_w) * 0.5).clamp(0.05, 1.0);
                        }
                    } else {
                        note.lane = air_x_to_lane(x_norm);
                        note.time_ms = snapped_time;
                    }
                    self.status = format!("air drag lane={} time={:.0}ms", note.lane, note.time_ms);
                }
            } else {
                self.drag_state = None;
                self.sort_notes();
            }
        }
    }

    fn handle_note_selection_click(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let (mx, my) = mouse_position();

        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected_note_id = None;
            self.drag_state = None;
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
            self.status = "selection cleared".to_owned();
            return;
        }

        if !is_mouse_button_pressed(MouseButton::Left) {
            return;
        }

        let (scope, candidates) = self.collect_hit_candidates(mx, my, ground_rect, air_rect, current_ms);
        if candidates.is_empty() {
            // Blank click or out-of-surface click should reset click-cycle + drag latch.
            // Keep selected_note_id unchanged so user can inspect last selection.
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
            self.drag_state = None;
            return;
        }

        let ordered_items: Vec<HitSignatureItem> = candidates.iter().map(hit_signature_item).collect();
        let signature = canonical_hit_signature(&ordered_items);
        let (anchor_x, anchor_y) = quantize_overlap_anchor(mx, my);
        let now_sec = get_time();
        let mut did_cycle = false;
        let selected_note_index = self
            .selected_note_id
            .and_then(|selected_id| candidates.iter().position(|c| c.note_id == selected_id));

        let selected_index = if candidates.len() > 1 {
            // In overlap region, prefer keeping current selected note on single click.
            // Cycling to another overlapped note is only via overlap double-click.
            let mut index = selected_note_index.unwrap_or(0);
            let mut double_click_armed = selected_note_index.is_some();
            if let Some(prev) = &self.overlap_cycle {
                if prev.scope == scope
                    && prev.anchor_x == anchor_x
                    && prev.anchor_y == anchor_y
                    && prev.signature == signature
                {
                    let previous_in_current = ordered_items
                        .iter()
                        .position(|item| *item == prev.selected_item)
                        .unwrap_or_else(|| prev.current_index.min(candidates.len().saturating_sub(1)));
                    if prev.double_click_armed {
                        let elapsed = now_sec - prev.last_click_time_sec;
                        if elapsed <= OVERLAP_DOUBLE_CLICK_SEC {
                            index = (previous_in_current + 1) % candidates.len();
                            did_cycle = true;
                            double_click_armed = false;
                        } else {
                            // Prior pair expired; this click becomes the new first click.
                            index = selected_note_index.unwrap_or(previous_in_current);
                            double_click_armed = true;
                        }
                    } else {
                        index = selected_note_index.unwrap_or(previous_in_current);
                        double_click_armed = true;
                    }
                }
            }
            let selected_item = ordered_items[index];
            self.overlap_cycle = Some(OverlapCycleState {
                signature,
                current_index: index,
                selected_item,
                anchor_x,
                anchor_y,
                scope,
                last_click_time_sec: now_sec,
                double_click_armed,
            });
            index
        } else {
            self.overlap_cycle = None;
            0
        };

        let selected = candidates[selected_index];
        self.selected_note_id = Some(selected.note_id);
        self.start_drag_for_candidate(selected, mx, my, current_ms, ground_rect, air_rect);
        if candidates.len() > 1 && did_cycle {
            self.status = format!(
                "overlap select {}/{} (note={})",
                selected_index + 1,
                candidates.len(),
                selected.note_id
            );
        }
    }

    fn update_hover_overlap_hint(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let (mx, my) = mouse_position();
        let (scope, candidates) = self.collect_hit_candidates(mx, my, ground_rect, air_rect, current_ms);
        if candidates.len() <= 1 {
            self.hover_overlap_hint = None;
            return;
        }

        let ordered_items: Vec<HitSignatureItem> = candidates.iter().map(hit_signature_item).collect();
        let signature = canonical_hit_signature(&ordered_items);
        let (anchor_x, anchor_y) = quantize_overlap_anchor(mx, my);
        let mut current_index = 0_usize;
        if let Some(cycle) = &self.overlap_cycle {
            if cycle.scope == scope
                && cycle.anchor_x == anchor_x
                && cycle.anchor_y == anchor_y
                && cycle.signature == signature
            {
                current_index = ordered_items
                    .iter()
                    .position(|item| *item == cycle.selected_item)
                    .unwrap_or_else(|| cycle.current_index.min(candidates.len().saturating_sub(1)));
            }
        }

        self.hover_overlap_hint = Some(HoverOverlapHint {
            mouse_x: mx,
            mouse_y: my,
            current_index,
            total: candidates.len(),
        });
    }

    fn draw_overlap_hint(&self) {
        let Some(hint) = self.hover_overlap_hint else {
            return;
        };
        if hint.total <= 1 {
            return;
        }

        let text = format!("{}/{}", hint.current_index + 1, hint.total);
        let ui = adaptive_ui_scale();
        let font_size = scaled_font_size(18.0, 12, 42);
        let metrics = measure_text(&text, None, font_size, 1.0);
        let box_w = metrics.width + 14.0 * ui;
        let box_h = 24.0 * ui;
        let x = (hint.mouse_x + 14.0 * ui).clamp(4.0 * ui, screen_width() - box_w - 4.0 * ui);
        let y = (hint.mouse_y - box_h - 10.0 * ui).clamp(4.0 * ui, screen_height() - box_h - 4.0 * ui);

        draw_rectangle(x, y, box_w, box_h, Color::from_rgba(20, 24, 34, 214));
        draw_rectangle_lines(x, y, box_w, box_h, 1.0, Color::from_rgba(140, 156, 198, 220));
        draw_text_ex(
            &text,
            x + 7.0 * ui,
            y + 17.0 * ui,
            TextParams {
                font_size,
                color: Color::from_rgba(228, 234, 248, 255),
                ..Default::default()
            },
        );
    }

    fn collect_hit_candidates(
        &self,
        mx: f32,
        my: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) -> (HitScope, Vec<HitCandidate>) {
        let mut candidates = Vec::new();

        if self.render_scope == RenderScope::Both {
            let Some(ground_rect) = ground_rect else {
                return (HitScope::Mixed, candidates);
            };
            candidates.extend(self.collect_hit_candidates_ground(mx, my, ground_rect, current_ms));
            if let Some(air_rect) = air_rect {
                candidates.extend(self.collect_hit_candidates_air(mx, my, air_rect, current_ms));
            }
            sort_hit_candidates(&mut candidates);
            return (HitScope::Mixed, candidates);
        }

        if let Some(rect) = ground_rect {
            candidates.extend(self.collect_hit_candidates_ground(mx, my, rect, current_ms));
        }

        if let Some(rect) = air_rect {
            candidates.extend(self.collect_hit_candidates_air(mx, my, rect, current_ms));
        }

        if candidates.is_empty() {
            return (HitScope::Ground, Vec::new());
        }

        sort_hit_candidates(&mut candidates);
        let has_ground = candidates.iter().any(|c| c.scope == HitScope::Ground);
        let has_air = candidates.iter().any(|c| c.scope == HitScope::Air);
        let scope = match (has_ground, has_air) {
            (true, true) => HitScope::Mixed,
            (false, true) => HitScope::Air,
            _ => HitScope::Ground,
        };
        (scope, candidates)
    }

    fn collect_hit_candidates_ground(
        &self,
        mx: f32,
        my: f32,
        rect: Rect,
        current_ms: f32,
    ) -> Vec<HitCandidate> {
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let mut candidates = Vec::new();

        for (z, note) in self.notes.iter().enumerate() {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let lane_x = rect.x + lane_w * note.lane as f32;
            let note_w = note_head_width(note, lane_w);
            let note_x = lane_x + (lane_w - note_w) * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let z_order = z as u32;
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);

            let head_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };
            if point_in_rect(mx, my, head_rect) {
                push_best_hit_candidate(
                    &mut candidates,
                    HitCandidate {
                        note_id: note.id,
                        scope: HitScope::Ground,
                        air_target: AirDragTarget::Body,
                        part: HitPart::Head,
                        distance_sq: distance_sq_to_rect(mx, my, head_rect),
                        z_order,
                    },
                );
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let tail_rect = note_end_hit_rect(note_x, note_w, tail_y);
                if point_in_rect(mx, my, tail_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Ground,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Tail,
                            distance_sq: distance_sq_to_rect(mx, my, tail_rect),
                            z_order,
                        },
                    );
                }

                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let (body_x, body_w) = match note.kind {
                    GroundNoteKind::Hold => (note_x + note_w * 0.04, note_w * 0.92),
                    GroundNoteKind::SkyArea => (note_x + note_w * 0.02, note_w * 0.96),
                    _ => (note_x + note_w * 0.34, note_w * 0.32),
                };
                let body_rect = note_body_hit_rect(body_x, body_w, y1, y2);
                if point_in_rect(mx, my, body_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Ground,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Body,
                            distance_sq: distance_sq_to_rect(mx, my, body_rect),
                            z_order,
                        },
                    );
                }
            }
        }

        candidates
    }

    fn collect_hit_candidates_air(
        &self,
        mx: f32,
        my: f32,
        rect: Rect,
        current_ms: f32,
    ) -> Vec<HitCandidate> {
        let judge_y = rect.y + rect.h * 0.82;
        let split_rect = air_split_rect(rect);

        let mut candidates = Vec::new();
        for (z, note) in self.notes.iter().enumerate() {
            if !is_air_kind(note.kind) {
                continue;
            }
            let z_order = z as u32;
            let center_x = split_rect.x + lane_to_air_x_norm(note.lane) * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_left = split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_right = split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);

                    let head_rect = note_end_hit_rect(
                        head_left,
                        (head_right - head_left).max(2.0),
                        head_y,
                    );
                    if point_in_rect(mx, my, head_rect) {
                        push_best_hit_candidate(
                            &mut candidates,
                            HitCandidate {
                                note_id: note.id,
                                scope: HitScope::Air,
                                air_target: AirDragTarget::SkyHead,
                                part: HitPart::Head,
                                distance_sq: distance_sq_to_rect(mx, my, head_rect),
                                z_order,
                            },
                        );
                    }

                    let tail_rect = note_end_hit_rect(
                        tail_left,
                        (tail_right - tail_left).max(2.0),
                        tail_y,
                    );
                    if point_in_rect(mx, my, tail_rect) {
                        push_best_hit_candidate(
                            &mut candidates,
                            HitCandidate {
                                note_id: note.id,
                                scope: HitScope::Air,
                                air_target: AirDragTarget::SkyTail,
                                part: HitPart::Tail,
                                distance_sq: distance_sq_to_rect(mx, my, tail_rect),
                                z_order,
                            },
                        );
                    }

                    let min_left = shape.start_left_norm.min(shape.end_left_norm).clamp(0.0, 1.0);
                    let max_right = shape.start_right_norm.max(shape.end_right_norm).clamp(0.0, 1.0);
                    let x1 = split_rect.x + min_left * split_rect.w;
                    let x2 = split_rect.x + max_right * split_rect.w;
                    let y1 = head_y.min(tail_y);
                    let y2 = head_y.max(tail_y);
                    let body_rect = note_body_hit_rect(x1, (x2 - x1).max(1.0), y1, y2);
                    if point_in_rect(mx, my, body_rect) {
                        let body_distance_sq =
                            skyarea_body_hit_distance_sq(mx, my, split_rect, shape, head_y, tail_y);
                        if let Some(distance_sq) = body_distance_sq {
                        push_best_hit_candidate(
                            &mut candidates,
                            HitCandidate {
                                note_id: note.id,
                                scope: HitScope::Air,
                                air_target: AirDragTarget::Body,
                                part: HitPart::Body,
                                distance_sq,
                                z_order,
                            },
                        );
                        }
                    }
                    continue;
                }
            }

            let head_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };
            if point_in_rect(mx, my, head_rect) {
                push_best_hit_candidate(
                    &mut candidates,
                    HitCandidate {
                        note_id: note.id,
                        scope: HitScope::Air,
                        air_target: AirDragTarget::Body,
                        part: HitPart::Head,
                        distance_sq: distance_sq_to_rect(mx, my, head_rect),
                        z_order,
                    },
                );
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let body_rect = note_body_hit_rect(note_x, note_w, y1, y2);
                if point_in_rect(mx, my, body_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Air,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Body,
                            distance_sq: distance_sq_to_rect(mx, my, body_rect),
                            z_order,
                        },
                    );
                }
            }
        }

        candidates
    }

    fn start_drag_for_candidate(
        &mut self,
        candidate: HitCandidate,
        mx: f32,
        my: f32,
        current_ms: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
    ) {
        let Some(note) = self.notes.iter().find(|note| note.id == candidate.note_id) else {
            self.drag_state = None;
            return;
        };

        let (judge_y, lane_h) = match candidate.scope {
            HitScope::Ground => {
                let Some(rect) = ground_rect else {
                    self.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Air => {
                let Some(rect) = air_rect else {
                    self.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Mixed => {
                self.drag_state = None;
                return;
            }
        };

        let pointer_time_ms = self.pointer_to_time(my, current_ms, judge_y, lane_h);
        let (sky_start_center_norm, sky_end_center_norm, sky_start_half_norm, sky_end_half_norm) =
            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let start_left = shape.start_left_norm.clamp(0.0, 1.0);
                    let start_right = shape.start_right_norm.clamp(0.0, 1.0);
                    let end_left = shape.end_left_norm.clamp(0.0, 1.0);
                    let end_right = shape.end_right_norm.clamp(0.0, 1.0);
                    (
                        (start_left + start_right) * 0.5,
                        (end_left + end_right) * 0.5,
                        ((start_right - start_left).abs() * 0.5).clamp(0.01, 0.5),
                        ((end_right - end_left).abs() * 0.5).clamp(0.01, 0.5),
                    )
                } else {
                    let center = lane_to_air_x_norm(note.lane);
                    (center, center, 0.25, 0.25)
                }
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

        let drag_anchor_time_ms =
            if note.kind == GroundNoteKind::SkyArea && candidate.air_target == AirDragTarget::SkyTail {
                note.end_time_ms()
            } else {
                note.time_ms
            };

        self.drag_state = Some(DragState {
            note_id: candidate.note_id,
            time_offset_ms: drag_anchor_time_ms - pointer_time_ms,
            start_time_sec: get_time(),
            start_mouse_x: mx,
            start_mouse_y: my,
            sky_start_center_norm,
            sky_end_center_norm,
            sky_start_half_norm,
            sky_end_half_norm,
            air_target: candidate.air_target,
        });
    }

    fn pointer_to_time(&self, mouse_y: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        current_ms + (judge_y - mouse_y) / (self.scroll_speed * lane_h).max(1.0) * 1000.0
    }

    fn time_to_y(&self, note_time_ms: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        judge_y - (note_time_ms - current_ms) / 1000.0 * (self.scroll_speed * lane_h)
    }

    fn flick_side_height_px(&self, note_time_ms: f32, lane_h: f32) -> f32 {
        let bpm = self
            .timeline
            .point_at_time(note_time_ms.max(0.0))
            .bpm
            .abs()
            .max(0.001);
        let beat_ms = 60_000.0 / bpm;
        let subdivision_ms = beat_ms / 16.0;
        let pixels_per_sec = (self.scroll_speed * lane_h).max(1.0);
        subdivision_ms / 1000.0 * pixels_per_sec
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

fn adaptive_ui_scale() -> f32 {
    (screen_width() / REFERENCE_WIDTH)
        .min(screen_height() / REFERENCE_HEIGHT)
        .clamp(0.75, 3.5)
}

fn scaled_px(px: f32) -> f32 {
    px * adaptive_ui_scale()
}

fn scaled_font_size(base: f32, min: u16, max: u16) -> u16 {
    let size = (base * adaptive_ui_scale()).round();
    size.clamp(min as f32, max as f32) as u16
}

fn push_best_hit_candidate(candidates: &mut Vec<HitCandidate>, candidate: HitCandidate) {
    if let Some(existing) = candidates
        .iter_mut()
        .find(|item| {
            item.note_id == candidate.note_id
                && item.scope == candidate.scope
                && item.air_target == candidate.air_target
                && item.part == candidate.part
        })
    {
        if should_replace_hit_candidate(*existing, candidate) {
            *existing = candidate;
        }
    } else {
        candidates.push(candidate);
    }
}

fn should_replace_hit_candidate(current: HitCandidate, incoming: HitCandidate) -> bool {
    if (incoming.distance_sq - current.distance_sq).abs() > 0.01 {
        return incoming.distance_sq < current.distance_sq;
    }

    let current_rank = hit_part_rank(current.part);
    let incoming_rank = hit_part_rank(incoming.part);
    if incoming_rank != current_rank {
        return incoming_rank > current_rank;
    }
    incoming.z_order > current.z_order
}

fn sort_hit_candidates(candidates: &mut Vec<HitCandidate>) {
    candidates.sort_by(|a, b| {
        hit_part_rank(b.part)
            .cmp(&hit_part_rank(a.part))
            .then_with(|| a.distance_sq.total_cmp(&b.distance_sq))
            .then_with(|| b.z_order.cmp(&a.z_order))
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
}

fn hit_part_rank(part: HitPart) -> u8 {
    match part {
        HitPart::Head | HitPart::Tail => 2,
        HitPart::Body => 1,
    }
}

fn distance_sq(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn distance_sq_to_rect(px: f32, py: f32, rect: Rect) -> f32 {
    let rx1 = rect.x;
    let ry1 = rect.y;
    let rx2 = rect.x + rect.w;
    let ry2 = rect.y + rect.h;
    let cx = px.clamp(rx1, rx2);
    let cy = py.clamp(ry1, ry2);
    distance_sq(px, py, cx, cy)
}

fn note_end_hit_rect(x: f32, w: f32, center_y: f32) -> Rect {
    let pad_x = scaled_px(NOTE_HEAD_HIT_PAD_X);
    let half_h = scaled_px(NOTE_HEAD_HIT_HALF_H);
    Rect::new(
        x - pad_x,
        center_y - half_h,
        (w + pad_x * 2.0).max(1.0),
        (half_h * 2.0).max(1.0),
    )
}

fn note_body_hit_rect(x: f32, w: f32, y1: f32, y2: f32) -> Rect {
    let edge_gap = scaled_px(NOTE_BODY_EDGE_GAP_Y);
    let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
    let top_raw = y1.min(y2);
    let bottom_raw = y1.max(y2);
    let top = (top_raw + edge_gap).min(bottom_raw);
    let bottom = (bottom_raw - edge_gap).max(top);
    let body_w = (w + pad_x * 2.0).max(1.0);
    let thin_h = scaled_px(2.0);
    if bottom - top < thin_h {
        let center_y = (top_raw + bottom_raw) * 0.5;
        return Rect::new(x - pad_x, center_y - thin_h, body_w, thin_h * 2.0);
    }
    Rect::new(
        x - pad_x,
        top,
        body_w,
        (bottom - top).max(1.0),
    )
}

fn flick_rect_hitbox(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> Rect {
    // Rectangle hitbox aligned to the actual flick footprint.
    flick_shape_bounds(note, note_x, note_w, head_y, side_h)
}

fn skyarea_screen_x_range_at_progress(
    split_rect: Rect,
    shape: SkyAreaShape,
    p: f32,
) -> (f32, f32) {
    let p = p.clamp(0.0, 1.0);
    let left_norm = lerp(
        shape.start_left_norm,
        shape.end_left_norm,
        ease_progress(shape.left_ease, p),
    )
    .clamp(0.0, 1.0);
    let right_norm = lerp(
        shape.start_right_norm,
        shape.end_right_norm,
        ease_progress(shape.right_ease, p),
    )
    .clamp(0.0, 1.0);
    (
        split_rect.x + left_norm * split_rect.w,
        split_rect.x + right_norm * split_rect.w,
    )
}

fn skyarea_body_vertical_range(head_y: f32, tail_y: f32) -> (f32, f32) {
    let edge_gap = scaled_px(NOTE_BODY_EDGE_GAP_Y);
    let min_y = head_y.min(tail_y);
    let max_y = head_y.max(tail_y);
    if max_y - min_y <= edge_gap * 2.0 {
        let mid = (min_y + max_y) * 0.5;
        let thin_h = scaled_px(2.0);
        return (mid - thin_h, mid + thin_h);
    }
    (min_y + edge_gap, max_y - edge_gap)
}

fn skyarea_body_hit_distance_sq(
    mx: f32,
    my: f32,
    split_rect: Rect,
    shape: SkyAreaShape,
    head_y: f32,
    tail_y: f32,
) -> Option<f32> {
    let dy = tail_y - head_y;
    if dy.abs() <= 0.000_1 {
        return None;
    }

    let (body_top, body_bottom) = skyarea_body_vertical_range(head_y, tail_y);
    if my < body_top || my > body_bottom {
        return None;
    }

    let p = ((my - head_y) / dy).clamp(0.0, 1.0);
    let (left_x, right_x) = skyarea_screen_x_range_at_progress(split_rect, shape, p);
    let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
    let x1 = left_x.min(right_x) - pad_x;
    let x2 = left_x.max(right_x) + pad_x;
    if mx < x1 || mx > x2 {
        return None;
    }

    let center_x = (x1 + x2) * 0.5;
    let dist = mx - center_x;
    Some(dist * dist)
}

fn quantize_overlap_anchor(x: f32, y: f32) -> (i32, i32) {
    let anchor_px = scaled_px(OVERLAP_CYCLE_ANCHOR_PX).max(1.0);
    (
        (x / anchor_px).round() as i32,
        (y / anchor_px).round() as i32,
    )
}

fn hit_signature_item(candidate: &HitCandidate) -> HitSignatureItem {
    HitSignatureItem {
        note_id: candidate.note_id,
        scope: candidate.scope,
        air_target: candidate.air_target,
        part: candidate.part,
    }
}

fn canonical_hit_signature(items: &[HitSignatureItem]) -> Vec<HitSignatureItem> {
    let mut signature = items.to_vec();
    signature.sort_by(|a, b| {
        hit_scope_rank(a.scope)
            .cmp(&hit_scope_rank(b.scope))
            .then_with(|| hit_part_rank(b.part).cmp(&hit_part_rank(a.part)))
            .then_with(|| air_target_rank(a.air_target).cmp(&air_target_rank(b.air_target)))
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
    signature
}

fn hit_scope_rank(scope: HitScope) -> u8 {
    match scope {
        HitScope::Ground => 0,
        HitScope::Air => 1,
        HitScope::Mixed => 2,
    }
}

fn air_target_rank(target: AirDragTarget) -> u8 {
    match target {
        AirDragTarget::Body => 0,
        AirDragTarget::SkyHead => 1,
        AirDragTarget::SkyTail => 2,
    }
}

fn draw_selected_note_darken_rect(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(
        x,
        y,
        w.max(1.0),
        h.max(1.0),
        Color::from_rgba(0, 0, 0, SELECTED_NOTE_DARKEN_ALPHA),
    );
}

fn draw_selected_note_outline(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle_lines(
        x - 1.4,
        y - 1.4,
        (w + 2.8).max(1.0),
        (h + 2.8).max(1.0),
        2.4,
        Color::from_rgba(255, 212, 102, 255),
    );
    draw_rectangle_lines(
        x + 0.8,
        y + 0.8,
        (w - 1.6).max(1.0),
        (h - 1.6).max(1.0),
        1.2,
        Color::from_rgba(255, 244, 170, 236),
    );
}

fn draw_debug_hitbox_rect(hit: Rect, clip: Rect, color: Color, thickness: f32) {
    let x1 = hit.x.max(clip.x);
    let y1 = hit.y.max(clip.y);
    let x2 = (hit.x + hit.w).min(clip.x + clip.w);
    let y2 = (hit.y + hit.h).min(clip.y + clip.h);
    if x2 <= x1 || y2 <= y1 {
        return;
    }
    draw_rectangle_lines(x1, y1, x2 - x1, y2 - y1, thickness, color);
}

fn draw_debug_hitbox_label(hit: Rect, clip: Rect, label: &str, color: Color) {
    let ui = adaptive_ui_scale();
    let x = hit.x.max(clip.x + 2.0 * ui);
    let y = (hit.y - 3.0 * ui).clamp(clip.y + 10.0 * ui, clip.y + clip.h - 2.0 * ui);
    draw_text_ex(
        label,
        x,
        y,
        TextParams {
            font_size: scaled_font_size(14.0, 10, 34),
            color,
            ..Default::default()
        },
    );
}

fn draw_debug_skyarea_body_hit_overlay(
    split_rect: Rect,
    shape: SkyAreaShape,
    head_y: f32,
    tail_y: f32,
    color: Color,
) {
    let dy = tail_y - head_y;
    if dy.abs() <= 0.000_1 {
        return;
    }

    let (body_top, body_bottom) = skyarea_body_vertical_range(head_y, tail_y);
    if body_bottom <= body_top {
        return;
    }

    let steps = 24;
    for i in 0..steps {
        let p0 = i as f32 / steps as f32;
        let p1 = (i + 1) as f32 / steps as f32;
        let y0 = lerp(head_y, tail_y, p0);
        let y1 = lerp(head_y, tail_y, p1);
        if (y0 < body_top && y1 < body_top) || (y0 > body_bottom && y1 > body_bottom) {
            continue;
        }

        let (l0, r0) = skyarea_screen_x_range_at_progress(split_rect, shape, p0);
        let (l1, r1) = skyarea_screen_x_range_at_progress(split_rect, shape, p1);
        let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
        let x0l = l0.min(r0) - pad_x;
        let x0r = l0.max(r0) + pad_x;
        let x1l = l1.min(r1) - pad_x;
        let x1r = l1.max(r1) + pad_x;
        let yy0 = y0.clamp(body_top, body_bottom);
        let yy1 = y1.clamp(body_top, body_bottom);
        if (yy1 - yy0).abs() < 0.001 {
            continue;
        }

        draw_triangle(
            Vec2::new(x0l, yy0),
            Vec2::new(x0r, yy0),
            Vec2::new(x1r, yy1),
            Color::new(color.r, color.g, color.b, 0.12),
        );
        draw_triangle(
            Vec2::new(x0l, yy0),
            Vec2::new(x1r, yy1),
            Vec2::new(x1l, yy1),
            Color::new(color.r, color.g, color.b, 0.12),
        );
        draw_line(x0l, yy0, x1l, yy1, 1.1, color);
        draw_line(x0r, yy0, x1r, yy1, 1.1, color);
    }
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

#[derive(Debug, Clone, Copy)]
struct FlickGeometry {
    x_start: f32,
    x_tip: f32,
    y_top: f32,
    y_bottom: f32,
    y_tip_top: f32,
    y_tip_bottom: f32,
    stroke: f32,
}

fn flick_geometry(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> FlickGeometry {
    let ui = adaptive_ui_scale();
    let stroke = (note_w * 0.05).clamp(1.0 * ui, 2.8 * ui);
    let side_h = side_h.max(0.0);
    // Align flick baseline with note/barline Y exactly.
    let y_bottom = head_y;
    let y_top = y_bottom - side_h;
    let y_tip_bottom = y_bottom;
    let y_tip_top = y_bottom - (side_h * 0.04).max(0.6 * ui);

    let (x_start, x_tip) = if note.flick_right {
        (note_x + note_w * 0.92, note_x + note_w * 0.02)
    } else {
        (note_x + note_w * 0.08, note_x + note_w * 0.98)
    };

    FlickGeometry {
        x_start,
        x_tip,
        y_top,
        y_bottom,
        y_tip_top,
        y_tip_bottom,
        stroke,
    }
}

fn draw_flick_curve_shape(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) {
    let (fill_color, edge_color) = flick_direction_shape_colors(note.flick_right);
    let geom = flick_geometry(note, note_x, note_w, head_y, side_h);

    let mut top_curve = Vec::with_capacity(25);
    for i in 0..=24 {
        let t = i as f32 / 24.0;
        let x = lerp(geom.x_start, geom.x_tip, t);
        let eased = ease_progress(Ease::SineOut, t);
        let y = lerp(geom.y_top, geom.y_tip_top, eased);
        top_curve.push(Vec2::new(x, y));
    }

    let mut polygon = Vec::with_capacity(28);
    polygon.push(Vec2::new(geom.x_start, geom.y_bottom));
    polygon.extend_from_slice(&top_curve);
    polygon.push(Vec2::new(geom.x_tip, geom.y_tip_bottom));

    for i in 1..(polygon.len() - 1) {
        draw_triangle(polygon[0], polygon[i], polygon[i + 1], fill_color);
    }

    for i in 0..(top_curve.len() - 1) {
        let a = top_curve[i];
        let b = top_curve[i + 1];
        draw_line(a.x, a.y, b.x, b.y, geom.stroke, edge_color);
    }
    draw_line(
        geom.x_start,
        geom.y_bottom,
        geom.x_tip,
        geom.y_tip_bottom,
        geom.stroke,
        edge_color,
    );
    draw_line(
        geom.x_start,
        geom.y_bottom,
        geom.x_start,
        geom.y_top,
        geom.stroke,
        edge_color,
    );
}

fn flick_shape_bounds(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> Rect {
    let geom = flick_geometry(note, note_x, note_w, head_y, side_h);
    let x1 = geom.x_start.min(geom.x_tip);
    let x2 = geom.x_start.max(geom.x_tip);
    Rect::new(
        x1,
        geom.y_top,
        (x2 - x1).max(1.0),
        (geom.y_bottom - geom.y_top).max(1.0),
    )
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
    (1.0 - t) * duration_sec
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn draw_small_button(rect: Rect, text: &str) -> bool {
    let ui = adaptive_ui_scale();
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

    let font_size = scaled_font_size(24.0, 12, 52);
    let metrics = measure_text(text, None, font_size, 1.0);
    draw_text_ex(
        text,
        rect.x + (rect.w - metrics.width) * 0.5,
        rect.y + rect.h * (0.68 + 0.04 / ui),
        TextParams {
            font_size,
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
                        duration_ms: (*duration as f32).max(0.0),
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
                    duration_ms: (*duration as f32).max(0.0),
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
