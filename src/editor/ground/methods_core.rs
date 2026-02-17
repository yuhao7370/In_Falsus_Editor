impl GroundEditor {
    pub fn new() -> Self {
        let mut editor = Self {
            notes: Vec::new(),
            next_note_id: 1,
            selected_note_id: None,
            drag_state: None,
            chart_path: DEFAULT_CHART_PATH.to_owned(),
            status_message: String::new(),
            base_bpm: 120.0,
            pixels_per_second: 120.0,
            snap_enabled: true,
            snap_division: 4,
            waveform: None,
            waveform_error: None,
            last_audio_path: None,
        };
        editor.load_chart(DEFAULT_CHART_PATH);
        editor
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        playhead_sec: f32,
        audio_duration_sec: f32,
        audio_path: Option<&str>,
    ) -> Vec<GroundEditorAction> {
        self.sync_waveform(audio_path);
        let mut actions = Vec::new();

        ui.horizontal(|ui| {
            ui.strong("6K Ground Editor");
            ui.separator();
            ui.label(format!("Chart: {}", self.chart_path));
            ui.separator();
            ui.label(format!("Notes: {}", self.notes.len()));
            ui.separator();
            ui.label(format!("BPM: {:.2}", self.base_bpm));
        });

        ui.horizontal(|ui| {
            ui.add(
                egui::Slider::new(&mut self.pixels_per_second, 40.0..=320.0)
                    .text("Zoom(px/s)"),
            );
            ui.checkbox(&mut self.snap_enabled, "Snap");

            egui::ComboBox::from_id_salt("snap_division")
                .selected_text(format!("1/{0}", self.snap_division))
                .show_ui(ui, |ui| {
                    for division in [1_u32, 2, 4, 8, 16] {
                        ui.selectable_value(
                            &mut self.snap_division,
                            division,
                            format!("1/{division}"),
                        );
                    }
                });

            ui.separator();
            ui.label("LMB empty: add Tap");
            ui.label("Shift+LMB: add Hold");
            ui.label("LMB drag: move note");
            ui.label("RMB: delete note");
        });

        if !self.status_message.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(176, 214, 255), &self.status_message);
        }
        if let Some(error) = &self.waveform_error {
            ui.colored_label(egui::Color32::from_rgb(255, 128, 128), error);
        }

        let timeline_duration_sec = self
            .max_note_end_sec()
            .max(audio_duration_sec.max(0.0))
            .max(8.0);
        let timeline_width = (timeline_duration_sec * self.pixels_per_second).max(ui.available_width());
        let lanes_total_height = (LANE_HEIGHT + LANE_GAP) * LANE_COUNT as f32;
        let timeline_height = WAVEFORM_HEIGHT + 18.0 + lanes_total_height;

        egui::ScrollArea::both().id_salt("ground_editor_scroll").show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(timeline_width, timeline_height),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(rect);

            let wave_rect = egui::Rect::from_min_max(
                rect.min,
                egui::pos2(rect.right(), rect.top() + WAVEFORM_HEIGHT),
            );
            let lane_rect = egui::Rect::from_min_max(
                egui::pos2(rect.left(), wave_rect.bottom() + 18.0),
                rect.max,
            );

            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(8, 8, 8));
            self.draw_second_grid(&painter, rect, timeline_duration_sec);
            self.draw_waveform(&painter, wave_rect);
            self.draw_lanes(&painter, lane_rect);
            self.draw_notes(&painter, lane_rect);
            self.draw_playhead(&painter, rect, playhead_sec, timeline_duration_sec);

            self.handle_pointer(
                ui,
                &response,
                rect,
                wave_rect,
                lane_rect,
                timeline_duration_sec,
                &mut actions,
            );
        });

        actions
    }



}

