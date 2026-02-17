// 文件说明：集中定义下落编辑器的常量配置。
// 主要功能：提供轨道数量、布局尺寸、颜色和默认行为参数。
/// 地面轨道数量（6K）。
const LANE_COUNT: usize = 6;

/// 自适应缩放的参考分辨率宽度（设计稿宽度）。
const REFERENCE_WIDTH: f32 = 1366.0;
/// 自适应缩放的参考分辨率高度（设计稿高度）。
const REFERENCE_HEIGHT: f32 = 768.0;

/// 新建空中 Flick 的默认归一化宽度（相对空轨区域）。
const DEFAULT_AIR_WIDTH_NORM: f32 = 0.5;
/// 新建 SkyArea 的默认归一化宽度（相对空轨区域）。
const DEFAULT_SKYAREA_WIDTH_NORM: f32 = 0.25;

/// 默认下落速度（单位：屏高/秒）。
const DEFAULT_SCROLL_SPEED: f32 = 1.25;
/// 下落速度下限，防止过慢导致编辑反馈滞后。
const MIN_SCROLL_SPEED: f32 = 0.2;
/// 下落速度上限，防止过快导致无法精确操作。
const MAX_SCROLL_SPEED: f32 = 4.0;
/// 每次按钮调整速度的步进值。
const SCROLL_SPEED_STEP: f32 = 0.1;

/// 可选的节拍吸附分母（1/N）。
pub const SNAP_DIVISION_OPTIONS: [u32; 10] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32];

/// SkyArea 头/体/尾的可视化颜色。
const AIR_SKYAREA_HEAD_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.84);
const AIR_SKYAREA_BODY_COLOR: Color = Color::new(0.72, 0.60, 0.98, 0.42);
const AIR_SKYAREA_TAIL_COLOR: Color = Color::new(0.78, 0.66, 1.0, 0.34);

/// 按住多长时间判定为“拖拽”，避免点按误触发拖动。
const DRAG_HOLD_TO_START_SEC: f64 = 0.22;
/// SkyArea 垂直拖动触发阈值（像素）。
const SKYAREA_VERTICAL_DRAG_THRESHOLD_PX: f32 = 4.0;

/// 竖屏分栏判定比例（宽/高 <= 10/16）。
const PORTRAIT_SCREEN_RATIO: f32 = 10.0 / 16.0;

/// 重叠音符循环选择时的锚点网格尺寸（像素）。
const OVERLAP_CYCLE_ANCHOR_PX: f32 = 14.0;
/// 两次点击在该时间内视为双击（秒）。
const OVERLAP_DOUBLE_CLICK_SEC: f64 = 0.20;

/// 音符头部命中框半高（像素）。
const NOTE_HEAD_HIT_HALF_H: f32 = 9.0;
/// 音符头部命中框水平扩展（像素）。
const NOTE_HEAD_HIT_PAD_X: f32 = 2.0;
/// 音符主体命中框水平扩展（像素）。
const NOTE_BODY_HIT_PAD_X: f32 = 2.0;
/// 音符主体命中框上下边缘内缩（像素）。
const NOTE_BODY_EDGE_GAP_Y: f32 = 8.0;

/// 选中态覆盖层的暗化透明度。
const SELECTED_NOTE_DARKEN_ALPHA: u8 = 72;

/// 小地图拖拽时触发 seek 事件的最小时间间隔（秒）。
const MINIMAP_DRAG_EMIT_EPS_SEC: f32 = 0.002;
