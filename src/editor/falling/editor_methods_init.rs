// 文件说明：编辑器初始化与基础状态切换实现。
// 主要功能：加载谱面、构建时间轴并初始化编辑器运行状态。
impl FallingGroundEditor {
    pub fn new(default_chart_path: &str) -> Self {
        Self::from_chart_path(default_chart_path)
    }

    pub fn from_chart_path(path: &str) -> Self {
        let (notes, next_note_id, timeline, track_timeline, track_source, timeline_events, next_event_id, status) = match Chart::from_file(path) {
            Ok(chart) => {
                let extracted = extract_chart_data(&chart);
                let bpm_tl = BpmTimeline::from_source(extracted.bpm_source);
                let track_src = extracted.track_source;
                let track_tl = TrackTimeline::from_source(&bpm_tl, track_src.clone());
                (
                    extracted.notes,
                    extracted.next_note_id,
                    bpm_tl,
                    track_tl,
                    track_src,
                    extracted.timeline_events,
                    extracted.next_event_id,
                    format!("chart loaded: {path}"),
                )
            }
            Err(err) => {
                let bpm_tl = BpmTimeline::from_source(BpmSourceData::default());
                let track_src = TrackSourceData::default();
                let track_tl = TrackTimeline::from_source(&bpm_tl, track_src.clone());
                (
                    Vec::new(),
                    1,
                    bpm_tl,
                    track_tl,
                    track_src,
                    vec![TimelineEvent {
                        id: 1,
                        kind: TimelineEventKind::Bpm,
                        source_index: 0,
                        time_ms: 0.0,
                        label: "chart 120.00/4.00".to_owned(),
                        color: Color::from_rgba(140, 214, 255, 255),
                    }],
                    2,
                    format!("failed to load chart: {err}"),
                )
            }
        };

        let initial_subdivision = 4u32;
        let cached_barlines = timeline.precompute_all_barlines(&track_timeline, 600_000.0, initial_subdivision);

        Self {
            chart_path: path.to_owned(),
            notes,
            next_note_id,
            selected_note_id: None,
            drag_state: None,
            timeline,
            track_timeline,
            track_source,
            track_speed_enabled: true,
            cached_barlines,
            cached_barlines_subdivision: initial_subdivision,
            timeline_events,
            selected_event_id: None,
            event_overlap_cycle: None,
            event_hover_hint: None,
            next_event_id,
            snap_enabled: true,
            snap_division: 4,
            scroll_speed: DEFAULT_SCROLL_SPEED,
            render_scope: RenderScope::Both,
            place_note_type: None,
            place_event_type: None,
            pending_hold: None,
            pending_skyarea: None,
            overlap_cycle: None,
            hover_overlap_hint: None,
            debug_show_hitboxes: false,
            autoplay_enabled: false,
            show_spectrum: true,
            show_minimap: false,
            waveform: None,
            waveform_error: None,
            waveform_seek_active: false,
            waveform_seek_sec: 0.0,
            minimap_drag_active: false,
            minimap_drag_offset_ms: 0.0,
            minimap_last_emit_sec: None,
            minimap_drag_target_sec: None,
            minimap_page: None,
            text_font: None,
            status,
            undo_history: UndoHistory::new(200),
            x_split: 128.0,
            dirty: false,
            editing_note_backup: None,
            editing_event_backup: None,
        }
    }

    pub fn chart_path(&self) -> &str {
        &self.chart_path
    }

    pub fn set_chart_path(&mut self, path: String) {
        self.chart_path = path;
    }

