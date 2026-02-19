// 文件说明：框选（Box Select）逻辑实现。
// 主要功能：Alt+左键拖拽框选音符/事件，note 和 event 互斥。

impl FallingGroundEditor {
    /// 每帧调用：处理 Alt+左键 框选交互。
    /// ground_rect / air_rect 是当前帧的音符渲染区域（屏幕坐标）。
    /// event_rect 是事件头渲染区域（仅 RenderScope::Both 时有值）。
    pub(crate) fn handle_box_select(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        event_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let (mx, my) = safe_mouse_position();
        let alt_held = safe_key_down(KeyCode::LeftAlt) || safe_key_down(KeyCode::RightAlt);

        // --- 开始框选 ---
        if alt_held && safe_mouse_button_pressed(MouseButton::Left) {
            // 开始框选时清除拖拽等状态，避免冲突
            self.drag_state = None;
            self.multi_drag_state = None;
            // 清空之前的选中集合
            self.selected_note_id = None;
            self.selected_note_ids.clear();
            self.selected_event_id = None;
            self.selected_event_ids.clear();
            self.box_select = Some(BoxSelectState {
                start_x: mx,
                start_y: my,
                current_x: mx,
                current_y: my,
            });
            return;
        }

        // --- 更新框选 ---
        if let Some(ref mut bs) = self.box_select {
            if safe_mouse_button_down(MouseButton::Left) {
                bs.current_x = mx;
                bs.current_y = my;

                // 实时计算框内元素
                let box_rect = box_select_rect(bs);
                let mut note_ids: Vec<u64> = Vec::new();
                let mut event_ids: Vec<u64> = Vec::new();

                // 收集框内 ground 音符
                if let Some(rect) = ground_rect {
                    self.collect_ground_notes_in_box(rect, current_ms, box_rect, &mut note_ids);
                }
                // 收集框内 air 音符
                if let Some(rect) = air_rect {
                    self.collect_air_notes_in_box(rect, current_ms, box_rect, &mut note_ids);
                }
                // 收集框内事件
                if let Some(rect) = event_rect {
                    self.collect_events_in_box(rect, current_ms, box_rect, &mut event_ids);
                }

                // 累积选中：曾经被框进去的元素持续保留
                // 互斥：note 优先
                if !note_ids.is_empty() {
                    self.selected_note_ids.extend(note_ids);
                    self.selected_note_id = self.selected_note_ids.iter().next().copied();
                    // note 存在时清空 event
                    self.selected_event_id = None;
                    self.selected_event_ids.clear();
                } else if !event_ids.is_empty() && self.selected_note_ids.is_empty() {
                    // 仅当没有累积的 note 时才收集 event
                    self.selected_event_ids.extend(event_ids);
                    self.selected_event_id = self.selected_event_ids.iter().next().copied();
                }
                // 如果当前帧框内为空，不做任何清除，保留已累积的选中
                return;
            }

            // --- 松开鼠标：结束框选 ---
            // 选中结果已在上面实时更新，直接清除框选状态
            let count_n = self.selected_note_ids.len();
            let count_e = self.selected_event_ids.len();
            self.box_select = None;
            if count_n > 0 {
                self.status = format!("box-selected {} note(s)", count_n);
            } else if count_e > 0 {
                self.status = format!("box-selected {} event(s)", count_e);
            }
        }
    }

    /// 绘制框选矩形（半透明蓝色填充 + 边框）。
    pub(crate) fn draw_box_select_overlay(&self) {
        if let Some(ref bs) = self.box_select {
            let r = box_select_rect(bs);
            // 填充
            draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(80, 140, 255, 40));
            // 边框
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::from_rgba(100, 170, 255, 180));
        }
    }

    // ---- 内部：收集框内 ground 音符 ----
    fn collect_ground_notes_in_box(
        &self,
        rect: Rect,
        current_ms: f32,
        box_rect: Rect,
        out: &mut Vec<u64>,
    ) {
        let lane_w = rect.w / LANE_COUNT as f32;
        let judge_y = rect.y + rect.h * 0.82;

        for note in &self.notes {
            if !is_ground_kind(note.kind) {
                continue;
            }
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let note_x = ground_note_x(note, rect.x, lane_w);
            let note_w = note_head_width(note, lane_w);
            let note_h = 16.0; // 音符头高度

            let note_rect = Rect::new(note_x, head_y - note_h * 0.5, note_w, note_h);
            if rects_overlap(box_rect, note_rect) {
                out.push(note.id);
            }
        }
    }

    // ---- 内部：收集框内 air 音符 ----
    fn collect_air_notes_in_box(
        &self,
        rect: Rect,
        current_ms: f32,
        box_rect: Rect,
        out: &mut Vec<u64>,
    ) {
        let split_rect = air_split_rect(rect);
        let judge_y = rect.y + rect.h * 0.82;

        for note in &self.notes {
            if !is_air_kind(note.kind) {
                continue;
            }
            let head_y = self.time_to_y(note.time_ms, current_ms, judge_y, rect.h);
            let center_x = split_rect.x + note.center_x_norm * split_rect.w;
            let note_w = air_note_width(note, split_rect.w);
            let note_h = 16.0;

            let note_rect = Rect::new(center_x - note_w * 0.5, head_y - note_h * 0.5, note_w, note_h);
            if rects_overlap(box_rect, note_rect) {
                // 避免重复（Both 模式下 ground_rect == air_rect）
                if !out.contains(&note.id) {
                    out.push(note.id);
                }
            }
        }
    }

    // ---- 内部：收集框内事件 ----
    fn collect_events_in_box(
        &self,
        rect: Rect,
        current_ms: f32,
        box_rect: Rect,
        out: &mut Vec<u64>,
    ) {
        let judge_y = rect.y + rect.h * 0.82;
        let ui = self.resolution_ui_scale();
        let event_half_h = 9.0 * ui;

        for event in &self.timeline_events {
            let ey = self.time_to_y_linear(event.time_ms, current_ms, judge_y, rect.h);
            // 事件渲染为水平条，宽度占满 rect
            let event_rect = Rect::new(rect.x, ey - event_half_h, rect.w, event_half_h * 2.0);
            if rects_overlap(box_rect, event_rect) {
                out.push(event.id);
            }
        }
    }
}

/// 从 BoxSelectState 计算规范化矩形（处理反向拖拽）。
fn box_select_rect(bs: &BoxSelectState) -> Rect {
    let x1 = bs.start_x.min(bs.current_x);
    let y1 = bs.start_y.min(bs.current_y);
    let x2 = bs.start_x.max(bs.current_x);
    let y2 = bs.start_y.max(bs.current_y);
    Rect::new(x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0))
}

/// 两个矩形是否重叠。
fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}
