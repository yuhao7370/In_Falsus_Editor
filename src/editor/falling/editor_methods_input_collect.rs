// 文件说明：输入命中候选收集逻辑。
// 主要功能：按鼠标位置收集可交互音符并提供判定优先级。
impl FallingGroundEditor {
    fn collect_hit_candidates(
        &self,
        mx: f32,
        my: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) -> (HitScope, Vec<HitCandidate>) {
        let mut candidates = Vec::new();

        if self.view.render_scope == RenderScope::Both {
            let Some(ground_rect) = ground_rect else {
                return (HitScope::Mixed, candidates);
            };
            candidates.extend(self.collect_hit_candidates_ground(mx, my, ground_rect, current_ms));
            if let Some(air_rect) = air_rect {
                candidates.extend(self.collect_hit_candidates_air(mx, my, air_rect, current_ms));
            }
            sort_hit_candidates(&mut candidates);
            return (HitScope::Mixed, candidates);
        }

        if let Some(rect) = ground_rect {
            candidates.extend(self.collect_hit_candidates_ground(mx, my, rect, current_ms));
        }

        if let Some(rect) = air_rect {
            candidates.extend(self.collect_hit_candidates_air(mx, my, rect, current_ms));
        }

        if candidates.is_empty() {
            return (HitScope::Ground, Vec::new());
        }

        sort_hit_candidates(&mut candidates);
        let has_ground = candidates.iter().any(|c| c.scope == HitScope::Ground);
        let has_air = candidates.iter().any(|c| c.scope == HitScope::Air);
        let scope = match (has_ground, has_air) {
            (true, true) => HitScope::Mixed,
            (false, true) => HitScope::Air,
            _ => HitScope::Ground,
        };
        (scope, candidates)
    }