    /// Convert editor state back to a Chart for saving.
    pub fn to_chart(&self) -> Chart {
        let mut events: Vec<ChartEvent> = Vec::new();

        // 1. Reconstruct chart header from base BPM
        let base_bpm = self.timeline.points[0].bpm as f64;
        let base_beats = self.timeline.points[0].beats_per_measure as f64;
        events.push(ChartEvent::Chart {
            bpm: base_bpm,
            beats: base_beats,
        });

        // 2. BPM change events (skip the base point at time 0)
        for point in self.timeline.points.iter().skip(1) {
            events.push(ChartEvent::Bpm {
                time: point.time_ms as f64,
                bpm: point.bpm as f64,
                beats: point.beats_per_measure as f64,
                unknown: 0.0,
            });
        }

        // 3. Track speed events
        for &(time_ms, speed) in &self.track_source.track_events {
            events.push(ChartEvent::Track {
                time: time_ms as f64,
                speed: speed as f64,
            });
        }

        // 4. Lane events from timeline_events
        for event in &self.timeline_events {
            if event.kind == TimelineEventKind::Lane {
                // Parse "lane N on/off" from label
                let parts: Vec<&str> = event.label.split_whitespace().collect();
                if parts.len() >= 3 {
                    let lane = parts[1].parse::<i32>().unwrap_or(0);
                    let enable = parts[2] == "on";
                    events.push(ChartEvent::Lane {
                        time: event.time_ms as f64,
                        lane,
                        enable,
                    });
                }
            }
        }

        // 5. Notes
        for note in &self.notes {
            match note.kind {
                GroundNoteKind::Tap => {
                    events.push(ChartEvent::Tap {
                        time: note.time_ms as f64,
                        width: note.width as f64,
                        lane: note.lane as i32,
                    });
                }
                GroundNoteKind::Hold => {
                    events.push(ChartEvent::Hold {
                        time: note.time_ms as f64,
                        lane: note.lane as i32,
                        width: note.width as f64,
                        duration: note.duration_ms as f64,
                    });
                }
                GroundNoteKind::Flick => {
                    let xs = note.x_split.max(1.0);
                    let width_norm = note.width.clamp(0.05, 1.0) as f64;
                    let lane_center = note.center_x_norm as f64;
                    let flick_type = if note.flick_right {
                        FlickType::Right
                    } else {
                        FlickType::Left
                    };
                    let width = width_norm * xs;
                    let x = lane_center * xs; // X is center point
                    events.push(ChartEvent::Flick {
                        time: note.time_ms as f64,
                        x,
                        x_split: xs,
                        width,
                        flick_type,
                    });
                }
                GroundNoteKind::SkyArea => {
                    if let Some(shape) = note.skyarea_shape {
                        let sxs = shape.start_x_split.max(1.0);
                        let exs = shape.end_x_split.max(1.0);
                        let start_center = ((shape.start_left_norm + shape.start_right_norm) * 0.5) as f64;
                        let end_center = ((shape.end_left_norm + shape.end_right_norm) * 0.5) as f64;
                        let start_width = ((shape.start_right_norm - shape.start_left_norm).abs()) as f64;
                        let end_width = ((shape.end_right_norm - shape.end_left_norm).abs()) as f64;
                        events.push(ChartEvent::SkyArea {
                            time: note.time_ms as f64,
                            start_x: start_center * sxs,
                            start_x_split: sxs,
                            start_width: start_width * sxs,
                            end_x: end_center * exs,
                            end_x_split: exs,
                            end_width: end_width * exs,
                            left_ease: shape.left_ease,
                            right_ease: shape.right_ease,
                            duration: note.duration_ms as f64,
                            group_id: 0,
                        });
                    }
                }
            }
        }

        // Sort by time (chart header stays first since time=0)
        events.sort_by(|a, b| {
            let time_a = chart_event_time(a);
            let time_b = chart_event_time(b);
            time_a.total_cmp(&time_b)
        });

        Chart { events }
    }

