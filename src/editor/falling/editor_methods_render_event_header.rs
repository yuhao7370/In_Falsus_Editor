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

        draw_text_ex(
            "EVENTS",
            rect.x + 8.0,
            rect.y + 20.0,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(198, 218, 250, 255),
                ..Default::default()
            },
        );

        let judge_y = rect.y + rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let ahead_ms = ((judge_y - rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((rect.y + rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 22.0 || y > rect.y + rect.h + 1.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (1.5, Color::from_rgba(102, 134, 180, 180)),
                BarLineKind::Beat => (1.1, Color::from_rgba(78, 104, 146, 152)),
                BarLineKind::Subdivision => (0.8, Color::from_rgba(58, 78, 112, 112)),
            };
            draw_line(rect.x + 6.0, y, rect.x + rect.w - 6.0, y, thickness, color);
        }

        draw_line(
            rect.x + 6.0,
            judge_y,
            rect.x + rect.w - 6.0,
            judge_y,
            2.2,
            Color::from_rgba(255, 146, 114, 240),
        );

        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;
        let mut drawn = 0_u32;
        let mut last_y = f32::NEG_INFINITY;
        for event in &self.timeline_events {
            if event.time_ms < start_ms - 0.001 || event.time_ms > end_ms + 0.001 {
                continue;
            }
            let y = self.time_to_y(event.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y + 28.0 || y > rect.y + rect.h - 4.0 {
                continue;
            }
            if y - last_y < 12.0 {
                continue;
            }
            draw_circle(rect.x + 10.0, y, 2.8, event.color);
            draw_line(rect.x + 14.0, y, rect.x + 26.0, y, 1.4, event.color);
            draw_text_ex(
                &event.label,
                rect.x + 30.0,
                y + 5.0,
                TextParams {
                    font_size: 16,
                    color: event.color,
                    ..Default::default()
                },
            );
            last_y = y;
            drawn += 1;
            if drawn >= 90 {
                break;
            }
        }
    }

    fn draw_header(&self, rect: Rect) {
        let ground_count = self
            .notes
            .iter()
            .filter(|note| is_ground_kind(note.kind))
            .count();
        let air_count = self
            .notes
            .iter()
            .filter(|note| is_air_kind(note.kind))
            .count();
        draw_text_ex(
            &format!(
                "Falling | chart={} | G:{} A:{} | view={} | tool={} | snap={} {}x | speed={:.2}H/s | hitbox={}",
                self.chart_path,
                ground_count,
                air_count,
                self.render_scope.label(),
                self.place_note_type
                    .map(PlaceNoteType::label)
                    .unwrap_or("None"),
                if self.snap_enabled { "on" } else { "off" },
                self.snap_division,
                self.scroll_speed,
                if self.debug_show_hitboxes { "on" } else { "off" }
            ),
            rect.x + 10.0,
            rect.y + 24.0,
            TextParams {
                font_size: 22,
                color: WHITE,
                ..Default::default()
            },
        );
    }

    fn handle_scroll_speed_controls(&mut self, header_rect: Rect) {
        let panel_w = 224.0;
        let panel_h = (header_rect.h - 8.0).max(24.0);
        let panel_rect = Rect::new(
            header_rect.x + header_rect.w - panel_w - 10.0,
            header_rect.y + 4.0,
            panel_w,
            panel_h,
        );
        draw_rectangle(
            panel_rect.x,
            panel_rect.y,
            panel_rect.w,
            panel_rect.h,
            Color::from_rgba(18, 18, 28, 232),
        );
        draw_rectangle_lines(
            panel_rect.x,
            panel_rect.y,
            panel_rect.w,
            panel_rect.h,
            1.0,
            Color::from_rgba(78, 78, 96, 255),
        );

        let minus_rect = Rect::new(panel_rect.x + 6.0, panel_rect.y + 3.0, 28.0, panel_rect.h - 6.0);
        let plus_rect = Rect::new(
            panel_rect.x + panel_rect.w - 34.0,
            panel_rect.y + 3.0,
            28.0,
            panel_rect.h - 6.0,
        );

        if draw_small_button(minus_rect, "-") {
            self.adjust_scroll_speed(-SCROLL_SPEED_STEP);
        }
        if draw_small_button(plus_rect, "+") {
            self.adjust_scroll_speed(SCROLL_SPEED_STEP);
        }

        let (mx, my) = mouse_position();
        if point_in_rect(mx, my, panel_rect) {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y.abs() > f32::EPSILON {
                self.adjust_scroll_speed(wheel_y * SCROLL_SPEED_STEP);
            }
        }

        draw_text_ex(
            &format!("Flow {:.2}H/s", self.scroll_speed),
            panel_rect.x + 42.0,
            panel_rect.y + panel_rect.h * 0.72,
            TextParams {
                font_size: 18,
                color: Color::from_rgba(220, 226, 240, 255),
                ..Default::default()
            },
        );
    }



}

