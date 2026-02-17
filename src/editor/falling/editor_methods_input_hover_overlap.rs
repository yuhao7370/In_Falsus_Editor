// 文件说明：重叠音符悬停提示逻辑。
// 主要功能：计算悬停候选并更新重叠切换提示状态。
impl FallingGroundEditor {
    fn update_hover_overlap_hint(
        &mut self,
        ground_rect: Option<Rect>,
        air_rect: Option<Rect>,
        current_ms: f32,
    ) {
        let (mx, my) = mouse_position();
        let (scope, candidates) = self.collect_hit_candidates(mx, my, ground_rect, air_rect, current_ms);
        if candidates.len() <= 1 {
            self.hover_overlap_hint = None;
            return;
        }

        let ordered_items: Vec<HitSignatureItem> = candidates.iter().map(hit_signature_item).collect();
        let signature = canonical_hit_signature(&ordered_items);
        let (anchor_x, anchor_y) = quantize_overlap_anchor(mx, my);
        let mut current_index = 0_usize;
        if let Some(cycle) = &self.overlap_cycle {
            if cycle.scope == scope
                && cycle.anchor_x == anchor_x
                && cycle.anchor_y == anchor_y
                && cycle.signature == signature
            {
                current_index = ordered_items
                    .iter()
                    .position(|item| *item == cycle.selected_item)
                    .unwrap_or_else(|| cycle.current_index.min(candidates.len().saturating_sub(1)));
            }
        }

        self.hover_overlap_hint = Some(HoverOverlapHint {
            mouse_x: mx,
            mouse_y: my,
            current_index,
            total: candidates.len(),
        });
    }

    fn draw_overlap_hint(&self) {
        let Some(hint) = self.hover_overlap_hint else {
            return;
        };
        if hint.total <= 1 {
            return;
        }

        let text = format!("{}/{}", hint.current_index + 1, hint.total);
        let ui = adaptive_ui_scale();
        let font_size = scaled_font_size(18.0, 12, 42);
        let metrics = measure_text(&text, None, font_size, 1.0);
        let box_w = metrics.width + 14.0 * ui;
        let box_h = 24.0 * ui;
        let x = (hint.mouse_x + 14.0 * ui).clamp(4.0 * ui, screen_width() - box_w - 4.0 * ui);
        let y = (hint.mouse_y - box_h - 10.0 * ui).clamp(4.0 * ui, screen_height() - box_h - 4.0 * ui);

        draw_rectangle(x, y, box_w, box_h, Color::from_rgba(20, 24, 34, 214));
        draw_rectangle_lines(x, y, box_w, box_h, 1.0, Color::from_rgba(140, 156, 198, 220));
        draw_text_ex(
            &text,
            x + 7.0 * ui,
            y + 17.0 * ui,
            TextParams {
                font_size,
                color: Color::from_rgba(228, 234, 248, 255),
                ..Default::default()
            },
        );
    }



}

