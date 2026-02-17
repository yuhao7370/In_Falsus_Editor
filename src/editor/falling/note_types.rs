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

