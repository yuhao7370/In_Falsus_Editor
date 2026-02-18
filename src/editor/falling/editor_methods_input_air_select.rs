// 文件说明：空中轨道的选择与放置输入处理。
// 主要功能：处理 Flick/SkyArea 的点击、放置和选中逻辑。
impl FallingGroundEditor {
    fn handle_air_input(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 {
            return;
        }
        let split_rect = air_split_rect(rect);
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();
        let inside = point_in_rect(mx, my, split_rect);

        if safe_mouse_button_pressed(MouseButton::Left) && inside {
            if let Some(tool) = self.place_note_type {
                if !is_air_tool(tool) {
                    return;
                }
                let time_ms = self.apply_snap(
                    self.pointer_to_time(my, current_ms, judge_y, rect.h)
                        .max(0.0),
                );
                let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                let lane = air_x_to_lane(x_norm);

                match tool {
                    PlaceNoteType::Flick => {
                        self.snapshot_for_undo();
                        self.push_note(GroundNote {
                            id: self.next_note_id,
                            kind: GroundNoteKind::Flick,
                            lane,
                            time_ms,
                            duration_ms: 0.0,
                            width: DEFAULT_AIR_WIDTH_NORM,
                            flick_right: true,
                            x_split: self.x_split,
                            center_x_norm: x_norm,
                            skyarea_shape: None,
                        });
                        self.status = "new flick created".to_owned();
                    }
                    PlaceNoteType::SkyArea => {
                        let width_norm = DEFAULT_SKYAREA_WIDTH_NORM;
                        let half = width_norm * 0.5;
                        if let Some(pending) = self.pending_skyarea.take() {
                            self.snapshot_for_undo();
                            let (start_time_ms, end_time_ms, start_center_norm, end_center_norm) =
                                if pending.start_time_ms <= time_ms {
                                    (pending.start_time_ms, time_ms, pending.start_center_norm, x_norm)
                                } else {
                                    (time_ms, pending.start_time_ms, x_norm, pending.start_center_norm)
                                };
                            let start_left = (start_center_norm - half).clamp(0.0, 1.0);
                            let start_right = (start_center_norm + half).clamp(0.0, 1.0);
                            let end_left = (end_center_norm - half).clamp(0.0, 1.0);
                            let end_right = (end_center_norm + half).clamp(0.0, 1.0);
                            let sky_avg_center = ((start_center_norm + end_center_norm) * 0.5).clamp(0.0, 1.0);
                            self.push_note(GroundNote {
                                id: self.next_note_id,
                                kind: GroundNoteKind::SkyArea,
                                lane: air_x_to_lane(sky_avg_center),
                                time_ms: start_time_ms,
                                duration_ms: (end_time_ms - start_time_ms).max(0.0),
                                width: width_norm,
                                flick_right: true,
                                x_split: self.x_split,
                                center_x_norm: sky_avg_center,
                                skyarea_shape: Some(SkyAreaShape {
                                    start_left_norm: start_left,
                                    start_right_norm: start_right,
                                    end_left_norm: end_left,
                                    end_right_norm: end_right,
                                    left_ease: Ease::Linear,
                                    right_ease: Ease::Linear,
                                    start_x_split: self.x_split,
                                    end_x_split: self.x_split,
                                }),
                            });
                            self.status = format!(
                                "new skyarea created {:.0}ms -> {:.0}ms",
                                start_time_ms.round(),
                                end_time_ms.round()
                            );
                        } else {
                            self.pending_skyarea = Some(PendingSkyAreaPlacement {
                                start_time_ms: time_ms,
                                start_center_norm: x_norm,
                            });
                            self.status = format!(
                                "skyarea head set x={:.3} time={:.0}ms",
                                x_norm,
                                time_ms.round()
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(drag) = self.drag_state {
            if safe_mouse_button_down(MouseButton::Left) {
                if get_time() - drag.start_time_sec < DRAG_HOLD_TO_START_SEC {
                    return;
                }
                let new_time =
                    self.pointer_to_time(my, current_ms, judge_y, rect.h) + drag.time_offset_ms;
                let snapped_time = self.apply_snap(new_time.max(0.0));
                let x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                if let Some(note) = self
                    .notes
                    .iter_mut()
                    .find(|note| note.id == drag.note_id && is_air_kind(note.kind))
                {
                    if note.kind == GroundNoteKind::SkyArea {
                        let old_tail = note.time_ms + note.duration_ms;
                        if let Some(shape) = note.skyarea_shape.as_mut() {
                            let start_half_now = ((shape.start_right_norm - shape.start_left_norm).abs() * 0.5)
                                .clamp(0.01, 0.5);
                            let end_half_now = ((shape.end_right_norm - shape.end_left_norm).abs() * 0.5)
                                .clamp(0.01, 0.5);

                            match drag.air_target {
                                AirDragTarget::Body => {
                                    // Body drag keeps skyarea easing shape, only translating start/end X together.
                                    // Use one shared delta and edge-based limits, so:
                                    // 1) head/tail widths stay unchanged
                                    // 2) head-tail X gap stays unchanged
                                    // 3) both head and tail stay in [0, 1]
                                    let start_half = drag.sky_start_half_norm.clamp(0.01, 0.5);
                                    let end_half = drag.sky_end_half_norm.clamp(0.01, 0.5);
                                    let start_left_0 = drag.sky_start_center_norm - start_half;
                                    let start_right_0 = drag.sky_start_center_norm + start_half;
                                    let end_left_0 = drag.sky_end_center_norm - end_half;
                                    let end_right_0 = drag.sky_end_center_norm + end_half;
                                    let delta_norm = (mx - drag.start_mouse_x) / split_rect.w.max(1.0);
                                    let delta_min = (-start_left_0).max(-end_left_0);
                                    let delta_max = (1.0 - start_right_0).min(1.0 - end_right_0);
                                    let delta = if delta_min <= delta_max {
                                        delta_norm.clamp(delta_min, delta_max)
                                    } else {
                                        0.0
                                    };
                                    shape.start_left_norm = start_left_0 + delta;
                                    shape.start_right_norm = start_right_0 + delta;
                                    shape.end_left_norm = end_left_0 + delta;
                                    shape.end_right_norm = end_right_0 + delta;

                                    note.time_ms = snapped_time;
                                }
                                AirDragTarget::SkyHead => {
                                    let start_center = x_norm.clamp(start_half_now, 1.0 - start_half_now);
                                    shape.start_left_norm = (start_center - start_half_now).clamp(0.0, 1.0);
                                    shape.start_right_norm = (start_center + start_half_now).clamp(0.0, 1.0);

                                    let new_start = snapped_time.min(old_tail);
                                    note.time_ms = new_start.max(0.0);
                                    note.duration_ms = (old_tail - note.time_ms).max(0.0);
                                }
                                AirDragTarget::SkyTail => {
                                    let end_center = x_norm.clamp(end_half_now, 1.0 - end_half_now);
                                    shape.end_left_norm = (end_center - end_half_now).clamp(0.0, 1.0);
                                    shape.end_right_norm = (end_center + end_half_now).clamp(0.0, 1.0);

                                    let tail_time = snapped_time.max(note.time_ms);
                                    note.duration_ms = (tail_time - note.time_ms).max(0.0);
                                }
                            }

                            let center_norm = ((shape.start_left_norm
                                + shape.start_right_norm
                                + shape.end_left_norm
                                + shape.end_right_norm)
                                * 0.25)
                                .clamp(0.0, 1.0);
                            let start_w = (shape.start_right_norm - shape.start_left_norm).abs().clamp(0.02, 1.0);
                            let end_w = (shape.end_right_norm - shape.end_left_norm).abs().clamp(0.02, 1.0);
                            note.lane = air_x_to_lane(center_norm);
                            note.center_x_norm = center_norm;
                            note.width = ((start_w + end_w) * 0.5).clamp(0.05, 1.0);
                        }
                    } else {
                        note.lane = air_x_to_lane(x_norm);
                        note.center_x_norm = x_norm;
                        note.time_ms = snapped_time;
                    }
                    self.status = format!("air drag lane={} time={:.0}ms", note.lane, note.time_ms);
                }
            } else {
                self.drag_state = None;
                self.sort_notes();
            }
        }
    }

    fn handle_note_selection_click(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let (mx, my) = safe_mouse_position();

        if safe_mouse_button_pressed(MouseButton::Right) {
            self.selected_note_id = None;
            self.drag_state = None;
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
            self.status = "selection cleared".to_owned();
            return;
        }

        if !safe_mouse_button_pressed(MouseButton::Left) {
            return;
        }

        let (scope, candidates) = self.collect_hit_candidates(mx, my, ground_rect, air_rect, current_ms);
        if candidates.is_empty() {
            // Blank click or out-of-surface click should reset click-cycle + drag latch.
            // Keep selected_note_id unchanged so user can inspect last selection.
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
            self.drag_state = None;
            return;
        }

        let ordered_items: Vec<HitSignatureItem> = candidates.iter().map(hit_signature_item).collect();
        let signature = canonical_hit_signature(&ordered_items);
        let (anchor_x, anchor_y) = quantize_overlap_anchor(mx, my);
        let now_sec = get_time();
        let mut did_cycle = false;
        let selected_note_index = self
            .selected_note_id
            .and_then(|selected_id| candidates.iter().position(|c| c.note_id == selected_id));

        let selected_index = if candidates.len() > 1 {
            // In overlap region, prefer keeping current selected note on single click.
            // Cycling to another overlapped note is only via overlap double-click.
            let mut index = selected_note_index.unwrap_or(0);
            let mut double_click_armed = selected_note_index.is_some();
            if let Some(prev) = &self.overlap_cycle {
                if prev.scope == scope
                    && prev.anchor_x == anchor_x
                    && prev.anchor_y == anchor_y
                    && prev.signature == signature
                {
                    let previous_in_current = ordered_items
                        .iter()
                        .position(|item| *item == prev.selected_item)
                        .unwrap_or_else(|| prev.current_index.min(candidates.len().saturating_sub(1)));
                    if prev.double_click_armed {
                        let elapsed = now_sec - prev.last_click_time_sec;
                        if elapsed <= OVERLAP_DOUBLE_CLICK_SEC {
                            index = (previous_in_current + 1) % candidates.len();
                            did_cycle = true;
                            double_click_armed = false;
                        } else {
                            // Prior pair expired; this click becomes the new first click.
                            index = selected_note_index.unwrap_or(previous_in_current);
                            double_click_armed = true;
                        }
                    } else {
                        index = selected_note_index.unwrap_or(previous_in_current);
                        double_click_armed = true;
                    }
                }
            }
            let selected_item = ordered_items[index];
            self.overlap_cycle = Some(OverlapCycleState {
                signature,
                current_index: index,
                selected_item,
                anchor_x,
                anchor_y,
                scope,
                last_click_time_sec: now_sec,
                double_click_armed,
            });
            index
        } else {
            self.overlap_cycle = None;
            0
        };

        let selected = candidates[selected_index];
        self.selected_note_id = Some(selected.note_id);
        self.selected_event_id = None;
        self.event_overlap_cycle = None;
        self.event_hover_hint = None;
        self.start_drag_for_candidate(selected, mx, my, current_ms, ground_rect, air_rect);
        if candidates.len() > 1 && did_cycle {
            self.status = format!(
                "overlap select {}/{} (note={})",
                selected_index + 1,
                candidates.len(),
                selected.note_id
            );
        }
    }



}

