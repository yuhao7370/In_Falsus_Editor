use macroquad::prelude::*;
use std::cell::{Cell, RefCell};

thread_local! {
    static POINTER_BLOCKED: Cell<bool> = Cell::new(false);
    static KEYBOARD_BLOCKED: Cell<bool> = Cell::new(false);
    static TOUCH_EMULATION: RefCell<TouchEmulationState> = RefCell::new(TouchEmulationState::default());
}

const TOUCH_TAP_MAX_DURATION_SEC: f64 = 0.26;
const TOUCH_TAP_MOVE_THRESHOLD_SQ: f32 = 8.0 * 8.0;
const TOUCH_SCROLL_PX_PER_WHEEL_STEP: f32 = 52.0;
const TOUCH_SCROLL_WHEEL_CLAMP: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
struct TouchTapCandidate {
    active: bool,
    start_time_sec: f64,
    start_center_x: f32,
    start_center_y: f32,
}

impl Default for TouchTapCandidate {
    fn default() -> Self {
        Self {
            active: false,
            start_time_sec: 0.0,
            start_center_x: 0.0,
            start_center_y: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct TouchEmulationState {
    right_pressed: bool,
    middle_pressed: bool,
    play_pause_pressed: bool,
    wheel_x: f32,
    wheel_y: f32,
    two_finger_scroll_prev_center: Option<(f32, f32)>,
    two_finger_tap: TouchTapCandidate,
    three_finger_tap: TouchTapCandidate,
    four_finger_tap: TouchTapCandidate,
}

fn active_touches_sorted() -> Vec<Touch> {
    let mut ts: Vec<Touch> = touches()
        .into_iter()
        .filter(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled))
        .collect();
    ts.sort_by_key(|t| t.id);
    ts
}

fn center_of_first_n_touches(touches: &[Touch], n: usize) -> Option<(f32, f32)> {
    if touches.len() < n || n == 0 {
        return None;
    }
    let mut sum_x = 0.0_f32;
    let mut sum_y = 0.0_f32;
    for t in touches.iter().take(n) {
        sum_x += t.position.x;
        sum_y += t.position.y;
    }
    let denom = n as f32;
    Some((sum_x / denom, sum_y / denom))
}

fn update_tap_candidate(
    candidate: &mut TouchTapCandidate,
    touches: &[Touch],
    expected_count: usize,
    pressed_out: &mut bool,
) {
    if let Some((cx, cy)) = center_of_first_n_touches(touches, expected_count) {
        if !candidate.active {
            candidate.active = true;
            candidate.start_time_sec = get_time();
            candidate.start_center_x = cx;
            candidate.start_center_y = cy;
        } else {
            let dx = cx - candidate.start_center_x;
            let dy = cy - candidate.start_center_y;
            if dx * dx + dy * dy > TOUCH_TAP_MOVE_THRESHOLD_SQ {
                candidate.active = false;
            }
        }
        return;
    }

    if candidate.active {
        let elapsed = get_time() - candidate.start_time_sec;
        if touches.len() < expected_count && elapsed <= TOUCH_TAP_MAX_DURATION_SEC {
            *pressed_out = true;
        }
    }
    candidate.active = false;
}

/// Run once per frame before polling safe_* input.
/// Android gestures:
/// - two-finger tap => right-click pressed
/// - three-finger tap => middle-click pressed
/// - two-finger drag => wheel delta
/// - four-finger tap => play/pause pressed
pub fn update_touch_emulation() {
    TOUCH_EMULATION.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        state.right_pressed = false;
        state.middle_pressed = false;
        state.play_pause_pressed = false;
        state.wheel_x = 0.0;
        state.wheel_y = 0.0;

        if !cfg!(target_os = "android") {
            state.two_finger_scroll_prev_center = None;
            state.two_finger_tap.active = false;
            state.three_finger_tap.active = false;
            state.four_finger_tap.active = false;
            return;
        }

        let touches = active_touches_sorted();

        if let Some((cx, cy)) = center_of_first_n_touches(&touches, 2) {
            if let Some((px, py)) = state.two_finger_scroll_prev_center {
                let dx = cx - px;
                let dy = cy - py;
                state.wheel_x = (-dx / TOUCH_SCROLL_PX_PER_WHEEL_STEP)
                    .clamp(-TOUCH_SCROLL_WHEEL_CLAMP, TOUCH_SCROLL_WHEEL_CLAMP);
                state.wheel_y = (-dy / TOUCH_SCROLL_PX_PER_WHEEL_STEP)
                    .clamp(-TOUCH_SCROLL_WHEEL_CLAMP, TOUCH_SCROLL_WHEEL_CLAMP);
            }
            state.two_finger_scroll_prev_center = Some((cx, cy));
        } else {
            state.two_finger_scroll_prev_center = None;
        }

        let mut right_pressed = false;
        let mut middle_pressed = false;
        let mut play_pause_pressed = false;
        update_tap_candidate(&mut state.two_finger_tap, &touches, 2, &mut right_pressed);
        update_tap_candidate(
            &mut state.three_finger_tap,
            &touches,
            3,
            &mut middle_pressed,
        );
        update_tap_candidate(
            &mut state.four_finger_tap,
            &touches,
            4,
            &mut play_pause_pressed,
        );
        state.right_pressed = right_pressed;
        state.middle_pressed = middle_pressed;
        state.play_pause_pressed = play_pause_pressed;
    });
}

