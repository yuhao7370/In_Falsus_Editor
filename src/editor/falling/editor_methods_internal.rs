// 文件说明：编辑器内部通用操作函数集合。
// 主要功能：封装音符增删改、排序、吸附和状态维护。
impl FallingGroundEditor {
    pub fn set_i18n(&mut self, i18n: crate::i18n::I18n) {
        self.i18n = i18n;
    }

    fn pointer_to_time(&self, mouse_y: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        // scroll_speed 单位：屏高/秒，visual_beat 单位：毫秒等效（speed=1 时 = dt_ms）
        let pixels_per_ms = (self.view.scroll_speed * lane_h / 1000.0).max(0.001);
        let current_vb = self.editor_state.track_timeline.visual_beat_at(current_ms);
        let delta_vb = (judge_y - mouse_y) / pixels_per_ms;
        let target_vb = current_vb + delta_vb;
        self.editor_state
            .track_timeline
            .visual_beat_to_time(target_vb)
    }

    fn time_to_y(&self, note_time_ms: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        let pixels_per_ms = (self.view.scroll_speed * lane_h / 1000.0).max(0.001);
        let note_vb = self
            .editor_state
            .track_timeline
            .visual_beat_at(note_time_ms);
        let current_vb = self.editor_state.track_timeline.visual_beat_at(current_ms);
        judge_y - (note_vb - current_vb) * pixels_per_ms
    }

    /// 计算当前视口中可见的时间范围（考虑 track speed 变化）。
    /// 返回 (ahead_ms, behind_ms)，ahead 是判定线上方的时间跨度，behind 是下方的。
    /// 当 track speed 为负时，top/bottom 时间可能反转，取 min/max 保证覆盖完整范围。
    fn visible_ahead_behind_ms(
        &self,
        rect_y: f32,
        rect_h: f32,
        current_ms: f32,
        judge_y: f32,
    ) -> (f32, f32) {
        let top_time = self.pointer_to_time(rect_y, current_ms, judge_y, rect_h);
        let bottom_time = self.pointer_to_time(rect_y + rect_h, current_ms, judge_y, rect_h);
        let min_time = top_time.min(bottom_time);
        let max_time = top_time.max(bottom_time);
        let ahead_ms = (max_time - current_ms).max(0.0);
        let behind_ms = (current_ms - min_time).max(0.0);
        (ahead_ms, behind_ms)
    }

    /// 纯线性时间→Y 坐标（不受 track speed 影响，等效 speed=1）。
    fn time_to_y_linear(
        &self,
        note_time_ms: f32,
        current_ms: f32,
        judge_y: f32,
        lane_h: f32,
    ) -> f32 {
        let pixels_per_ms = (self.view.scroll_speed * lane_h / 1000.0).max(0.001);
        judge_y - (note_time_ms - current_ms) * pixels_per_ms
    }

    /// 纯线性可见时间范围（不受 track speed 影响）。
    fn visible_ahead_behind_ms_linear(
        &self,
        rect_y: f32,
        rect_h: f32,
        _current_ms: f32,
        judge_y: f32,
    ) -> (f32, f32) {
        let pixels_per_ms = (self.view.scroll_speed * rect_h / 1000.0).max(0.001);
        let ahead_ms = (judge_y - rect_y) / pixels_per_ms;
        let behind_ms = (rect_y + rect_h - judge_y) / pixels_per_ms;
        (ahead_ms.max(0.0), behind_ms.max(0.0))
    }

    fn chart_header_bpm_for_flick(&self) -> f32 {
        self.editor_state
            .timeline_events
            .iter()
            .find(|event| event.kind == TimelineEventKind::Bpm && event.label.starts_with("chart "))
            .and_then(|event| {
                event
                    .label
                    .strip_prefix("chart ")
                    .and_then(|value| value.split('/').next())
                    .and_then(|bpm| bpm.trim().parse::<f32>().ok())
            })
            .unwrap_or(self.editor_state.timeline.points[0].bpm)
    }

