use macroquad::prelude::*;
use crate::audio::controller::FrameContext;
use super::input_state::{
    is_pointer_blocked, safe_mouse_button_down, safe_mouse_button_pressed,
};

/// 通过系统原生 API 查询鼠标左键物理状态，不依赖窗口事件。
/// 解决鼠标拖出窗口外松开后 macroquad 收不到 MouseButtonUp 的问题。
#[cfg(target_os = "windows")]
fn is_left_mouse_physically_down() -> bool {
    unsafe extern "system" {
        fn GetAsyncKeyState(vKey: i32) -> i16;
    }
    const VK_LBUTTON: i32 = 0x01;
    unsafe { GetAsyncKeyState(VK_LBUTTON) & (1i16 << 15) != 0 }
}

#[cfg(target_os = "macos")]
fn is_left_mouse_physically_down() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceButtonState(state_id: i32, button: u32) -> bool;
    }
    const K_CG_EVENT_SOURCE_STATE_COMBINED_SESSION: i32 = 0;
    const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
    unsafe {
        CGEventSourceButtonState(K_CG_EVENT_SOURCE_STATE_COMBINED_SESSION, K_CG_MOUSE_BUTTON_LEFT)
    }
}

#[cfg(target_os = "linux")]
fn is_left_mouse_physically_down() -> bool {
    use std::ptr;
    unsafe extern "C" {
        fn XOpenDisplay(name: *const i8) -> *mut std::ffi::c_void;
        fn XDefaultRootWindow(display: *mut std::ffi::c_void) -> u64;
        fn XQueryPointer(
            display: *mut std::ffi::c_void, w: u64,
            root_return: *mut u64, child_return: *mut u64,
            root_x: *mut i32, root_y: *mut i32,
            win_x: *mut i32, win_y: *mut i32,
            mask_return: *mut u32,
        ) -> i32;
        fn XCloseDisplay(display: *mut std::ffi::c_void) -> i32;
    }
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return is_mouse_button_down(MouseButton::Left);
        }
        let root = XDefaultRootWindow(display);
        let (mut rr, mut cr) = (0u64, 0u64);
        let (mut rx, mut ry, mut wx, mut wy) = (0i32, 0i32, 0i32, 0i32);
        let mut mask = 0u32;
        XQueryPointer(
            display, root, &mut rr, &mut cr,
            &mut rx, &mut ry, &mut wx, &mut wy, &mut mask,
        );
        XCloseDisplay(display);
        const BUTTON1_MASK: u32 = 1 << 8;
        mask & BUTTON1_MASK != 0
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn is_left_mouse_physically_down() -> bool {
    is_mouse_button_down(MouseButton::Left)
}

#[derive(Debug, Clone, Copy)]
pub struct TopProgressBarState {
    drag_active: bool,
    seek_sec: f32,
}

