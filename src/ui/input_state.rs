// 全局鼠标输入状态管理：当 egui 拦截鼠标时，阻止 macroquad 侧的鼠标交互。
// 用法：每帧 egui 处理完后调用 set_pointer_blocked()，
//       其余代码用 safe_* 系列函数替代 macroquad 原生鼠标函数。

use std::cell::Cell;
use macroquad::prelude::*;

thread_local! {
    static POINTER_BLOCKED: Cell<bool> = Cell::new(false);
    static KEYBOARD_BLOCKED: Cell<bool> = Cell::new(false);
}

/// 每帧由 main loop 调用，设置当前帧 egui 是否拦截了鼠标
pub fn set_pointer_blocked(blocked: bool) {
    POINTER_BLOCKED.with(|c| c.set(blocked));
}

/// 查询当前帧鼠标是否被 egui 拦截
pub fn is_pointer_blocked() -> bool {
    POINTER_BLOCKED.with(|c| c.get())
}

/// 安全版 mouse_position — 被拦截时返回屏幕外坐标，不会命中任何 UI rect
pub fn safe_mouse_position() -> (f32, f32) {
    if is_pointer_blocked() {
        (-9999.0, -9999.0)
    } else {
        mouse_position()
    }
}

/// 安全版 is_mouse_button_pressed
pub fn safe_mouse_button_pressed(btn: MouseButton) -> bool {
    !is_pointer_blocked() && is_mouse_button_pressed(btn)
}

/// 安全版 is_mouse_button_down
pub fn safe_mouse_button_down(btn: MouseButton) -> bool {
    !is_pointer_blocked() && is_mouse_button_down(btn)
}

/// 安全版 is_mouse_button_released
pub fn safe_mouse_button_released(btn: MouseButton) -> bool {
    !is_pointer_blocked() && is_mouse_button_released(btn)
}

/// 自由版 mouse_wheel — 不受 egui 拦截影响，任何时候都返回真实滚轮值
pub fn free_mouse_wheel() -> (f32, f32) {
    mouse_wheel()
}

/// 安全版 mouse_wheel — 被 egui 拦截时返回 (0, 0)，防止滚轮穿透
pub fn safe_mouse_wheel() -> (f32, f32) {
    if is_pointer_blocked() {
        (0.0, 0.0)
    } else {
        mouse_wheel()
    }
}

/// 每帧由 main loop 调用，设置当前帧 egui 是否拦截了键盘（文本框获得焦点 / 弹窗打开）
pub fn set_keyboard_blocked(blocked: bool) {
    KEYBOARD_BLOCKED.with(|c| c.set(blocked));
}

/// 查询当前帧键盘是否被 egui 拦截
pub fn is_keyboard_blocked() -> bool {
    KEYBOARD_BLOCKED.with(|c| c.get())
}

/// 安全版 is_key_pressed — 被 egui 键盘拦截时返回 false，防止按键穿透到编辑器
pub fn safe_key_pressed(key: KeyCode) -> bool {
    !is_keyboard_blocked() && is_key_pressed(key)
}

/// 安全版 is_key_down — 被 egui 键盘拦截时返回 false
pub fn safe_key_down(key: KeyCode) -> bool {
    !is_keyboard_blocked() && is_key_down(key)
}

/// 自由版 is_key_pressed — 不受 egui 拦截影响，任何时候都返回真实按键状态
pub fn free_key_pressed(key: KeyCode) -> bool {
    is_key_pressed(key)
}

/// 自由版 is_key_down — 不受 egui 拦截影响
pub fn free_key_down(key: KeyCode) -> bool {
    is_key_down(key)
}
