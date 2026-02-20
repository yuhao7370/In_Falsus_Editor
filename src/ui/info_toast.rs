use std::collections::VecDeque;

use macroquad::prelude::*;

const INFO_TOAST_TOTAL_SEC: f32 = 2.8;
const INFO_TOAST_ENTER_SEC: f32 = 0.24;
const INFO_TOAST_EXIT_SEC: f32 = 0.26;
const INFO_TOAST_MAX_COUNT: usize = 12;
const PINNED_TOAST_EXIT_SEC: f32 = 0.32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToastLevel {
    Info,
    Warn,
}

#[derive(Debug, Clone)]
struct InfoToastItem {
    text: String,
    created_at: f64,
    level: ToastLevel,
}

/// 持久 toast：一直显示直到手动 dismiss
#[derive(Debug, Clone)]
struct PinnedToast {
    text: String,
    created_at: f64,
    /// dismiss 被调用的时刻；None 表示仍在显示
    dismiss_at: Option<f64>,
}

#[derive(Debug, Default)]
pub struct InfoToastManager {
    items: VecDeque<InfoToastItem>,
    pinned: Option<PinnedToast>,
}

impl InfoToastManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.trim().is_empty() {
            return;
        }
        self.items.push_back(InfoToastItem {
            text,
            created_at: get_time(),
            level: ToastLevel::Info,
        });
        while self.items.len() > INFO_TOAST_MAX_COUNT {
            self.items.pop_front();
        }
    }

    pub fn push_warn(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.trim().is_empty() {
            return;
        }
        self.items.push_back(InfoToastItem {
            text,
            created_at: get_time(),
            level: ToastLevel::Warn,
        });
        while self.items.len() > INFO_TOAST_MAX_COUNT {
            self.items.pop_front();
        }
    }

    /// 显示一个持久 toast，一直保持到调用 dismiss_pinned() 或 update_pinned()。
    /// 如果已有 pinned toast，会替换文本（保留入场动画进度）。
    pub fn pin(&mut self, text: impl Into<String>) {
        let text = text.into();
        if let Some(ref mut p) = self.pinned {
            p.text = text;
            p.dismiss_at = None; // 取消正在退出的状态
        } else {
            self.pinned = Some(PinnedToast {
                text,
                created_at: get_time(),
                dismiss_at: None,
            });
        }
    }

    /// 更新 pinned toast 的文本（如果存在）。
    #[allow(dead_code)]
    pub fn update_pinned(&mut self, text: impl Into<String>) {
        if let Some(ref mut p) = self.pinned {
            p.text = text.into();
        }
    }

    /// 开始 pinned toast 的退出动画，动画结束后自动移除。
    pub fn dismiss_pinned(&mut self) {
        if let Some(ref mut p) = self.pinned {
            if p.dismiss_at.is_none() {
                p.dismiss_at = Some(get_time());
            }
        }
    }

    /// pinned toast 是否正在显示（包括退出动画中）。
    #[allow(dead_code)]
    pub fn has_pinned(&self) -> bool {
        self.pinned.is_some()
    }

    pub fn draw(&mut self, ui_scale: f32, anchor_y: f32, font: Option<&Font>) {
        let now = get_time();

        // 清理已过期的 pinned toast（退出动画结束）
        if let Some(ref p) = self.pinned {
            if let Some(dismiss_t) = p.dismiss_at {
                let exit_elapsed = (now - dismiss_t) as f32;
                if exit_elapsed >= PINNED_TOAST_EXIT_SEC {
                    self.pinned = None;
                }
            }
        }

        while let Some(front) = self.items.front() {
            let elapsed = (now - front.created_at) as f32;
            if elapsed >= INFO_TOAST_TOTAL_SEC {
                self.items.pop_front();
            } else {
                break;
            }
        }

        let mut y = anchor_y + 6.0 * ui_scale;
        let x = 8.0 * ui_scale;
        let max_content_w = (screen_width() * 0.42).max(220.0 * ui_scale);
        let gap = 8.0 * ui_scale;
        let font_size = (18.0 * ui_scale).round().clamp(12.0, 30.0) as u16;
        let pad_x = 14.0 * ui_scale;
        let pad_y = 8.0 * ui_scale;
        let radius = 8.0 * ui_scale;

        // 绘制 pinned toast（在普通 toast 上方）
        let pinned_x = x;
        if let Some(ref p) = self.pinned {
            let enter_elapsed = (now - p.created_at) as f32;
            let mut alpha = 1.0_f32;
            let mut slide_x = 0.0_f32;

            // 入场动画
            if enter_elapsed < INFO_TOAST_ENTER_SEC {
                let prog = (enter_elapsed / INFO_TOAST_ENTER_SEC).clamp(0.0, 1.0);
                let e = ease_out_cubic(prog);
                alpha *= e;
                slide_x = (1.0 - e) * -22.0 * ui_scale;
            }

            // 退出动画
            if let Some(dismiss_t) = p.dismiss_at {
                let exit_elapsed = (now - dismiss_t) as f32;
                let prog = (exit_elapsed / PINNED_TOAST_EXIT_SEC).clamp(0.0, 1.0);
                let e = ease_in_cubic(prog);
                alpha *= 1.0 - e;
                slide_x += e * 18.0 * ui_scale;
            }

            if alpha > 0.001 {
                let text = trim_text_to_width(&p.text, max_content_w, font, font_size);
                let metrics = measure_text(&text, font, font_size, 1.0);
                let rect_w = (metrics.width + pad_x * 2.0).max(120.0 * ui_scale);
                let rect_h = (metrics.height + pad_y * 2.0).max(30.0 * ui_scale);
                let rect = Rect::new(pinned_x + slide_x, y, rect_w, rect_h);

                // 阴影
                draw_rounded_rect(
                    Rect::new(rect.x + 2.0 * ui_scale, rect.y + 2.0 * ui_scale, rect.w, rect.h),
                    radius,
                    Color::new(0.02, 0.04, 0.08, 0.24 * alpha),
                );
                // 背景：用稍微不同的颜色区分 pinned
                let bg = Color::new(0.50, 0.75, 0.95, 0.85 * alpha);
                let hl = Color::new(0.85, 0.93, 1.0, 0.18 * alpha);
                draw_rounded_rect(rect, radius, bg);
                draw_rounded_rect(
                    Rect::new(rect.x, rect.y, rect.w, rect.h * 0.45),
                    radius, hl,
                );
                // 文字
                draw_text_ex(
                    &text,
                    rect.x + pad_x,
                    rect.y + pad_y + metrics.offset_y,
                    TextParams {
                        font,
                        font_size,
                        color: Color::new(0.05, 0.10, 0.16, 0.96 * alpha),
                        ..Default::default()
                    },
                );

                y += rect_h + gap;
            }
        }

        for item in &self.items {
            let elapsed = (now - item.created_at) as f32;
            if elapsed < 0.0 || elapsed >= INFO_TOAST_TOTAL_SEC {
                continue;
            }

            let mut alpha = 1.0_f32;
            let mut slide_x = 0.0_f32;
            if elapsed < INFO_TOAST_ENTER_SEC {
                let p = (elapsed / INFO_TOAST_ENTER_SEC).clamp(0.0, 1.0);
                let e = ease_out_cubic(p);
                alpha *= e;
                slide_x = (1.0 - e) * -22.0 * ui_scale;
            } else if elapsed > INFO_TOAST_TOTAL_SEC - INFO_TOAST_EXIT_SEC {
                let p = ((elapsed - (INFO_TOAST_TOTAL_SEC - INFO_TOAST_EXIT_SEC))
                    / INFO_TOAST_EXIT_SEC)
                    .clamp(0.0, 1.0);
                let e = ease_in_cubic(p);
                alpha *= 1.0 - e;
                slide_x = e * 18.0 * ui_scale;
            }
            if alpha <= 0.001 {
                continue;
            }

            let text = trim_text_to_width(&item.text, max_content_w, font, font_size);
            let metrics = measure_text(&text, font, font_size, 1.0);
            let rect_w = (metrics.width + pad_x * 2.0).max(120.0 * ui_scale);
            let rect_h = (metrics.height + pad_y * 2.0).max(30.0 * ui_scale);
            let rect = Rect::new(x + slide_x, y, rect_w, rect_h);

            draw_rounded_rect(
                Rect::new(
                    rect.x + 2.0 * ui_scale,
                    rect.y + 2.0 * ui_scale,
                    rect.w,
                    rect.h,
                ),
                radius,
                Color::new(0.02, 0.04, 0.08, 0.24 * alpha),
            );
            let (bg_color, highlight_color) = match item.level {
                ToastLevel::Info => (
                    Color::new(0.63, 0.81, 1.0, 0.82 * alpha),
                    Color::new(0.92, 0.96, 1.0, 0.16 * alpha),
                ),
                ToastLevel::Warn => (
                    Color::new(1.0, 0.88, 0.42, 0.82 * alpha),
                    Color::new(1.0, 0.96, 0.80, 0.16 * alpha),
                ),
            };
            draw_rounded_rect(rect, radius, bg_color);
            draw_rounded_rect(
                Rect::new(rect.x, rect.y, rect.w, rect.h * 0.45),
                radius,
                highlight_color,
            );

            draw_text_ex(
                &text,
                rect.x + pad_x,
                rect.y + pad_y + metrics.offset_y,
                TextParams {
                    font,
                    font_size,
                    color: Color::new(0.05, 0.10, 0.16, 0.96 * alpha),
                    ..Default::default()
                },
            );

            y += rect_h + gap;
        }
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t
}

