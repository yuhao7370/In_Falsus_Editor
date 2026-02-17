use macroquad::prelude::*;

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
    current_sec: f32,
    duration_sec: f32,
    is_playing: bool,
    font: Option<&Font>,
    state: &mut TopProgressBarState,
) -> TopProgressBarOutput {
    let mut display_sec = current_sec;
    let mut seek_to_sec = None;

    let top_bar = Rect::new(0.0, menu_height, screen_width(), top_bar_height);
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
        screen_width(),
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
    let inside_progress = mx >= progress_rect.x
        && mx <= progress_rect.x + progress_rect.w
        && my >= progress_rect.y
        && my <= progress_rect.y + progress_rect.h;
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
            if is_mouse_button_pressed(MouseButton::Left) && inside_progress {
                seek_to_sec = Some(mouse_seek_sec);
                display_sec = mouse_seek_sec;
            }
        } else {
            if is_mouse_button_pressed(MouseButton::Left) && inside_progress {
                state.drag_active = true;
                state.seek_sec = mouse_seek_sec;
            }
            if state.drag_active && is_mouse_button_down(MouseButton::Left) {
                state.seek_sec = mouse_seek_sec;
            }
            if state.drag_active && is_mouse_button_released(MouseButton::Left) {
                state.drag_active = false;
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
    }
}
