// 文件说明：调试绘制辅助函数集合。
// 主要功能：绘制选中遮罩、高亮边框等调试可视元素。
fn draw_selected_note_darken_rect(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(
        x,
        y,
        w.max(1.0),
        h.max(1.0),
        Color::from_rgba(0, 0, 0, SELECTED_NOTE_DARKEN_ALPHA),
    );
}

fn draw_selected_note_outline(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle_lines(
        x - 1.4,
        y - 1.4,
        (w + 2.8).max(1.0),
        (h + 2.8).max(1.0),
        2.4,
        Color::from_rgba(255, 212, 102, 255),
    );
    draw_rectangle_lines(
        x + 0.8,
        y + 0.8,
        (w - 1.6).max(1.0),
        (h - 1.6).max(1.0),
        1.2,
        Color::from_rgba(255, 244, 170, 236),
    );
}

fn draw_debug_hitbox_rect(hit: Rect, clip: Rect, color: Color, thickness: f32) {
    let x1 = hit.x.max(clip.x);
    let y1 = hit.y.max(clip.y);
    let x2 = (hit.x + hit.w).min(clip.x + clip.w);
    let y2 = (hit.y + hit.h).min(clip.y + clip.h);
    if x2 <= x1 || y2 <= y1 {
        return;
    }
    draw_rectangle_lines(x1, y1, x2 - x1, y2 - y1, thickness, color);
}

fn draw_debug_hitbox_label(hit: Rect, clip: Rect, label: &str, color: Color, font: Option<&Font>) {
    let ui = adaptive_ui_scale();
    let x = hit.x.max(clip.x + 2.0 * ui);
    let y = (hit.y - 3.0 * ui).clamp(clip.y + 10.0 * ui, clip.y + clip.h - 2.0 * ui);
    draw_text_ex(
        label,
        x,
        y,
        TextParams {
            font,
            font_size: scaled_font_size(14.0, 10, 34),
            color,
            ..Default::default()
        },
    );
}

fn draw_debug_skyarea_body_hit_overlay(
    split_rect: Rect,
    shape: SkyAreaShape,
    head_y: f32,
    tail_y: f32,
    color: Color,
) {
    let dy = tail_y - head_y;
    if dy.abs() <= 0.000_1 {
        return;
    }

    let (body_top, body_bottom) = skyarea_body_vertical_range(head_y, tail_y);
    if body_bottom <= body_top {
        return;
    }

    let steps = 24;
    for i in 0..steps {
        let p0 = i as f32 / steps as f32;
        let p1 = (i + 1) as f32 / steps as f32;
        let y0 = lerp(head_y, tail_y, p0);
        let y1 = lerp(head_y, tail_y, p1);
        if (y0 < body_top && y1 < body_top) || (y0 > body_bottom && y1 > body_bottom) {
            continue;
        }

        let (l0, r0) = skyarea_screen_x_range_at_progress(split_rect, shape, p0);
        let (l1, r1) = skyarea_screen_x_range_at_progress(split_rect, shape, p1);
        let pad_x = scaled_px(NOTE_BODY_HIT_PAD_X);
        let x0l = l0.min(r0) - pad_x;
        let x0r = l0.max(r0) + pad_x;
        let x1l = l1.min(r1) - pad_x;
        let x1r = l1.max(r1) + pad_x;
        let yy0 = y0.clamp(body_top, body_bottom);
        let yy1 = y1.clamp(body_top, body_bottom);
        if (yy1 - yy0).abs() < 0.001 {
            continue;
        }

        draw_triangle(
            Vec2::new(x0l, yy0),
            Vec2::new(x0r, yy0),
            Vec2::new(x1r, yy1),
            Color::new(color.r, color.g, color.b, 0.12),
        );
        draw_triangle(
            Vec2::new(x0l, yy0),
            Vec2::new(x1r, yy1),
            Vec2::new(x1l, yy1),
            Color::new(color.r, color.g, color.b, 0.12),
        );
        draw_line(x0l, yy0, x1l, yy1, 1.1, color);
        draw_line(x0r, yy0, x1r, yy1, 1.1, color);
    }
}

