impl GroundEditor {
    fn handle_pointer(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        timeline_rect: egui::Rect,
        wave_rect: egui::Rect,
        lane_rect: egui::Rect,
        timeline_duration_sec: f32,
        actions: &mut Vec<GroundEditorAction>,
    ) {
        let pointer = response.interact_pointer_pos();
        let modifiers = ui.input(|input| input.modifiers);

        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pointer_pos) = pointer {
                if wave_rect.contains(pointer_pos) {
                    let seek = self.pointer_to_time(pointer_pos.x, timeline_rect, timeline_duration_sec);
                    actions.push(GroundEditorAction::SeekTo(seek));
                } else if lane_rect.contains(pointer_pos) {
                    if let Some(hit_note) = self.hit_test(pointer_pos, lane_rect, timeline_rect) {
                        self.selected_note_id = Some(hit_note);
                        if let Some(note) = self.notes.iter().find(|note| note.id == hit_note) {
                            self.drag_state = Some(DragState {
                                note_id: note.id,
                                pointer_origin: pointer_pos,
                                note_origin_time_ms: note.time_ms,
                                note_origin_lane: note.lane,
                            });
                        }
                    } else {
                        self.create_note(pointer_pos, lane_rect, timeline_rect, timeline_duration_sec, modifiers.shift);
                    }
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Secondary) {
            if let Some(pointer_pos) = pointer {
                if lane_rect.contains(pointer_pos) {
                    if let Some(hit_note) = self.hit_test(pointer_pos, lane_rect, timeline_rect) {
                        self.notes.retain(|note| note.id != hit_note);
                        if self.selected_note_id == Some(hit_note) {
                            self.selected_note_id = None;
                        }
                        self.drag_state = None;
                        self.status_message = "note deleted".to_owned();
                    }
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            if let (Some(pointer_pos), Some(drag)) = (pointer, self.drag_state.clone()) {
                let delta_x = pointer_pos.x - drag.pointer_origin.x;
                let delta_time_ms = (delta_x / self.pixels_per_second * 1000.0) as f64;
                let new_time = self.snap_time((drag.note_origin_time_ms + delta_time_ms).max(0.0));
                let lane_delta = ((pointer_pos.y - drag.pointer_origin.y) / (LANE_HEIGHT + LANE_GAP))
                    .round() as i32;
                let new_lane = (drag.note_origin_lane as i32 + lane_delta)
                    .clamp(0, (LANE_COUNT as i32) - 1) as usize;

                if let Some(note) = self.notes.iter_mut().find(|note| note.id == drag.note_id) {
                    note.time_ms = new_time;
                    note.lane = new_lane;
                    self.status_message =
                        format!("dragging: lane={} time={:.0}ms", note.lane, note.time_ms);
                }
            } else if let Some(pointer_pos) = pointer {
                if wave_rect.contains(pointer_pos) {
                    let seek = self.pointer_to_time(pointer_pos.x, timeline_rect, timeline_duration_sec);
                    actions.push(GroundEditorAction::SeekTo(seek));
                }
            }
        }

        if response.drag_stopped() {
            self.drag_state = None;
            self.sort_notes();
        }
    }



}

