use crate::editor::falling::SNAP_DIVISION_OPTIONS;
use egui_macroquad::egui;

/// Total widget height (labels + track + bottom ticks).
const WIDGET_HEIGHT: f32 = 38.0;
/// Height of the track area between the two rail lines.
const RAIL_GAP: f32 = 14.0;
/// Tick mark length protruding inward from each rail line.
const TICK_LEN: f32 = 3.5;
/// Diamond (rhombus) half-size for the thumb indicator.
const DIAMOND_HALF: f32 = 5.0;
/// Label font size.
const LABEL_SIZE: f32 = 10.0;
/// Only show labels for these divisions to avoid clutter.
const LABEL_DIVISIONS: &[u32] = &[1, 4, 8, 16, 32, 64];

/// A custom discrete snap-division slider styled after RotaenoChartTool.
/// Two horizontal rail lines with tick marks and a diamond thumb.
/// Returns `Some(new_division)` when the user changes the value.
pub fn draw_snap_slider(ui: &mut egui::Ui, current: u32, width: f32) -> Option<u32> {
    let options = &SNAP_DIVISION_OPTIONS;
    let count = options.len();
    let current_idx = options.iter().position(|&v| v == current).unwrap_or(0);

    let desired = egui::vec2(width.max(100.0), WIDGET_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    let painter = ui.painter_at(rect);

    // Layout: labels on top, then rail area
    let label_area_h = LABEL_SIZE + 3.0;
    let rail_top = rect.top() + label_area_h;
    let rail_bottom = rail_top + RAIL_GAP;
    let rail_cy = (rail_top + rail_bottom) * 0.5;

    // Horizontal inset so diamond doesn't clip
    let x_pad = DIAMOND_HALF + 4.0;
    let x_min = rect.left() + x_pad;
    let x_max = rect.right() - x_pad;
    let x_range = (x_max - x_min).max(1.0);

    // Map index to x position
    let idx_to_x = |idx: usize| -> f32 {
        if count <= 1 {
            (x_min + x_max) * 0.5
        } else {
            x_min + (idx as f32 / (count - 1) as f32) * x_range
        }
    };

    // Interaction: determine new index
    let mut new_idx = current_idx;
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let t = ((pos.x - x_min) / x_range).clamp(0.0, 1.0);
            let float_idx = t * (count - 1) as f32;
            new_idx = float_idx.round() as usize;
            new_idx = new_idx.min(count - 1);
        }
    }

    // Colors
    let rail_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 100);
    let tick_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80);
    let label_color = egui::Color32::from_rgba_unmultiplied(200, 200, 210, 180);
    let label_active_color = egui::Color32::from_rgb(106, 168, 255);
    let diamond_color = egui::Color32::from_rgb(106, 168, 255);
    let diamond_hover = egui::Color32::from_rgb(150, 200, 255);
    let rail_stroke = egui::Stroke::new(1.0, rail_color);
    let tick_stroke = egui::Stroke::new(1.0, tick_color);

    // Draw top rail line
    painter.line_segment(
        [egui::pos2(x_min - 3.0, rail_top), egui::pos2(x_max + 3.0, rail_top)],
        rail_stroke,
    );
    // Draw bottom rail line
    painter.line_segment(
        [egui::pos2(x_min - 3.0, rail_bottom), egui::pos2(x_max + 3.0, rail_bottom)],
        rail_stroke,
    );

    // Draw ticks and labels for each option
    let font = egui::FontId::proportional(LABEL_SIZE);
    for i in 0..count {
        let x = idx_to_x(i);
        let val = options[i];

        // Top tick (downward from top rail)
        painter.line_segment(
            [egui::pos2(x, rail_top), egui::pos2(x, rail_top + TICK_LEN)],
            tick_stroke,
        );
        // Bottom tick (upward from bottom rail)
        painter.line_segment(
            [egui::pos2(x, rail_bottom), egui::pos2(x, rail_bottom - TICK_LEN)],
            tick_stroke,
        );

        // Label (only for key divisions)
        if LABEL_DIVISIONS.contains(&val) {
            let text = format!("x{}", val);
            let is_active = i == new_idx;
            let color = if is_active { label_active_color } else { label_color };
            painter.text(
                egui::pos2(x, rail_top - 2.0),
                egui::Align2::CENTER_BOTTOM,
                &text,
                font.clone(),
                color,
            );
        }
    }

    // Draw diamond thumb at current position
    let thumb_x = idx_to_x(new_idx);
    let is_hot = response.hovered() || response.dragged();
    let dc = if is_hot { diamond_hover } else { diamond_color };

    // Diamond as a rotated square (4 vertices)
    let diamond_points = vec![
        egui::pos2(thumb_x, rail_cy - DIAMOND_HALF),       // top
        egui::pos2(thumb_x + DIAMOND_HALF, rail_cy),       // right
        egui::pos2(thumb_x, rail_cy + DIAMOND_HALF),       // bottom
        egui::pos2(thumb_x - DIAMOND_HALF, rail_cy),       // left
    ];
    painter.add(egui::Shape::convex_polygon(
        diamond_points,
        dc,
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
    ));

    // Return changed value
    let new_val = options[new_idx];
    if new_val != current {
        Some(new_val)
    } else {
        None
    }
}

