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



}

