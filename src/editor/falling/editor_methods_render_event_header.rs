// 文件说明：事件头部与顶部信息渲染。
// 主要功能：绘制谱面事件摘要、状态文本和工具提示信息。
impl FallingGroundEditor {
    fn draw_event_view(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 || rect.w <= 8.0 {
            return;
        }

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(9, 11, 19, 255));
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(44, 58, 86, 255),
        );

        let judge_y = rect.y + rect.h * 0.82;
        let (ahead_ms, behind_ms) = self.visible_ahead_behind_ms_linear(rect.y, rect.h, current_ms, judge_y);

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y_linear(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + self.scaled_ui_px(22.0) || y > rect.y + rect.h + 1.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (1.5, Color::from_rgba(102, 134, 180, 180)),
                BarLineKind::Beat => (1.1, Color::from_rgba(78, 104, 146, 152)),
                BarLineKind::Subdivision => (0.8, Color::from_rgba(58, 78, 112, 112)),
            };
            draw_line(rect.x + 6.0, y, rect.x + rect.w - 6.0, y, thickness, color);
        }

        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;

        // 收集可见 event 的 y 坐标，按 y 升序排列后去重绘制
        let mut visible: Vec<(f32, usize)> = Vec::new();
        for (i, event) in self.timeline_events.iter().enumerate() {
            if event.time_ms < start_ms - 0.001 || event.time_ms > end_ms + 0.001 {
                continue;
            }
            let y = self.time_to_y_linear(event.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 28.0 || y > rect.y + rect.h - 4.0 {
                continue;
            }
            visible.push((y, i));
        }
        visible.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut drawn = 0_u32;
        let mut last_y = f32::NEG_INFINITY;
        for (y, idx) in &visible {
            if *y - last_y < 12.0 {
                continue;
            }
            let event = &self.timeline_events[*idx];
            draw_circle(rect.x + 10.0, *y, 2.8, event.color);
            draw_line(rect.x + 14.0, *y, rect.x + 26.0, *y, 1.4, event.color);
            draw_text_ex(
                &event.label,
                rect.x + 30.0,
                *y + 5.0,
                TextParams {
                    font: self.text_font.as_ref(),
                    font_size: 16,
                    color: event.color,
                    ..Default::default()
                },
            );
            last_y = *y;
            drawn += 1;
            if drawn >= 90 {
                break;
            }
        }

        draw_line(
            rect.x + 6.0,
            judge_y,
            rect.x + rect.w - 6.0,
            judge_y,
            2.2,
            Color::from_rgba(255, 146, 114, 240),
        );
        draw_text_ex(
            "EVENTS",
            rect.x + self.title_side_margin_px(),
            rect.y + self.title_top_baseline_px(),
            TextParams {
                font: self.text_font.as_ref(),
                font_size: self.title_font_size(),
                color: Color::from_rgba(198, 218, 250, 255),
                ..Default::default()
            },
        );
    }
}

