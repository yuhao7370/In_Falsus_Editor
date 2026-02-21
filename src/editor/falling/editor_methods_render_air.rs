// 文件说明：空中轨道渲染实现。
// 主要功能：绘制 SKY 视图背景、网格、拍线与空中音符。
impl FallingGroundEditor {
    fn draw_air_view(&self, rect: Rect, current_ms: f32, overlay_mode: bool, show_spectrum: bool) {
        if rect.h <= 8.0 {
            return;
        }
        self.begin_view_clip_rect(rect);
        let split_rect = air_split_rect(rect);

        if overlay_mode {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::from_rgba(48, 40, 78, 28),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.0,
                Color::from_rgba(86, 94, 124, 120),
            );
        } else {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::from_rgba(14, 18, 26, 255),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                1.0,
                Color::from_rgba(44, 58, 84, 255),
            );
        }

        for i in 0..=4 {
            let x = split_rect.x + split_rect.w * (i as f32 / 4.0);
            let color = if i == 0 || i == 4 {
                if overlay_mode {
                    Color::from_rgba(136, 152, 196, 180)
                } else {
                    Color::from_rgba(56, 76, 110, 255)
                }
            } else {
                if overlay_mode {
                    Color::from_rgba(102, 118, 160, 138)
                } else {
                    Color::from_rgba(42, 56, 84, 220)
                }
            };
            draw_line(x, rect.y, x, rect.y + rect.h, 1.0, color);
        }

        let judge_y = rect.y + rect.h * 0.82;
        let flick_side_h = self.flick_side_height_px(rect.h);
        let (_ahead_ms, _behind_ms) =
            self.visible_ahead_behind_ms(rect.y, rect.h, current_ms, judge_y);
        let top_label_baseline = self.title_top_baseline_px();
        let barline_label_font_size = self.barline_label_font_size();
        let barline_label_min_y = rect.y + self.scaled_ui_px(14.0);
        let barline_label_baseline_offset = self.scaled_ui_px(2.0);
        let mut measure_labels: Vec<(f32, f32)> = Vec::new();

        if show_spectrum {
            self.draw_falling_spectrum(
                split_rect,
                current_ms,
                judge_y,
                Color::from_rgba(178, 196, 255, 255),
            );
        }

        let current_vb = self.editor_state.track_timeline.visual_beat_at(current_ms);
        let pixels_per_ms_bl = (self.view.scroll_speed * rect.h / 1000.0).max(0.001);
        // 计算视口对应的 visual_beat 范围（上方和下方）
        let vb_above = (judge_y - rect.y) / pixels_per_ms_bl;
        let vb_below = (rect.y + rect.h - judge_y) / pixels_per_ms_bl;
        let start_vb = current_vb - vb_below - 1.0;
        let end_vb = current_vb + vb_above + 1.0;
        if self.view.show_barlines {
            for barline in self.visible_barlines_cached(start_vb, end_vb) {
                let y = judge_y - (barline.visual_beat - current_vb) * pixels_per_ms_bl;
                if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                    continue;
                }
                let (thickness, color) = match barline.kind {
                    BarLineKind::Measure => (2.1, Color::from_rgba(164, 198, 255, 210)),
                    BarLineKind::Beat => (1.3, Color::from_rgba(108, 140, 186, 182)),
                    BarLineKind::Subdivision => (0.9, Color::from_rgba(74, 102, 136, 140)),
                };
                draw_line(
                    split_rect.x,
                    y,
                    split_rect.x + split_rect.w,
                    y,
                    thickness,
                    color,
                );
                if !overlay_mode
                    && barline.show_measure_label
                    && y >= barline_label_min_y
                    && y <= rect.y + rect.h - barline_label_baseline_offset
                {
                    measure_labels.push((y, barline.measure_pos));
                }
            }
        }

        // 分两次绘制空中音符，保证 Flick 永远在 SkyArea 上层。
        for flick_pass in [false, true] {
            for note in &self.editor_state.notes {
                if !is_air_kind(note.kind) {
                    continue;
                }
                if flick_pass != (note.kind == GroundNoteKind::Flick) {
                    continue;
                }

                // AutoPlay: 已被判定的音符不显示（或裁剪判定线以下部分）
                let judged = self.view.autoplay_enabled && note.time_ms <= current_ms;
                if judged && !note.has_tail() {
                    continue;
                }
                if judged && note.has_tail() && note.end_time_ms() <= current_ms {
                    continue;
                }

                let x_norm = note.center_x_norm;
                let center_x = split_rect.x + x_norm * split_rect.w;
                let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
                let selected = self.selection.selected_note_ids.contains(&note.id);
                let lane_for_palette = note.lane.clamp(0, LANE_COUNT - 1);
                let palette = lane_note_palette(lane_for_palette);

                let note_w = air_note_width(note, split_rect.w);
                let note_x = center_x - note_w * 0.5;

                if note.kind == GroundNoteKind::SkyArea {
                    if let Some(shape) = note.skyarea_shape {
                        self.draw_skyarea_shape(
                            split_rect, current_ms, judge_y, rect.h, note, shape, selected,
                        );
                        continue;
                    }
                }

                if note.has_tail() {
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                    let y1 = head_y.min(tail_y);
                    let y2 = head_y.max(tail_y);
                    if y2 >= rect.y && y1 <= rect.y + rect.h {
                        let body_y = y1.max(rect.y);
                        let mut body_end = y2.min(rect.y + rect.h);
                        // AutoPlay: 裁剪判定线以下部分
                        if judged {
                            body_end = body_end.min(judge_y);
                        }
                        let body_h = (body_end - body_y).max(0.0);
                        if body_h > 0.0 {
                            let body_color = match note.kind {
                                GroundNoteKind::SkyArea => AIR_SKYAREA_BODY_COLOR,
                                _ => palette.hold_body,
                            };
                            draw_rectangle(note_x, body_y, note_w, body_h, body_color);
                            if selected {
                                draw_selected_note_darken_rect(note_x, body_y, note_w, body_h);
                            }
                        }
                    }
                }

                // AutoPlay: head 已判定则不画 head
                if !judged && head_y >= rect.y - 24.0 && head_y <= rect.y + rect.h + 24.0 {
                    if note.kind == GroundNoteKind::Flick {
                        draw_flick_curve_shape(note, note_x, note_w, head_y, flick_side_h);
                        if selected {
                            let bounds =
                                flick_shape_bounds(note, note_x, note_w, head_y, flick_side_h);
                            draw_selected_note_darken_rect(bounds.x, bounds.y, bounds.w, bounds.h);
                            draw_selected_note_outline(bounds.x, bounds.y, bounds.w, bounds.h);
                        }
                    } else {
                        let head_color = match note.kind {
                            GroundNoteKind::SkyArea => AIR_SKYAREA_HEAD_COLOR,
                            _ => palette.tap,
                        };
                        draw_rectangle(note_x, head_y - 8.0, note_w, 16.0, head_color);
                        draw_rectangle(
                            note_x + 1.0,
                            head_y - 7.0,
                            (note_w - 2.0).max(1.0),
                            5.0,
                            Color::from_rgba(255, 255, 255, 34),
                        );

                        if selected {
                            draw_selected_note_darken_rect(note_x, head_y - 8.0, note_w, 16.0);
                            draw_selected_note_outline(note_x, head_y - 8.0, note_w, 16.0);
                        }
                    }
                }
            }
        }

        if self.view.debug_show_hitboxes {
            self.draw_air_hitbox_overlay(rect, current_ms);
        }

        draw_line(
            split_rect.x,
            judge_y,
            split_rect.x + split_rect.w,
            judge_y,
            3.0,
            if overlay_mode {
                Color::from_rgba(170, 206, 255, 220)
            } else {
                Color::from_rgba(132, 196, 255, 255)
            },
        );
        draw_text_ex(
            "SKY",
            rect.x + self.title_side_margin_px(),
            rect.y + top_label_baseline,
            TextParams {
                font: self.view.text_font.as_ref(),
                font_size: self.title_font_size(),
                color: if overlay_mode {
                    Color::from_rgba(214, 226, 250, 230)
                } else {
                    Color::from_rgba(190, 216, 255, 255)
                },
                ..Default::default()
            },
        );
        for (y, measure_pos) in measure_labels {
            let label = self.format_measure_label(measure_pos);
            draw_text_ex(
                &label,
                split_rect.x + self.title_side_margin_px(),
                y - barline_label_baseline_offset,
                TextParams {
                    font: self.view.text_font.as_ref(),
                    font_size: barline_label_font_size,
                    color: Color::from_rgba(188, 216, 255, 236),
                    ..Default::default()
                },
            );
        }
        self.end_view_clip_rect();
    }
}
