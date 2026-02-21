// 文件说明：放置光标与预览渲染实现。
// 主要功能：根据当前工具绘制放置预览形状和辅助指示。
impl FallingGroundEditor {
    fn draw_place_cursor(&self, rect: Rect, current_ms: f32) {
        let Some(place_type) = self.selection.place_note_type else {
            return;
        };
        if rect.h <= 8.0 {
            return;
        }

        let (mx, my) = safe_mouse_position();
        if !point_in_rect(mx, my, rect) {
            return;
        }

        if is_ground_tool(place_type) {
            let lane_w = rect.w / LANE_COUNT as f32;
            let judge_y = rect.y + rect.h * 0.82;
            let current_vb = self.editor_state.track_timeline.visual_beat_at(current_ms);
            let pixels_per_ms = self.pixels_per_ms(rect.h);
            let preview_time = self.apply_snap(
                self.pointer_to_time(my, current_ms, judge_y, rect.h)
                    .max(0.0),
            );
            let preview_y =
                self.time_to_y_from_metrics(preview_time, current_vb, judge_y, pixels_per_ms);
            let lane = lane_from_x(mx, rect.x, lane_w);
            let palette = lane_note_palette(lane);
            draw_line(
                rect.x,
                preview_y,
                rect.x + rect.w,
                preview_y,
                1.2,
                Color::from_rgba(255, 230, 132, 190),
            );
            match place_type {
                PlaceNoteType::Hold => {
                    if let Some(pending) = self.selection.pending_hold {
                        let lane_x = rect.x + lane_w * pending.lane as f32;
                        let note_w = lane_w * 0.94;
                        let note_x = lane_x + (lane_w - note_w) * 0.5;
                        let start_y = self.time_to_y_from_metrics(
                            pending.start_time_ms,
                            current_vb,
                            judge_y,
                            pixels_per_ms,
                        );
                        let y1 = start_y.min(preview_y);
                        let y2 = start_y.max(preview_y);

                        draw_rectangle(
                            note_x + note_w * 0.04,
                            y1,
                            note_w * 0.92,
                            (y2 - y1).max(1.0),
                            Color::from_rgba(236, 204, 120, 116),
                        );
                        draw_rectangle(
                            note_x,
                            start_y - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 222, 140, 220),
                        );
                        draw_rectangle(
                            note_x,
                            preview_y - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 236, 170, 220),
                        );
                    } else {
                        let lane_x = rect.x + lane_w * lane as f32;
                        let note_w = lane_w * 0.94;
                        let note_x = lane_x + (lane_w - note_w) * 0.5;
                        draw_rectangle(
                            note_x,
                            preview_y - 8.0,
                            note_w,
                            16.0,
                            Color::from_rgba(255, 222, 140, 220),
                        );
                    }
                }
                PlaceNoteType::Tap => {
                    let lane_x = rect.x + lane_w * lane as f32;
                    let note_w = lane_w * 0.78;
                    let note_x = lane_x + (lane_w - note_w) * 0.5;
                    draw_rectangle(
                        note_x,
                        preview_y - 8.0,
                        note_w,
                        16.0,
                        Color::new(palette.tap.r, palette.tap.g, palette.tap.b, 0.82),
                    );
                }
                _ => {}
            }
        } else if is_air_tool(place_type) {
            let split_rect = air_split_rect(rect);
            let flick_side_h = self.flick_side_height_px(rect.h);
            if !point_in_rect(mx, my, split_rect) {
                return;
            }
            let judge_y = rect.y + rect.h * 0.82;
            let current_vb = self.editor_state.track_timeline.visual_beat_at(current_ms);
            let pixels_per_ms = self.pixels_per_ms(rect.h);
            let preview_time = self.apply_snap(
                self.pointer_to_time(my, current_ms, judge_y, rect.h)
                    .max(0.0),
            );
            let preview_y =
                self.time_to_y_from_metrics(preview_time, current_vb, judge_y, pixels_per_ms);
            draw_line(
                split_rect.x,
                preview_y,
                split_rect.x + split_rect.w,
                preview_y,
                1.2,
                Color::from_rgba(216, 232, 255, 188),
            );
            let x_norm_raw = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
            let x_norm = snap_x_to_grid(x_norm_raw, self.editor_state.x_split);
            let flick_width_norm = (1.0_f32 / 4.0).clamp(0.05, 1.0);
            let preview_half = if place_type == PlaceNoteType::Flick {
                flick_width_norm * 0.5
            } else {
                DEFAULT_SKYAREA_WIDTH_NORM * 0.5
            };
            let center_norm = x_norm.clamp(preview_half, 1.0 - preview_half);
            let lane = air_x_to_lane(center_norm);
            let center_x = split_rect.x + center_norm * split_rect.w;
            let note_w = match place_type {
                PlaceNoteType::SkyArea => split_rect.w * DEFAULT_SKYAREA_WIDTH_NORM,
                _ => split_rect.w * flick_width_norm,
            };
            let note_x = center_x - note_w * 0.5;
            let preview = GroundNote {
                id: 0,
                kind: GroundNoteKind::Flick,
                lane,
                time_ms: preview_time,
                duration_ms: 0.0,
                width: flick_width_norm,
                flick_right: self.selection.place_flick_right,
                x_split: self.editor_state.x_split,
                center_x_norm: center_norm,
                skyarea_shape: None,
            };
            if place_type == PlaceNoteType::Flick {
                draw_flick_curve_shape(&preview, note_x, note_w, preview_y, flick_side_h);
            } else {
                if let Some(pending) = self.selection.pending_skyarea {
                    let half = DEFAULT_SKYAREA_WIDTH_NORM * 0.5;
                    let (start_time_ms, end_time_ms, raw_start, raw_end) =
                        if pending.start_time_ms <= preview_time {
                            (
                                pending.start_time_ms,
                                preview_time,
                                pending.start_center_norm,
                                x_norm,
                            )
                        } else {
                            (
                                preview_time,
                                pending.start_time_ms,
                                x_norm,
                                pending.start_center_norm,
                            )
                        };
                    let start_center_norm = raw_start.clamp(half, 1.0 - half);
                    let end_center_norm = raw_end.clamp(half, 1.0 - half);
                    let start_left = (start_center_norm - half).clamp(0.0, 1.0);
                    let start_right = (start_center_norm + half).clamp(0.0, 1.0);
                    let end_left = (end_center_norm - half).clamp(0.0, 1.0);
                    let end_right = (end_center_norm + half).clamp(0.0, 1.0);
                    let shape = SkyAreaShape {
                        start_left_norm: start_left,
                        start_right_norm: start_right,
                        end_left_norm: end_left,
                        end_right_norm: end_right,
                        left_ease: Ease::Linear,
                        right_ease: Ease::Linear,
                        start_x_split: self.editor_state.x_split,
                        end_x_split: self.editor_state.x_split,
                        group_id: -1,
                    };
                    let sky_preview_center =
                        ((start_center_norm + end_center_norm) * 0.5).clamp(0.0, 1.0);
                    let preview_note = GroundNote {
                        id: 0,
                        kind: GroundNoteKind::SkyArea,
                        lane: air_x_to_lane(sky_preview_center),
                        time_ms: start_time_ms,
                        duration_ms: (end_time_ms - start_time_ms).max(0.0),
                        width: DEFAULT_SKYAREA_WIDTH_NORM,
                        flick_right: true,
                        x_split: self.editor_state.x_split,
                        center_x_norm: sky_preview_center,
                        skyarea_shape: Some(shape),
                    };
                    self.draw_skyarea_shape(
                        split_rect,
                        current_ms,
                        current_vb,
                        judge_y,
                        pixels_per_ms,
                        &preview_note,
                        shape,
                        false,
                    );
                } else {
                    draw_rectangle(
                        note_x,
                        preview_y - 8.0,
                        note_w,
                        16.0,
                        AIR_SKYAREA_HEAD_COLOR,
                    );
                }
            }
        }
    }
}