/// Vertical snap slider: rail runs top-to-bottom, values increase upward (1 at bottom, 64 at top).
/// Every division value is labeled. Returns `Some(new_division)` on change.
pub fn draw_snap_slider_vertical(ui: &mut egui::Ui, current: u32, height: f32) -> Option<u32> {
    let options = &SNAP_DIVISION_OPTIONS;
    let count = options.len();
    let current_idx = options.iter().position(|&v| v == current).unwrap_or(0);

    // Widget dimensions
    let label_w = 16.0_f32; // space for "64" text
    let rail_w = 8.0_f32;
    let widget_w = label_w + rail_w + 2.0;
    let desired = egui::vec2(widget_w, height.max(100.0));
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    let painter = ui.painter_at(rect);

    // Vertical rail area
    let y_pad = 8.0_f32;
    let y_min = rect.top() + y_pad;
    let y_max = rect.bottom() - y_pad;
    let y_range = (y_max - y_min).max(1.0);

    // Rail x positions (two vertical lines)
    let rail_left = rect.left() + label_w + 2.0;
    let rail_right = rail_left + rail_w;
    let rail_cx = (rail_left + rail_right) * 0.5;

    // Map index to y: index 0 (value 1) at top, last index (value 64) at bottom
    let idx_to_y = |idx: usize| -> f32 {
        if count <= 1 {
            (y_min + y_max) * 0.5
        } else {
            y_min + (idx as f32 / (count - 1) as f32) * y_range
        }
    };

    // Interaction
    let mut new_idx = current_idx;
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            // top = index 0 (value 1), bottom = last index (value 64)
            let t = ((pos.y - y_min) / y_range).clamp(0.0, 1.0);
            let float_idx = t * (count - 1) as f32;
            new_idx = float_idx.round() as usize;
            new_idx = new_idx.min(count - 1);
        }
    }

    // Colors
    let rail_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80);
    let tick_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60);
    let label_color = egui::Color32::from_rgba_unmultiplied(180, 180, 190, 180);
    let label_active = egui::Color32::from_rgb(106, 168, 255);
    // Draw left rail
    painter.line_segment(
        [egui::pos2(rail_left, y_min - 2.0), egui::pos2(rail_left, y_max + 2.0)],
        egui::Stroke::new(1.0, rail_color),
    );
    // Draw right rail
    painter.line_segment(
        [egui::pos2(rail_right, y_min - 2.0), egui::pos2(rail_right, y_max + 2.0)],
        egui::Stroke::new(1.0, rail_color),
    );

    // Ticks and labels
    let tick_len = 3.0_f32;
    let font = egui::FontId::proportional(10.0);
    for i in 0..count {
        let y = idx_to_y(i);
        let val = options[i];

        // Left tick (inward from left rail)
        painter.line_segment(
            [egui::pos2(rail_left, y), egui::pos2(rail_left + tick_len, y)],
            egui::Stroke::new(1.0, tick_color),
        );
        // Right tick (inward from right rail)
        painter.line_segment(
            [egui::pos2(rail_right, y), egui::pos2(rail_right - tick_len, y)],
            egui::Stroke::new(1.0, tick_color),
        );

        // Label to the left of the rail
        let is_active = i == new_idx;
        let color = if is_active { label_active } else { label_color };
        painter.text(
            egui::pos2(rail_left - 3.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{}", val),
            font.clone(),
            color,
        );
    }

    // Circle thumb (gray fill + white stroke)
    let thumb_y = idx_to_y(new_idx);
    let is_hot = response.hovered() || response.dragged();
    let fill = if is_hot {
        egui::Color32::from_rgb(160, 160, 168)
    } else {
        egui::Color32::from_rgb(120, 120, 128)
    };
    let radius = 5.0_f32;
    painter.circle(
        egui::pos2(rail_cx, thumb_y),
        radius,
        fill,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(240, 240, 240)),
    );

    let new_val = options[new_idx];
    if new_val != current { Some(new_val) } else { None }
}
