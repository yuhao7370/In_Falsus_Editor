impl GroundEditor {
    fn draw_second_grid(&self, painter: &egui::Painter, rect: egui::Rect, duration_sec: f32) {
        let seconds = duration_sec.ceil() as i32;
        for second in 0..=seconds {
            let x = rect.left() + second as f32 * self.pixels_per_second;
            let strong = second % 5 == 0;
            let color = if strong {
                egui::Color32::from_rgb(50, 50, 50)
            } else {
                egui::Color32::from_rgb(28, 28, 28)
            };
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1.0, color),
            );
        }
    }

    fn draw_waveform(&self, painter: &egui::Painter, rect: egui::Rect) {
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(12, 14, 18));
        painter.line_segment(
            [
                egui::pos2(rect.left(), rect.center().y),
                egui::pos2(rect.right(), rect.center().y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 52)),
        );

        let Some(waveform) = &self.waveform else {
            return;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 {
            return;
        }

        let peak_count = waveform.peaks.len();
        for (index, peak) in waveform.peaks.iter().enumerate() {
            let t = index as f32 / (peak_count - 1) as f32;
            let x = rect.left() + t * waveform.duration_sec * self.pixels_per_second;
            if x < rect.left() || x > rect.right() {
                continue;
            }

            let amp = (peak * (rect.height() * 0.45)).max(1.0);
            painter.line_segment(
                [
                    egui::pos2(x, rect.center().y - amp),
                    egui::pos2(x, rect.center().y + amp),
                ],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(86, 171, 255)),
            );
        }
    }

    fn draw_lanes(&self, painter: &egui::Painter, lane_rect: egui::Rect) {
        for lane in 0..LANE_COUNT {
            let top = lane_rect.top() + lane as f32 * (LANE_HEIGHT + LANE_GAP);
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(lane_rect.left(), top),
                egui::pos2(lane_rect.right(), top + LANE_HEIGHT),
            );
            let bg = if lane % 2 == 0 {
                egui::Color32::from_rgb(15, 15, 15)
            } else {
                egui::Color32::from_rgb(18, 18, 18)
            };
            painter.rect_filled(row_rect, 0.0, bg);

            painter.text(
                egui::pos2(row_rect.left() + 6.0, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!("L{lane}"),
                egui::FontId::proportional(12.0),
                egui::Color32::from_rgb(160, 160, 160),
            );
        }
    }

    fn draw_notes(&self, painter: &egui::Painter, lane_rect: egui::Rect) {
        for note in &self.notes {
            let Some(rect) = self.note_rect(note, lane_rect) else {
                continue;
            };

            let is_selected = self.selected_note_id == Some(note.id);
            let base_color = match note.kind() {
                NoteKind::Tap => egui::Color32::from_rgb(76, 185, 255),
                NoteKind::Hold => egui::Color32::from_rgb(120, 220, 120),
            };
            let color = if is_selected {
                egui::Color32::from_rgb(255, 206, 86)
            } else {
                base_color
            };

            painter.rect_filled(rect, 3.0, color);
            if note.kind() == NoteKind::Hold {
                painter.line_segment(
                    [
                        egui::pos2(rect.left(), rect.center().y),
                        egui::pos2(rect.right(), rect.center().y),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(36, 80, 36)),
                );
            }
        }
    }

    fn draw_playhead(&self, painter: &egui::Painter, rect: egui::Rect, playhead_sec: f32, duration_sec: f32) {
        if duration_sec <= 0.0 {
            return;
        }
        let x = rect.left() + playhead_sec.max(0.0) * self.pixels_per_second;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 106, 106)),
        );
    }



}

