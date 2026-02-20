#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineEventKind {
    Bpm,
    Track,
    Lane,
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    id: u64,
    kind: TimelineEventKind,
    time_ms: f32,
    label: String,
    color: Color,
}

#[derive(Debug, Clone)]
struct EventOverlapCycle {
    candidates: Vec<u64>,
    current_index: usize,
    col: usize,
    anchor_y: i32,
    last_click_time_sec: f64,
    double_click_armed: bool,
}

#[derive(Debug, Clone, Copy)]
struct EventHoverOverlapHint {
    mouse_x: f32,
    mouse_y: f32,
    current_index: usize,
    total: usize,
}
