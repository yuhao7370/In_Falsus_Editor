impl FallingGroundEditor {
    pub fn new() -> Self {
        Self::from_chart_path(DEFAULT_CHART_PATH)
    }

    pub fn from_chart_path(path: &str) -> Self {
        let (notes, timeline, timeline_events, status) = match Chart::from_file(path) {
            Ok(chart) => {
                (
                    extract_ground_notes(&chart),
                    BpmTimeline::from_chart(&chart),
                    extract_timeline_events(&chart),
                    format!("chart loaded: {path}"),
                )
            }
            Err(err) => {
                (
                    Vec::new(),
                    BpmTimeline {
                        points: vec![BpmPoint {
                            time_ms: 0.0,
                            bpm: 120.0,
                            beats_per_measure: 4.0,
                            start_beat: 0.0,
                        }],
                    },
                    vec![TimelineEvent {
                        time_ms: 0.0,
                        label: "chart 120.00/4.00".to_owned(),
                        color: Color::from_rgba(140, 214, 255, 255),
                    }],
                    format!("failed to load chart: {err}"),
                )
            }
        };

        let next_note_id = notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        Self {
            chart_path: path.to_owned(),
            notes,
            next_note_id,
            selected_note_id: None,
            drag_state: None,
            timeline,
            timeline_events,
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
            status,
        }
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
            self.status = format!("snap division: {}x", division);
        }
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



}

