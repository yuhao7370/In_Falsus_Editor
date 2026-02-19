use macroquad::prelude::*;
use std::cell::Cell;

pub const BASE_WIDTH: f32 = 1366.0;
pub const BASE_HEIGHT: f32 = 768.0;
pub const UI_SCALE_MIN: f32 = 0.75;
pub const UI_SCALE_MAX: f32 = 3.5;

thread_local! {
    static CACHED_UI_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// 每帧开始时调用一次，刷新缓存的 ui_scale 值。
pub fn refresh_ui_scale() {
    let scale = ui_scale_factor_with(BASE_WIDTH, BASE_HEIGHT, UI_SCALE_MIN, UI_SCALE_MAX);
    CACHED_UI_SCALE.with(|c| c.set(scale));
}

pub fn ui_scale_factor() -> f32 {
    CACHED_UI_SCALE.with(|c| c.get())
}

pub fn ui_scale_factor_with(
    base_width: f32,
    base_height: f32,
    min_scale: f32,
    max_scale: f32,
) -> f32 {
    (screen_width() / base_width)
        .min(screen_height() / base_height)
        .clamp(min_scale, max_scale)
}

pub fn scaled_px(px: f32) -> f32 {
    px * ui_scale_factor()
}

pub fn scaled_font_size(base: f32, min: u16, max: u16) -> u16 {
    let size = (base * ui_scale_factor()).round();
    size.clamp(min as f32, max as f32) as u16
}
