// 文件说明：音符拖拽编辑行为实现。
// 主要功能：处理拖拽中的位置更新、尾部调整和约束校验。
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
        let Some(note) = self.notes.iter().find(|note| note.id == candidate.note_id) else {
            self.drag_state = None;
            return;
        };

        let (judge_y, lane_h) = match candidate.scope {
            HitScope::Ground => {
                let Some(rect) = ground_rect else {
                    self.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Air => {
                let Some(rect) = air_rect else {
                    self.drag_state = None;
                    return;
                };
                (rect.y + rect.h * 0.82, rect.h)
            }
            HitScope::Mixed => {
                self.drag_state = None;
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

        let drag_anchor_time_ms =
            if note.kind == GroundNoteKind::SkyArea && candidate.air_target == AirDragTarget::SkyTail {
                note.end_time_ms()
            } else {
                note.time_ms
            };

        // 将 time_offset 量化到当前 snap 网格的整数倍，
        // 避免拖拽中 apply_snap 因非网格偏移产生不对称吸附。
        let raw_offset = drag_anchor_time_ms - pointer_time_ms;
        let quantized_offset = if self.snap_enabled && self.snap_division > 0 {
            let point = self.timeline.point_at_time(drag_anchor_time_ms);
            let bpm = point.bpm.abs().max(0.001);
            let sub_ms = 60_000.0 / bpm / self.snap_division as f32;
            (raw_offset / sub_ms).round() * sub_ms
        } else {
            raw_offset
        };

        // 计算拖拽开始时鼠标所在轨道与音符 lane 的偏移
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
        self.drag_state = Some(DragState {
            note_id: candidate.note_id,
            time_offset_ms: quantized_offset,
            start_time_sec: get_time(),
            start_mouse_x: mx,
            start_mouse_y: my,
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
        if self.selected_note_ids.len() < 2 {
            return;
        }

        // Determine scope: all ground, all air, or mixed
        let mut has_ground = false;
        let mut has_air = false;
        let mut initial_notes: Vec<MultiDragNoteSnapshot> = Vec::new();
        let mut x_splits: Vec<f64> = Vec::new();

        for &nid in &self.selected_note_ids {
            if let Some(note) = self.notes.iter().find(|n| n.id == nid) {
                if is_ground_kind(note.kind) {
                    has_ground = true;
                } else {
                    has_air = true;
                    x_splits.push(note.x_split);
                }
                let (sl, sr, el, er) = if let Some(shape) = note.skyarea_shape {
                    (shape.start_left_norm, shape.start_right_norm,
                     shape.end_left_norm, shape.end_right_norm)
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                };
                initial_notes.push(MultiDragNoteSnapshot {
                    note_id: nid,
                    original_time_ms: note.time_ms,
                    original_lane: note.lane,
                    original_width: note.width,
                    original_center_x_norm: note.center_x_norm,
                    original_duration_ms: note.duration_ms,
                    sky_start_left: sl,
                    sky_start_right: sr,
                    sky_end_left: el,
                    sky_end_right: er,
                });
            }
        }

        if initial_notes.is_empty() {
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
                if all_same_xsplit { MultiDragMode::AirFull } else { MultiDragMode::TimeOnly }
            }
            HitScope::Mixed => MultiDragMode::TimeOnly,
        };

        let common_x_split = x_splits.first().copied().unwrap_or(self.x_split);

        // Compute time offset from anchor note
        let anchor_note = self.notes.iter().find(|n| n.id == anchor_id);
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
        let anchor_time = anchor_note.map(|n| n.time_ms).unwrap_or(pointer_time);

        let raw_offset = anchor_time - pointer_time;
        let quantized_offset = if self.snap_enabled && self.snap_division > 0 {
            let point = self.timeline.point_at_time(anchor_time);
            let bpm = point.bpm.abs().max(0.001);
            let sub_ms = 60_000.0 / bpm / self.snap_division as f32;
            (raw_offset / sub_ms).round() * sub_ms
        } else {
            raw_offset
        };

        // Lane offset for ground
        let lane_offset = if mode == MultiDragMode::GroundFull {
            if let Some(rect) = ground_rect {
                let lane_w = rect.w / LANE_COUNT as f32;
                let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
                let anchor_lane = anchor_note.map(|n| n.lane as i32).unwrap_or(0);
                mouse_lane - anchor_lane
            } else {
                0
            }
        } else {
            0
        };

        self.snapshot_for_undo();
        self.multi_drag_state = Some(MultiDragState {
            anchor_note_id: anchor_id,
            time_offset_ms: quantized_offset,
            lane_offset,
            start_time_sec: get_time(),
            start_mouse_x: mx,
            start_mouse_y: my,
            mode,
            common_x_split,
            scope,
            initial_notes,
        });
    }

    /// Update all notes during multi-drag (ground scope).
    fn update_multi_drag_ground(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = &self.multi_drag_state else { return };
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();

        let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + mdrag.time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        // anchor 目标 lane = mouse_lane - lane_offset（鼠标与 anchor 的初始偏移）
        let target_anchor_lane = mouse_lane - mdrag.lane_offset;
        let anchor_snap = mdrag.initial_notes.iter()
            .find(|s| s.note_id == mdrag.anchor_note_id);
        let anchor_orig_lane = anchor_snap.map(|s| s.original_lane as i32).unwrap_or(0);
        let anchor_orig_time = anchor_snap.map(|s| s.original_time_ms).unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        // 期望的 lane 偏移量
        let desired_lane_shift = target_anchor_lane - anchor_orig_lane;

        // 计算全局可移动范围：遍历所有 ground note，找到 shift 的上下界
        let mut global_min = i32::MIN;
        let mut global_max = i32::MAX;
        for snap in &mdrag.initial_notes {
            if let Some(note) = self.notes.iter().find(|n| n.id == snap.note_id) {
                if is_ground_kind(note.kind) {
                    let eff_w = ground_note_effective_width(snap.original_lane, snap.original_width);
                    let orig = snap.original_lane as i32;
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

        // Clone to avoid borrow issues
        let snapshots: Vec<MultiDragNoteSnapshot> = mdrag.initial_notes.clone();

        for snap in &snapshots {
            if let Some(note) = self.notes.iter_mut().find(|n| n.id == snap.note_id) {
                note.time_ms = (snap.original_time_ms + time_delta).max(0.0);
                if is_ground_kind(note.kind) {
                    note.lane = (snap.original_lane as i32 + lane_shift) as usize;
                }
            }
        }
    }

    /// Update all notes during multi-drag (air scope).
    fn update_multi_drag_air(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = &self.multi_drag_state else { return };
        let split_rect = air_split_rect(rect);
        let judge_y = rect.y + rect.h * 0.82;
        let (mx, my) = safe_mouse_position();

        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + mdrag.time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        let anchor_snap = mdrag.initial_notes.iter()
            .find(|s| s.note_id == mdrag.anchor_note_id);
        let anchor_orig_time = anchor_snap.map(|s| s.original_time_ms).unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        let raw_x_delta = if mdrag.mode == MultiDragMode::AirFull {
            (mx - mdrag.start_mouse_x) / split_rect.w.max(1.0)
        } else {
            0.0
        };

        let snapshots: Vec<MultiDragNoteSnapshot> = mdrag.initial_notes.clone();
        let mode = mdrag.mode;

        // 全局计算所有 air note 的最左和最右边界，统一 clamp x_delta
        let x_delta = if mode == MultiDragMode::AirFull {
            let mut global_left_min: f32 = 0.0;  // 所有 note 中最小的左边界
            let mut global_right_max: f32 = 1.0;  // 所有 note 中最大的右边界
            let mut first = true;
            for snap in &snapshots {
                if let Some(note) = self.notes.iter().find(|n| n.id == snap.note_id) {
                    if !is_air_kind(note.kind) { continue; }
                    if note.kind == GroundNoteKind::SkyArea {
                        let left = snap.sky_start_left.min(snap.sky_end_left);
                        let right = snap.sky_start_right.max(snap.sky_end_right);
                        if first {
                            global_left_min = left;
                            global_right_max = right;
                            first = false;
                        } else {
                            global_left_min = global_left_min.min(left);
                            global_right_max = global_right_max.max(right);
                        }
                    } else {
                        // Flick
                        let half_w = snap.original_width.clamp(0.05, 1.0) * 0.5;
                        let left = snap.original_center_x_norm - half_w;
                        let right = snap.original_center_x_norm + half_w;
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
            // delta 范围：不能让全局最左超出 0，全局最右超出 1
            raw_x_delta.clamp(-global_left_min, 1.0 - global_right_max)
        } else {
            0.0
        };

        for snap in &snapshots {
            if let Some(note) = self.notes.iter_mut().find(|n| n.id == snap.note_id) {
                note.time_ms = (snap.original_time_ms + time_delta).max(0.0);

                if !is_air_kind(note.kind) {
                    continue;
                }

                if mode == MultiDragMode::AirFull {
                    if note.kind == GroundNoteKind::SkyArea {
                        if let Some(shape) = note.skyarea_shape.as_mut() {
                            shape.start_left_norm = (snap.sky_start_left + x_delta).clamp(0.0, 1.0);
                            shape.start_right_norm = (snap.sky_start_right + x_delta).clamp(0.0, 1.0);
                            shape.end_left_norm = (snap.sky_end_left + x_delta).clamp(0.0, 1.0);
                            shape.end_right_norm = (snap.sky_end_right + x_delta).clamp(0.0, 1.0);

                            let center = ((shape.start_left_norm + shape.start_right_norm
                                + shape.end_left_norm + shape.end_right_norm) * 0.25).clamp(0.0, 1.0);
                            note.lane = air_x_to_lane(center);
                            note.center_x_norm = center;
                        }
                    } else {
                        // Flick
                        let half_w = snap.original_width.clamp(0.05, 1.0) * 0.5;
                        let new_center = (snap.original_center_x_norm + x_delta).clamp(half_w, 1.0 - half_w);
                        note.center_x_norm = new_center;
                        note.lane = air_x_to_lane(new_center);
                    }
                }
            }
        }
    }

    /// Update all notes during multi-drag (time-only for mixed scope).
    fn update_multi_drag_time_only(&mut self, rect: Rect, current_ms: f32) {
        let Some(mdrag) = &self.multi_drag_state else { return };
        let judge_y = rect.y + rect.h * 0.82;
        let (_mx, my) = safe_mouse_position();

        let new_time = self.pointer_to_time(my, current_ms, judge_y, rect.h) + mdrag.time_offset_ms;
        let snapped_time = self.apply_snap(new_time.max(0.0));

        let anchor_snap = mdrag.initial_notes.iter()
            .find(|s| s.note_id == mdrag.anchor_note_id);
        let anchor_orig_time = anchor_snap.map(|s| s.original_time_ms).unwrap_or(0.0);
        let time_delta = snapped_time - anchor_orig_time;

        let snapshots: Vec<MultiDragNoteSnapshot> = mdrag.initial_notes.clone();

        for snap in &snapshots {
            if let Some(note) = self.notes.iter_mut().find(|n| n.id == snap.note_id) {
                note.time_ms = (snap.original_time_ms + time_delta).max(0.0);
            }
        }
    }

    /// Finish multi-drag: clear state and sort.
    fn finish_multi_drag(&mut self) {
        self.multi_drag_state = None;
        self.sort_notes();
    }
}

