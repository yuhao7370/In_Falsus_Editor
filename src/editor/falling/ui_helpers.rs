fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_progress(ease: Ease, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        Ease::Linear => t,
        Ease::SineOut => (t * std::f32::consts::FRAC_PI_2).sin(),
        Ease::SineIn => 1.0 - (t * std::f32::consts::FRAC_PI_2).cos(),
    }
}

fn y_to_time_sec(y: f32, rect: Rect, duration_sec: f32) -> f32 {
    let t = ((y - rect.y) / rect.h).clamp(0.0, 1.0);
    (1.0 - t) * duration_sec
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn draw_small_button(rect: Rect, text: &str) -> bool {
    let ui = adaptive_ui_scale();
    let (mx, my) = mouse_position();
    let hovered = point_in_rect(mx, my, rect);
    let bg = if hovered {
        Color::from_rgba(104, 108, 138, 255)
    } else {
        Color::from_rgba(64, 68, 92, 255)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(150, 154, 186, 255),
    );

    let font_size = scaled_font_size(24.0, 12, 52);
    let metrics = measure_text(text, None, font_size, 1.0);
    draw_text_ex(
        text,
        rect.x + (rect.w - metrics.width) * 0.5,
        rect.y + rect.h * (0.68 + 0.04 / ui),
        TextParams {
            font_size,
            color: Color::from_rgba(235, 238, 255, 255),
            ..Default::default()
        },
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