    fn flick_side_height_px(&self, lane_h: f32) -> f32 {
        // Use the unique chart header BPM as requested, falling back to timeline base if missing.
        let chart_bpm = self.chart_header_bpm_for_flick().abs().max(0.001);
        let beat_ms = 60_000.0 / chart_bpm;
        let subdivision_ms = beat_ms / 16.0;
        let pixels_per_sec = (self.view.scroll_speed * lane_h).max(1.0);
        subdivision_ms / 1000.0 * pixels_per_sec
    }

    /// 根据谱面内容和波形时长动态计算小节线预计算范围（毫秒）。
    fn effective_duration_ms(&self) -> f32 {
        let mut max_ms: f32 = 0.0;
        for n in &self.editor_state.notes {
            let end = n.time_ms + n.duration_ms;
            if end > max_ms {
                max_ms = end;
            }
        }
        for e in &self.editor_state.timeline_events {
            if e.time_ms > max_ms {
                max_ms = e.time_ms;
            }
        }
        if let Some(w) = &self.view.waveform {
            let w_ms = w.duration_sec * 1000.0;
            if w_ms > max_ms {
                max_ms = w_ms;
            }
        }
        // 加 30 秒缓冲，最小 60 秒
        (max_ms + 30_000.0).max(60_000.0)
    }

    /// 重建小节线缓存。在 BPM/track/subdivision 变化时调用。
    fn rebuild_barline_cache(&mut self) {
        self.editor_state.cached_barlines = self.editor_state.timeline.precompute_all_barlines(
            &self.editor_state.track_timeline,
            self.effective_duration_ms(),
            self.view.snap_division,
        );
        self.editor_state.cached_barlines_subdivision = self.view.snap_division;
    }

    /// 用二分查找从缓存中获取 visual_beat 在 [start_vb, end_vb] 范围内的小节线切片。
    /// 缓存已按 visual_beat 排序。
    fn visible_barlines_cached(&self, start_vb: f32, end_vb: f32) -> &[BarLine] {
        let lines = &self.editor_state.cached_barlines;
        if lines.is_empty() {
            return &[];
        }
        let lo = lines.partition_point(|bl| bl.visual_beat < start_vb - 0.001);
        let hi = lines.partition_point(|bl| bl.visual_beat <= end_vb + 0.001);
        &lines[lo..hi]
    }

    fn apply_snap(&self, time_ms: f32) -> f32 {
        if self.view.snap_enabled {
            self.editor_state
                .timeline
                .snap_time_ms(time_ms, self.view.snap_division)
        } else {
            time_ms.max(0.0)
        }
    }

