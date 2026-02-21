// 文件说明：事件头部与顶部信息渲染。
// 主要功能：绘制谱面事件摘要、状态文本和工具提示信息。
impl FallingGroundEditor {
    fn draw_event_view(&mut self, rect: Rect, current_ms: f32) {
        if rect.h <= 8.0 || rect.w <= 8.0 {
            return;
        }

        let ui = self.resolution_ui_scale();

        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::from_rgba(9, 11, 19, 255),
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(44, 58, 86, 255),
        );

        let judge_y = rect.y + rect.h * 0.82;
        let (ahead_ms, behind_ms) =
            self.visible_ahead_behind_ms_linear(rect.y, rect.h, current_ms, judge_y);

        if self.view.show_barlines {
            for barline in self.editor_state.timeline.visible_barlines(
                current_ms,
                ahead_ms,
                behind_ms,
                self.view.snap_division,
            ) {
                let y = self.time_to_y_linear(barline.time_ms, current_ms, judge_y, rect.h);
                if y < rect.y - 2.0 || y > rect.y + rect.h + 2.0 {
                    continue;
                }
                let (thickness, color) = match barline.kind {
                    BarLineKind::Measure => (1.5, Color::from_rgba(102, 134, 180, 180)),
                    BarLineKind::Beat => (1.1, Color::from_rgba(78, 104, 146, 152)),
                    BarLineKind::Subdivision => (0.8, Color::from_rgba(58, 78, 112, 112)),
                };
                let margin = 6.0 * ui;
                draw_line(
                    rect.x + margin,
                    y,
                    rect.x + rect.w - margin,
                    y,
                    thickness,
                    color,
                );
            }
        }

        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;

