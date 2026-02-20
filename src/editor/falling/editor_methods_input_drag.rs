// Drag editing behavior implementation.
// Handles drag position updates and constraints.
impl FallingGroundEditor {
    fn start_drag_for_candidate(
        &mut self,
        candidate: HitCandidate,
        mx: f32,
        my: f32,
        current_ms: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
    ) {
        let Some(note) = self
            .editor_state
            .notes
            .iter()
            .find(|note| note.id == candidate.note_id)
        else {
            self.selection.drag_state = None;
            return;
        };

        let (judge_y, lane_h) = match candidate.scope {
            HitScope::Ground => {
                let Some(rect) = ground_rect else {
                    self.selection.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Air => {
                let Some(rect) = air_rect else {
                    self.selection.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Mixed => {
                self.selection.drag_state = None;
                return;
            }
        };

        let raw_pointer_time_ms = self.pointer_to_time(my, current_ms, judge_y, lane_h);
        let pointer_time_ms = self.apply_snap(raw_pointer_time_ms.max(0.0));
        let (sky_start_center_norm, sky_end_center_norm, sky_start_half_norm, sky_end_half_norm) =
            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let start_left = shape.start_left_norm.clamp(0.0, 1.0);
                    let start_right = shape.start_right_norm.clamp(0.0, 1.0);
                    let end_left = shape.end_left_norm.clamp(0.0, 1.0);
                    let end_right = shape.end_right_norm.clamp(0.0, 1.0);
                    (
                        (start_left + start_right) * 0.5,
                        (end_left + end_right) * 0.5,
                        ((start_right - start_left).abs() * 0.5).clamp(0.01, 0.5),
                        ((end_right - end_left).abs() * 0.5).clamp(0.01, 0.5),
                    )
                } else {
                    let center = note.center_x_norm;
                    (center, center, 0.25, 0.25)
                }
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

        let drag_anchor_time_ms = if note.kind == GroundNoteKind::SkyArea
            && candidate.air_target == AirDragTarget::SkyTail
        {
            note.end_time_ms()
        } else {
            note.time_ms
        };

        // Quantize offset to snap grid to keep drag behavior stable.
        let raw_offset = drag_anchor_time_ms - pointer_time_ms;
        let quantized_offset = if self.view.snap_enabled && self.view.snap_division > 0 {
            let point = self
                .editor_state
                .timeline
                .point_at_time(drag_anchor_time_ms);
            let bpm = point.bpm.abs().max(0.001);
            let sub_ms = 60_000.0 / bpm / self.view.snap_division as f32;
            (raw_offset / sub_ms).round() * sub_ms
        } else {
            raw_offset
        };

        // Mouse lane offset from the dragged note lane at drag start.
        let lane_offset = if candidate.scope == HitScope::Ground {
            if let Some(rect) = ground_rect {
                let lane_w = rect.w / LANE_COUNT as f32;
                let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
                mouse_lane - note.lane as i32
            } else {
                0
            }
        } else {
            0
        };

        self.snapshot_for_undo();
        self.selection.drag_state = Some(DragState {
            note_id: candidate.note_id,
            time_offset_ms: quantized_offset,
            start_time_sec: get_time(),
            start_mouse_x: mx,
            lane_offset,
            sky_start_center_norm,
            sky_end_center_norm,
            sky_start_half_norm,
            sky_end_half_norm,
            air_target: candidate.air_target,
        });
    }

    /// Start a multi-note drag for all notes in `selected_note_ids`.
    /// `anchor_id` is the note the user clicked on (used as reference for offset).
    fn start_multi_drag(
        &mut self,
        anchor_id: u64,
        mx: f32,
        my: f32,
        current_ms: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
    ) {
        if self.selection.selected_note_ids.len() < 2 {
            return;
        }

        // Determine scope: all ground, all air, or mixed.
        // Build index-based bindings once, then update by index each frame.
        let mut has_ground = false;
        let mut has_air = false;
        let mut bindings: Vec<MultiDragBinding> = Vec::new();
        let mut x_splits: Vec<f64> = Vec::new();

        for (note_index, note) in self.editor_state.notes.iter().enumerate() {
            if !self.selection.selected_note_ids.contains(&note.id) {
                continue;
            }
            if is_ground_kind(note.kind) {
                has_ground = true;
            } else {
                has_air = true;
                x_splits.push(note.x_split);
            }
            let (sl, sr, el, er) = if let Some(shape) = note.skyarea_shape {
                (
                    shape.start_left_norm,
                    shape.start_right_norm,
                    shape.end_left_norm,
                    shape.end_right_norm,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
            bindings.push(MultiDragBinding {
                note_index,
                snapshot: MultiDragNoteSnapshot {
                    note_id: note.id,
                    original_time_ms: note.time_ms,
                    original_lane: note.lane,
                    original_width: note.width,
                    original_center_x_norm: note.center_x_norm,
                    sky_start_left: sl,
                    sky_start_right: sr,
                    sky_end_left: el,
                    sky_end_right: er,
                },
            });
        }

        if bindings.is_empty() {
            return;
        }

        let scope = match (has_ground, has_air) {
            (true, false) => HitScope::Ground,
            (false, true) => HitScope::Air,
            _ => HitScope::Mixed,
        };

        // Determine mode
        let mode = match scope {
            HitScope::Ground => MultiDragMode::GroundFull,
            HitScope::Air => {
                // Check if all air notes share the same x_split
                let all_same_xsplit = if let Some(&first) = x_splits.first() {
                    x_splits.iter().all(|&xs| (xs - first).abs() < 0.001)
                } else {
                    false
                };
                if all_same_xsplit {
                    MultiDragMode::AirFull
                } else {
                    MultiDragMode::TimeOnly
                }
            }
            HitScope::Mixed => MultiDragMode::TimeOnly,
        };

        // Compute time offset from anchor note.
        let anchor_snapshot = bindings
            .iter()
            .find(|binding| binding.snapshot.note_id == anchor_id)
            .map(|binding| binding.snapshot);
        let (judge_y, lane_h) = if has_ground && !has_air {
            if let Some(rect) = ground_rect {
                (rect.y + rect.h * 0.82, rect.h)
            } else if let Some(rect) = air_rect {
                (rect.y + rect.h * 0.82, rect.h)
            } else {
                return;
            }
        } else if let Some(rect) = air_rect {
            (rect.y + rect.h * 0.82, rect.h)
        } else if let Some(rect) = ground_rect {
            (rect.y + rect.h * 0.82, rect.h)
        } else {
            return;
        };

        let raw_pointer_time = self.pointer_to_time(my, current_ms, judge_y, lane_h);
        let pointer_time = self.apply_snap(raw_pointer_time.max(0.0));
        let anchor_time = anchor_snapshot
            .map(|snapshot| snapshot.original_time_ms)
            .unwrap_or(pointer_time);

        let raw_offset = anchor_time - pointer_time;
        let quantized_offset = if self.view.snap_enabled && self.view.snap_division > 0 {
            let point = self.editor_state.timeline.point_at_time(anchor_time);
            let bpm = point.bpm.abs().max(0.001);
            let sub_ms = 60_000.0 / bpm / self.view.snap_division as f32;
            (raw_offset / sub_ms).round() * sub_ms
        } else {
            raw_offset
        };

        // Lane offset for ground
        let lane_offset = if mode == MultiDragMode::GroundFull {
            if let Some(rect) = ground_rect {
                let lane_w = rect.w / LANE_COUNT as f32;
                let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
                let anchor_lane = anchor_snapshot
                    .map(|snapshot| snapshot.original_lane as i32)
                    .unwrap_or(0);
                mouse_lane - anchor_lane
            } else {
                0
            }
        } else {
            0
        };

        self.snapshot_for_undo();
        self.selection.multi_drag_state = Some(MultiDragState {
            anchor_note_id: anchor_id,
            time_offset_ms: quantized_offset,
            lane_offset,
            start_time_sec: get_time(),
            start_mouse_x: mx,
            mode,
            bindings,
        });
    }

    /// Update all notes during multi-drag (ground scope).
    fn update_multi_drag_ground(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = self.selection.multi_drag_state.as_ref() else {
            return;
        };
        let time_offset_ms = mdrag.time_offset_ms;
        let lane_offset = mdrag.lane_offset;
        let anchor_note_id = mdrag.anchor_note_id;
        let bindings = mdrag.bindings.clone();

        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();

        let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        let target_anchor_lane = mouse_lane - lane_offset;
        let anchor_snap = bindings
            .iter()
            .find(|binding| binding.snapshot.note_id == anchor_note_id)
            .map(|binding| binding.snapshot);
        let anchor_orig_lane = anchor_snap
            .map(|snapshot| snapshot.original_lane as i32)
            .unwrap_or(0);
        let anchor_orig_time = anchor_snap
            .map(|snapshot| snapshot.original_time_ms)
            .unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        let desired_lane_shift = target_anchor_lane - anchor_orig_lane;

        let mut global_min = i32::MIN;
        let mut global_max = i32::MAX;
        for binding in &bindings {
            let snapshot = binding.snapshot;
            if let Some(note) = self.editor_state.notes.get(binding.note_index) {
                if is_ground_kind(note.kind) {
                    let eff_w =
                        ground_note_effective_width(snapshot.original_lane, snapshot.original_width);
                    let orig = snapshot.original_lane as i32;
                    let (lo, hi) = if eff_w > 1 {
                        (1 - orig, (5 - eff_w as i32).max(1) - orig)
                    } else {
                        (0 - orig, (LANE_COUNT as i32 - 1) - orig)
                    };
                    global_min = global_min.max(lo);
                    global_max = global_max.min(hi);
                }
            }
        }

        let lane_shift = desired_lane_shift.clamp(global_min, global_max);

        for binding in &bindings {
            let snapshot = binding.snapshot;
            if let Some(note) = self.editor_state.notes.get_mut(binding.note_index) {
                note.time_ms = (snapshot.original_time_ms + time_delta).max(0.0);
                if is_ground_kind(note.kind) {
                    note.lane = (snapshot.original_lane as i32 + lane_shift) as usize;
                }
            }
        }
        self.editor_state.cached_note_heads_dirty = true;
    }

    /// Update all notes during multi-drag (air scope).
    fn update_multi_drag_air(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = self.selection.multi_drag_state.as_ref() else {
            return;
        };
        let time_offset_ms = mdrag.time_offset_ms;
        let mode = mdrag.mode;
        let anchor_note_id = mdrag.anchor_note_id;
        let start_mouse_x = mdrag.start_mouse_x;
        let bindings = mdrag.bindings.clone();

        let split_rect = air_split_rect(rect);
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();

        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        let anchor_snap = bindings
            .iter()
            .find(|binding| binding.snapshot.note_id == anchor_note_id)
            .map(|binding| binding.snapshot);
        let anchor_orig_time = anchor_snap
            .map(|snapshot| snapshot.original_time_ms)
            .unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        let raw_x_delta = if mode == MultiDragMode::AirFull {
            (mx - start_mouse_x) / split_rect.w.max(1.0)
        } else {
            0.0
        };

        let x_delta = if mode == MultiDragMode::AirFull {
            let mut global_left_min: f32 = 0.0;
            let mut global_right_max: f32 = 1.0;
            let mut first = true;
            for binding in &bindings {
                let snapshot = binding.snapshot;
                if let Some(note) = self.editor_state.notes.get(binding.note_index) {
                    if !is_air_kind(note.kind) {
                        continue;
                    }
                    if note.kind == GroundNoteKind::SkyArea {
                        let left = snapshot.sky_start_left.min(snapshot.sky_end_left);
                        let right = snapshot.sky_start_right.max(snapshot.sky_end_right);
                        if first {
                            global_left_min = left;
                            global_right_max = right;
                            first = false;
                        } else {
                            global_left_min = global_left_min.min(left);
                            global_right_max = global_right_max.max(right);
                        }
                    } else {
                        let half_w = snapshot.original_width.clamp(0.05, 1.0) * 0.5;
                        let left = snapshot.original_center_x_norm - half_w;
                        let right = snapshot.original_center_x_norm + half_w;
                        if first {
                            global_left_min = left;
                            global_right_max = right;
                            first = false;
                        } else {
                            global_left_min = global_left_min.min(left);
                            global_right_max = global_right_max.max(right);
                        }
                    }
                }
            }
            raw_x_delta.clamp(-global_left_min, 1.0 - global_right_max)
        } else {
            0.0
        };

        for binding in &bindings {
            let snapshot = binding.snapshot;
            if let Some(note) = self.editor_state.notes.get_mut(binding.note_index) {
                note.time_ms = (snapshot.original_time_ms + time_delta).max(0.0);

                if !is_air_kind(note.kind) {
                    continue;
                }

                if mode == MultiDragMode::AirFull {
                    if note.kind == GroundNoteKind::SkyArea {
                        if let Some(shape) = note.skyarea_shape.as_mut() {
                            shape.start_left_norm = (snapshot.sky_start_left + x_delta).clamp(0.0, 1.0);
                            shape.start_right_norm =
                                (snapshot.sky_start_right + x_delta).clamp(0.0, 1.0);
                            shape.end_left_norm = (snapshot.sky_end_left + x_delta).clamp(0.0, 1.0);
                            shape.end_right_norm = (snapshot.sky_end_right + x_delta).clamp(0.0, 1.0);

                            let center = ((shape.start_left_norm
                                + shape.start_right_norm
                                + shape.end_left_norm
                                + shape.end_right_norm)
                                * 0.25)
                                .clamp(0.0, 1.0);
                            note.lane = air_x_to_lane(center);
                            note.center_x_norm = center;
                        }
                    } else {
                        let half_w = snapshot.original_width.clamp(0.05, 1.0) * 0.5;
                        let new_center =
                            (snapshot.original_center_x_norm + x_delta).clamp(half_w, 1.0 - half_w);
                        note.center_x_norm = new_center;
                        note.lane = air_x_to_lane(new_center);
                    }
                }
            }
        }
        self.editor_state.cached_note_heads_dirty = true;
    }

    /// Update all notes during multi-drag (time-only for mixed scope).
    fn update_multi_drag_time_only(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = self.selection.multi_drag_state.as_ref() else {
            return;
        };
        let time_offset_ms = mdrag.time_offset_ms;
        let anchor_note_id = mdrag.anchor_note_id;
        let bindings = mdrag.bindings.clone();

        let judge_y = rect.y + rect.h * 0.82;
        let (_mx, my) = safe_mouse_position();

        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        let anchor_snap = bindings
            .iter()
            .find(|binding| binding.snapshot.note_id == anchor_note_id)
            .map(|binding| binding.snapshot);
        let anchor_orig_time = anchor_snap
            .map(|snapshot| snapshot.original_time_ms)
            .unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        for binding in &bindings {
            let snapshot = binding.snapshot;
            if let Some(note) = self.editor_state.notes.get_mut(binding.note_index) {
                note.time_ms = (snapshot.original_time_ms + time_delta).max(0.0);
            }
        }
        self.editor_state.cached_note_heads_dirty = true;
    }

    /// Finish multi-drag: clear state and sort.
    fn finish_multi_drag(&mut self) {
        self.selection.multi_drag_state = None;
        self.sort_notes();
    }
}




