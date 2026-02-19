// 文件说明：命中区域调试渲染实现。
// 主要功能：可视化音符命中盒与交互判定区域，便于调试。
impl FallingGroundEditor {
    fn draw_ground_hitbox_overlay(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let head_color = Color::from_rgba(84, 230, 255, 232);
        let tail_color = Color::from_rgba(255, 164, 88, 228);
        let body_color = Color::from_rgba(138, 255, 152, 218);

        let view_top = rect.y - 40.0;
        let view_bottom = rect.y + rect.h + 40.0;

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let note_w = note_head_width(note, lane_w);
            let note_x = ground_note_x(note, rect.x, lane_w);
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);

            // 屏幕外裁剪：无尾音符只看 head，有尾音符看 head+tail 范围
            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y_min = head_y.min(tail_y);
                let y_max = head_y.max(tail_y);
                if y_max < view_top || y_min > view_bottom {
                    continue;
                }
            } else if head_y < view_top || head_y > view_bottom {
                continue;
            }

            let head_rect = note_end_hit_rect(note_x, note_w, head_y);
            draw_debug_hitbox_rect(head_rect, rect, head_color, 1.3);

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let tail_rect = note_end_hit_rect(note_x, note_w, tail_y);
                draw_debug_hitbox_rect(tail_rect, rect, tail_color, 1.3);

                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let (body_x, body_w) = match note.kind {
                    GroundNoteKind::Hold => (note_x + note_w * 0.04, note_w * 0.92),
                    GroundNoteKind::SkyArea => (note_x + note_w * 0.02, note_w * 0.96),
                    _ => (note_x + note_w * 0.34, note_w * 0.32),
                };
                let body_rect = note_body_hit_rect(body_x, body_w, y1, y2);
                draw_debug_hitbox_rect(body_rect, rect, body_color, 1.2);
            }

            let label = format!("#{}", note.id);
            let label_color = if self.selected_note_ids.contains(&note.id)
                || self.selected_note_id == Some(note.id)
            {
                Color::from_rgba(255, 255, 60, 255)
            } else {
                head_color
            };
            draw_debug_hitbox_label(head_rect, rect, &label, label_color, self.text_font.as_ref());
        }
    }

    fn draw_air_hitbox_overlay(&self, rect: Rect, current_ms: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }
        let split_rect = air_split_rect(rect);
        let clip_rect = rect;
        let judge_y = rect.y + rect.h * 0.82;
        let head_color = Color::from_rgba(116, 234, 255, 232);
        let tail_color = Color::from_rgba(246, 186, 114, 228);
        let body_color = Color::from_rgba(176, 144, 255, 214);

        let view_top = rect.y - 40.0;
        let view_bottom = rect.y + rect.h + 40.0;

        for note in &self.notes {
            if !is_air_kind(note.kind) {
                continue;
            }

            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            // 屏幕外裁剪
            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y_min = head_y.min(tail_y);
                let y_max = head_y.max(tail_y);
                if y_max < view_top || y_min > view_bottom {
                    continue;
                }
            } else if head_y < view_top || head_y > view_bottom {
                continue;
            }

            let center_x = split_rect.x + note.center_x_norm * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);
            let mut label_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_left = split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_right = split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);

                    let head_rect =
                        note_end_hit_rect(head_left, (head_right - head_left).max(2.0), head_y);
                    let tail_rect =
                        note_end_hit_rect(tail_left, (tail_right - tail_left).max(2.0), tail_y);
                    draw_debug_hitbox_rect(head_rect, clip_rect, head_color, 1.3);
                    draw_debug_hitbox_rect(tail_rect, clip_rect, tail_color, 1.3);
                    draw_debug_skyarea_body_hit_overlay(split_rect, shape, head_y, tail_y, body_color);
                    label_rect = head_rect;
                }
            } else {
                let head_rect = if note.kind == GroundNoteKind::Flick {
                    flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
                } else {
                    note_end_hit_rect(note_x, note_w, head_y)
                };
                draw_debug_hitbox_rect(head_rect, clip_rect, head_color, 1.3);
                label_rect = head_rect;
                if note.has_tail() {
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                    let body_rect = note_body_hit_rect(note_x, note_w, head_y.min(tail_y), head_y.max(tail_y));
                    draw_debug_hitbox_rect(body_rect, clip_rect, body_color, 1.2);
                }
            }

            let label = format!("#{}", note.id);
            let label_color = if self.selected_note_ids.contains(&note.id)
                || self.selected_note_id == Some(note.id)
            {
                Color::from_rgba(255, 255, 60, 255)
            } else {
                head_color
            };
            draw_debug_hitbox_label(
                label_rect,
                clip_rect,
                &label,
                label_color,
                self.text_font.as_ref(),
            );
        }
    }



}

