// 文件说明：小地图交互输入处理实现。
// 主要功能：处理小地图拖拽、点击跳转和同步状态更新。
impl FallingGroundEditor {
    fn handle_minimap_seek_drag(
        &mut self,
        minimap: MinimapRenderInfo,
        render_current_ms: f32,
        duration_sec: f32,
        is_playing: bool,
        actions: &mut Vec<FallingEditorAction>,
    ) {
        let duration_ms = duration_sec.max(0.001) * 1000.0;
        if minimap.content_rect.w <= 2.0 || minimap.content_rect.h <= 2.0 {
            self.view.minimap_drag_active = false;
            self.view.minimap_drag_target_sec = None;
            self.view.minimap_last_emit_sec = None;
            return;
        }

        let (mx, my) = safe_mouse_position();
        let ui = adaptive_ui_scale();
        let min_hit_h = (26.0 * ui).max(minimap.highlight_rect.h);
        let cy = minimap.highlight_rect.y + minimap.highlight_rect.h * 0.5;
        let hit_top = (cy - min_hit_h * 0.5).max(minimap.content_rect.y);
        let hit_bottom =
            (cy + min_hit_h * 0.5).min(minimap.content_rect.y + minimap.content_rect.h);
        let hit_pad_x = (8.0 * ui).max(4.0);
        let hit_rect = Rect::new(
            minimap.content_rect.x - hit_pad_x,
            hit_top,
            minimap.content_rect.w + hit_pad_x * 2.0,
            (hit_bottom - hit_top).max(1.0),
        );
        let inside_highlight = point_in_rect(mx, my, hit_rect);

        if safe_mouse_button_pressed(MouseButton::Left) && inside_highlight {
            if is_playing {
                self.status = "pause to scrub minimap".to_owned();
                return;
            }
            self.selection.drag_state = None;
            self.view.waveform_seek_active = false;
            self.view.minimap_drag_active = true;
            let mouse_ms = self.minimap_segment_y_to_time(
                my,
                minimap.content_rect,
                minimap.seek_start_ms,
                minimap.seek_end_ms,
            );
            self.view.minimap_drag_offset_ms = render_current_ms - mouse_ms;
            self.view.minimap_last_emit_sec = None;
        }

        if !self.view.minimap_drag_active {
            return;
        }

        if is_playing {
            self.view.minimap_drag_active = false;
            self.view.minimap_drag_target_sec = None;
            self.view.minimap_last_emit_sec = None;
            self.status = "pause to scrub minimap".to_owned();
            return;
        }

        if safe_mouse_button_down(MouseButton::Left) {
            let mouse_ms = self.minimap_segment_y_to_time(
                my,
                minimap.content_rect,
                minimap.seek_start_ms,
                minimap.seek_end_ms,
            );
            let target_ms = (mouse_ms + self.view.minimap_drag_offset_ms).clamp(0.0, duration_ms);
            let target_sec = target_ms / 1000.0;
            self.view.minimap_drag_target_sec = Some(target_sec);
            self.view.waveform_seek_sec = target_sec;

            let should_emit = self
                .view
                .minimap_last_emit_sec
                .map(|last| (last - target_sec).abs() >= MINIMAP_DRAG_EMIT_EPS_SEC)
                .unwrap_or(true);
            if should_emit {
                actions.push(FallingEditorAction::MinimapSeekTo(target_sec));
                self.view.minimap_last_emit_sec = Some(target_sec);
                self.status = format!("minimap seek {:.2}s", target_sec);
            }
        } else {
            self.view.minimap_drag_active = false;
            self.view.minimap_drag_offset_ms = 0.0;
            self.view.minimap_last_emit_sec = None;
            self.view.minimap_drag_target_sec = None;
        }
    }
}
