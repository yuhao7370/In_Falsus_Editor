#[derive(Debug)]
struct EditorState {
    notes: Vec<GroundNote>,
    next_note_id: u64,
    timeline: BpmTimeline,
    track_timeline: TrackTimeline,
    track_source: TrackSourceData,
    track_speed_enabled: bool,
    cached_barlines: Vec<BarLine>,
    cached_barlines_subdivision: u32,
    timeline_events: Vec<TimelineEvent>,
    next_event_id: u64,
    x_split: f64,
    xsplit_editable: bool,
    dirty: bool,
    cached_note_heads: Vec<(f32, bool)>,
    cached_note_heads_dirty: bool,
    cached_note_render: Vec<NoteRenderCache>,
    cached_note_render_dirty: bool,
}

#[derive(Debug)]
struct SelectionState {
    selected_note_id: Option<u64>,
    selected_note_ids: HashSet<u64>,
    drag_state: Option<DragState>,
    multi_drag_state: Option<MultiDragState>,
    overlap_cycle: Option<OverlapCycleState>,
    hover_overlap_hint: Option<HoverOverlapHint>,
    selected_event_id: Option<u64>,
    selected_event_ids: HashSet<u64>,
    event_overlap_cycle: Option<EventOverlapCycle>,
    event_hover_hint: Option<EventHoverOverlapHint>,
    place_note_type: Option<PlaceNoteType>,
    place_event_type: Option<PlaceEventType>,
    place_flick_right: bool,
    pending_hold: Option<PendingHoldPlacement>,
    pending_skyarea: Option<PendingSkyAreaPlacement>,
    editing_note_backup: Option<GroundNote>,
    editing_event_backup: Option<TimelineEvent>,
    box_select: Option<BoxSelectState>,
}

#[derive(Debug)]
struct ViewState {
    snap_enabled: bool,
    snap_division: u32,
    scroll_speed: f32,
    render_scope: RenderScope,
    debug_show_hitboxes: bool,
    debug_skyarea_body_only: bool,
    autoplay_enabled: bool,
    show_spectrum: bool,
    show_barlines: bool,
    color_barlines: bool,
    show_minimap: bool,
    waveform: Option<Waveform>,
    waveform_error: Option<String>,
    waveform_task: Option<mpsc::Receiver<Result<Waveform, String>>>,
    waveform_loading_path: Option<String>,
    waveform_seek_active: bool,
    waveform_seek_sec: f32,
    minimap_drag_active: bool,
    minimap_drag_offset_ms: f32,
    minimap_last_emit_sec: Option<f32>,
    minimap_drag_target_sec: Option<f32>,
    #[allow(dead_code)]
    minimap_page: Option<MinimapPageConfig>,
    paste_preview_cache: Option<PastePreviewCache>,
    text_font: Option<Font>,
}

#[derive(Debug, Default)]
struct ClipboardManager {
    clipboard: Vec<GroundNote>,
    paste_mode: Option<PasteMode>,
    version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PastePreviewCacheKey {
    mode: PasteMode,
    clipboard_version: u64,
    mouse_x_q: i32,
    mouse_y_q: i32,
    time_q: i32,
    ground_rect_q: Option<(i32, i32, i32, i32)>,
    air_rect_q: Option<(i32, i32, i32, i32)>,
}

#[derive(Debug, Clone)]
struct PastePreviewCache {
    key: PastePreviewCacheKey,
    notes: std::sync::Arc<[GroundNote]>,
}

impl EditorState {
    fn bpm_source(&self) -> BpmSourceData {
        BpmSourceData {
            base_bpm: self.timeline.points[0].bpm,
            base_beats_per_measure: self.timeline.points[0].beats_per_measure,
            bpm_events: self
                .timeline
                .points
                .iter()
                .skip(1)
                .map(|p| (p.time_ms, p.bpm, p.beats_per_measure))
                .collect(),
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            notes: self.notes.clone(),
            next_note_id: self.next_note_id,
            timeline_events: self.timeline_events.clone(),
            next_event_id: self.next_event_id,
            bpm_source: self.bpm_source(),
            track_source: self.track_source.clone(),
        }
    }

    fn rebuild_track_timeline(&mut self) {
        let track_src = if self.track_speed_enabled {
            self.track_source.clone()
        } else {
            TrackSourceData::default()
        };
        self.track_timeline = TrackTimeline::from_source(&self.timeline, track_src);
        self.cached_note_render_dirty = true;
    }

    fn apply_snapshot(&mut self, snapshot: EditorSnapshot) {
        self.notes = snapshot.notes;
        self.next_note_id = snapshot.next_note_id;
        self.timeline_events = snapshot.timeline_events;
        self.next_event_id = snapshot.next_event_id;
        self.track_source = snapshot.track_source;
        self.timeline = BpmTimeline::from_source(snapshot.bpm_source);
        self.rebuild_track_timeline();
        self.cached_note_render.clear();
        self.cached_note_render_dirty = true;
    }
}

impl SelectionState {
    fn clear_note_selection(&mut self) {
        self.selected_note_id = None;
        self.selected_note_ids.clear();
    }

    fn clear_event_selection(&mut self) {
        self.selected_event_id = None;
        self.selected_event_ids.clear();
    }

    fn clear_interactions(&mut self) {
        self.drag_state = None;
        self.multi_drag_state = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
        self.event_overlap_cycle = None;
        self.event_hover_hint = None;
        self.box_select = None;
    }

    fn prepare_for_paste_mode(&mut self) {
        self.place_note_type = None;
        self.place_event_type = None;
        self.pending_hold = None;
        self.pending_skyarea = None;
        self.drag_state = None;
        self.multi_drag_state = None;
        self.overlap_cycle = None;
        self.hover_overlap_hint = None;
    }
}

impl ClipboardManager {
    fn is_empty(&self) -> bool {
        self.clipboard.is_empty()
    }

    fn set_notes(&mut self, mut notes: Vec<GroundNote>) {
        notes.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));
        self.clipboard = notes;
        self.version = self.version.wrapping_add(1);
    }

    fn notes(&self) -> &[GroundNote] {
        &self.clipboard
    }

    fn notes_cloned(&self) -> Vec<GroundNote> {
        self.clipboard.clone()
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn paste_mode(&self) -> Option<PasteMode> {
        self.paste_mode
    }

    fn set_paste_mode(&mut self, mode: PasteMode) {
        self.paste_mode = Some(mode);
    }

    fn clear_paste_mode(&mut self) {
        self.paste_mode = None;
    }
}