    fn adjust_scroll_speed(&mut self, delta: f32) {
        let old_speed = self.view.scroll_speed;
        let new_speed = (self.view.scroll_speed + delta).clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED);
        self.view.scroll_speed = new_speed;
        if (old_speed - new_speed).abs() > 0.01 {
            self.status = format!("scroll speed set to {:.2}H/s", self.view.scroll_speed);
        }
    }

    /// Take a snapshot of current state for undo history.
    fn snapshot_for_undo(&mut self) {
        self.undo.capture(&self.editor_state);
        self.editor_state.dirty = true;
    }

    /// Restore editor state from a snapshot (shared by undo/redo).
    fn apply_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.editor_state.apply_snapshot(snapshot);
        self.rebuild_barline_cache();
        self.selection.clear_note_selection();
        self.selection.clear_event_selection();
        self.selection.clear_interactions();
        self.editor_state.dirty = true;
        self.editor_state.cached_note_heads_dirty = true;
    }

    /// Undo: restore previous state.
    pub fn undo(&mut self) -> bool {
        // If we're at the top of the stack, the current (post-edit) state
        // hasn't been saved yet. Push it so redo can return to it later.
        self.undo.capture_if_at_top(&self.editor_state);
        if let Some(snapshot) = self.undo.undo_snapshot() {
            self.apply_snapshot(snapshot);
            self.status = "undo".to_owned();
            true
        } else {
            self.status = "nothing to undo".to_owned();
            false
        }
    }

    /// Redo: restore next state.
    pub fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.undo.redo_snapshot() {
            self.apply_snapshot(snapshot);
            self.status = "redo".to_owned();
            true
        } else {
            self.status = "nothing to redo".to_owned();
            false
        }
    }

    fn push_note(&mut self, note: GroundNote) {
        self.editor_state.next_note_id = self.editor_state.next_note_id.saturating_add(1);
        self.editor_state.notes.push(note);
        self.sort_notes();
        self.editor_state.cached_note_heads_dirty = true;
    }

    fn push_timeline_event(&mut self, event: TimelineEvent) {
        self.editor_state.next_event_id = self.editor_state.next_event_id.saturating_add(1);
        self.editor_state.timeline_events.push(event);
        self.editor_state.timeline_events.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.label.cmp(&b.label))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn place_timeline_event(&mut self, tool: PlaceEventType, time_ms: f32) {
        self.snapshot_for_undo();
        let time_ms = time_ms.max(0.0);
        match tool {
            PlaceEventType::Bpm => {
                let point = self.editor_state.timeline.point_at_time(time_ms);
                let bpm = point.bpm.abs().max(0.001);
                let beats = point.beats_per_measure.max(1.0);
                let mut bpm_source = BpmSourceData {
                    base_bpm: self.editor_state.timeline.points[0].bpm,
                    base_beats_per_measure: self.editor_state.timeline.points[0].beats_per_measure,
                    bpm_events: self
                        .editor_state
                        .timeline
                        .points
                        .iter()
                        .skip(1)
                        .map(|p| (p.time_ms, p.bpm, p.beats_per_measure))
                        .collect(),
                };
                bpm_source.bpm_events.push((time_ms, bpm, beats));
                self.editor_state.timeline = BpmTimeline::from_source(bpm_source);
                let track_source = if self.editor_state.track_speed_enabled {
                    self.editor_state.track_source.clone()
                } else {
                    TrackSourceData::default()
                };
                self.editor_state.track_timeline =
                    TrackTimeline::from_source(&self.editor_state.timeline, track_source);
                self.rebuild_barline_cache();
                self.push_timeline_event(TimelineEvent {
                    id: self.editor_state.next_event_id,
                    kind: TimelineEventKind::Bpm,
                    source_index: 0,
                    time_ms,
                    label: format!("bpm {:.2} (beats {:.2})", bpm, beats),
                    color: Color::from_rgba(124, 226, 255, 255),
                });
                self.status = format!("new bpm event {:.0}ms", time_ms.round());
            }
            PlaceEventType::Track => {
                let idx = self
                    .editor_state
                    .track_timeline
                    .point_index_at_or_before(time_ms);
                let speed = self.editor_state.track_timeline.points[idx].speed;
                self.editor_state
                    .track_source
                    .track_events
                    .push((time_ms, speed));
                let track_source = if self.editor_state.track_speed_enabled {
                    self.editor_state.track_source.clone()
                } else {
                    TrackSourceData::default()
                };
                self.editor_state.track_timeline =
                    TrackTimeline::from_source(&self.editor_state.timeline, track_source);
                self.rebuild_barline_cache();
                let color = if speed >= 0.0 {
                    Color::from_rgba(150, 240, 170, 255)
                } else {
                    Color::from_rgba(255, 168, 128, 255)
                };
                self.push_timeline_event(TimelineEvent {
                    id: self.editor_state.next_event_id,
                    kind: TimelineEventKind::Track,
                    source_index: 0,
                    time_ms,
                    label: format!("track x{:.2}", speed),
                    color,
                });
                self.status = format!("new track event {:.0}ms", time_ms.round());
            }
            PlaceEventType::Lane => {
                self.push_timeline_event(TimelineEvent {
                    id: self.editor_state.next_event_id,
                    kind: TimelineEventKind::Lane,
                    source_index: 0,
                    time_ms,
                    label: "lane 0 on".to_owned(),
                    color: Color::from_rgba(232, 198, 124, 255),
                });
                self.status = format!("new lane event {:.0}ms", time_ms.round());
            }
        }
    }

    fn sort_notes(&mut self) {
        self.editor_state.notes.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.lane.cmp(&b.lane))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn sync_waveform(&mut self, audio_path: Option<&str>) {
        // Poll pending async task first
        if let Some(rx) = &self.view.waveform_task {
            match rx.try_recv() {
                Ok(Ok(waveform)) => {
                    let path = waveform.path.clone();
                    self.view.waveform = Some(waveform);
                    self.view.waveform_error = None;
                    self.view.waveform_task = None;
                    self.view.waveform_loading_path = None;
                    self.status = format!("waveform loaded: {path}");
                    let msg = self
                        .i18n
                        .t(crate::i18n::TextKey::SpectrumLoadedOk)
                        .to_owned();
                    self.push_toast(msg);
                }
                Ok(Err(err)) => {
                    self.view.waveform = None;
                    self.view.waveform_error = Some(err);
                    self.view.waveform_task = None;
                    self.view.waveform_loading_path = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still computing — do nothing
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.view.waveform_error = Some("waveform task crashed".to_owned());
                    self.view.waveform_task = None;
                    self.view.waveform_loading_path = None;
                }
            }
        }

        let Some(path) = audio_path else {
            return;
        };

        // Already loaded for this path
        let already_loaded = self
            .view
            .waveform
            .as_ref()
            .map(|w| w.path.as_str() == path)
            .unwrap_or(false);
        if already_loaded {
            return;
        }

        // Already loading this path
        let already_loading = self
            .view
            .waveform_loading_path
            .as_deref()
            .map(|p| p == path)
            .unwrap_or(false);
        if already_loading {
            return;
        }

        // Spawn background thread for FFT analysis
        let (tx, rx) = mpsc::channel();
        let path_owned = path.to_owned();
        std::thread::spawn(move || {
            let result = Waveform::from_audio_file(&path_owned, 4096);
            let _ = tx.send(result);
        });
        self.view.waveform_loading_path = Some(path.to_owned());
        self.view.waveform_task = Some(rx);
        self.status = format!("loading waveform: {path}");
    }

    fn estimate_duration(&self, audio_duration_sec: f32) -> f32 {
        if audio_duration_sec > 0.0 {
            return audio_duration_sec;
        }
        self.view
            .waveform
            .as_ref()
            .map(|waveform| waveform.duration_sec)
            .unwrap_or(1.0)
            .max(1.0)
    }

    fn format_measure_label(&self, measure_pos: f32) -> String {
        let snapped = (measure_pos * 2.0).round() * 0.5;
        if (snapped - snapped.round()).abs() < 0.001 {
            format!("{}", snapped.round() as i32)
        } else {
            format!("{snapped:.1}")
        }
    }

    fn resolution_ui_scale(&self) -> f32 {
        crate::ui::scale::ui_scale_factor().clamp(0.7, 1.8)
    }

    fn scaled_ui_px(&self, px: f32) -> f32 {
        px * self.resolution_ui_scale()
    }

    fn title_top_baseline_px(&self) -> f32 {
        self.scaled_ui_px(16.0)
    }

    fn title_side_margin_px(&self) -> f32 {
        self.scaled_ui_px(5.0)
    }

    fn title_font_size(&self) -> u16 {
        let scale = self.resolution_ui_scale();
        (14.0 * scale).round().clamp(11.0, 28.0) as u16
    }

    fn barline_label_font_size(&self) -> u16 {
        let scale = self.resolution_ui_scale();
        (14.0 * scale).round().clamp(10.0, 26.0) as u16
    }

    fn judge_label_font_size(&self) -> u16 {
        let scale = self.resolution_ui_scale();
        (17.0 * scale).round().clamp(12.0, 30.0) as u16
    }

    fn begin_view_clip_rect(&self, rect: Rect) {
        let sw = screen_width();
        let sh = screen_height();
        let x1 = rect.x.floor().clamp(0.0, sw);
        let y1 = rect.y.floor().clamp(0.0, sh);
        let x2 = (rect.x + rect.w).ceil().clamp(0.0, sw);
        let y2 = (rect.y + rect.h).ceil().clamp(0.0, sh);

        let clip = if x2 > x1 && y2 > y1 {
            Some((x1 as i32, y1 as i32, (x2 - x1) as i32, (y2 - y1) as i32))
        } else {
            Some((0, 0, 0, 0))
        };

        unsafe {
            let gl = macroquad::window::get_internal_gl();
            gl.quad_gl.scissor(clip);
        }
    }

    fn end_view_clip_rect(&self) {
        unsafe {
            let gl = macroquad::window::get_internal_gl();
            gl.quad_gl.scissor(None);
        }
    }

    /// 镜像一个音符：Ground lane 做 5-lane，Air 做 1.0-x，Flick 翻转方向。
    fn mirror_note(note: &GroundNote) -> GroundNote {
        let mut mirrored = note.clone();
        match mirrored.kind {
            GroundNoteKind::Tap | GroundNoteKind::Hold => {
                // Ground: lane 镜像 0↔5, 1↔4, 2↔3
                mirrored.lane = 5 - mirrored.lane.min(5);
            }
            GroundNoteKind::Flick => {
                // Air Flick: center_x 镜像, flick_right 翻转
                mirrored.center_x_norm = 1.0 - mirrored.center_x_norm;
                mirrored.lane = air_x_to_lane(mirrored.center_x_norm);
                mirrored.flick_right = !mirrored.flick_right;
            }
            GroundNoteKind::SkyArea => {
                // SkyArea: shape 的 left/right 做 1.0-x 并交换
                mirrored.center_x_norm = 1.0 - mirrored.center_x_norm;
                mirrored.lane = air_x_to_lane(mirrored.center_x_norm);
                if let Some(shape) = mirrored.skyarea_shape.as_mut() {
                    let new_start_left = 1.0 - shape.start_right_norm;
                    let new_start_right = 1.0 - shape.start_left_norm;
                    let new_end_left = 1.0 - shape.end_right_norm;
                    let new_end_right = 1.0 - shape.end_left_norm;
                    shape.start_left_norm = new_start_left;
                    shape.start_right_norm = new_start_right;
                    shape.end_left_norm = new_end_left;
                    shape.end_right_norm = new_end_right;
                }
            }
        }
        mirrored
    }

    /// 复制选中音符到剪贴板
    fn copy_selected_to_clipboard(&mut self) {
        if self.selection.selected_note_ids.is_empty() {
            let msg = self
                .i18n
                .t(crate::i18n::TextKey::EditorNothingToCopy)
                .to_owned();
            self.status = msg;
            return;
        }

        let mut copied = Vec::new();
        for &nid in &self.selection.selected_note_ids {
            if let Some(note) = self.editor_state.notes.iter().find(|n| n.id == nid) {
                copied.push(note.clone());
            }
        }
        self.clipboard.set_notes(copied);
        let count = self.clipboard.notes().len();
        let msg = self
            .i18n
            .t(crate::i18n::TextKey::EditorCopiedNotes)
            .replace("{count}", &count.to_string());
        self.status = msg.clone();
        self.push_toast(msg);
    }

    /// 剪切选中音符到剪贴板
    fn cut_selected_to_clipboard(&mut self) {
        if self.selection.selected_note_ids.is_empty() {
            let msg = self
                .i18n
                .t(crate::i18n::TextKey::EditorNothingToCut)
                .to_owned();
            self.status = msg.clone();
            self.push_toast_warn(msg);
            return;
        }

        let mut cut_notes = Vec::new();
        for &nid in &self.selection.selected_note_ids {
            if let Some(note) = self.editor_state.notes.iter().find(|n| n.id == nid) {
                cut_notes.push(note.clone());
            }
        }
        self.clipboard.set_notes(cut_notes);
        let count = self.clipboard.notes().len();
        self.selection.editing_note_backup = None;
        self.snapshot_for_undo();
        let ids = self.selection.selected_note_ids.clone();
        self.editor_state.notes.retain(|n| !ids.contains(&n.id));
        self.editor_state.cached_note_heads_dirty = true;
        self.selection.clear_note_selection();
        self.selection.drag_state = None;
        self.selection.overlap_cycle = None;
        self.selection.hover_overlap_hint = None;
        let msg = self
            .i18n
            .t(crate::i18n::TextKey::EditorCutNotes)
            .replace("{count}", &count.to_string());
        self.status = msg.clone();
        self.push_toast(msg);
    }

    /// 原地镜像选中音符，不复制 (Ctrl+B)
    fn mirror_selected_notes(&mut self) {
        if self.selection.selected_note_ids.is_empty() {
            let msg = self
                .i18n
                .t(crate::i18n::TextKey::EditorNothingToMirror)
                .to_owned();
            self.status = msg.clone();
            self.push_toast_warn(msg);
            return;
        }
        self.snapshot_for_undo();
        let ids: Vec<u64> = self.selection.selected_note_ids.iter().copied().collect();
        let mut count = 0usize;
        for &nid in &ids {
            if let Some(note) = self
                .editor_state
                .notes
                .iter()
                .find(|n| n.id == nid)
                .cloned()
            {
                let mirrored = Self::mirror_note(&note);
                if let Some(n) = self.editor_state.notes.iter_mut().find(|n| n.id == nid) {
                    *n = mirrored;
                    n.id = nid; // 保持原 ID
                    count += 1;
                }
            }
        }
        self.sort_notes();
        self.editor_state.cached_note_heads_dirty = true;
        let msg = self
            .i18n
            .t(crate::i18n::TextKey::EditorMirroredNotes)
            .replace("{count}", &count.to_string());
        self.status = msg.clone();
        self.push_toast(msg);
    }

    /// 复制并镜像选中音符 (Ctrl+M)
    fn mirror_selected_in_place(&mut self) {
        if self.selection.selected_note_ids.is_empty() {
            let msg = self
                .i18n
                .t(crate::i18n::TextKey::EditorNothingToMirror)
                .to_owned();
            self.status = msg.clone();
            self.push_toast_warn(msg);
            return;
        }
        self.snapshot_for_undo();
        let ids: Vec<u64> = self.selection.selected_note_ids.iter().copied().collect();
        let mut new_notes = Vec::new();
        for &nid in &ids {
            if let Some(note) = self.editor_state.notes.iter().find(|n| n.id == nid) {
                let mut mirrored = Self::mirror_note(note);
                mirrored.id = self.editor_state.next_note_id;
                self.editor_state.next_note_id = self.editor_state.next_note_id.saturating_add(1);
                new_notes.push(mirrored);
            }
        }
        let count = new_notes.len();
        for n in new_notes {
            self.editor_state.notes.push(n);
        }
        self.sort_notes();
        self.editor_state.cached_note_heads_dirty = true;
        self.selection.clear_note_selection();
        let msg = self
            .i18n
            .t(crate::i18n::TextKey::EditorCopyMirroredNotes)
            .replace("{count}", &count.to_string());
        self.status = msg.clone();
        self.push_toast(msg);
    }

    /// 进入粘贴模式
    fn enter_paste_mode(&mut self, mode: PasteMode) {
        if self.clipboard.is_empty() {
            self.status = self
                .i18n
                .t(crate::i18n::TextKey::EditorClipboardEmpty)
                .to_owned();
            return;
        }
        self.clipboard.set_paste_mode(mode);
        self.view.paste_preview_cache = None;
        self.selection.prepare_for_paste_mode();
        let key = match mode {
            PasteMode::Normal => crate::i18n::TextKey::EditorPasteModeNormal,
            PasteMode::Mirrored => crate::i18n::TextKey::EditorPasteModeMirrored,
        };
        self.status = self.i18n.t(key).to_owned();
    }

    /// 退出粘贴模式
    fn exit_paste_mode(&mut self) {
        self.clipboard.clear_paste_mode();
        self.view.paste_preview_cache = None;
        self.status = self
            .i18n
            .t(crate::i18n::TextKey::EditorPasteCancelled)
            .to_owned();
    }

    /// 推送一条 info toast（由 main.rs drain）
    fn push_toast(&mut self, msg: impl Into<String>) {
        self.pending_toasts.push((msg.into(), false));
    }

    /// 推送一条 warn toast（由 main.rs drain）
    fn push_toast_warn(&mut self, msg: impl Into<String>) {
        self.pending_toasts.push((msg.into(), true));
    }

    /// Unified check: if spectrum would be visible but waveform is still loading, warn.
    /// Called from set_show_spectrum and set_track_speed_enabled.
    fn check_spectrum_loading_toast(&mut self) {
        if self.view.show_spectrum
            && !self.editor_state.track_speed_enabled
            && self.view.waveform.is_none()
            && self.view.waveform_task.is_some()
        {
            let msg = self
                .i18n
                .t(crate::i18n::TextKey::SpectrumStillLoading)
                .to_owned();
            self.push_toast_warn(msg);
        }
    }

    /// 取出所有待发送的 toast（由 main.rs 调用）
    pub fn drain_toasts(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.pending_toasts)
    }
}
