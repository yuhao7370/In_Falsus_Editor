// 文件说明：编辑器状态数据结构定义。
// 主要功能：定义运行时状态、交互上下文与缓存字段。
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

#[derive(Debug, Clone, Copy)]
struct MinimapDrawLayout {
    duration_ms: f32,
    half_ms: f32,
    ui: f32,
    ground_rect_1: Rect,
    sky_rect_1: Rect,
    ground_rect_2: Rect,
    sky_rect_2: Rect,
    left_group_rect: Rect,
    right_group_rect: Rect,
    active_group_rect: Rect,
    active_start_ms: f32,
    active_end_ms: f32,
}

pub struct FallingGroundEditor {
    chart_path: String,
    notes: Vec<GroundNote>,
    next_note_id: u64,
    selected_note_id: Option<u64>,
    drag_state: Option<DragState>,
    timeline: BpmTimeline,
    track_timeline: TrackTimeline,
    track_source: TrackSourceData,
    track_speed_enabled: bool,
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
    text_font: Option<Font>,
    status: String,
}

