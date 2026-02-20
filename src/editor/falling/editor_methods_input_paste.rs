// 文件说明：粘贴预览与确认逻辑实现。
// 主要功能：计算粘贴预览音符、绘制半透明预览、处理粘贴确认/取消。
impl FallingGroundEditor {
    /// 计算粘贴预览音符列表。
    /// 根据鼠标位置计算时间偏移和位置偏移，返回预览用的音符副本。
    fn compute_paste_preview(
        &self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
        mirrored: bool,
    ) -> Vec<GroundNote> {
        if self.clipboard.is_empty() {
            return Vec::new();
        }

        let (mx, my) = safe_mouse_position();

        // 准备剪贴板音符（可能需要镜像）
        let source_notes: Vec<GroundNote> = if mirrored {
            self.clipboard
                .notes()
                .iter()
                .map(|n| Self::mirror_note(n))
                .collect()
        } else {
            self.clipboard.notes_cloned()
        };

        // 锚点：剪贴板中最早的音符
        let anchor_time = source_notes
            .iter()
            .map(|n| n.time_ms)
            .fold(f32::MAX, f32::min);

        // 判断剪贴板内容的 scope
        let has_ground = source_notes.iter().any(|n| is_ground_kind(n.kind));
        let has_air = source_notes.iter().any(|n| is_air_kind(n.kind));

        // 确定鼠标指向的时间
        let (target_time, lane_shift, x_shift) = self.compute_paste_offsets(
            mx,
            my,
            current_ms,
            ground_rect,
            air_rect,
            has_ground,
            has_air,
            &source_notes,
            anchor_time,
        );

        let time_delta = target_time - anchor_time;

        // 生成预览音符
        let mut preview = Vec::with_capacity(source_notes.len());
        for note in &source_notes {
            let mut p = note.clone();
            p.time_ms = (note.time_ms + time_delta).max(0.0);

            match p.kind {
                GroundNoteKind::Tap | GroundNoteKind::Hold => {
                    if !has_air || !has_ground {
                        // 纯 ground：应用 lane 偏移
                        let new_lane = (p.lane as i32 + lane_shift).clamp(0, LANE_COUNT as i32 - 1);
                        p.lane = new_lane as usize;
                    }
                }
                GroundNoteKind::Flick => {
                    if !has_ground || !has_air {
                        // 纯 air：应用 x 偏移
                        let half_w = p.width.clamp(0.05, 1.0) * 0.5;
                        let new_center = (p.center_x_norm + x_shift).clamp(half_w, 1.0 - half_w);
                        p.center_x_norm = new_center;
                        p.lane = air_x_to_lane(new_center);
                    }
                }
                GroundNoteKind::SkyArea => {
                    if !has_ground || !has_air {
                        // 纯 air：应用 x 偏移
                        if let Some(shape) = p.skyarea_shape.as_mut() {
                            shape.start_left_norm =
                                (shape.start_left_norm + x_shift).clamp(0.0, 1.0);
                            shape.start_right_norm =
                                (shape.start_right_norm + x_shift).clamp(0.0, 1.0);
                            shape.end_left_norm = (shape.end_left_norm + x_shift).clamp(0.0, 1.0);
                            shape.end_right_norm = (shape.end_right_norm + x_shift).clamp(0.0, 1.0);
                        }
                        let new_center = (p.center_x_norm + x_shift).clamp(0.0, 1.0);
                        p.center_x_norm = new_center;
                        p.lane = air_x_to_lane(new_center);
                    }
                }
            }
            preview.push(p);
        }
        preview
    }

    fn quantize_preview_rect(rect: Option<Rect>) -> Option<(i32, i32, i32, i32)> {
        rect.map(|r| {
            (
                r.x.round() as i32,
                r.y.round() as i32,
                r.w.round() as i32,
                r.h.round() as i32,
            )
        })
    }

