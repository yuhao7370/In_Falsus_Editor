// Falling editor constants.

const LANE_COUNT: usize = 6;

const DEFAULT_SKYAREA_WIDTH_NORM: f32 = 0.25;

const DEFAULT_SCROLL_SPEED: f32 = 1.0;
const MIN_SCROLL_SPEED: f32 = 0.2;
const MAX_SCROLL_SPEED: f32 = 8.0;
const SCROLL_SPEED_STEP: f32 = 0.05;

pub const SNAP_DIVISION_OPTIONS: [u32; 12] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64];

const AIR_SKYAREA_HEAD_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.84);
const AIR_SKYAREA_BODY_COLOR: Color = Color::new(0.72, 0.60, 0.98, 0.42);
const AIR_SKYAREA_TAIL_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.34);
// SkyArea body tessellation count used by both draw and cache sampling.
const SKYAREA_SEGMENT_COUNT: usize = 20;

const DRAG_HOLD_TO_START_SEC: f64 = 0.22;
const FLICK_SIDE_HEIGHT_SCALE: f32 = 1.5;

const PORTRAIT_SCREEN_RATIO: f32 = 10.0 / 16.0;

const OVERLAP_CYCLE_ANCHOR_PX: f32 = 14.0;
const OVERLAP_DOUBLE_CLICK_SEC: f64 = 0.20;

const NOTE_HEAD_HIT_HALF_H: f32 = 9.0;
const NOTE_HEAD_HIT_PAD_X: f32 = 2.0;
const FLICK_HITBOX_EXPAND_PX: f32 = 4.0;
const NOTE_BODY_HIT_PAD_X: f32 = 2.0;
const NOTE_BODY_EDGE_GAP_Y: f32 = 8.0;

const SELECTED_NOTE_DARKEN_ALPHA: u8 = 72;

const MINIMAP_DRAG_EMIT_EPS_SEC: f32 = 0.002;
