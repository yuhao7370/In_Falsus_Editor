// 文件说明：地面轨道渲染实现。
// 主要功能：绘制 GROUND 视图、判定线、拍线与地面音符。
impl FallingGroundEditor {
    fn draw_ground_view(&self, rect: Rect, current_ms: f32, show_spectrum: bool) {
        if rect.h <= 8.0 {
            return;
        }
        self.begin_view_clip_rect(rect);

        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (_ahead_ms, _behind_ms) = self.visible_ahead_behind_ms(rect.y, rect.h, current_ms, judge_y);
        let top_label_baseline = self.title_top_baseline_px();
        let barline_label_font_size = self.barline_label_font_size();
        let barline_label_min_y = rect.y + self.scaled_ui_px(14.0);
        let barline_label_baseline_offset = self.scaled_ui_px(2.0);
        let judge_label_font_size = self.judge_label_font_size();
        let judge_label_baseline_offset = self.scaled_ui_px(6.0);
        let mut measure_labels: Vec<(f32, f32)> = Vec::new();

        for lane in 0..LANE_COUNT {
            let x = rect.x + lane as f32 * lane_w;
            let bg = if lane % 2 == 0 {
                Color::from_rgba(18, 18, 22, 255)
            } else {
                Color::from_rgba(22, 22, 28, 255)
            };
            draw_rectangle(x, rect.y, lane_w, rect.h, bg);
            draw_line(x, rect.y, x, rect.y + rect.h, 1.0, Color::from_rgba(36, 36, 48, 255));
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

        let current_vb = self.track_timeline.visual_beat_at(current_ms);
        let pixels_per_ms_bl = (self.scroll_speed * rect.h / 1000.0).max(0.001);
        // 计算视口对应的 visual_beat 范围（上方和下方）
        let vb_above = (judge_y - rect.y) / pixels_per_ms_bl;
        let vb_below = (rect.y + rect.h - judge_y) / pixels_per_ms_bl;
        let start_vb = current_vb - vb_below - 1.0;
        let end_vb = current_vb + vb_above + 1.0;
        for barline in self.visible_barlines_cached(start_vb, end_vb) {
            let y = judge_y - (barline.visual_beat - current_vb) * pixels_per_ms_bl;
            if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                continue;
            }
            let (thickness, color) = match barline.kind {
                BarLineKind::Measure => (2.1, Color::from_rgba(170, 205, 255, 210)),
                BarLineKind::Beat => (1.3, Color::from_rgba(112, 148, 192, 186)),
                BarLineKind::Subdivision => (0.9, Color::from_rgba(80, 108, 142, 142)),
            };
            draw_line(rect.x, y, rect.x + rect.w, y, thickness, color);
            if barline.show_measure_label && y >= barline_label_min_y && y <= rect.y + rect.h - barline_label_baseline_offset {
                measure_labels.push((y, barline.measure_pos));
            }
        }

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }

            // AutoPlay: 已被判定的音符不显示（或裁剪判定线以下部分）
            let judged = self.autoplay_enabled && note.time_ms <= current_ms;
            if judged && !note.has_tail() {
                // 无尾音符：已判定则完全跳过
                continue;
            }
            if judged && note.has_tail() && note.end_time_ms() <= current_ms {
                // 有尾音符：尾部也已判定，完全跳过
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
                    let mut body_end = y2.min(rect.y + rect.h);
                    // AutoPlay: 裁剪判定线以下部分（head 已过判定线）
                    if judged {
                        // 判定线以下不显示：body 底部截断到 judge_y
                        body_end = body_end.min(judge_y);
                    }
                    let body_h = (body_end - body_y).max(0.0);
                    if body_h > 0.0 {
                        draw_rectangle(body_x, body_y, body_w, body_h, body_color);
                        if selected {
                            draw_selected_note_darken_rect(body_x, body_y, body_w, body_h);
                        }
                    }
                }
            }

            // AutoPlay: head 已判定则不画 head
            if !judged && head_y >= rect.y - 28.0 && head_y <= rect.y + rect.h + 28.0 {
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
            rect.x + self.title_side_margin_px(),
            judge_y - judge_label_baseline_offset,
            TextParams {
                font: self.text_font.as_ref(),
                font_size: judge_label_font_size,
                color: Color::from_rgba(255, 170, 140, 255),
                ..Default::default()
            },
        );

        let ground_label = "GROUND";
        let ground_label_font_size = self.title_font_size();
        let ground_label_metrics =
            measure_text(ground_label, self.text_font.as_ref(), ground_label_font_size, 1.0);
        draw_text_ex(
            ground_label,
            rect.x + rect.w - self.title_side_margin_px() - ground_label_metrics.width,
            rect.y + top_label_baseline,
            TextParams {
                font: self.text_font.as_ref(),
                font_size: ground_label_font_size,
                color: Color::from_rgba(185, 198, 224, 255),
                ..Default::default()
            },
        );
        for (y, measure_pos) in measure_labels {
            let label = self.format_measure_label(measure_pos);
            draw_text_ex(
                &label,
                rect.x + self.title_side_margin_px(),
                y - barline_label_baseline_offset,
                TextParams {
                    font: self.text_font.as_ref(),
                    font_size: barline_label_font_size,
                    color: Color::from_rgba(182, 212, 255, 240),
                    ..Default::default()
                },
            );
        }
        self.end_view_clip_rect();

    }



}