    /// Save current editor state to the .spc file.
    pub fn save_chart(&mut self) -> Result<(), String> {
        let chart = self.to_chart();
        let content = chart.to_spc();
        std::fs::write(&self.chart_path, content)
            .map_err(|e| format!("写入文件失败: {e}"))?;
        self.dirty = false;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_text_font(&mut self, font: Option<Font>) {
        self.text_font = font;
    }

    pub fn place_note_type(&self) -> Option<PlaceNoteType> {
        self.place_note_type
    }

    pub fn place_event_type(&self) -> Option<PlaceEventType> {
        self.place_event_type
    }

    pub fn render_scope(&self) -> RenderScope {
        self.render_scope
    }

    pub fn set_render_scope(&mut self, scope: RenderScope) {
        self.render_scope = scope;
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.minimap_drag_active = false;
        self.minimap_drag_target_sec = None;
        self.minimap_last_emit_sec = None;
        self.status = format!("render scope: {}", scope.label());
    }

    pub fn set_place_note_type(&mut self, note_type: Option<PlaceNoteType>) {
        self.place_note_type = note_type;
        self.place_event_type = None;
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.status = match note_type {
            Some(kind) => format!("place mode: {}", kind.label()),
            None => "place mode cleared".to_owned(),
        };
    }

    pub fn set_place_event_type(&mut self, event_type: Option<PlaceEventType>) {
        self.place_event_type = event_type;
        self.place_note_type = None;
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.status = match event_type {
            Some(kind) => format!("place mode: {}", kind.label()),
            None => "place mode cleared".to_owned(),
        };
    }

    pub fn snap_division(&self) -> u32 {
        self.snap_division
    }

    pub fn set_snap_division(&mut self, division: u32) {
        if SNAP_DIVISION_OPTIONS.contains(&division) {
            self.snap_division = division;
            self.rebuild_barline_cache();
            self.status = format!("snap division: {}x", division);
        }
    }

    pub fn scroll_speed(&self) -> f32 {
        self.scroll_speed
    }

    pub fn min_scroll_speed(&self) -> f32 {
        MIN_SCROLL_SPEED
    }

    pub fn max_scroll_speed(&self) -> f32 {
        MAX_SCROLL_SPEED
    }

    pub fn scroll_speed_step(&self) -> f32 {
        SCROLL_SPEED_STEP
    }

    pub fn set_scroll_speed(&mut self, speed: f32) {
        let old_speed = self.scroll_speed;
        self.scroll_speed = speed.clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED);
        if (old_speed - self.scroll_speed).abs() > 0.001 {
            self.status = format!("scroll speed set to {:.2}H/s", self.scroll_speed);
        }
    }

    pub fn nudge_scroll_speed(&mut self, delta: f32) {
        self.adjust_scroll_speed(delta);
    }

    pub fn pending_hold_head_time_ms(&self) -> Option<f32> {
        self.pending_hold.map(|pending| pending.start_time_ms)
    }

    pub fn pending_skyarea_head_time_ms(&self) -> Option<f32> {
        self.pending_skyarea.map(|pending| pending.start_time_ms)
    }

    pub fn debug_show_hitboxes(&self) -> bool {
        self.debug_show_hitboxes
    }

    pub fn set_debug_show_hitboxes(&mut self, enabled: bool) {
        self.debug_show_hitboxes = enabled;
        self.status = format!("debug hitbox {}", if enabled { "on" } else { "off" });
    }

    pub fn autoplay_enabled(&self) -> bool {
        self.autoplay_enabled
    }

    pub fn set_autoplay_enabled(&mut self, enabled: bool) {
        self.autoplay_enabled = enabled;
        self.status = format!("autoplay {}", if enabled { "on" } else { "off" });
    }

    pub fn show_spectrum(&self) -> bool {
        self.show_spectrum
    }

    pub fn set_show_spectrum(&mut self, enabled: bool) {
        self.show_spectrum = enabled;
        self.status = format!("spectrum {}", if enabled { "on" } else { "off" });
    }

    pub fn show_minimap(&self) -> bool {
        self.show_minimap
    }

    pub fn set_show_minimap(&mut self, enabled: bool) {
        self.show_minimap = enabled;
        if !enabled {
            self.minimap_drag_active = false;
            self.minimap_drag_offset_ms = 0.0;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
        }
    }

    pub fn x_split(&self) -> f64 {
        self.x_split
    }

    pub fn set_x_split(&mut self, value: f64) {
        self.x_split = value.clamp(1.0, 1024.0);
        self.status = format!("x_split set to {}", self.x_split);
    }

    pub fn track_speed_enabled(&self) -> bool {
        self.track_speed_enabled
    }

    // ── Beat conversion (public) ──

    pub fn time_to_beat(&self, time_ms: f32) -> f32 {
        self.timeline.time_to_beat(time_ms)
    }

    pub fn beat_to_time(&self, beat: f32) -> f32 {
        self.timeline.beat_to_time(beat)
    }

    // ── Property panel: Note ──