        // 三列布局：BPM | Track | Lane，每列独立去重
        let col_count = 3_usize;
        let col_w = rect.w / col_count as f32;
        // 按类型分桶收集可见事件 (y, index)
        let mut cols: [Vec<(f32, usize)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for (i, event) in self.editor_state.timeline_events.iter().enumerate() {
            if event.time_ms < start_ms - 0.001 || event.time_ms > end_ms + 0.001 {
                continue;
            }
            let y = self.time_to_y_linear(event.time_ms, current_ms, judge_y, rect.h);
            let clip_top = 28.0 * ui;
            let clip_bottom = 4.0 * ui;
            if y < rect.y + clip_top || y > rect.y + rect.h - clip_bottom {
                continue;
            }
            let col = match event.kind {
                TimelineEventKind::Bpm => 0,
                TimelineEventKind::Track => 1,
                TimelineEventKind::Lane => 2,
            };
            cols[col].push((y, i));
        }
        for col in &mut cols {
            col.sort_by(|a, b| a.0.total_cmp(&b.0));
        }

        // 绘制列分隔线
        for ci in 1..col_count {
            let lx = rect.x + col_w * ci as f32;
            draw_line(
                lx,
                rect.y + self.scaled_ui_px(22.0),
                lx,
                rect.y + rect.h,
                0.6,
                Color::from_rgba(36, 48, 72, 180),
            );
        }

        // 点击检测
        let mouse = safe_mouse_position();
        let mouse_in_rect = mouse.0 >= rect.x
            && mouse.0 <= rect.x + rect.w
            && mouse.1 >= rect.y
            && mouse.1 <= rect.y + rect.h;
        let clicked = mouse_in_rect && safe_mouse_button_pressed(MouseButton::Left);
        let right_clicked = mouse_in_rect && safe_mouse_button_pressed(MouseButton::Right);
        if right_clicked {
            self.selection.clear_event_selection();
            self.selection.event_overlap_cycle = None;
            self.selection.event_hover_hint = None;
        }
        let event_hit_half_h = 9.0_f32 * ui;
        let font_size = ((14.0 * ui).round() as u16).clamp(9, 32);
        let time_font_size = ((10.0 * ui).round() as u16).clamp(7, 22);
        let min_gap = event_hit_half_h * 2.2;
        let placement_clip_top = 28.0 * ui;
        let placement_clip_bottom = 4.0 * ui;
        let mut consumed_click_for_placement = false;
        if clicked {
            if let Some(event_tool) = self.selection.place_event_type {
                let y_min = rect.y + placement_clip_top;
                let y_max = rect.y + rect.h - placement_clip_bottom;
                if mouse.1 >= y_min && mouse.1 <= y_max {
                    let pixels_per_ms = (self.view.scroll_speed * rect.h / 1000.0).max(0.001);
                    let placed_time_ms = self
                        .apply_snap((current_ms + (judge_y - mouse.1) / pixels_per_ms).max(0.0));
                    self.place_timeline_event(event_tool, placed_time_ms);
                    self.selection.selected_event_id = None;
                    self.selection.event_overlap_cycle = None;
                    self.selection.event_hover_hint = None;
                    consumed_click_for_placement = true;
                }
            }
        }

        // Placement preview for event tools (Bpm/Track/Lane), aligned to snapped time.
        if let Some(event_tool) = self.selection.place_event_type {
            let y_min = rect.y + placement_clip_top;
            let y_max = rect.y + rect.h - placement_clip_bottom;
            if mouse_in_rect && mouse.1 >= y_min && mouse.1 <= y_max {
                let pixels_per_ms = (self.view.scroll_speed * rect.h / 1000.0).max(0.001);
                let preview_time_ms =
                    self.apply_snap((current_ms + (judge_y - mouse.1) / pixels_per_ms).max(0.0));
                let preview_y = self.time_to_y_linear(preview_time_ms, current_ms, judge_y, rect.h);
                let (col_idx, color) = match event_tool {
                    PlaceEventType::Bpm => (0usize, Color::from_rgba(124, 226, 255, 220)),
                    PlaceEventType::Track => (1usize, Color::from_rgba(150, 240, 170, 220)),
                    PlaceEventType::Lane => (2usize, Color::from_rgba(232, 198, 124, 220)),
                };
                let col_x = rect.x + col_w * col_idx as f32;
                let sub_x = col_x + 1.0;
                let sub_w = col_w - 2.0;

                draw_line(
                    rect.x + 6.0 * ui,
                    preview_y,
                    rect.x + rect.w - 6.0 * ui,
                    preview_y,
                    1.2,
                    Color::new(color.r, color.g, color.b, 0.72),
                );
                draw_rectangle(
                    sub_x,
                    preview_y - event_hit_half_h,
                    sub_w,
                    event_hit_half_h * 2.0,
                    Color::new(color.r, color.g, color.b, 0.22),
                );

                let label = event_tool.label();
                let label_metrics =
                    measure_text(label, self.view.text_font.as_ref(), font_size, 1.0);
                let label_x = sub_x + (sub_w - label_metrics.width) * 0.5;
                draw_text_ex(
                    label,
                    label_x,
                    preview_y + 4.0 * ui,
                    TextParams {
                        font: self.view.text_font.as_ref(),
                        font_size,
                        color,
                        ..Default::default()
                    },
                );

                let time_str = format!("{:05.0}", preview_time_ms);
                let time_metrics =
                    measure_text(&time_str, self.view.text_font.as_ref(), time_font_size, 1.0);
                let time_x = sub_x + (sub_w - time_metrics.width) * 0.5;
                let time_y = preview_y + 4.0 * ui + (font_size as f32) * 0.7;
                draw_text_ex(
                    &time_str,
                    time_x,
                    time_y,
                    TextParams {
                        font: self.view.text_font.as_ref(),
                        font_size: time_font_size,
                        color: Color::new(color.r, color.g, color.b, 0.62),
                        ..Default::default()
                    },
                );
            }
        }

        // 为每列计算左右错开偏移: 0=居中, -1=偏左, 1=偏右
        let mut col_offsets: [Vec<i8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for ci in 0..3 {
            let events = &cols[ci];
            let mut offsets = vec![0i8; events.len()];
            let mut i = 0;
            while i < events.len() {
                // 找出一组连续靠近的事件
                let mut j = i + 1;
                while j < events.len() && (events[j].0 - events[j - 1].0).abs() < min_gap {
                    j += 1;
                }
                if j - i > 1 {
                    // 这组事件需要错开
                    for k in i..j {
                        offsets[k] = if (k - i) % 2 == 0 { -1 } else { 1 };
                    }
                }
                i = j;
            }
            col_offsets[ci] = offsets;
        }

        // 全部渲染 + 收集悬停/点击候选
        let mut hover_candidates: Vec<(u64, usize)> = Vec::new();
        let mut click_candidates: Vec<(u64, usize)> = Vec::new();
        for (ci, col_events) in cols.iter().enumerate() {
            let col_x = rect.x + col_w * ci as f32;
            for (ei, (y, idx)) in col_events.iter().enumerate() {
                let event = &self.editor_state.timeline_events[*idx];
                let is_selected = self.selection.selected_event_id == Some(event.id)
                    || self.selection.selected_event_ids.contains(&event.id);
                let offset = col_offsets[ci][ei];

                // 计算实际渲染子区域
                let (sub_x, sub_w) = match offset {
                    -1 => (col_x, col_w * 0.5),              // 偏左半列
                    1 => (col_x + col_w * 0.5, col_w * 0.5), // 偏右半列
                    _ => (col_x, col_w),                     // 居中整列
                };

                if is_selected {
                    draw_rectangle(
                        sub_x + 1.0,
                        *y - event_hit_half_h,
                        sub_w - 2.0,
                        event_hit_half_h * 2.0,
                        Color::from_rgba(80, 160, 255, 48),
                    );
                }

                // 绘制关键字（只显示第一个词）
                let display_text = event
                    .label
                    .split_whitespace()
                    .next()
                    .unwrap_or(&event.label);
                let metrics =
                    measure_text(display_text, self.view.text_font.as_ref(), font_size, 1.0);
                let text_x = sub_x + (sub_w - metrics.width) * 0.5;
                let text_baseline_offset = 4.0 * ui;
                draw_text_ex(
                    display_text,
                    text_x,
                    *y + text_baseline_offset,
                    TextParams {
                        font: self.view.text_font.as_ref(),
                        font_size,
                        color: event.color,
                        ..Default::default()
                    },
                );

                // 绘制时间标注（在关键字下方，五位毫秒）
                let time_str = format!("{:05.0}", event.time_ms);
                let time_metrics =
                    measure_text(&time_str, self.view.text_font.as_ref(), time_font_size, 1.0);
                let time_x = sub_x + (sub_w - time_metrics.width) * 0.5;
                let time_y = *y + text_baseline_offset + (font_size as f32) * 0.7;
                let time_color = Color::from_rgba(
                    event.color.r.min(1.0).max(0.0).mul_add(255.0, 0.0) as u8,
                    event.color.g.min(1.0).max(0.0).mul_add(255.0, 0.0) as u8,
                    event.color.b.min(1.0).max(0.0).mul_add(255.0, 0.0) as u8,
                    120,
                );
                draw_text_ex(
                    &time_str,
                    time_x,
                    time_y,
                    TextParams {
                        font: self.view.text_font.as_ref(),
                        font_size: time_font_size,
                        color: time_color,
                        ..Default::default()
                    },
                );

                // debug hitbox: 绘制 event 点击判定区域
                if self.view.debug_show_hitboxes {
                    let hit_rect = Rect {
                        x: sub_x + 1.0,
                        y: *y - event_hit_half_h,
                        w: sub_w - 2.0,
                        h: event_hit_half_h * 2.0,
                    };
                    let hitbox_color = match event.kind {
                        TimelineEventKind::Bpm => Color::from_rgba(255, 180, 80, 200),
                        TimelineEventKind::Track => Color::from_rgba(80, 220, 255, 200),
                        TimelineEventKind::Lane => Color::from_rgba(180, 255, 120, 200),
                    };
                    draw_debug_hitbox_rect(hit_rect, rect, hitbox_color, 1.2);
                }

                // 悬停/点击候选收集（使用偏移后的子区域）
                let in_sub = mouse.0 >= sub_x && mouse.0 < sub_x + sub_w;
                if in_sub {
                    let dy = (mouse.1 - *y).abs();
                    if dy <= event_hit_half_h {
                        if mouse_in_rect {
                            hover_candidates.push((event.id, ci));
                        }
                        if clicked {
                            click_candidates.push((event.id, ci));
                        }
                    }
                }
            }
        }

        // 悬停提示：显示 N/M
        if !consumed_click_for_placement {
            if hover_candidates.len() > 1 {
                let mut current_index = 0_usize;
                if let Some(ref cycle) = self.selection.event_overlap_cycle {
                    let ids: Vec<u64> = hover_candidates.iter().map(|c| c.0).collect();
                    if cycle.candidates == ids {
                        current_index = cycle.current_index.min(ids.len().saturating_sub(1));
                    }
                }
                self.selection.event_hover_hint = Some(EventHoverOverlapHint {
                    mouse_x: mouse.0,
                    mouse_y: mouse.1,
                    current_index,
                    total: hover_candidates.len(),
                });
            } else {
                self.selection.event_hover_hint = None;
            }

            // 双击切换选中逻辑（与 note 一致）
            if clicked && !click_candidates.is_empty() {
                let shift_held =
                    safe_key_down(KeyCode::LeftShift) || safe_key_down(KeyCode::RightShift);
                let anchor_y = (mouse.1 * 0.5) as i32;
                let col = click_candidates[0].1;
                let ids: Vec<u64> = click_candidates.iter().map(|c| c.0).collect();
                let now_sec = get_time();

                let selected_index = self
                    .selection
                    .selected_event_id
                    .and_then(|sel| ids.iter().position(|id| *id == sel));

                if shift_held {
                    // Shift+Click: toggle event in/out of multi-select
                    let clicked_id = ids[0];
                    if self.selection.selected_event_ids.contains(&clicked_id) {
                        self.selection.selected_event_ids.remove(&clicked_id);
                        if self.selection.selected_event_id == Some(clicked_id) {
                            self.selection.selected_event_id =
                                self.selection.selected_event_ids.iter().next().copied();
                        }
                    } else {
                        // Add previous selected_event_id to the set first
                        if let Some(prev_id) = self.selection.selected_event_id {
                            self.selection.selected_event_ids.insert(prev_id);
                        }
                        self.selection.selected_event_ids.insert(clicked_id);
                        self.selection.selected_event_id = Some(clicked_id);
                    }
                    // Clear note selection
                    self.selection.clear_note_selection();
                    self.selection.overlap_cycle = None;
                    self.selection.hover_overlap_hint = None;
                    let count = self.selection.selected_event_ids.len();
                    self.status = format!("selected {} event(s)", count);
                } else if ids.len() > 1 {
                    let mut index = selected_index.unwrap_or(0);
                    let mut double_click_armed = selected_index.is_some();
                    let mut did_cycle = false;

                    if let Some(ref prev) = self.selection.event_overlap_cycle {
                        if prev.col == col && prev.anchor_y == anchor_y && prev.candidates == ids {
                            let previous_in_current = ids
                                .iter()
                                .position(|id| {
                                    *id == ids[prev.current_index.min(ids.len().saturating_sub(1))]
                                })
                                .unwrap_or(0);
                            if prev.double_click_armed {
                                let elapsed = now_sec - prev.last_click_time_sec;
                                if elapsed <= OVERLAP_DOUBLE_CLICK_SEC {
                                    index = (previous_in_current + 1) % ids.len();
                                    did_cycle = true;
                                    double_click_armed = false;
                                } else {
                                    index = selected_index.unwrap_or(previous_in_current);
                                    double_click_armed = true;
                                }
                            } else {
                                index = selected_index.unwrap_or(previous_in_current);
                                double_click_armed = true;
                            }
                        }
                    }

                    self.selection.selected_event_id = Some(ids[index]);
                    self.selection.selected_event_ids.clear();
                    self.selection.clear_note_selection();
                    self.selection.overlap_cycle = None;
                    self.selection.hover_overlap_hint = None;
                    self.selection.event_overlap_cycle = Some(EventOverlapCycle {
                        candidates: ids.clone(),
                        current_index: index,
                        col,
                        anchor_y,
                        last_click_time_sec: now_sec,
                        double_click_armed,
                    });
                    if did_cycle {
                        self.status = format!(
                            "overlap event {}/{} (id={})",
                            index + 1,
                            ids.len(),
                            ids[index]
                        );
                    } else if let Some(ev) = self
                        .editor_state
                        .timeline_events
                        .iter()
                        .find(|e| e.id == ids[index])
                    {
                        self.status = format!("selected event: {}", ev.label);
                    }
                } else {
                    self.selection.selected_event_id = Some(ids[0]);
                    self.selection.selected_event_ids.clear();
                    self.selection.clear_note_selection();
                    self.selection.overlap_cycle = None;
                    self.selection.hover_overlap_hint = None;
                    self.selection.event_overlap_cycle = None;
                    if let Some(ev) = self
                        .editor_state
                        .timeline_events
                        .iter()
                        .find(|e| e.id == ids[0])
                    {
                        self.status = format!("selected event: {}", ev.label);
                    }
                }
            } else if clicked {
                let shift_held =
                    safe_key_down(KeyCode::LeftShift) || safe_key_down(KeyCode::RightShift);
                if !shift_held {
                    self.selection.clear_event_selection();
                    self.selection.event_overlap_cycle = None;
                }
            }

            // 绘制悬停提示框
            if let Some(hint) = self.selection.event_hover_hint {
                if hint.total > 1 {
                    let text = format!("{}/{}", hint.current_index + 1, hint.total);
                    let ui = adaptive_ui_scale();
                    let hint_font_size = scaled_font_size(18.0, 12, 42);
                    let metrics =
                        measure_text(&text, self.view.text_font.as_ref(), hint_font_size, 1.0);
                    let box_w = metrics.width + 14.0 * ui;
                    let box_h = 24.0 * ui;
                    let hx = (hint.mouse_x + 14.0 * ui)
                        .clamp(4.0 * ui, screen_width() - box_w - 4.0 * ui);
                    let hy = (hint.mouse_y - box_h - 10.0 * ui)
                        .clamp(4.0 * ui, screen_height() - box_h - 4.0 * ui);
                    draw_rectangle(hx, hy, box_w, box_h, Color::from_rgba(20, 24, 34, 214));
                    draw_rectangle_lines(
                        hx,
                        hy,
                        box_w,
                        box_h,
                        1.0,
                        Color::from_rgba(140, 156, 198, 220),
                    );
                    draw_text_ex(
                        &text,
                        hx + 7.0 * ui,
                        hy + 17.0 * ui,
                        TextParams {
                            font: self.view.text_font.as_ref(),
                            font_size: hint_font_size,
                            color: Color::from_rgba(228, 234, 248, 255),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        let judge_margin = 6.0 * ui;
        draw_line(
            rect.x + judge_margin,
            judge_y,
            rect.x + rect.w - judge_margin,
            judge_y,
            2.2,
            Color::from_rgba(255, 146, 114, 240),
        );
        draw_text_ex(
            "EVENTS",
            rect.x + self.title_side_margin_px(),
            rect.y + self.title_top_baseline_px(),
            TextParams {
                font: self.view.text_font.as_ref(),
                font_size: self.title_font_size(),
                color: Color::from_rgba(198, 218, 250, 255),
                ..Default::default()
            },
        );
    }
}
