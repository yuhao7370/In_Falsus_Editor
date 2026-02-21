// 文件说明：地面轨道输入与定位处理。
// 主要功能：处理 Tap/Hold 的放置、续点与时间定位操作。
impl FallingGroundEditor {
    fn handle_ground_input(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 {
            return;
        }
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();
        let inside = point_in_rect(mx, my, rect);

        if safe_mouse_button_pressed(MouseButton::Left) && inside {
            if let Some(tool) = self.selection.place_note_type {
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
                        self.snapshot_for_undo();
                        self.push_note(GroundNote {
                            id: self.editor_state.next_note_id,
                            kind: GroundNoteKind::Tap,
                            lane,
                            time_ms,
                            duration_ms: 0.0,
                            width: 1.0,
                            flick_right: true,
                            x_split: 1.0,
                            center_x_norm: 0.0,
                            skyarea_shape: None,
                        });
                        self.status = "new tap created".to_owned();
                    }
                    PlaceNoteType::Hold => {
                        if let Some(pending) = self.selection.pending_hold.take() {
                            let start = pending.start_time_ms.min(time_ms);
                            let end = pending.start_time_ms.max(time_ms);
                            let duration = (end - start).max(0.0);
                            self.snapshot_for_undo();
                            self.push_note(GroundNote {
                                id: self.editor_state.next_note_id,
                                kind: GroundNoteKind::Hold,
                                lane: pending.lane,
                                time_ms: start,
                                duration_ms: duration,
                                width: 1.0,
                                flick_right: true,
                                x_split: 1.0,
                                center_x_norm: 0.0,
                                skyarea_shape: None,
                            });
                            self.status = format!(
                                "new hold created lane={} {}ms -> {}ms",
                                pending.lane,
                                start.round(),
                                end.round()
                            );
                        } else {
                            self.selection.pending_hold = Some(PendingHoldPlacement {
                                lane,
                                start_time_ms: time_ms,
                            });
                            self.status =
                                format!("hold head set: lane={} time={:.0}ms", lane, time_ms);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Multi-drag: update all selected ground notes together
        if self.selection.multi_drag_state.is_some() {
            if safe_mouse_button_down(MouseButton::Left) {
                let start_sec = self
                    .selection
                    .multi_drag_state
                    .as_ref()
                    .unwrap()
                    .start_time_sec;
                if get_time() - start_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
                match self.selection.multi_drag_state.as_ref().unwrap().mode {
                    MultiDragMode::GroundFull => self.update_multi_drag_ground(rect, current_ms),
                    MultiDragMode::TimeOnly => self.update_multi_drag_time_only(rect, current_ms),
                    MultiDragMode::AirFull => self.update_multi_drag_time_only(rect, current_ms),
                }
                self.status = format!(
                    "multi-drag {} note(s)",
                    self.selection.selected_note_ids.len()
                );
            } else {
                self.finish_multi_drag();
            }
            return;
        }

        if let Some(drag) = self.selection.drag_state {
            if safe_mouse_button_down(MouseButton::Left) {
                if get_time() - drag.start_time_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
                let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                if let Some(note) = self
                    .editor_state
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_ground_kind(note.kind))
                {
                    let target_lane = mouse_lane - drag.lane_offset;
                    let eff_w = ground_note_effective_width(note.lane, note.width);
                    let lane = if eff_w > 1 {
                        // 宽音符只能在 1 到 5-eff_w 范围内
                        target_lane.clamp(1, (5 - eff_w as i32).max(1)) as usize
                    } else {
                        target_lane.clamp(0, (LANE_COUNT as i32) - 1) as usize
                    };
                    note.lane = lane;
                    note.time_ms = snapped_time;
                    self.editor_state.cached_note_heads_dirty = true;
                    self.editor_state.cached_note_render_dirty = true;
                    self.status = format!("dragging lane={} time={:.0}ms", lane, note.time_ms);
                }
            } else {
                self.selection.drag_state = None;
                self.sort_notes();
                self.refresh_note_edit_backup();
            }
        }
    }
}