    fn paste_preview_cache_key(
        &self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
        mode: PasteMode,
    ) -> PastePreviewCacheKey {
        let (mx, my) = safe_mouse_position();
        PastePreviewCacheKey {
            mode,
            clipboard_version: self.clipboard.version(),
            mouse_x_q: mx.round() as i32,
            mouse_y_q: my.round() as i32,
            time_q: (current_ms * 0.5).round() as i32,
            ground_rect_q: Self::quantize_preview_rect(ground_rect),
            air_rect_q: Self::quantize_preview_rect(air_rect),
        }
    }

    fn cached_paste_preview(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
        mode: PasteMode,
    ) -> std::sync::Arc<[GroundNote]> {
        let key = self.paste_preview_cache_key(ground_rect, air_rect, current_ms, mode);
        let cache_hit = self
            .view
            .paste_preview_cache
            .as_ref()
            .map(|cache| cache.key == key)
            .unwrap_or(false);
        if !cache_hit {
            let mirrored = mode == PasteMode::Mirrored;
            let notes_vec = self.compute_paste_preview(ground_rect, air_rect, current_ms, mirrored);
            let notes: std::sync::Arc<[GroundNote]> = notes_vec.into();
            self.view.paste_preview_cache = Some(PastePreviewCache { key, notes });
        }
        self.view
            .paste_preview_cache
            .as_ref()
            .map(|cache| cache.notes.clone())
            .unwrap_or_else(|| std::sync::Arc::<[GroundNote]>::from([]))
    }

    /// 计算粘贴偏移量（时间、lane、x）
    fn compute_paste_offsets(
        &self,
        mx: f32,
        my: f32,
        current_ms: f32,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        has_ground: bool,
        has_air: bool,
        source_notes: &[GroundNote],
        anchor_time: f32,
    ) -> (f32, i32, f32) {
        let _mixed = has_ground && has_air;

        // 找锚点音符（最早的那个）
        let anchor_note = source_notes
            .iter()
            .min_by(|a, b| a.time_ms.total_cmp(&b.time_ms))
            .unwrap();

        // 尝试从 ground_rect 或 air_rect 获取时间
        let mut target_time = anchor_time;
        let mut lane_shift: i32 = 0;
        let mut x_shift: f32 = 0.0;

        if has_ground && !has_air {
            // 纯 ground
            if let Some(rect) = ground_rect {
                let judge_y = rect.y + rect.h * 0.82;
                let raw_time = self.pointer_to_time(my, current_ms, judge_y, rect.h);
                target_time = self.apply_snap(raw_time.max(0.0));
                let lane_w = rect.w / LANE_COUNT as f32;
                let mouse_lane = lane_from_x(mx, rect.x, lane_w) as i32;
                lane_shift = mouse_lane - anchor_note.lane as i32;
                // Clamp lane_shift：考虑每个音符的有效宽度
                let mut shift_min = i32::MIN;
                let mut shift_max = i32::MAX;
                for n in source_notes.iter().filter(|n| is_ground_kind(n.kind)) {
                    // 用原始宽度判断约束，确保目标 lane 不会截断宽度
                    let intended_w = (n.width.round() as usize).max(1);
                    let (min_lane, max_lane) = if intended_w > 1 {
                        // 宽音符只能在 lane 1..4，且 lane + intended_w <= 5
                        (1i32, 5i32.saturating_sub(intended_w as i32).max(1))
                    } else {
                        (0i32, LANE_COUNT as i32 - 1)
                    };
                    shift_min = shift_min.max(min_lane - n.lane as i32);
                    shift_max = shift_max.min(max_lane - n.lane as i32);
                }
                if shift_min > shift_max {
                    // 无法满足所有约束，不移动
                    lane_shift = 0;
                } else {
                    lane_shift = lane_shift.clamp(shift_min, shift_max);
                }
            }
        } else if has_air && !has_ground {
            // 纯 air
            if let Some(rect) = air_rect {
                let judge_y = rect.y + rect.h * 0.82;
                let raw_time = self.pointer_to_time(my, current_ms, judge_y, rect.h);
                target_time = self.apply_snap(raw_time.max(0.0));
                let split_rect = air_split_rect(rect);
                let mouse_x_norm = ((mx - split_rect.x) / split_rect.w).clamp(0.0, 1.0);
                x_shift = mouse_x_norm - anchor_note.center_x_norm;
                // Clamp x_shift 使所有 air 音符都在范围内
                for n in source_notes.iter().filter(|n| is_air_kind(n.kind)) {
                    if n.kind == GroundNoteKind::SkyArea {
                        if let Some(shape) = &n.skyarea_shape {
                            let min_left = shape.start_left_norm.min(shape.end_left_norm);
                            let max_right = shape.start_right_norm.max(shape.end_right_norm);
                            if min_left + x_shift < 0.0 {
                                x_shift = -min_left;
                            }
                            if max_right + x_shift > 1.0 {
                                x_shift = 1.0 - max_right;
                            }
                        }
                    } else {
                        let half_w = n.width.clamp(0.05, 1.0) * 0.5;
                        let left = n.center_x_norm - half_w;
                        let right = n.center_x_norm + half_w;
                        if left + x_shift < 0.0 {
                            x_shift = -left;
                        }
                        if right + x_shift > 1.0 {
                            x_shift = 1.0 - right;
                        }
                    }
                }
            }
        } else {
            // 混合：仅时间偏移，取任意可用 rect
            let rect = ground_rect.or(air_rect);
            if let Some(rect) = rect {
                let judge_y = rect.y + rect.h * 0.82;
                let raw_time = self.pointer_to_time(my, current_ms, judge_y, rect.h);
                target_time = self.apply_snap(raw_time.max(0.0));
            }
        }

        (target_time, lane_shift, x_shift)
    }

