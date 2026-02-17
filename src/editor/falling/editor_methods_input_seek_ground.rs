// 文件说明：地面轨道输入与定位处理。
// 主要功能：处理 Tap/Hold 的放置、续点与时间定位操作。
impl FallingGroundEditor {
    fn handle_ground_input(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 {
            return;
        }
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = mouse_position();
        let inside = point_in_rect(mx, my, rect);

        if is_mouse_button_pressed(MouseButton::Left) && inside {
            if let Some(tool) = self.place_note_type {
                if !is_ground_tool(tool) {
                    return;
                }
                let lane = lane_from_x(mx, rect.x, lane_w);
                let time_ms = self.apply_snap(
                    self.pointer_to_time(my, current_ms, judge_y, rect.h)
                        .max(0.0),
                );

                match tool {
                    PlaceNoteType::Tap => {
                        self.push_note(GroundNote {
                            id: self.next_note_id,
                            kind: GroundNoteKind::Tap,
                            lane,
                            time_ms,
                            duration_ms: 0.0,
                            width: 1.0,
                            flick_right: true,
                            skyarea_shape: None,
                        });
                        self.status = "new tap created".to_owned();
                    }
                    PlaceNoteType::Hold => {
                        if let Some(pending) = self.pending_hold.take() {
                            let start = pending.start_time_ms.min(time_ms);
                            let end = pending.start_time_ms.max(time_ms);
                            let duration = (end - start).max(0.0);
                            self.push_note(GroundNote {
                                id: self.next_note_id,
                                kind: GroundNoteKind::Hold,
                                lane: pending.lane,
                                time_ms: start,
                                duration_ms: duration,
                                width: 1.0,
                                flick_right: true,
                                skyarea_shape: None,
                            });
                            self.status = format!(
                                "new hold created lane={} {}ms -> {}ms",
                                pending.lane,
                                start.round(),
                                end.round()
                            );
                        } else {
                            self.pending_hold = Some(PendingHoldPlacement {
                                lane,
                                start_time_ms: time_ms,
                            });
                            self.status = format!("hold head set: lane={} time={:.0}ms", lane, time_ms);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(drag) = self.drag_state {
            if is_mouse_button_down(MouseButton::Left) {
                if get_time() - drag.start_time_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
                let lane = lane_from_x(mx, rect.x, lane_w);
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                if let Some(note) = self
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_ground_kind(note.kind))
                {
                    note.lane = lane;
                    note.time_ms = snapped_time;
                    self.status = format!("dragging lane={} time={:.0}ms", lane, note.time_ms);
                }
            } else {
                self.drag_state = None;
                self.sort_notes();
            }
        }
    }



}

