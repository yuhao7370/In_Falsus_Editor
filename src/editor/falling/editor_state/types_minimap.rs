#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct MinimapPageConfig {
    measures_per_page: u32,
    page_index: u32,
}

#[derive(Debug, Clone, Copy)]
struct TimeWindowMs {
    start_ms: f32,
    end_ms: f32,
    current_ms: f32,
}

#[derive(Debug, Clone, Copy)]
struct MinimapRenderInfo {
    content_rect: Rect,
    highlight_rect: Rect,
    seek_start_ms: f32,
    seek_end_ms: f32,
}

#[derive(Debug, Clone, Copy)]
struct MinimapDrawLayout {
    duration_ms: f32,
    half_ms: f32,
    ui: f32,
    ground_rect_1: Rect,
    sky_rect_1: Rect,
    ground_rect_2: Rect,
    sky_rect_2: Rect,
    left_group_rect: Rect,
    right_group_rect: Rect,
    active_group_rect: Rect,
    active_start_ms: f32,
    active_end_ms: f32,
}