    /// 处理粘贴模式下的输入（左键确认，右键/Esc取消）
    fn handle_paste_input(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let Some(mode) = self.clipboard.paste_mode() else {
            return;
        };

        // Esc 或右键取消
        if safe_key_pressed(KeyCode::Escape) || safe_mouse_button_pressed(MouseButton::Right) {
            self.exit_paste_mode();
            return;
        }

        // 左键确认粘贴
        if safe_mouse_button_pressed(MouseButton::Left) {
            let mirrored = mode == PasteMode::Mirrored;
            let preview = self.compute_paste_preview(ground_rect, air_rect, current_ms, mirrored);
            if preview.is_empty() {
                return;
            }
            // 验证地面音符宽度约束：宽音符不能放在 lane 0/5，不能超出范围
            for note in &preview {
                if is_ground_kind(note.kind) {
                    let intended_w = (note.width.round() as usize).max(1);
                    if intended_w > 1 && (note.lane == 0 || note.lane >= 5) {
                        let msg = self
                            .i18n
                            .t(crate::i18n::TextKey::EditorCannotPasteWideSideLane)
                            .to_owned();
                        self.push_toast_warn(msg);
                        return;
                    }
                    if intended_w > 1 && note.lane + intended_w > 5 {
                        let msg = self
                            .i18n
                            .t(crate::i18n::TextKey::EditorCannotPasteExceedLane)
                            .to_owned();
                        self.push_toast_warn(msg);
                        return;
                    }
                }
            }
            self.snapshot_for_undo();
            let count = preview.len();
            for mut note in preview {
                note.id = self.editor_state.next_note_id;
                self.editor_state.next_note_id = self.editor_state.next_note_id.saturating_add(1);
                self.editor_state.notes.push(note);
            }
            self.sort_notes();
            self.editor_state.cached_note_heads_dirty = true;
            self.selection.clear_note_selection();
            let key = if mirrored {
                crate::i18n::TextKey::EditorMirrorPastedNotes
            } else {
                crate::i18n::TextKey::EditorPastedNotes
            };
            let msg = self.i18n.t(key).replace("{count}", &count.to_string());
            self.status = msg.clone();
            self.push_toast(msg);
            // 不退出粘贴模式，允许连续粘贴（和参考项目一致）
            // 如果想粘贴后退出，取消下面的注释：
            // self.clipboard.clear_paste_mode();
        }
    }

