// 文件说明：地面轨道渲染实现。
// 主要功能：绘制 GROUND 视图、判定线、拍线与地面音符。
impl FallingGroundEditor {
    fn draw_ground_view(&self, rect: Rect, current_ms: f32, show_spectrum: bool) {
        if rect.h <= 8.0 {
            return;
        }

        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let ahead_ms = ((judge_y - rect.y) / pixels_per_sec * 1000.0).max(0.0);
        let behind_ms = (((rect.y + rect.h) - judge_y) / pixels_per_sec * 1000.0).max(0.0);

        for lane in 0..LANE_COUNT {
            let x = rect.x + lane as f32 * lane_w;
            let bg = if lane % 2 == 0 {
                Color::from_rgba(18, 18, 22, 255)
            } else {
                Color::from_rgba(22, 22, 28, 255)
            };
            draw_rectangle(x, rect.y, lane_w, rect.h, bg);
            draw_line(x, rect.y, x, rect.y + rect.h, 1.0, Color::from_rgba(36, 36, 48, 255));
            draw_text_ex(
                &format!("L{lane}"),
                x + 8.0,
                rect.y + 20.0,
                TextParams {
                    font: self.text_font.as_ref(),
                    font_size: 18,
                    color: Color::from_rgba(170, 170, 180, 255),
                    ..Default::default()
                },
            );
        }
        draw_line(
            rect.x + rect.w,
            rect.y,
            rect.x + rect.w,
            rect.y + rect.h,
            1.0,
            Color::from_rgba(36, 36, 48, 255),
        );

        if show_spectrum {
            self.draw_falling_spectrum(
                rect,
                current_ms,
                judge_y,
                Color::from_rgba(86, 176, 255, 255),
            );
        }

        for barline in self
            .timeline
            .visible_barlines(current_ms, ahead_ms, behind_ms, self.snap_division)
        {
            let y = self.time_to_y(barline.time_ms, current_ms, judge_y, rect.h);
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (2.1, Color::from_rgba(170, 205, 255, 210)),
                BarLineKind::Beat => (1.3, Color::from_rgba(112, 148, 192, 186)),
                BarLineKind::Subdivision => (0.9, Color::from_rgba(80, 108, 142, 142)),
            };
            draw_line(rect.x, y, rect.x + rect.w, y, thickness, color);
        }

        draw_line(
            rect.x,
            judge_y,
            rect.x + rect.w,
            judge_y,
            3.0,
            Color::from_rgba(255, 120, 96, 255),
        );
        draw_text_ex(
            "JUDGE",
            rect.x + 8.0,
            judge_y - 6.0,
            TextParams {
                font: self.text_font.as_ref(),
                font_size: 18,
                color: Color::from_rgba(255, 170, 140, 255),
                ..Default::default()
            },
        );
        draw_text_ex(
            "GROUND",
            rect.x + rect.w - 112.0,
            rect.y + 22.0,
            TextParams {
                font: self.text_font.as_ref(),
                font_size: 18,
                color: Color::from_rgba(185, 198, 224, 255),
                ..Default::default()
            },
        );

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let lane_x = rect.x + lane_w * note.lane as f32;
            let note_w = note_head_width(note, lane_w);
            let note_x = lane_x + (lane_w - note_w) * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let selected = self.selected_note_id == Some(note.id);
            let palette = lane_note_palette(note.lane);

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                if y2 >= rect.y && y1 <= rect.y + rect.h {
                    let (body_x, body_w, body_color) = match note.kind {
                        GroundNoteKind::Hold => (
                            note_x + note_w * 0.04,
                            note_w * 0.92,
                            palette.hold_body,
                        ),
                        GroundNoteKind::SkyArea => (
                            note_x + note_w * 0.02,
                            note_w * 0.96,
                            palette.skyarea_body,
                        ),
                        _ => (
                            note_x + note_w * 0.35,
                            note_w * 0.3,
                            palette.hold_body,
                        ),
                    };
                    let body_y = y1.max(rect.y);
                    let body_h = (y2.min(rect.y + rect.h) - body_y).max(1.0);
                    draw_rectangle(body_x, body_y, body_w, body_h, body_color);
                    if selected {
                        draw_selected_note_darken_rect(body_x, body_y, body_w, body_h);
                    }
                }
            }

            if head_y >= rect.y - 28.0 && head_y <= rect.y + rect.h + 28.0 {
                let head_color = match note.kind {
                    GroundNoteKind::Tap => palette.tap,
                    GroundNoteKind::Hold => palette.hold_head,
                    _ => palette.tap,
                };

                draw_rectangle(note_x, head_y - 8.0, note_w, 16.0, head_color);
                draw_rectangle(
                    note_x + 1.5,
                    head_y - 7.0,
                    (note_w - 3.0).max(1.0),
                    5.0,
                    Color::from_rgba(255, 255, 255, 34),
                );

                if selected {
                    draw_selected_note_darken_rect(note_x, head_y - 8.0, note_w, 16.0);
                    draw_selected_note_outline(note_x, head_y - 8.0, note_w, 16.0);
                }
            }
        }

        if self.debug_show_hitboxes {
            self.draw_ground_hitbox_overlay(rect, current_ms);
        }

    }



}

