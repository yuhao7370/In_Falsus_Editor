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
    fn visible_ahead_behind_ms(&self, rect_y: f32, rect_h: f32, current_ms: f32, judge_y: f32) -> (f32, f32) {
        let top_time = self.pointer_to_time(rect_y, current_ms, judge_y, rect_h);
        let bottom_time = self.pointer_to_time(rect_y + rect_h, current_ms, judge_y, rect_h);
        let ahead_ms = (top_time - current_ms).max(0.0);
        let behind_ms = (current_ms - bottom_time).max(0.0);
        (ahead_ms, behind_ms)
    }

    fn flick_side_height_px(&self, _note_time_ms: f32, lane_h: f32) -> f32 {
        let base_bpm = self.timeline.points[0].bpm.abs().max(0.001);
        let beat_ms = 60_000.0 / base_bpm;
        let subdivision_ms = beat_ms / 16.0;
        let pixels_per_sec = (self.scroll_speed * lane_h).max(1.0);
        subdivision_ms / 1000.0 * pixels_per_sec
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

    fn push_note(&mut self, note: GroundNote) {
        self.next_note_id = self.next_note_id.saturating_add(1);
        self.selected_note_id = Some(note.id);
        self.notes.push(note);
        self.sort_notes();
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