    /// 绘制粘贴预览（半透明音符）
    fn draw_paste_preview(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let Some(mode) = self.clipboard.paste_mode() else {
            return;
        };
        let mirrored = mode == PasteMode::Mirrored;
        let preview = self.cached_paste_preview(ground_rect, air_rect, current_ms, mode);
        if preview.is_empty() {
            return;
        }

        // 绘制 ground 预览
        if let Some(rect) = ground_rect {
            let lane_w = rect.w / LANE_COUNT as f32;
            let judge_y = rect.y + rect.h * 0.82;
            self.begin_view_clip_rect(rect);
            for note in preview.iter().filter(|n| is_ground_kind(n.kind)) {
                let note_w = note_head_width(note, lane_w);
                let note_x = ground_note_x(note, rect.x, lane_w);
                let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
                let preview_alpha = 140u8;

                if note.has_tail() {
                    let tail_y = self.time_to_y(note.end_time_ms(), current_ms, judge_y, rect.h);
                    let y1 = head_y.min(tail_y);
                    let y2 = head_y.max(tail_y);
                    let (body_x, body_w) = (note_x + note_w * 0.04, note_w * 0.92);
                    draw_rectangle(
                        body_x,
                        y1,
                        body_w,
                        (y2 - y1).max(1.0),
                        Color::from_rgba(236, 204, 120, preview_alpha / 2),
                    );
                }

                let head_color = Color::from_rgba(255, 222, 140, preview_alpha);
                draw_rectangle(note_x, head_y - 8.0, note_w, 16.0, head_color);
            }
            self.end_view_clip_rect();
        }

        // 绘制 air 预览
        if let Some(rect) = air_rect {
            let split_rect = air_split_rect(rect);
            let judge_y = rect.y + rect.h * 0.82;
            let flick_side_h = self.flick_side_height_px(rect.h);
            self.begin_view_clip_rect(rect);
            for note in preview.iter().filter(|n| is_air_kind(n.kind)) {
                let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);

                if note.kind == GroundNoteKind::SkyArea {
                    if let Some(shape) = note.skyarea_shape {
                        self.draw_skyarea_shape(
                            split_rect, current_ms, judge_y, rect.h, note, shape, false,
                        );
                    }
                } else if note.kind == GroundNoteKind::Flick {
                    let center_x = split_rect.x + note.center_x_norm * split_rect.w;
                    let note_w = air_note_width(note, split_rect.w);
                    let note_x = center_x - note_w * 0.5;
                    draw_flick_curve_shape(note, note_x, note_w, head_y, flick_side_h);
                }
            }
            self.end_view_clip_rect();
        }

        // 绘制粘贴模式提示线（最早音符的时间线）
        let anchor_time = preview.iter().map(|n| n.time_ms).fold(f32::MAX, f32::min);
        if let Some(rect) = ground_rect {
            let judge_y = rect.y + rect.h * 0.82;
            let y = self.time_to_y(anchor_time, current_ms, judge_y, rect.h);
            if y >= rect.y && y <= rect.y + rect.h {
                let color = if mirrored {
                    Color::from_rgba(255, 160, 255, 180)
                } else {
                    Color::from_rgba(140, 255, 200, 180)
                };
                draw_line(rect.x, y, rect.x + rect.w, y, 1.5, color);
            }
        }
        if let Some(rect) = air_rect {
            let judge_y = rect.y + rect.h * 0.82;
            let y = self.time_to_y(anchor_time, current_ms, judge_y, rect.h);
            if y >= rect.y && y <= rect.y + rect.h {
                let color = if mirrored {
                    Color::from_rgba(255, 160, 255, 180)
                } else {
                    Color::from_rgba(140, 255, 200, 180)
                };
                let split_rect = air_split_rect(rect);
                draw_line(split_rect.x, y, split_rect.x + split_rect.w, y, 1.5, color);
            }
        }
    }
}
