// 文件说明：编辑器状态数据结构定义。
// 主要功能：定义运行时状态、交互上下文与缓存字段。
#[derive(Debug, Clone, Copy)]
struct DragState {
    note_id: u64,
    time_offset_ms: f32,
    start_time_sec: f64,
    start_mouse_x: f32,
    start_mouse_y: f32,
    /// 拖拽开始时鼠标所在轨道与音符 lane 的偏移（mouse_lane - note.lane）。
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
    /// 仅地面音符：支持 lane 平移 + 时间移动
    GroundFull,
    /// 仅天空音符且 x_split 相同：支持 X 平移 + 时间移动
    AirFull,
    /// 混合选择 或 天空音符 x_split 不同：仅时间移动
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
    // SkyArea shape snapshot
    sky_start_left: f32,
    sky_start_right: f32,
    sky_end_left: f32,
    sky_end_right: f32,
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
    initial_notes: Vec<MultiDragNoteSnapshot>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineEventKind {
    Bpm,
    Track,
    Lane,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    id: u64,
    kind: TimelineEventKind,
    source_index: usize,
    time_ms: f32,
    label: String,
    color: Color,
}

#[derive(Debug, Clone)]
struct EventOverlapCycle {
    /// 当前列内重叠候选的 event id 列表
    candidates: Vec<u64>,
    /// 当前选中在 candidates 中的索引
    current_index: usize,
    /// 所属列 (0=Bpm, 1=Track, 2=Lane)
    col: usize,
    /// 锚点 y（量化后）
    anchor_y: i32,
    /// 上次点击时间（秒）
    last_click_time_sec: f64,
    /// 是否已准备好双击切换
    double_click_armed: bool,
}

#[derive(Debug, Clone, Copy)]
struct EventHoverOverlapHint {
    mouse_x: f32,
    mouse_y: f32,
    current_index: usize,
    total: usize,
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

#[derive(Debug, Clone, Copy)]
struct BoxSelectState {
    start_x: f32,
    start_y: f32,
    current_x: f32,
    current_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteMode {
    /// 普通粘贴
    Normal,
    /// 镜像粘贴
    Mirrored,
}

/// Snapshot of editor state for undo/redo.
#[derive(Debug, Clone)]
struct EditorSnapshot {
    notes: Vec<GroundNote>,
    next_note_id: u64,
    timeline_events: Vec<TimelineEvent>,
    next_event_id: u64,
    bpm_source: BpmSourceData,
    track_source: TrackSourceData,
}

/// Undo/Redo history manager.
#[derive(Debug)]
struct UndoHistory {
    stack: Vec<EditorSnapshot>,
    index: usize, // points to current state
    max_size: usize,
}

impl UndoHistory {
    fn new(max_size: usize) -> Self {
        Self {
            stack: Vec::new(),
            index: 0,
            max_size,
        }
    }

    fn push(&mut self, snapshot: EditorSnapshot) {
        // Discard any redo states
        if self.index + 1 < self.stack.len() {
            self.stack.truncate(self.index + 1);
        }
        self.stack.push(snapshot);
        if self.stack.len() > self.max_size {
            self.stack.remove(0);
        }
        self.index = self.stack.len().saturating_sub(1);
    }

    fn is_at_top(&self) -> bool {
        !self.stack.is_empty() && self.index == self.stack.len() - 1
    }

    fn can_undo(&self) -> bool {
        self.index > 0
    }

    fn can_redo(&self) -> bool {
        self.index + 1 < self.stack.len()
    }

    fn undo(&mut self) -> Option<&EditorSnapshot> {
        if self.can_undo() {
            self.index -= 1;
            Some(&self.stack[self.index])
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<&EditorSnapshot> {
        if self.can_redo() {
            self.index += 1;
            Some(&self.stack[self.index])
        } else {
            None
        }
    }
}

pub struct FallingGroundEditor {
    chart_path: String,
    notes: Vec<GroundNote>,
    next_note_id: u64,
    selected_note_id: Option<u64>,
    selected_note_ids: HashSet<u64>,
    drag_state: Option<DragState>,
    multi_drag_state: Option<MultiDragState>,
    timeline: BpmTimeline,
    track_timeline: TrackTimeline,
    track_source: TrackSourceData,
    track_speed_enabled: bool,
    cached_barlines: Vec<BarLine>,
    cached_barlines_subdivision: u32,
    timeline_events: Vec<TimelineEvent>,
    selected_event_id: Option<u64>,
    selected_event_ids: HashSet<u64>,
    event_overlap_cycle: Option<EventOverlapCycle>,
    event_hover_hint: Option<EventHoverOverlapHint>,
    next_event_id: u64,
    snap_enabled: bool,
    snap_division: u32,
    scroll_speed: f32,
    render_scope: RenderScope,
    place_note_type: Option<PlaceNoteType>,
    place_event_type: Option<PlaceEventType>,
    place_flick_right: bool,
    pending_hold: Option<PendingHoldPlacement>,
    pending_skyarea: Option<PendingSkyAreaPlacement>,
    overlap_cycle: Option<OverlapCycleState>,
    hover_overlap_hint: Option<HoverOverlapHint>,
    debug_show_hitboxes: bool,
    autoplay_enabled: bool,
    show_spectrum: bool,
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
    undo_history: UndoHistory,
    x_split: f64,
    xsplit_editable: bool,
    dirty: bool,
    /// Backup of note being edited in property panel (for cancel/preview).
    editing_note_backup: Option<GroundNote>,
    /// Backup of event being edited in property panel (for cancel/preview).
    editing_event_backup: Option<TimelineEvent>,
    /// 框选状态
    box_select: Option<BoxSelectState>,
    /// 剪贴板：存储复制/剪切的音符
    clipboard: Vec<GroundNote>,
    /// 粘贴模式：Normal 或 Mirrored
    paste_mode: Option<PasteMode>,
}