    pub fn selected_note_properties(&self) -> Option<NotePropertyData> {
        let id = self.selected_note_id?;
        let note = self.notes.iter().find(|n| n.id == id)?;
        let default_shape = SkyAreaShape {
            start_left_norm: 0.0, start_right_norm: 0.0,
            end_left_norm: 0.0, end_right_norm: 0.0,
            left_ease: Ease::Linear, right_ease: Ease::Linear,
            start_x_split: self.x_split, end_x_split: self.x_split,
        };
        let shape = note.skyarea_shape.unwrap_or(default_shape);
        // Flick: use per-note x_split
        let fxs = note.x_split.max(1.0);
        let flick_center_norm = note.center_x_norm as f64;
        let flick_width_norm = note.width.clamp(0.05, 1.0) as f64;
        let flick_center_raw = flick_center_norm * fxs;
        // SkyArea: use per-shape x_split
        let sxs = shape.start_x_split.max(1.0);
        let exs = shape.end_x_split.max(1.0);
        let start_center = ((shape.start_left_norm + shape.start_right_norm) * 0.5) as f64;
        let start_w = ((shape.start_right_norm - shape.start_left_norm).abs()) as f64;
        let end_center = ((shape.end_left_norm + shape.end_right_norm) * 0.5) as f64;
        let end_w = ((shape.end_right_norm - shape.end_left_norm).abs()) as f64;
        // duration beat
        let end_beat = self.timeline.time_to_beat(note.time_ms + note.duration_ms);
        let start_beat = self.timeline.time_to_beat(note.time_ms);
        // Flick width in its own xsplit coordinates; Tap/Hold use raw width
        let out_width = if note.kind == GroundNoteKind::Flick {
            (flick_width_norm * fxs) as f32
        } else {
            note.width
        };
        Some(NotePropertyData {
            id: note.id,
            kind: match note.kind {
                GroundNoteKind::Tap => "Tap",
                GroundNoteKind::Hold => "Hold",
                GroundNoteKind::Flick => "Flick",
                GroundNoteKind::SkyArea => "SkyArea",
            }.to_owned(),
            lane: note.lane,
            time_ms: note.time_ms,
            beat: start_beat,
            duration_ms: note.duration_ms,
            duration_beat: end_beat - start_beat,
            width: out_width,
            flick_right: note.flick_right,
            x: flick_center_raw,
            x_split: fxs,
            start_x: start_center * sxs,
            start_x_split: sxs,
            start_width: start_w * sxs,
            end_x: end_center * exs,
            end_x_split: exs,
            end_width: end_w * exs,
            left_ease: shape.left_ease.to_value(),
            right_ease: shape.right_ease.to_value(),
        })
    }

    /// Begin editing: save backup of the note so we can cancel later.
    pub fn begin_note_edit(&mut self) {
        if let Some(id) = self.selected_note_id {
            if let Some(note) = self.notes.iter().find(|n| n.id == id) {
                self.editing_note_backup = Some(note.clone());
            }
        }
    }