pub fn android_play_pause_pressed() -> bool {
    TOUCH_EMULATION.with(|state_cell| {
        let state = state_cell.borrow();
        state.play_pause_pressed
    })
}

pub fn set_pointer_blocked(blocked: bool) {
    POINTER_BLOCKED.with(|c| c.set(blocked));
}

pub fn is_pointer_blocked() -> bool {
    POINTER_BLOCKED.with(|c| c.get())
}

pub fn safe_mouse_position() -> (f32, f32) {
    if is_pointer_blocked() {
        (-9999.0, -9999.0)
    } else {
        mouse_position()
    }
}

fn synthetic_mouse_button_pressed(btn: MouseButton) -> bool {
    TOUCH_EMULATION.with(|state_cell| {
        let state = state_cell.borrow();
        match btn {
            MouseButton::Right => state.right_pressed,
            MouseButton::Middle => state.middle_pressed,
            _ => false,
        }
    })
}

fn synthetic_mouse_wheel() -> (f32, f32) {
    TOUCH_EMULATION.with(|state_cell| {
        let state = state_cell.borrow();
        (state.wheel_x, state.wheel_y)
    })
}

pub fn safe_mouse_button_pressed(btn: MouseButton) -> bool {
    !is_pointer_blocked() && (is_mouse_button_pressed(btn) || synthetic_mouse_button_pressed(btn))
}

pub fn safe_mouse_button_down(btn: MouseButton) -> bool {
    !is_pointer_blocked() && is_mouse_button_down(btn)
}

pub fn safe_mouse_button_released(btn: MouseButton) -> bool {
    !is_pointer_blocked() && is_mouse_button_released(btn)
}

pub fn free_mouse_wheel() -> (f32, f32) {
    let (mx, my) = mouse_wheel();
    let (tx, ty) = synthetic_mouse_wheel();
    (mx + tx, my + ty)
}

pub fn safe_mouse_wheel() -> (f32, f32) {
    if is_pointer_blocked() {
        (0.0, 0.0)
    } else {
        free_mouse_wheel()
    }
}

pub fn set_keyboard_blocked(blocked: bool) {
    KEYBOARD_BLOCKED.with(|c| c.set(blocked));
}

pub fn is_keyboard_blocked() -> bool {
    KEYBOARD_BLOCKED.with(|c| c.get())
}

pub fn safe_key_pressed(key: KeyCode) -> bool {
    !is_keyboard_blocked() && is_key_pressed(key)
}

pub fn safe_key_down(key: KeyCode) -> bool {
    !is_keyboard_blocked() && is_key_down(key)
}

pub fn free_key_pressed(key: KeyCode) -> bool {
    is_key_pressed(key)
}

pub fn free_key_down(key: KeyCode) -> bool {
    is_key_down(key)
}
