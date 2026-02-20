// 文件说明：编辑器每帧绘制与输入分发的主入口实现。
// 主要功能：负责布局计算、视图绘制顺序和交互调用编排。

impl FallingGroundEditor {
    pub fn draw(&mut self, area: Rect, frame_ctx: &FrameContext) -> Vec<FallingEditorAction> {
        let current_sec = frame_ctx.current_sec;
        let audio_duration_sec = frame_ctx.duration_sec;
        let is_playing = frame_ctx.is_playing;
        self.sync_waveform(frame_ctx.track_path.as_deref());
        let mut actions = Vec::new();

        let header_h = 0.0;
        let footer_h = 0.0;
        let content_rect = Rect::new(
            area.x + 8.0,
            area.y + header_h + 6.0,
            (area.w - 16.0).max(40.0),
            (area.h - header_h - footer_h - 10.0).max(40.0),
        );
        let (left_screen, right_screen) = self.split_portrait_screens(content_rect);
        let minimap_screen = if self.view.show_minimap {
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

        draw_rectangle(
            area.x,
            area.y,
            area.w,
            area.h,
            Color::from_rgba(10, 10, 12, 255),
        );
        draw_rectangle_lines(
            area.x,
            area.y,
            area.w,
            area.h,
            1.0,
            Color::from_rgba(44, 44, 52, 255),
        );

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

        let duration_sec = self.estimate_duration(audio_duration_sec).max(0.001);
        let mut render_current_sec = if self.view.waveform_seek_active {
            self.view.waveform_seek_sec.clamp(0.0, duration_sec)
        } else {
            current_sec
        };
        if let Some(target_sec) = self.view.minimap_drag_target_sec {
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
            self.view.minimap_drag_active = false;
            self.view.minimap_drag_offset_ms = 0.0;
            self.view.minimap_drag_target_sec = None;
            self.view.minimap_last_emit_sec = None;
        }
        if let Some(target_sec) = self.view.minimap_drag_target_sec {
            render_current_sec = target_sec.clamp(0.0, duration_sec);
            current_ms = render_current_sec * 1000.0;
        }
        let (ground_rect, air_rect, event_rect) = match self.view.render_scope {
            RenderScope::Both => {
                self.draw_event_view(left_inner, current_ms);
                (Some(lanes_rect), Some(lanes_rect), Some(left_inner))
            }
            RenderScope::Split => (Some(left_inner), Some(lanes_rect), None),
        };

        let allow_editor_input = !self.view.minimap_drag_active;
        // Delete key: remove selected note or event
        // (keyboard is already blocked when egui text input has focus or popup is open)
        if safe_key_pressed(KeyCode::Delete) {
            let is_chart_header_event = |event: &TimelineEvent| {
                event.kind == TimelineEventKind::Bpm && event.label.starts_with("chart ")
            };

            if !self.selection.selected_note_ids.is_empty() {
                self.selection.editing_note_backup = None;
                self.snapshot_for_undo();
                let ids = self.selection.selected_note_ids.clone();
                let count = ids.len();
                self.editor_state.notes.retain(|n| !ids.contains(&n.id));
                self.selection.clear_note_selection();
                self.selection.drag_state = None;
                self.selection.overlap_cycle = None;
                self.selection.hover_overlap_hint = None;
                self.status = format!("{} note(s) deleted", count);
            } else if let Some(note_id) = self.selection.selected_note_id.take() {
                self.selection.editing_note_backup = None;
                self.snapshot_for_undo();
                self.editor_state.notes.retain(|n| n.id != note_id);
                self.selection.drag_state = None;
                self.selection.overlap_cycle = None;
                self.selection.hover_overlap_hint = None;
                self.status = format!("note {} deleted", note_id);
            } else if !self.selection.selected_event_ids.is_empty() {
                let ids = self.selection.selected_event_ids.clone();
                let has_chart_header = self
                    .editor_state
                    .timeline_events
                    .iter()
                    .any(|event| ids.contains(&event.id) && is_chart_header_event(event));
                let deletable_count = self
                    .editor_state
                    .timeline_events
                    .iter()
                    .filter(|event| ids.contains(&event.id) && !is_chart_header_event(event))
                    .count();

                if deletable_count > 0 {
                    self.selection.editing_event_backup = None;
                    self.snapshot_for_undo();
                    self.editor_state
                        .timeline_events
                        .retain(|event| !ids.contains(&event.id) || is_chart_header_event(event));
                    self.rebuild_bpm_timeline_from_events();
                    self.rebuild_track_source_from_events();
                    self.selection.clear_event_selection();
                    self.selection.event_overlap_cycle = None;
                    self.selection.event_hover_hint = None;
                    self.status = if has_chart_header {
                        format!("{} event(s) deleted (chart header kept)", deletable_count)
                    } else {
                        format!("{} event(s) deleted", deletable_count)
                    };
                } else if has_chart_header {
                    self.status = "chart header cannot be deleted".to_owned();
                }
            } else if let Some(event_id) = self.selection.selected_event_id {
                if self
                    .editor_state
                    .timeline_events
                    .iter()
                    .any(|event| event.id == event_id && is_chart_header_event(event))
                {
                    self.status = "chart header cannot be deleted".to_owned();
                } else {
                    self.selection.selected_event_id = None;
                    self.selection.editing_event_backup = None;
                    self.snapshot_for_undo();
                    self.editor_state
                        .timeline_events
                        .retain(|event| event.id != event_id);
                    self.rebuild_bpm_timeline_from_events();
                    self.rebuild_track_source_from_events();
                    self.selection.event_overlap_cycle = None;
                    self.selection.event_hover_hint = None;
                    self.status = format!("event {} deleted", event_id);
                }
            }
        }

        // ── 剪切/复制/粘贴/镜像 快捷键 ──
        let ctrl_held = safe_key_down(KeyCode::LeftControl) || safe_key_down(KeyCode::RightControl);
        if ctrl_held && self.clipboard.paste_mode().is_none() {
            if safe_key_pressed(KeyCode::C) {
                self.copy_selected_to_clipboard();
            } else if safe_key_pressed(KeyCode::X) {
                self.cut_selected_to_clipboard();
            } else if safe_key_pressed(KeyCode::V) {
                self.enter_paste_mode(PasteMode::Normal);
            } else if safe_key_pressed(KeyCode::B) {
                // Ctrl+B：有选中 → 原地镜像（不复制），无选中 → 镜像粘贴
                if !self.selection.selected_note_ids.is_empty()
                    || self.selection.selected_note_id.is_some()
                {
                    self.mirror_selected_notes();
                } else {
                    self.enter_paste_mode(PasteMode::Mirrored);
                }
            } else if safe_key_pressed(KeyCode::M) {
                // Ctrl+M：复制并镜像
                self.mirror_selected_in_place();
            }
        } else if ctrl_held && self.clipboard.paste_mode().is_some() {
            // 在粘贴模式中也允许切换粘贴类型
            if safe_key_pressed(KeyCode::V) {
                self.clipboard.set_paste_mode(PasteMode::Normal);
                self.status = self
                    .i18n
                    .t(crate::i18n::TextKey::EditorPasteModeNormal)
                    .to_owned();
            } else if safe_key_pressed(KeyCode::B) {
                self.clipboard.set_paste_mode(PasteMode::Mirrored);
                self.status = self
                    .i18n
                    .t(crate::i18n::TextKey::EditorPasteModeMirrored)
                    .to_owned();
            }
        }

        // ── 粘贴模式处理 ──
        if self.clipboard.paste_mode().is_some() {
            self.handle_paste_input(ground_rect, air_rect, current_ms);
        }

        if allow_editor_input && self.clipboard.paste_mode().is_none() {
            if safe_mouse_button_pressed(MouseButton::Right)
                && (self.selection.place_note_type.is_some()
                    || self.selection.place_event_type.is_some()
                    || self.selection.pending_hold.is_some()
                    || self.selection.pending_skyarea.is_some())
            {
                self.selection.place_note_type = None;
                self.selection.place_event_type = None;
                self.selection.pending_hold = None;
                self.selection.pending_skyarea = None;
                self.selection.drag_state = None;
                self.selection.overlap_cycle = None;
                self.selection.hover_overlap_hint = None;
                self.selection.clear_event_selection();
                self.selection.event_overlap_cycle = None;
                self.selection.event_hover_hint = None;
                self.status = self
                    .i18n
                    .t(crate::i18n::TextKey::EditorPlaceModeCleared)
                    .to_owned();
            }

            if self.selection.place_note_type.is_none() && self.selection.place_event_type.is_none()
            {
                self.handle_note_selection_click(ground_rect, air_rect, current_ms);
                self.update_hover_overlap_hint(ground_rect, air_rect, current_ms);
            } else {
                self.selection.overlap_cycle = None;
                self.selection.hover_overlap_hint = None;
            }
        } else {
            self.selection.drag_state = None;
            self.selection.overlap_cycle = None;
            self.selection.hover_overlap_hint = None;
        }

        if allow_editor_input {
            // 框选优先：Alt+左键 启动/更新/结束框选
            self.handle_box_select(ground_rect, air_rect, event_rect, current_ms);

            if self.selection.box_select.is_none() {
                if let Some(rect) = ground_rect {
                    self.handle_ground_input(rect, current_ms);
                }
                if let Some(rect) = air_rect {
                    self.handle_air_input(rect, current_ms);
                }
            }
        }

        let spectrum_ok = self.view.show_spectrum && !self.editor_state.track_speed_enabled;
        match self.view.render_scope {
            RenderScope::Both => {
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, spectrum_ok);
                }
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, true, false);
                }
            }
            RenderScope::Split => {
                if let Some(rect) = air_rect {
                    self.draw_air_view(rect, current_ms, false, spectrum_ok);
                }
                if let Some(rect) = ground_rect {
                    self.draw_ground_view(rect, current_ms, spectrum_ok);
                }
            }
        }

        let (mx, my) = safe_mouse_position();
        let using_note_cursor = if allow_editor_input {
            match self.selection.place_note_type {
                Some(tool) if is_ground_tool(tool) => ground_rect
                    .map(|r| point_in_rect(mx, my, r))
                    .unwrap_or(false),
                Some(tool) if is_air_tool(tool) => air_rect
                    .map(|r| point_in_rect(mx, my, air_split_rect(r)))
                    .unwrap_or(false),
                _ => false,
            }
        } else {
            false
        };
        if using_note_cursor {
            match self.selection.place_note_type {
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
        self.draw_box_select_overlay();

        // 绘制粘贴预览
        if self.clipboard.paste_mode().is_some() {
            self.draw_paste_preview(ground_rect, air_rect, current_ms);
        }

        actions
    }
}
