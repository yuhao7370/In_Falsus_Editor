impl GroundEditor {
    fn load_chart(&mut self, path: &str) {
        self.chart_path = path.to_owned();
        match Chart::from_file(path) {
            Ok(chart) => {
                self.base_bpm = chart.chart_info().map(|(bpm, _)| bpm).unwrap_or(120.0);
                self.notes = Self::extract_ground_notes(&chart);
                self.next_note_id = self
                    .notes
                    .iter()
                    .map(|note| note.id)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                self.status_message =
                    format!("chart loaded: {} ground notes", self.notes.len());
            }
            Err(err) => {
                self.notes.clear();
                self.next_note_id = 1;
                self.status_message = format!("failed to read chart: {err}");
            }
        }
    }

    fn extract_ground_notes(chart: &Chart) -> Vec<GroundNote> {
        let mut notes = Vec::new();
        let mut next_note_id = 1_u64;
        for event in &chart.events {
            match event {
                ChartEvent::Tap { time, lane, .. } => {
                    if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                        notes.push(GroundNote {
                            id: next_note_id,
                            lane: *lane as usize,
                            time_ms: *time,
                            duration_ms: 0.0,
                        });
                        next_note_id += 1;
                    }
                }
                ChartEvent::Hold {
                    time,
                    lane,
                    duration,
                    ..
                } => {
                    if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                        notes.push(GroundNote {
                            id: next_note_id,
                            lane: *lane as usize,
                            time_ms: *time,
                            duration_ms: (*duration).max(1.0),
                        });
                        next_note_id += 1;
                    }
                }
                _ => {}
            }
        }
        notes.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));
        notes
    }


}

