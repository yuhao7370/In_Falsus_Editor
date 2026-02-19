#[derive(Debug, Clone, Copy)]
struct DragState {
    note_id: u64,
    time_offset_ms: f32,
    start_time_sec: f64,
    start_mouse_x: f32,
    start_mouse_y: f32,
    /// (mouse_lane - note.lane) at drag start.
    lane_offset: i32,
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
enum MultiDragMode {
    GroundFull,
    AirFull,
    TimeOnly,
}

#[derive(Debug, Clone, Copy)]
struct MultiDragNoteSnapshot {
    note_id: u64,
    original_time_ms: f32,
    original_lane: usize,
    original_width: f32,
    original_center_x_norm: f32,
    original_duration_ms: f32,
    sky_start_left: f32,
    sky_start_right: f32,
    sky_end_left: f32,
    sky_end_right: f32,
}

#[derive(Debug, Clone, Copy)]
struct MultiDragBinding {
    note_index: usize,
    snapshot: MultiDragNoteSnapshot,
}

#[derive(Debug, Clone)]
struct MultiDragState {
    anchor_note_id: u64,
    time_offset_ms: f32,
    lane_offset: i32,
    start_time_sec: f64,
    start_mouse_x: f32,
    start_mouse_y: f32,
    mode: MultiDragMode,
    common_x_split: f64,
    scope: HitScope,
    bindings: Vec<MultiDragBinding>,
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

#[derive(Debug, Clone, Copy)]
struct BoxSelectState {
    start_x: f32,
    start_y: f32,
    current_x: f32,
    current_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteMode {
    Normal,
    Mirrored,
}
