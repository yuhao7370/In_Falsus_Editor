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
        }
    }

    pub fn set_text_font(&mut self, font: Option<Font>) {
        self.text_font = font;
    }

    pub fn place_note_type(&self) -> Option<PlaceNoteType> {
        self.place_note_type
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
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.status = match note_type {
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

    pub fn track_speed_enabled(&self) -> bool {
        self.track_speed_enabled
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

