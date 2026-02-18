// 文件说明：下落编辑器音符与工具类型定义。
// 主要功能：定义音符枚举、编辑动作和共用数据结构。
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
pub enum PlaceEventType {
    Bpm,
    Track,
    Lane,
}

impl PlaceEventType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bpm => "Bpm",
            Self::Track => "Track",
            Self::Lane => "Lane",
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
    MinimapSeekTo(f32),
}

/// Public note property data for UI editing.
/// X / Width use the chart's raw coordinate system (not normalized).
/// x_split is displayed but locked (read-only in UI).
#[derive(Debug, Clone)]
pub struct NotePropertyData {
    pub id: u64,
    pub kind: String,
    pub lane: usize,
    pub time_ms: f32,
    pub beat: f32,
    pub duration_ms: f32,
    pub duration_beat: f32,
    pub width: f32,
    pub flick_right: bool,
    // Flick: raw x, x_split, width  (width reuses the `width` field above but in raw scale)
    pub x: f64,
    pub x_split: f64,
    // SkyArea: raw start/end x, x_split, width
    pub start_x: f64,
    pub start_x_split: f64,
    pub start_width: f64,
    pub end_x: f64,
    pub end_x_split: f64,
    pub end_width: f64,
    pub left_ease: i32,
    pub right_ease: i32,
}

/// Public event property data for UI editing.
#[derive(Debug, Clone)]
pub struct EventPropertyData {
    pub id: u64,
    pub kind: String,
    pub time_ms: f32,
    pub beat: f32,
    pub label: String,
    // BPM event params
    pub bpm: f32,
    pub beats_per_measure: f32,
    // Track event params
    pub speed: f32,
    // Lane event params
    pub lane: i32,
    pub enable: bool,
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
    /// Per-note x_split from the chart file. Only meaningful for Flick/SkyArea.
    x_split: f64,
    /// Precise normalized center X for air notes (0.0–1.0, continuous).
    /// Ground notes (Tap/Hold) don't use this; their position comes from `lane`.
    center_x_norm: f32,
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
    /// Per-shape x_split from the chart file.
    start_x_split: f64,
    end_x_split: f64,
}

