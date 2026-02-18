// 文件说明：编辑器内部通用操作函数集合。
// 主要功能：封装音符增删改、排序、吸附和状态维护。
impl FallingGroundEditor {
    fn pointer_to_time(&self, mouse_y: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        // scroll_speed 单位：屏高/秒，visual_beat 单位：毫秒等效（speed=1 时 = dt_ms）
        let pixels_per_ms = (self.scroll_speed * lane_h / 1000.0).max(0.001);
        let current_vb = self.track_timeline.visual_beat_at(current_ms);
        let delta_vb = (judge_y - mouse_y) / pixels_per_ms;
        let target_vb = current_vb + delta_vb;
        self.track_timeline.visual_beat_to_time(target_vb)
    }

    fn time_to_y(&self, note_time_ms: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        let pixels_per_ms = (self.scroll_speed * lane_h / 1000.0).max(0.001);
        let note_vb = self.track_timeline.visual_beat_at(note_time_ms);
        let current_vb = self.track_timeline.visual_beat_at(current_ms);
        judge_y - (note_vb - current_vb) * pixels_per_ms
    }

    /// 计算当前视口中可见的时间范围（考虑 track speed 变化）。
    /// 返回 (ahead_ms, behind_ms)，ahead 是判定线上方的时间跨度，behind 是下方的。
    /// 当 track speed 为负时，top/bottom 时间可能反转，取 min/max 保证覆盖完整范围。
    fn visible_ahead_behind_ms(&self, rect_y: f32, rect_h: f32, current_ms: f32, judge_y: f32) -> (f32, f32) {
        let top_time = self.pointer_to_time(rect_y, current_ms, judge_y, rect_h);
        let bottom_time = self.pointer_to_time(rect_y + rect_h, current_ms, judge_y, rect_h);
        let min_time = top_time.min(bottom_time);
        let max_time = top_time.max(bottom_time);
        let ahead_ms = (max_time - current_ms).max(0.0);
        let behind_ms = (current_ms - min_time).max(0.0);
        (ahead_ms, behind_ms)
    }

    /// 纯线性时间→Y 坐标（不受 track speed 影响，等效 speed=1）。
    fn time_to_y_linear(&self, note_time_ms: f32, current_ms: f32, judge_y: f32, lane_h: f32) -> f32 {
        let pixels_per_ms = (self.scroll_speed * lane_h / 1000.0).max(0.001);
        judge_y - (note_time_ms - current_ms) * pixels_per_ms
    }

    /// 纯线性可见时间范围（不受 track speed 影响）。
    fn visible_ahead_behind_ms_linear(&self, rect_y: f32, rect_h: f32, _current_ms: f32, judge_y: f32) -> (f32, f32) {
        let pixels_per_ms = (self.scroll_speed * rect_h / 1000.0).max(0.001);
        let ahead_ms = (judge_y - rect_y) / pixels_per_ms;
        let behind_ms = (rect_y + rect_h - judge_y) / pixels_per_ms;
        (ahead_ms.max(0.0), behind_ms.max(0.0))
    }

    fn flick_side_height_px(&self, _note_time_ms: f32, lane_h: f32) -> f32 {
        let base_bpm = self.timeline.points[0].bpm.abs().max(0.001);
        let beat_ms = 60_000.0 / base_bpm;
        let subdivision_ms = beat_ms / 16.0;
        let pixels_per_sec = (self.scroll_speed * lane_h).max(1.0);
        subdivision_ms / 1000.0 * pixels_per_sec
    }

    /// 重建小节线缓存。在 BPM/track/subdivision 变化时调用。
    fn rebuild_barline_cache(&mut self) {
        self.cached_barlines = self.timeline.precompute_all_barlines(
            &self.track_timeline,
            600_000.0,
            self.snap_division,
        );
        self.cached_barlines_subdivision = self.snap_division;
    }

