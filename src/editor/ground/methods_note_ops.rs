impl GroundEditor {
    fn create_note(
        &mut self,
        pointer: egui::Pos2,
        lane_rect: egui::Rect,
        timeline_rect: egui::Rect,
        timeline_duration_sec: f32,
        make_hold: bool,
    ) {
        let lane = self.pointer_to_lane(pointer.y, lane_rect);
        let time_sec = self.pointer_to_time(pointer.x, timeline_rect, timeline_duration_sec);
        let time_ms = self.snap_time(time_sec as f64 * 1000.0);
        let duration_ms = if make_hold { DEFAULT_HOLD_MS } else { 0.0 };

        let note = GroundNote {
            id: self.next_note_id,
            lane,
            time_ms,
            duration_ms,
        };
        self.next_note_id += 1;
        self.selected_note_id = Some(note.id);
        self.notes.push(note);
        self.sort_notes();
        self.status_message = if make_hold {
            format!("new Hold: lane={lane}, time={time_ms:.0}ms")
        } else {
            format!("new Tap: lane={lane}, time={time_ms:.0}ms")
        };
    }

    fn pointer_to_time(&self, x: f32, timeline_rect: egui::Rect, duration_sec: f32) -> f32 {
        let time_sec = (x - timeline_rect.left()) / self.pixels_per_second;
        time_sec.clamp(0.0, duration_sec.max(0.0))
    }

    fn pointer_to_lane(&self, y: f32, lane_rect: egui::Rect) -> usize {
        let lane = ((y - lane_rect.top()) / (LANE_HEIGHT + LANE_GAP)).floor() as i32;
        lane.clamp(0, (LANE_COUNT as i32) - 1) as usize
    }

    fn hit_test(
        &self,
        pointer: egui::Pos2,
        lane_rect: egui::Rect,
        timeline_rect: egui::Rect,
    ) -> Option<u64> {
        for note in self.notes.iter().rev() {
            let Some(rect) = self.note_rect(note, lane_rect) else {
                continue;
            };
            if rect.contains(pointer) {
                return Some(note.id);
            }

            if note.kind() == NoteKind::Tap {
                let center = egui::pos2(
                    timeline_rect.left() + (note.time_ms as f32 / 1000.0) * self.pixels_per_second,
                    rect.center().y,
                );
                if center.distance(pointer) <= 8.0 {
                    return Some(note.id);
                }
            }
        }
        None
    }

    fn note_rect(&self, note: &GroundNote, lane_rect: egui::Rect) -> Option<egui::Rect> {
        if note.lane >= LANE_COUNT {
            return None;
        }
        let lane_top = lane_rect.top() + note.lane as f32 * (LANE_HEIGHT + LANE_GAP);
        let y1 = lane_top + 4.0;
        let y2 = lane_top + LANE_HEIGHT - 4.0;
        let x1 = lane_rect.left() + (note.time_ms as f32 / 1000.0) * self.pixels_per_second;
        let x2 = if note.kind() == NoteKind::Hold {
            lane_rect.left() + (note.end_ms() as f32 / 1000.0) * self.pixels_per_second
        } else {
            x1 + 8.0
        };
        Some(egui::Rect::from_min_max(
            egui::pos2(x1, y1),
            egui::pos2((x2).max(x1 + 6.0), y2),
        ))
    }

    fn snap_time(&self, time_ms: f64) -> f64 {
        if !self.snap_enabled {
            return time_ms.max(0.0);
        }
        let bpm = self.base_bpm.max(1e-6);
        let division = self.snap_division.max(1) as f64;
        let step = (60000.0 / bpm) / division;
        if step <= 0.0 {
            return time_ms.max(0.0);
        }
        (time_ms / step).round() * step
    }

    fn sort_notes(&mut self) {
        self.notes.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| a.lane.cmp(&b.lane))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    fn max_note_end_sec(&self) -> f32 {
        self.notes
            .iter()
            .map(|note| note.end_ms() as f32 / 1000.0)
            .fold(0.0, f32::max)
    }

    fn sync_waveform(&mut self, audio_path: Option<&str>) {
        let Some(path) = audio_path else {
            return;
        };
        let changed = self
            .last_audio_path
            .as_ref()
            .map(|last| last != path)
            .unwrap_or(true);
        if !changed {
            return;
        }

        self.last_audio_path = Some(path.to_owned());
        match WaveformData::from_audio_file(path, 4096) {
            Ok(data) => {
                self.waveform = Some(data);
                self.waveform_error = None;
                self.status_message = format!("waveform loaded: {path}");
            }
            Err(err) => {
                self.waveform = None;
                self.waveform_error = Some(err);
            }
        }
    }



}