    /// Preview: apply property changes live (no undo snapshot).
    pub fn preview_note_properties(&mut self, data: &NotePropertyData) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == data.id) {
            // Clamp lane for ground notes
            let max_lane = LANE_COUNT.saturating_sub(1);
            note.lane = if is_ground_kind(note.kind) { data.lane.min(max_lane) } else { data.lane };
            note.time_ms = data.time_ms.max(0.0);
            note.duration_ms = data.duration_ms.max(0.0);
            note.width = data.width.clamp(0.05, 8.0);
            note.flick_right = data.flick_right;
            // Flick: x is center point, convert to lane + normalized width
            if note.kind == GroundNoteKind::Flick {
                let xs = data.x_split.max(1.0);
                let raw_w = data.width as f64;
                let norm_x = (data.x / xs) as f32; // x is already center
                note.lane = lane_from_normalized_x(norm_x);
                // Flick width: raw width / x_split → normalized width ratio
                note.width = normalized_width_to_air_ratio((raw_w / xs) as f32);
            }
            // SkyArea: convert raw x/width back to normalized left/right
            if note.kind == GroundNoteKind::SkyArea {
                if let Some(shape) = note.skyarea_shape.as_mut() {
                    let sxs = data.start_x_split.max(1.0);
                    let exs = data.end_x_split.max(1.0);
                    let sc = (data.start_x / sxs) as f32;
                    let sh = ((data.start_width / sxs) as f32).abs() * 0.5;
                    let ec = (data.end_x / exs) as f32;
                    let eh = ((data.end_width / exs) as f32).abs() * 0.5;
                    shape.start_left_norm = (sc - sh).clamp(0.0, 1.0);
                    shape.start_right_norm = (sc + sh).clamp(0.0, 1.0);
                    shape.end_left_norm = (ec - eh).clamp(0.0, 1.0);
                    shape.end_right_norm = (ec + eh).clamp(0.0, 1.0);
                    shape.left_ease = Ease::from_value(data.left_ease);
                    shape.right_ease = Ease::from_value(data.right_ease);
                }
            }
        }
    }

    /// Apply: commit the edit with undo support.
    pub fn apply_note_properties(&mut self, data: &NotePropertyData) {
        // Restore backup first so snapshot captures the pre-edit state
        if let Some(backup) = self.editing_note_backup.take() {
            if let Some(note) = self.notes.iter_mut().find(|n| n.id == backup.id) {
                *note = backup;
            }
        }
        self.snapshot_for_undo();
        self.preview_note_properties(data);
        self.sort_notes();
        self.editing_note_backup = None;
    }

    /// Cancel: restore the backup and deselect.
    pub fn cancel_note_edit(&mut self) {
        if let Some(backup) = self.editing_note_backup.take() {
            if let Some(note) = self.notes.iter_mut().find(|n| n.id == backup.id) {
                *note = backup;
            }
        }
        self.deselect_note();
    }

    pub fn deselect_note(&mut self) {
        self.selected_note_id = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
    }

    // ── Property panel: Event ──

    pub fn selected_event_properties(&self) -> Option<EventPropertyData> {
        let id = self.selected_event_id?;
        let event = self.timeline_events.iter().find(|e| e.id == id)?;
        Some(EventPropertyData {
            id: event.id,
            kind: match event.kind {
                TimelineEventKind::Bpm => "Bpm",
                TimelineEventKind::Track => "Track",
                TimelineEventKind::Lane => "Lane",
            }.to_owned(),
            time_ms: event.time_ms,
            beat: self.timeline.time_to_beat(event.time_ms),
            label: event.label.clone(),
        })
    }

    pub fn begin_event_edit(&mut self) {
        if let Some(id) = self.selected_event_id {
            if let Some(event) = self.timeline_events.iter().find(|e| e.id == id) {
                self.editing_event_backup = Some(event.clone());
            }
        }
    }

    pub fn preview_event_properties(&mut self, data: &EventPropertyData) {
        if let Some(event) = self.timeline_events.iter_mut().find(|e| e.id == data.id) {
            event.time_ms = data.time_ms.max(0.0);
            event.label = data.label.clone();
        }
    }

    pub fn apply_event_properties(&mut self, data: &EventPropertyData) {
        if let Some(backup) = self.editing_event_backup.take() {
            if let Some(event) = self.timeline_events.iter_mut().find(|e| e.id == backup.id) {
                *event = backup;
            }
        }
        self.snapshot_for_undo();
        self.preview_event_properties(data);
        self.editing_event_backup = None;
    }

    pub fn cancel_event_edit(&mut self) {
        if let Some(backup) = self.editing_event_backup.take() {
            if let Some(event) = self.timeline_events.iter_mut().find(|e| e.id == backup.id) {
                *event = backup;
            }
        }
        self.deselect_event();
    }

    pub fn deselect_event(&mut self) {
        self.selected_event_id = None;
        self.event_overlap_cycle = None;
        self.event_hover_hint = None;
    }

    pub fn is_editing_note(&self) -> bool {
        self.editing_note_backup.is_some()
    }

    pub fn is_editing_event(&self) -> bool {
        self.editing_event_backup.is_some()
    }

    pub fn set_track_speed_enabled(&mut self, enabled: bool) {
        if self.track_speed_enabled == enabled {
            return;
        }
        self.track_speed_enabled = enabled;
        let source = if enabled {
            self.track_source.clone()
        } else {
            TrackSourceData::default()
        };
        self.track_timeline = TrackTimeline::from_source(&self.timeline, source);
        self.rebuild_barline_cache();
        self.status = format!(
            "track speed {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }
}