    /// 用二分查找从缓存中获取 visual_beat 在 [start_vb, end_vb] 范围内的小节线切片。
    /// 缓存已按 visual_beat 排序。
    fn visible_barlines_cached(&self, start_vb: f32, end_vb: f32) -> &[BarLine] {
        let lines = &self.cached_barlines;
        if lines.is_empty() {
            return &[];
        }
        let lo = lines.partition_point(|bl| bl.visual_beat < start_vb - 0.001);
        let hi = lines.partition_point(|bl| bl.visual_beat <= end_vb + 0.001);
        &lines[lo..hi]
    }

    fn apply_snap(&self, time_ms: f32) -> f32 {
        if self.snap_enabled {
            self.timeline.snap_time_ms(time_ms, self.snap_division)
        } else {
            time_ms.max(0.0)
        }
    }

    fn adjust_scroll_speed(&mut self, delta: f32) {
        let old_speed = self.scroll_speed;
        let new_speed = (self.scroll_speed + delta).clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED);
        self.scroll_speed = new_speed;
        if (old_speed - new_speed).abs() > 0.01 {
            self.status = format!("scroll speed set to {:.2}H/s", self.scroll_speed);
        }
    }

    /// Take a snapshot of current state for undo history.
    fn snapshot_for_undo(&mut self) {
        let bpm_source = BpmSourceData {
            base_bpm: self.timeline.points[0].bpm,
            base_beats_per_measure: self.timeline.points[0].beats_per_measure,
            bpm_events: self
                .timeline
                .points
                .iter()
                .skip(1)
                .map(|p| (p.time_ms, p.bpm, p.beats_per_measure))
                .collect(),
        };
        self.undo_history.push(EditorSnapshot {
            notes: self.notes.clone(),
            next_note_id: self.next_note_id,
            timeline_events: self.timeline_events.clone(),
            next_event_id: self.next_event_id,
            selected_note_id: self.selected_note_id,
            selected_event_id: self.selected_event_id,
            bpm_source,
            track_source: self.track_source.clone(),
        });
        self.dirty = true;
    }

    /// Restore editor state from a snapshot (shared by undo/redo).
    fn apply_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.notes = snapshot.notes;
        self.next_note_id = snapshot.next_note_id;
        self.timeline_events = snapshot.timeline_events;
        self.next_event_id = snapshot.next_event_id;
        self.selected_note_id = snapshot.selected_note_id;
        self.selected_event_id = snapshot.selected_event_id;
        self.track_source = snapshot.track_source;
        self.timeline = BpmTimeline::from_source(snapshot.bpm_source);
        let track_src = if self.track_speed_enabled {
            self.track_source.clone()
        } else {
            TrackSourceData::default()
        };
        self.track_timeline = TrackTimeline::from_source(&self.timeline, track_src);
        self.rebuild_barline_cache();
        self.drag_state = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.event_overlap_cycle = None;
        self.event_hover_hint = None;
        self.dirty = true;
    }

    /// Undo: restore previous state.
    pub fn undo(&mut self) -> bool {
        // If we're at the top of the stack, the current (post-edit) state
        // hasn't been saved yet. Push it so redo can return to it later.
        if self.undo_history.is_at_top() {
            let bpm_source = BpmSourceData {
                base_bpm: self.timeline.points[0].bpm,
                base_beats_per_measure: self.timeline.points[0].beats_per_measure,
                bpm_events: self
                    .timeline
                    .points
                    .iter()
                    .skip(1)
                    .map(|p| (p.time_ms, p.bpm, p.beats_per_measure))
                    .collect(),
            };
            self.undo_history.push(EditorSnapshot {
                notes: self.notes.clone(),
                next_note_id: self.next_note_id,
                timeline_events: self.timeline_events.clone(),
                next_event_id: self.next_event_id,
                selected_note_id: self.selected_note_id,
                selected_event_id: self.selected_event_id,
                bpm_source,
                track_source: self.track_source.clone(),
            });
        }
        if let Some(snapshot) = self.undo_history.undo() {
            let snapshot = snapshot.clone();
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
        if let Some(snapshot) = self.undo_history.redo() {
            let snapshot = snapshot.clone();
            self.apply_snapshot(snapshot);
            self.status = "redo".to_owned();
            true
        } else {
            self.status = "nothing to redo".to_owned();
            false
        }
    }

    fn push_note(&mut self, note: GroundNote) {
        self.next_note_id = self.next_note_id.saturating_add(1);
        self.selected_note_id = Some(note.id);
        self.selected_event_id = None;
        self.event_overlap_cycle = None;
        self.event_hover_hint = None;
        self.notes.push(note);
        self.sort_notes();
    }

    fn push_timeline_event(&mut self, event: TimelineEvent) {
        self.next_event_id = self.next_event_id.saturating_add(1);
        self.selected_event_id = Some(event.id);
        self.selected_note_id = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.timeline_events.push(event);
        self.timeline_events.sort_by(|a, b| {
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
                let point = self.timeline.point_at_time(time_ms);
                let bpm = point.bpm.abs().max(0.001);
                let beats = point.beats_per_measure.max(1.0);
                let mut bpm_source = BpmSourceData {
                    base_bpm: self.timeline.points[0].bpm,
                    base_beats_per_measure: self.timeline.points[0].beats_per_measure,
                    bpm_events: self
                        .timeline
                        .points
                        .iter()
                        .skip(1)
                        .map(|p| (p.time_ms, p.bpm, p.beats_per_measure))
                        .collect(),
                };
                bpm_source.bpm_events.push((time_ms, bpm, beats));
                self.timeline = BpmTimeline::from_source(bpm_source);
                let track_source = if self.track_speed_enabled {
                    self.track_source.clone()
                } else {
                    TrackSourceData::default()
                };
                self.track_timeline = TrackTimeline::from_source(&self.timeline, track_source);
                self.rebuild_barline_cache();
                self.push_timeline_event(TimelineEvent {
                    id: self.next_event_id,
                    kind: TimelineEventKind::Bpm,
                    source_index: 0,
                    time_ms,
                    label: format!("bpm {:.2} (beats {:.2})", bpm, beats),
                    color: Color::from_rgba(124, 226, 255, 255),
                });
                self.status = format!("new bpm event {:.0}ms", time_ms.round());
            }
            PlaceEventType::Track => {
                let idx = self.track_timeline.point_index_at_or_before(time_ms);
                let speed = self.track_timeline.points[idx].speed;
                self.track_source.track_events.push((time_ms, speed));
                let track_source = if self.track_speed_enabled {
                    self.track_source.clone()
                } else {
                    TrackSourceData::default()
                };
                self.track_timeline = TrackTimeline::from_source(&self.timeline, track_source);
                self.rebuild_barline_cache();
                let color = if speed >= 0.0 {
                    Color::from_rgba(150, 240, 170, 255)
                } else {
                    Color::from_rgba(255, 168, 128, 255)
                };
                self.push_timeline_event(TimelineEvent {
                    id: self.next_event_id,
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
                    id: self.next_event_id,
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
        self.notes.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.lane.cmp(&b.lane))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn sync_waveform(&mut self, audio_path: Option<&str>) {
        let Some(path) = audio_path else {
            return;
        };
        let changed = self
            .waveform
            .as_ref()
            .map(|wave| wave.path.as_str() != path)
            .unwrap_or(true);
        if !changed {
            return;
        }

        match Waveform::from_audio_file(path, 4096) {
            Ok(waveform) => {
                self.waveform = Some(waveform);
                self.waveform_error = None;
                self.status = format!("waveform loaded: {path}");
            }
            Err(err) => {
                self.waveform = None;
                self.waveform_error = Some(err);
            }
        }
    }

    fn estimate_duration(&self, audio_duration_sec: f32) -> f32 {
        if audio_duration_sec > 0.0 {
            return audio_duration_sec;
        }
        self.waveform
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


}