    fn collect_hit_candidates_ground(
        &self,
        mx: f32,
        my: f32,
        rect: Rect,
        current_ms: f32,
    ) -> Vec<HitCandidate> {
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;
        let mut candidates = Vec::new();

        for (z, note) in self.editor_state.notes.iter().enumerate() {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let note_w = note_head_width(note, lane_w);
            let note_x = ground_note_x(note, rect.x, lane_w);
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let z_order = ground_hit_z_order(z);
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);

            let head_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };
            if point_in_rect(mx, my, head_rect) {
                push_best_hit_candidate(
                    &mut candidates,
                    HitCandidate {
                        note_id: note.id,
                        scope: HitScope::Ground,
                        air_target: AirDragTarget::Body,
                        part: HitPart::Head,
                        distance_sq: distance_sq_to_rect(mx, my, head_rect),
                        z_order,
                    },
                );
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let tail_rect = note_end_hit_rect(note_x, note_w, tail_y);
                if point_in_rect(mx, my, tail_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Ground,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Tail,
                            distance_sq: distance_sq_to_rect(mx, my, tail_rect),
                            z_order,
                        },
                    );
                }

                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let (body_x, body_w) = match note.kind {
                    GroundNoteKind::Hold => (note_x + note_w * 0.04, note_w * 0.92),
                    GroundNoteKind::SkyArea => (note_x + note_w * 0.02, note_w * 0.96),
                    _ => (note_x + note_w * 0.34, note_w * 0.32),
                };
                let body_rect = note_body_hit_rect(body_x, body_w, y1, y2);
                if point_in_rect(mx, my, body_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Ground,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Body,
                            distance_sq: distance_sq_to_rect(mx, my, body_rect),
                            z_order,
                        },
                    );
                }
            }
        }

        candidates
    }

    fn collect_hit_candidates_air(
        &self,
        mx: f32,
        my: f32,
        rect: Rect,
        current_ms: f32,
    ) -> Vec<HitCandidate> {
        let judge_y = rect.y + rect.h * 0.82;
        let split_rect = air_split_rect(rect);

        let mut candidates = Vec::new();
        for (z, note) in self.editor_state.notes.iter().enumerate() {
            if !is_air_kind(note.kind) {
                continue;
            }
            let z_order = air_hit_z_order(z, note.kind);
            let center_x = split_rect.x + note.center_x_norm * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_x = center_x - note_w * 0.5;
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let side_h = self.flick_side_height_px(note.time_ms, rect.h);

            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape {
                    let head_left =
                        split_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let head_right =
                        split_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_left =
                        split_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_right =
                        split_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * split_rect.w;
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);

                    let head_rect =
                        note_end_hit_rect(head_left, (head_right - head_left).max(2.0), head_y);
                    if point_in_rect(mx, my, head_rect) {
                        push_best_hit_candidate(
                            &mut candidates,
                            HitCandidate {
                                note_id: note.id,
                                scope: HitScope::Air,
                                air_target: AirDragTarget::SkyHead,
                                part: HitPart::Head,
                                distance_sq: distance_sq_to_rect(mx, my, head_rect),
                                z_order,
                            },
                        );
                    }

                    let tail_rect =
                        note_end_hit_rect(tail_left, (tail_right - tail_left).max(2.0), tail_y);
                    if point_in_rect(mx, my, tail_rect) {
                        push_best_hit_candidate(
                            &mut candidates,
                            HitCandidate {
                                note_id: note.id,
                                scope: HitScope::Air,
                                air_target: AirDragTarget::SkyTail,
                                part: HitPart::Tail,
                                distance_sq: distance_sq_to_rect(mx, my, tail_rect),
                                z_order,
                            },
                        );
                    }

                    let min_left = shape
                        .start_left_norm
                        .min(shape.end_left_norm)
                        .clamp(0.0, 1.0);
                    let max_right = shape
                        .start_right_norm
                        .max(shape.end_right_norm)
                        .clamp(0.0, 1.0);
                    let x1 = split_rect.x + min_left * split_rect.w;
                    let x2 = split_rect.x + max_right * split_rect.w;
                    let y1 = head_y.min(tail_y);
                    let y2 = head_y.max(tail_y);
                    let body_rect = note_body_hit_rect(x1, (x2 - x1).max(1.0), y1, y2);
                    if point_in_rect(mx, my, body_rect) {
                        let body_distance_sq =
                            skyarea_body_hit_distance_sq(mx, my, split_rect, shape, head_y, tail_y);
                        if let Some(distance_sq) = body_distance_sq {
                            push_best_hit_candidate(
                                &mut candidates,
                                HitCandidate {
                                    note_id: note.id,
                                    scope: HitScope::Air,
                                    air_target: AirDragTarget::Body,
                                    part: HitPart::Body,
                                    distance_sq,
                                    z_order,
                                },
                            );
                        }
                    }
                    continue;
                }
            }

            let head_rect = if note.kind == GroundNoteKind::Flick {
                flick_rect_hitbox(note, note_x, note_w, head_y, side_h)
            } else {
                note_end_hit_rect(note_x, note_w, head_y)
            };
            if point_in_rect(mx, my, head_rect) {
                push_best_hit_candidate(
                    &mut candidates,
                    HitCandidate {
                        note_id: note.id,
                        scope: HitScope::Air,
                        air_target: AirDragTarget::Body,
                        part: HitPart::Head,
                        distance_sq: distance_sq_to_rect(mx, my, head_rect),
                        z_order,
                    },
                );
            }

            if note.has_tail() {
                let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                let y1 = head_y.min(tail_y);
                let y2 = head_y.max(tail_y);
                let body_rect = note_body_hit_rect(note_x, note_w, y1, y2);
                if point_in_rect(mx, my, body_rect) {
                    push_best_hit_candidate(
                        &mut candidates,
                        HitCandidate {
                            note_id: note.id,
                            scope: HitScope::Air,
                            air_target: AirDragTarget::Body,
                            part: HitPart::Body,
                            distance_sq: distance_sq_to_rect(mx, my, body_rect),
                            z_order,
                        },
                    );
                }
            }
        }

        candidates
    }
}
