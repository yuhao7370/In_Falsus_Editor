use crate::chart::{Chart, ChartEvent};
use egui_macroquad::egui;
use sasa::AudioClip;

/// 启动时默认加载的地面谱面文件。
const DEFAULT_CHART_PATH: &str = "grievouslady2.spc";
/// Shift+点击创建 Hold 时的默认时值（毫秒）。
const DEFAULT_HOLD_MS: f64 = 500.0;
/// 波形区高度（像素）。
const WAVEFORM_HEIGHT: f32 = 110.0;
/// 地面轨道数（6K）。
const LANE_COUNT: usize = 6;
/// 单条轨道可视高度（像素）。
const LANE_HEIGHT: f32 = 38.0;
/// 轨道之间的垂直间隔（像素）。
const LANE_GAP: f32 = 2.0;

include!("ground/types.rs");
include!("ground/methods_core.rs");
include!("ground/methods_pointer.rs");
include!("ground/methods_render.rs");
include!("ground/methods_note_ops.rs");
include!("ground/methods_chart_io.rs");