impl TopProgressBarState {
    pub const fn new() -> Self {
        Self {
            drag_active: false,
            seek_sec: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TopProgressBarOutput {
    pub display_sec: f32,
    pub seek_to_sec: Option<f32>,
    pub blocks_editor_pointer: bool,
}

fn format_time(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00.00".to_owned();
    }
    let minutes = (seconds / 60.0).floor() as i32;
    let sec = seconds % 60.0;
    format!("{minutes:02}:{sec:05.2}")
}

pub fn draw_top_progress_bar(
    ui_scale: f32,
    menu_height: f32,
    top_bar_height: f32,
    note_panel_width_px: f32,
    frame_ctx: &FrameContext,
    font: Option<&Font>,
    state: &mut TopProgressBarState,
) -> TopProgressBarOutput {
    let current_sec = frame_ctx.current_sec;
    let duration_sec = frame_ctx.duration_sec;
    let is_playing = frame_ctx.is_playing;
    let pointer_blocked = is_pointer_blocked();
    let dragging_progress = state.drag_active;
    let mut display_sec = current_sec;
    let mut seek_to_sec = None;

    // Background spans full width (extends under snap slider panel)
    let full_w = screen_width();
    let top_bar = Rect::new(0.0, menu_height, full_w, top_bar_height);
    draw_rectangle(
        top_bar.x,
        top_bar.y,
        top_bar.w,
        top_bar.h,
        Color::from_rgba(15, 15, 20, 255),
    );
    draw_line(
        0.0,
        top_bar.y + top_bar.h,
        full_w,
        top_bar.y + top_bar.h,
        (1.0 * ui_scale).max(1.0),
        Color::from_rgba(40, 40, 50, 255),
    );

    let progress_h = (top_bar_height - 8.0 * ui_scale).max(8.0);
    let progress_side_pad = 4.0 * ui_scale;
    let progress_rect = Rect::new(
        progress_side_pad,
        menu_height + (top_bar_height - progress_h) * 0.5,
        (screen_width() - note_panel_width_px - progress_side_pad * 2.0).max(120.0),
        progress_h,
    );
    let (mx, my) = mouse_position();
    // If the drag started on the progress bar, keep tracking even when pointer enters egui panels.
    let inside_progress = (!pointer_blocked || dragging_progress)
        && mx >= progress_rect.x
        && mx <= progress_rect.x + progress_rect.w
        && my >= progress_rect.y
        && my <= progress_rect.y + progress_rect.h;
    let left_pressed_on_progress = safe_mouse_button_pressed(MouseButton::Left) && inside_progress;
    let left_down_on_progress = safe_mouse_button_down(MouseButton::Left) && inside_progress;
    let frame_border = if inside_progress || state.drag_active {
        Color::from_rgba(108, 122, 154, 255)
    } else {
        Color::from_rgba(66, 74, 98, 255)
    };
    draw_rectangle(
        progress_rect.x,
        progress_rect.y,
        progress_rect.w,
        progress_rect.h,
        Color::from_rgba(18, 21, 30, 255),
    );
    draw_rectangle_lines(
        progress_rect.x,
        progress_rect.y,
        progress_rect.w,
        progress_rect.h,
        (1.0 * ui_scale).max(1.0),
        frame_border,
    );

    let inner_rect = progress_rect;
    draw_rectangle(
        inner_rect.x,
        inner_rect.y,
        inner_rect.w,
        inner_rect.h,
        Color::from_rgba(10, 13, 20, 255),
    );
    draw_rectangle(
        inner_rect.x,
        inner_rect.y,
        inner_rect.w,
        inner_rect.h * 0.46,
        Color::from_rgba(255, 255, 255, 10),
    );
    draw_rectangle_lines(
        progress_rect.x,
        progress_rect.y,
        progress_rect.w,
        progress_rect.h,
        (1.0 * ui_scale).max(1.0),
        frame_border,
    );

    if duration_sec > 0.001 {
        let mouse_seek_sec = {
            let t = ((mx - inner_rect.x) / inner_rect.w).clamp(0.0, 1.0);
            duration_sec * t
        };

        if is_playing {
            state.drag_active = false;
            if left_pressed_on_progress {
                seek_to_sec = Some(mouse_seek_sec);
                display_sec = mouse_seek_sec;
            }
        } else {
            if left_pressed_on_progress {
                state.drag_active = true;
                state.seek_sec = mouse_seek_sec;
            }
            // Use OS-level API to check physical mouse state, bypassing macroquad's
            // window event system which misses MouseButtonUp when released outside window.
            let drag_down = if state.drag_active {
                is_left_mouse_physically_down()
            } else {
                safe_mouse_button_down(MouseButton::Left)
            };
            if state.drag_active && drag_down {
                // When mouse is at the window edge, it likely left the window —
                // extrapolate to the nearest boundary so fast drags reach 0 or end.
                if mx <= 1.0 {
                    state.seek_sec = 0.0;
                } else if mx >= screen_width() - 1.0 {
                    state.seek_sec = duration_sec;
                } else {
                    state.seek_sec = mouse_seek_sec;
                }
            }
            if state.drag_active && !drag_down {
                state.drag_active = false;
                // macroquad reports (0,0) when mouse is outside the window.
                // Use that to extrapolate: if mouse left from the left side, seek to 0;
                // if from the right side, seek to end.
                if mx <= 1.0 {
                    state.seek_sec = 0.0;
                } else if mx >= screen_width() - 1.0 {
                    state.seek_sec = duration_sec;
                }
                seek_to_sec = Some(state.seek_sec);
                display_sec = state.seek_sec.clamp(0.0, duration_sec);
            } else if state.drag_active {
                display_sec = state.seek_sec.clamp(0.0, duration_sec);
            }
        }
    } else {
        state.drag_active = false;
    }

    let progress = if duration_sec > 0.001 {
        (display_sec / duration_sec).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill_w = (inner_rect.w * progress).max(0.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::new(inner_rect.x, inner_rect.y, fill_w, inner_rect.h);
        draw_rectangle(
            fill_rect.x,
            fill_rect.y,
            fill_rect.w,
            fill_rect.h,
            Color::from_rgba(92, 146, 224, 220),
        );
        let grad_start = Color::from_rgba(76, 126, 204, 238);
        let grad_end = Color::from_rgba(136, 196, 255, 246);
        let seg_count = ((fill_rect.w / (10.0 * ui_scale).max(1.0)).ceil() as i32).clamp(1, 64);
        for i in 0..seg_count {
            let t0 = i as f32 / seg_count as f32;
            let t1 = (i + 1) as f32 / seg_count as f32;
            let x0 = fill_rect.x + fill_rect.w * t0;
            let x1 = fill_rect.x + fill_rect.w * t1;
            let t = (t0 + t1) * 0.5;
            let color = Color::new(
                grad_start.r + (grad_end.r - grad_start.r) * t,
                grad_start.g + (grad_end.g - grad_start.g) * t,
                grad_start.b + (grad_end.b - grad_start.b) * t,
                grad_start.a + (grad_end.a - grad_start.a) * t,
            );
            draw_rectangle(x0, fill_rect.y, (x1 - x0).max(1.0), fill_rect.h, color);
        }
        draw_rectangle(
            fill_rect.x,
            fill_rect.y,
            fill_rect.w,
            fill_rect.h * 0.48,
            Color::from_rgba(224, 240, 255, 26),
        );
        draw_rectangle(
            fill_rect.x,
            fill_rect.y + fill_rect.h - (2.0 * ui_scale).max(1.0),
            fill_rect.w,
            (2.0 * ui_scale).max(1.0),
            Color::from_rgba(26, 56, 92, 94),
        );
    }

    let playhead_x = inner_rect.x + inner_rect.w * progress;
    draw_line(
        playhead_x,
        inner_rect.y - 0.5 * ui_scale,
        playhead_x,
        inner_rect.y + inner_rect.h + 0.5 * ui_scale,
        (3.0 * ui_scale).max(1.0),
        Color::from_rgba(154, 204, 255, 64),
    );
    draw_line(
        playhead_x,
        inner_rect.y,
        playhead_x,
        inner_rect.y + inner_rect.h,
        (1.4 * ui_scale).max(1.0),
        Color::from_rgba(214, 236, 255, 232),
    );

    let time_text = format!(
        "{} / {}",
        format_time(display_sec),
        format_time(duration_sec)
    );
    let mut time_font_size = (progress_h * 0.62).round().clamp(12.0, 28.0) as u16;
    let mut time_metrics = measure_text(&time_text, font, time_font_size, 1.0);
    let max_text_w = (inner_rect.w - 16.0 * ui_scale).max(40.0);
    while time_font_size > 11 && time_metrics.width > max_text_w {
        time_font_size -= 1;
        time_metrics = measure_text(&time_text, font, time_font_size, 1.0);
    }
    let time_x = inner_rect.x + (inner_rect.w - time_metrics.width) * 0.5;
    let time_y = inner_rect.y + (inner_rect.h - time_metrics.height) * 0.5 + time_metrics.offset_y;
    draw_text_ex(
        &time_text,
        time_x + 1.0 * ui_scale,
        time_y + 1.0 * ui_scale,
        TextParams {
            font,
            font_size: time_font_size,
            color: Color::from_rgba(8, 12, 22, 186),
            ..Default::default()
        },
    );
    draw_text_ex(
        &time_text,
        time_x,
        time_y,
        TextParams {
            font,
            font_size: time_font_size,
            color: Color::from_rgba(238, 245, 255, 255),
            ..Default::default()
        },
    );

    TopProgressBarOutput {
        display_sec,
        seek_to_sec,
        blocks_editor_pointer: left_pressed_on_progress || left_down_on_progress || state.drag_active,
    }
}
