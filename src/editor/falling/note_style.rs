#[derive(Debug, Clone, Copy)]
struct LaneNotePalette {
    tap: Color,
    hold_head: Color,
    hold_body: Color,
    flick_head: Color,
    flick_arrow: Color,
    skyarea_head: Color,
    skyarea_body: Color,
}

fn lane_note_palette(lane: usize) -> LaneNotePalette {
    match lane {
        0 => LaneNotePalette {
            tap: Color::from_rgba(174, 118, 255, 255),
            hold_head: Color::from_rgba(202, 156, 255, 255),
            hold_body: Color::from_rgba(124, 84, 192, 212),
            flick_head: Color::from_rgba(192, 138, 255, 255),
            flick_arrow: Color::from_rgba(248, 224, 255, 255),
            skyarea_head: Color::from_rgba(160, 110, 238, 255),
            skyarea_body: Color::from_rgba(120, 84, 182, 124),
        },
        1 => LaneNotePalette {
            tap: Color::from_rgba(100, 206, 255, 255),
            hold_head: Color::from_rgba(129, 220, 255, 255),
            hold_body: Color::from_rgba(73, 145, 186, 212),
            flick_head: Color::from_rgba(128, 220, 255, 255),
            flick_arrow: Color::from_rgba(220, 245, 255, 255),
            skyarea_head: Color::from_rgba(82, 186, 236, 255),
            skyarea_body: Color::from_rgba(52, 130, 170, 124),
        },
        2 => LaneNotePalette {
            tap: Color::from_rgba(108, 220, 255, 255),
            hold_head: Color::from_rgba(138, 232, 255, 255),
            hold_body: Color::from_rgba(77, 156, 190, 212),
            flick_head: Color::from_rgba(140, 233, 255, 255),
            flick_arrow: Color::from_rgba(226, 248, 255, 255),
            skyarea_head: Color::from_rgba(90, 194, 238, 255),
            skyarea_body: Color::from_rgba(58, 136, 174, 124),
        },
        3 => LaneNotePalette {
            tap: Color::from_rgba(120, 216, 255, 255),
            hold_head: Color::from_rgba(149, 228, 255, 255),
            hold_body: Color::from_rgba(84, 153, 188, 212),
            flick_head: Color::from_rgba(148, 228, 255, 255),
            flick_arrow: Color::from_rgba(226, 248, 255, 255),
            skyarea_head: Color::from_rgba(96, 191, 238, 255),
            skyarea_body: Color::from_rgba(64, 134, 172, 124),
        },
        4 => LaneNotePalette {
            tap: Color::from_rgba(131, 205, 255, 255),
            hold_head: Color::from_rgba(161, 218, 255, 255),
            hold_body: Color::from_rgba(92, 142, 184, 212),
            flick_head: Color::from_rgba(162, 220, 255, 255),
            flick_arrow: Color::from_rgba(226, 244, 255, 255),
            skyarea_head: Color::from_rgba(106, 181, 232, 255),
            skyarea_body: Color::from_rgba(72, 122, 168, 124),
        },
        _ => LaneNotePalette {
            tap: Color::from_rgba(255, 112, 108, 255),
            hold_head: Color::from_rgba(255, 142, 138, 255),
            hold_body: Color::from_rgba(194, 82, 78, 212),
            flick_head: Color::from_rgba(255, 134, 128, 255),
            flick_arrow: Color::from_rgba(255, 228, 226, 255),
            skyarea_head: Color::from_rgba(238, 100, 94, 255),
            skyarea_body: Color::from_rgba(176, 72, 68, 124),
        },
    }
}

fn note_head_width(note: &GroundNote, lane_w: f32) -> f32 {
    match note.kind {
        GroundNoteKind::Hold | GroundNoteKind::SkyArea => lane_w * 0.94,
        GroundNoteKind::Tap | GroundNoteKind::Flick => lane_w * (0.78 * note.width.clamp(0.5, 1.2)),
    }
}

fn flick_direction_shape_colors(flick_right: bool) -> (Color, Color) {
    if flick_right {
        (
            Color::from_rgba(74, 216, 136, 136),
            Color::from_rgba(154, 255, 190, 242),
        )
    } else {
        (
            Color::from_rgba(238, 214, 84, 128),
            Color::from_rgba(255, 246, 154, 242),
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct FlickGeometry {
    x_start: f32,
    x_tip: f32,
    y_top: f32,
    y_bottom: f32,
    y_tip_top: f32,
    y_tip_bottom: f32,
    stroke: f32,
}

fn flick_geometry(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> FlickGeometry {
    let ui = adaptive_ui_scale();
    let stroke = (note_w * 0.05).clamp(1.0 * ui, 2.8 * ui);
    let side_h = side_h.max(0.0);
    // Align flick baseline with note/barline Y exactly.
    let y_bottom = head_y;
    let y_top = y_bottom - side_h;
    let y_tip_bottom = y_bottom;
    let y_tip_top = y_bottom - (side_h * 0.04).max(0.6 * ui);

    let (x_start, x_tip) = if note.flick_right {
        (note_x + note_w * 0.92, note_x + note_w * 0.02)
    } else {
        (note_x + note_w * 0.08, note_x + note_w * 0.98)
    };

    FlickGeometry {
        x_start,
        x_tip,
        y_top,
        y_bottom,
        y_tip_top,
        y_tip_bottom,
        stroke,
    }
}

fn draw_flick_curve_shape(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) {
    let (fill_color, edge_color) = flick_direction_shape_colors(note.flick_right);
    let geom = flick_geometry(note, note_x, note_w, head_y, side_h);

    let mut top_curve = Vec::with_capacity(25);
    for i in 0..=24 {
        let t = i as f32 / 24.0;
        let x = lerp(geom.x_start, geom.x_tip, t);
        let eased = ease_progress(Ease::SineOut, t);
        let y = lerp(geom.y_top, geom.y_tip_top, eased);
        top_curve.push(Vec2::new(x, y));
    }

    let mut polygon = Vec::with_capacity(28);
    polygon.push(Vec2::new(geom.x_start, geom.y_bottom));
    polygon.extend_from_slice(&top_curve);
    polygon.push(Vec2::new(geom.x_tip, geom.y_tip_bottom));

    for i in 1..(polygon.len() - 1) {
        draw_triangle(polygon[0], polygon[i], polygon[i + 1], fill_color);
    }

    for i in 0..(top_curve.len() - 1) {
        let a = top_curve[i];
        let b = top_curve[i + 1];
        draw_line(a.x, a.y, b.x, b.y, geom.stroke, edge_color);
    }
    draw_line(
        geom.x_start,
        geom.y_bottom,
        geom.x_tip,
        geom.y_tip_bottom,
        geom.stroke,
        edge_color,
    );
    draw_line(
        geom.x_start,
        geom.y_bottom,
        geom.x_start,
        geom.y_top,
        geom.stroke,
        edge_color,
    );
}

fn flick_shape_bounds(note: &GroundNote, note_x: f32, note_w: f32, head_y: f32, side_h: f32) -> Rect {
    let geom = flick_geometry(note, note_x, note_w, head_y, side_h);
    let x1 = geom.x_start.min(geom.x_tip);
    let x2 = geom.x_start.max(geom.x_tip);
    Rect::new(
        x1,
        geom.y_top,
        (x2 - x1).max(1.0),
        (geom.y_bottom - geom.y_top).max(1.0),
    )
}

