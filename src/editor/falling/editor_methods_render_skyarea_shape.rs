impl FallingGroundEditor {
    fn draw_skyarea_shape(
        &self,
        split_rect: Rect,
        current_ms: f32,
        judge_y: f32,
        lane_h: f32,
        note: &GroundNote,
        shape: SkyAreaShape,
        selected: bool,
    ) {
        let clip_top = split_rect.y;
        let clip_bottom = split_rect.y + split_rect.h;
        let has_tail = note.duration_ms > 0.0;

        if has_tail {
            let seg_count = 20;
            for i in 0..seg_count {
                let p0 = i as f32 / seg_count as f32;
                let p1 = (i + 1) as f32 / seg_count as f32;
                let t0 = note.time_ms + note.duration_ms * p0;
                let t1 = note.time_ms + note.duration_ms * p1;
                let y0_raw = self.time_to_y(t0, current_ms, judge_y, lane_h);
                let y1_raw = self.time_to_y(t1, current_ms, judge_y, lane_h);
                if (y0_raw < clip_top && y1_raw < clip_top)
                    || (y0_raw > clip_bottom && y1_raw > clip_bottom)
                {
                    continue;
                }
                let y0 = y0_raw.clamp(clip_top, clip_bottom);
                let y1 = y1_raw.clamp(clip_top, clip_bottom);

                let left0 = lerp(
                    shape.start_left_norm,
                    shape.end_left_norm,
                    ease_progress(shape.left_ease, p0),
                );
                let right0 = lerp(
                    shape.start_right_norm,
                    shape.end_right_norm,
                    ease_progress(shape.right_ease, p0),
                );
                let left1 = lerp(
                    shape.start_left_norm,
                    shape.end_left_norm,
                    ease_progress(shape.left_ease, p1),
                );
                let right1 = lerp(
                    shape.start_right_norm,
                    shape.end_right_norm,
                    ease_progress(shape.right_ease, p1),
                );

                let lx0 = split_rect.x + left0.clamp(0.0, 1.0) * split_rect.w;
                let rx0 = split_rect.x + right0.clamp(0.0, 1.0) * split_rect.w;
                let lx1 = split_rect.x + left1.clamp(0.0, 1.0) * split_rect.w;
                let rx1 = split_rect.x + right1.clamp(0.0, 1.0) * split_rect.w;

                draw_triangle(
                    Vec2::new(lx0, y0),
                    Vec2::new(rx0, y0),
                    Vec2::new(rx1, y1),
                    AIR_SKYAREA_BODY_COLOR,
                );
                draw_triangle(
                    Vec2::new(lx0, y0),
                    Vec2::new(rx1, y1),
                    Vec2::new(lx1, y1),
                    AIR_SKYAREA_BODY_COLOR,
                );
                if selected {
                    let dark = Color::from_rgba(0, 0, 0, SELECTED_NOTE_DARKEN_ALPHA);
                    draw_triangle(
                        Vec2::new(lx0, y0),
                        Vec2::new(rx0, y0),
                        Vec2::new(rx1, y1),
                        dark,
                    );
                    draw_triangle(
                        Vec2::new(lx0, y0),
                        Vec2::new(rx1, y1),
                        Vec2::new(lx1, y1),
                        dark,
                    );
                }
            }
        }

        let head_left = split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_right = split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
        let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, lane_h);
        let tail_left = split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
        let tail_right = split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
        let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, lane_h);

        let head_w = (head_right - head_left).max(2.0);
        if head_y >= clip_top - 18.0 && head_y <= clip_bottom + 18.0 {
            draw_rectangle(head_left, head_y - 8.0, head_w, 16.0, AIR_SKYAREA_HEAD_COLOR);
            if selected {
                draw_selected_note_darken_rect(head_left, head_y - 8.0, head_w, 16.0);
                draw_selected_note_outline(head_left, head_y - 8.0, head_w, 16.0);
            }
        }

        let tail_w = (tail_right - tail_left).max(2.0);
        if has_tail && tail_y >= clip_top - 18.0 && tail_y <= clip_bottom + 18.0 {
            draw_rectangle(tail_left, tail_y - 8.0, tail_w, 16.0, AIR_SKYAREA_TAIL_COLOR);
            if selected {
                draw_selected_note_darken_rect(tail_left, tail_y - 8.0, tail_w, 16.0);
                draw_selected_note_outline(tail_left, tail_y - 8.0, tail_w, 16.0);
            }
        }
    }



}

