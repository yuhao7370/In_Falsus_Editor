// 文件说明：编辑器每帧绘制与输入分发的主入口实现。
// 主要功能：负责布局计算、视图绘制顺序和交互调用编排。
impl FallingGroundEditor {
    pub fn draw(
        &mut self,
        area: Rect,
        current_sec: f32,
        audio_duration_sec: f32,
        audio_path: Option<&str>,
        is_playing: bool,
    ) -> Vec<FallingEditorAction> {
        self.sync_waveform(audio_path);
        let mut actions = Vec::new();

        let header_h = 34.0;
        let footer_h = 22.0;
        let header_rect = Rect::new(area.x, area.y, area.w, header_h);
        let content_rect = Rect::new(
            area.x + 8.0,
            area.y + header_h + 6.0,
            (area.w - 16.0).max(40.0),
            (area.h - header_h - footer_h - 10.0).max(40.0),
        );
        let (left_screen, right_screen) = self.split_portrait_screens(content_rect);
        let minimap_screen = if self.show_minimap {
            self.minimap_screen_from_left_gap(content_rect, left_screen)
        } else {
            None
        };

        let inner_rect = |screen: Rect| {
            Rect::new(
                screen.x + 8.0,
                screen.y + 8.0,
                (screen.w - 16.0).max(8.0),
                (screen.h - 16.0).max(8.0),
            )
        };
        let minimap_inner = minimap_screen.map(inner_rect);
        let left_inner = inner_rect(left_screen);
        let right_inner = inner_rect(right_screen);

        let lanes_rect = right_inner;

        draw_rectangle(area.x, area.y, area.w, area.h, Color::from_rgba(10, 10, 12, 255));
        draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, Color::from_rgba(44, 44, 52, 255));

        if let Some(screen) = minimap_screen {
            draw_rectangle(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                Color::from_rgba(12, 12, 18, 255),
            );
            draw_rectangle_lines(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                1.0,
                Color::from_rgba(56, 62, 86, 255),
            );
        }
        for screen in [left_screen, right_screen] {
            draw_rectangle(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                Color::from_rgba(12, 12, 18, 255),
            );
            draw_rectangle_lines(
                screen.x,
                screen.y,
                screen.w,
                screen.h,
                1.0,
                Color::from_rgba(56, 62, 86, 255),
            );
        }

        self.draw_header(header_rect);
        self.handle_scroll_speed_controls(header_rect);
        let duration_sec = self.estimate_duration(audio_duration_sec).max(0.001);
        let mut render_current_sec = if self.waveform_seek_active {
            self.waveform_seek_sec
                .clamp(0.0, duration_sec)
        } else {
            current_sec
        };
        if let Some(target_sec) = self.minimap_drag_target_sec {
            render_current_sec = target_sec.clamp(0.0, duration_sec);
        }
        let mut current_ms = render_current_sec * 1000.0;

        let visible_window = self.compute_visible_window_ms(lanes_rect, current_ms);
        if let Some(minimap_inner) = minimap_inner {
            let minimap_info = self.draw_minimap_view(minimap_inner, duration_sec, visible_window);
            self.handle_minimap_seek_drag(
                minimap_info,
                current_ms,
                duration_sec,
                is_playing,
                &mut actions,
            );
        } else {
            self.minimap_drag_active = false;
            self.minimap_drag_offset_ms = 0.0;
            self.minimap_drag_target_sec = None;
            self.minimap_last_emit_sec = None;
        }
        if let Some(target_sec) = self.minimap_drag_target_sec {
            render_current_sec = target_sec.clamp(0.0, duration_sec);
            current_ms = render_current_sec * 1000.0;
        }
        let (ground_rect, air_rect) = match self.render_scope {
            RenderScope::Both => {
                self.draw_event_view(left_inner, current_ms);
                (Some(lanes_rect), Some(lanes_rect))
            }
            RenderScope::Split => (Some(left_inner), Some(lanes_rect)),
        };

        let allow_editor_input = !self.minimap_drag_active;
        if allow_editor_input {
            if is_mouse_button_pressed(MouseButton::Right)
                && (self.place_note_type.is_some()
                    || self.pending_hold.is_some()
                    || self.pending_skyarea.is_some())
            {
                self.place_note_type = None;
                self.pending_hold = None;
                self.pending_skyarea = None;
                self.drag_state = None;
                self.overlap_cycle = None;
                self.hover_overlap_hint = None;
                self.status = "place mode cleared".to_owned();
            }

            if self.place_note_type.is_none() {
                self.handle_note_selection_click(ground_rect, air_rect, current_ms);
                self.update_hover_overlap_hint(ground_rect, air_rect, current_ms);
            } else {
                self.overlap_cycle = None;
                self.hover_overlap_hint = None;
            }
        } else {
            self.drag_state = None;
            self.overlap_cycle = None;
            self.hover_overlap_hint = None;
        }

        if allow_editor_input {
            if let Some(rect) = ground_rect {
                self.handle_ground_input(rect, current_ms);
            }
            if let Some(rect) = air_rect {
                self.handle_air_input(rect, current_ms);
            }
        }

        match self.render_scope {
            RenderScope::Both => {
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, true);
                }
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, true, false);
                }
            }
            RenderScope::Split => {
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, false, true);
                }
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, true);
                }
            }
        }

        let (mx, my) = mouse_position();
        let using_note_cursor = if allow_editor_input {
            match self.place_note_type {
                Some(tool) if is_ground_tool(tool) => {
                    ground_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false)
                }
                Some(tool) if is_air_tool(tool) => {
                    air_rect.map(|r| point_in_rect(mx, my, r)).unwrap_or(false)
                }
                _ => false,
            }
        } else {
            false
        };
        show_mouse(!using_note_cursor);
        if using_note_cursor {
            match self.place_note_type {
                Some(tool) if is_ground_tool(tool) => {
                    if let Some(rect) = ground_rect {
                        self.draw_place_cursor(rect, current_ms);
                    }
                }
                Some(tool) if is_air_tool(tool) => {
                    if let Some(rect) = air_rect {
                        self.draw_place_cursor(rect, current_ms);
                    }
                }
                _ => {}
            }
        }

        self.draw_overlap_hint();

        if let Some(error) = &self.waveform_error {
            draw_text_ex(
                error,
                area.x + 12.0,
                area.y + area.h - 6.0,
                TextParams {
                    font: self.text_font.as_ref(),
                    font_size: 18,
                    color: Color::from_rgba(255, 100, 100, 255),
                    ..Default::default()
                },
            );
        } else {
            draw_text_ex(
                &self.status,
                area.x + 12.0,
                area.y + area.h - 6.0,
                TextParams {
                    font: self.text_font.as_ref(),
                    font_size: 18,
                    color: Color::from_rgba(176, 210, 255, 255),
                    ..Default::default()
                },
            );
        }

        actions
    }



}

