use crate::editor::falling::SNAP_DIVISION_OPTIONS;
use egui_macroquad::egui;

/// Total widget height (labels + track + bottom ticks).
const WIDGET_HEIGHT: f32 = 38.0;
/// Height of the track area between the two rail lines.
const RAIL_GAP: f32 = 14.0;
/// Tick mark length protruding inward from each rail line.
const TICK_LEN: f32 = 3.5;
/// Circle thumb radius.
const DIAMOND_HALF: f32 = 5.0;
/// Label font size.
const LABEL_SIZE: f32 = 10.0;

/// A custom discrete snap-division slider styled after RotaenoChartTool.
/// Two horizontal rail lines with tick marks and a circle thumb.
/// Returns `(Option<new_division>, finished)`:
/// - first element is `Some(val)` when the value changed,
/// - second element is `true` when the user released the drag or clicked (for toast).
pub fn draw_snap_slider(ui: &mut egui::Ui, current: u32, width: f32) -> (Option<u32>, bool) {
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

    // Horizontal inset so thumb doesn't clip
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

        // Label for every division
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

    // Draw circle thumb at current position
    let thumb_x = idx_to_x(new_idx);
    let is_hot = response.hovered() || response.dragged();
    let fill = if is_hot {
        egui::Color32::from_rgb(160, 160, 168)
    } else {
        egui::Color32::from_rgb(120, 120, 128)
    };
    painter.circle(
        egui::pos2(thumb_x, rail_cy),
        DIAMOND_HALF,
        fill,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(240, 240, 240)),
    );

    // Return (changed_value, interaction_finished)
    let finished = response.drag_stopped() || response.clicked();
    let new_val = options[new_idx];
    let changed = if new_val != current { Some(new_val) } else { None };
    (changed, finished)
}

/// Vertical snap slider: rail runs top-to-bottom, values increase downward (1 at top, 64 at bottom).
/// Every division value is labeled. Returns `Some(new_division)` on change.
/// All sizes are in egui logical points — scaling is handled by `ctx.set_pixels_per_point()`.
pub fn draw_snap_slider_vertical(ui: &mut egui::Ui, current: u32, height: f32) -> Option<u32> {
    let options = &SNAP_DIVISION_OPTIONS;
    let count = options.len();
    let current_idx = options.iter().position(|&v| v == current).unwrap_or(0);

    let label_w: f32 = 26.0;
    let rail_w: f32 = 18.0;
    let gap: f32 = 0.0;
    let widget_w = label_w + rail_w + gap;
    let desired = egui::vec2(widget_w, height.max(100.0));
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click_and_drag());

    let painter = ui.painter_at(rect);

    let y_pad: f32 = 8.0;
    let y_min = rect.top() + y_pad;
    let y_max = rect.bottom() - y_pad;
    let y_range = (y_max - y_min).max(1.0);

    let rail_left = rect.left() + label_w + gap;
    let rail_right = (rail_left + rail_w).min(rect.right() - 1.5);
    let rail_cx = (rail_left + rail_right) * 0.5;

    let idx_to_y = |idx: usize| -> f32 {
        if count <= 1 {
            (y_min + y_max) * 0.5
        } else {
            y_min + (idx as f32 / (count - 1) as f32) * y_range
        }
    };

    let mut new_idx = current_idx;
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let t = ((pos.y - y_min) / y_range).clamp(0.0, 1.0);
            let float_idx = t * (count - 1) as f32;
            new_idx = float_idx.round() as usize;
            new_idx = new_idx.min(count - 1);
        }
    }

    let rail_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 80);
    let tick_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60);
    let label_color = egui::Color32::from_rgba_unmultiplied(180, 180, 190, 180);
    let label_active = egui::Color32::from_rgb(106, 168, 255);
    let rail_stroke = egui::Stroke::new(1.0, rail_color);
    let tick_stroke = egui::Stroke::new(1.0, tick_color);

    painter.line_segment(
        [egui::pos2(rail_left, y_min - 2.0), egui::pos2(rail_left, y_max + 2.0)],
        rail_stroke,
    );
    painter.line_segment(
        [egui::pos2(rail_right, y_min - 2.0), egui::pos2(rail_right, y_max + 2.0)],
        rail_stroke,
    );

    let tick_len: f32 = 3.0;
    let font = egui::FontId::proportional(10.0);
    for i in 0..count {
        let y = idx_to_y(i);
        let val = options[i];

        painter.line_segment(
            [egui::pos2(rail_left, y), egui::pos2(rail_left + tick_len, y)],
            tick_stroke,
        );
        painter.line_segment(
            [egui::pos2(rail_right, y), egui::pos2(rail_right - tick_len, y)],
            tick_stroke,
        );

        let is_active = i == new_idx;
        let color = if is_active { label_active } else { label_color };
        painter.text(
            egui::pos2(rail_left - 3.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("x{}", val),
            font.clone(),
            color,
        );
    }

    let thumb_y = idx_to_y(new_idx);
    let is_hot = response.hovered() || response.dragged();
    let fill = if is_hot {
        egui::Color32::from_rgb(160, 160, 168)
    } else {
        egui::Color32::from_rgb(120, 120, 128)
    };
    painter.circle(
        egui::pos2(rail_cx, thumb_y),
        5.0,
        fill,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(240, 240, 240)),
    );

    let new_val = options[new_idx];
    if new_val != current { Some(new_val) } else { None }
}
