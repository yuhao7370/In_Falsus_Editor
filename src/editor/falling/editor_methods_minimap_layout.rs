// 文件说明：小地图布局与可视范围计算。
// 主要功能：计算小地图区域尺寸、窗口映射和定位参数。
impl FallingGroundEditor {
    fn split_portrait_screens(&self, rect: Rect) -> (Rect, Rect) {
        let gap = (rect.w * 0.04).clamp(10.0, 28.0);
        let max_h_by_width = ((rect.w - gap).max(10.0)) / (2.0 * PORTRAIT_SCREEN_RATIO);
        let screen_h = rect.h.min(max_h_by_width).max(20.0);
        let screen_w = (screen_h * PORTRAIT_SCREEN_RATIO).max(12.0);
        let pair_w = screen_w * 2.0 + gap;
        let start_x = rect.x + (rect.w - pair_w) * 0.5;
        let y = rect.y + (rect.h - screen_h) * 0.5;
        (
            Rect::new(start_x, y, screen_w, screen_h),
            Rect::new(start_x + screen_w + gap, y, screen_w, screen_h),
        )
    }

    fn minimap_screen_from_left_gap(&self, content_rect: Rect, left_screen: Rect) -> Option<Rect> {
        let gap = (content_rect.w * 0.008).clamp(2.0, 6.0);
        let available_w = left_screen.x - content_rect.x - gap;
        if available_w < 34.0 {
            return None;
        }
        Some(Rect::new(
            content_rect.x,
            left_screen.y,
            available_w,
            left_screen.h,
        ))
    }

    fn compute_visible_window_ms(&self, render_rect: Rect, current_ms: f32) -> TimeWindowMs {
        if render_rect.h <= 1.0 {
            return TimeWindowMs {
                start_ms: current_ms.max(0.0),
                end_ms: current_ms.max(0.0),
                current_ms: current_ms.max(0.0),
            };
        }
        let judge_y = render_rect.y + render_rect.h * 0.82;
        let (ahead_ms, behind_ms) = self.visible_ahead_behind_ms(render_rect.y, render_rect.h, current_ms, judge_y);
        let start_ms = (current_ms - behind_ms).max(0.0);
        let end_ms = (current_ms + ahead_ms).max(start_ms);
        TimeWindowMs {
            start_ms,
            end_ms,
            current_ms: current_ms.max(0.0),
        }
    }

    fn minimap_segment_time_to_y(
        &self,
        time_ms: f32,
        rect: Rect,
        seg_start_ms: f32,
        seg_end_ms: f32,
    ) -> f32 {
        let span = (seg_end_ms - seg_start_ms).max(0.001);
        let t = ((time_ms - seg_start_ms) / span).clamp(0.0, 1.0);
        rect.y + rect.h * (1.0 - t)
    }

    fn minimap_segment_y_to_time(
        &self,
        y: f32,
        rect: Rect,
        seg_start_ms: f32,
        seg_end_ms: f32,
    ) -> f32 {
        let span = (seg_end_ms - seg_start_ms).max(0.001);
        let t = ((y - rect.y) / rect.h.max(0.001)).clamp(0.0, 1.0);
        (1.0 - t) * span + seg_start_ms
    }



}