fn draw_rounded_rect(rect: Rect, radius: f32, color: Color) {
    let r = radius.min(rect.w * 0.5).min(rect.h * 0.5).max(0.0);
    if r <= 0.5 {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
        return;
    }

    let mid_h = (rect.h - 2.0 * r).max(0.0);
    let mid_w = (rect.w - 2.0 * r).max(0.0);

    if mid_h > 0.0 {
        draw_rectangle(rect.x, rect.y + r, rect.w, mid_h, color);
    }
    if mid_w > 0.0 {
        draw_rectangle(rect.x + r, rect.y, mid_w, r, color);
        draw_rectangle(rect.x + r, rect.y + rect.h - r, mid_w, r, color);
    }

    // Use non-overlapping quarter fans for corners to avoid alpha stacking artifacts.
    draw_corner_fan(
        vec2(rect.x + r, rect.y + r),
        r,
        std::f32::consts::PI,
        std::f32::consts::PI * 1.5,
        color,
    );
    draw_corner_fan(
        vec2(rect.x + rect.w - r, rect.y + r),
        r,
        std::f32::consts::PI * 1.5,
        std::f32::consts::PI * 2.0,
        color,
    );
    draw_corner_fan(
        vec2(rect.x + rect.w - r, rect.y + rect.h - r),
        r,
        0.0,
        std::f32::consts::PI * 0.5,
        color,
    );
    draw_corner_fan(
        vec2(rect.x + r, rect.y + rect.h - r),
        r,
        std::f32::consts::PI * 0.5,
        std::f32::consts::PI,
        color,
    );
}

fn draw_corner_fan(center: Vec2, radius: f32, start: f32, end: f32, color: Color) {
    let segs = ((radius * 0.35).round() as i32).clamp(6, 18) as usize;
    let mut prev = vec2(
        center.x + radius * start.cos(),
        center.y + radius * start.sin(),
    );
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let ang = start + (end - start) * t;
        let p = vec2(center.x + radius * ang.cos(), center.y + radius * ang.sin());
        draw_triangle(center, prev, p, color);
        prev = p;
    }
}

fn trim_text_to_width(text: &str, max_width: f32, font: Option<&Font>, font_size: u16) -> String {
    if measure_text(text, font, font_size, 1.0).width <= max_width {
        return text.to_owned();
    }

    let ellipsis = "...";
    let ellipsis_w = measure_text(ellipsis, font, font_size, 1.0).width;
    if ellipsis_w > max_width {
        return ellipsis.to_owned();
    }

    let mut out = String::new();
    for ch in text.chars() {
        let mut candidate = out.clone();
        candidate.push(ch);
        let mut check = candidate.clone();
        check.push_str(ellipsis);
        if measure_text(&check, font, font_size, 1.0).width > max_width {
            break;
        }
        out = candidate;
    }
    out.push_str(ellipsis);
    out
}
