// 文件说明：小地图视图绘制实现。
// 主要功能：绘制全曲概览、可视窗口和小地图音符分布。
impl FallingGroundEditor {
    fn draw_minimap_view(
        &self,
        rect: Rect,
        duration_sec: f32,
        visible: TimeWindowMs,
    ) -> MinimapRenderInfo {
        let duration_ms = duration_sec.max(0.001) * 1000.0;
        if rect.h <= 6.0 || rect.w <= 6.0 {
            return MinimapRenderInfo {
                content_rect: rect,
                highlight_rect: rect,
                seek_start_ms: 0.0,
                seek_end_ms: duration_ms,
            };
        }

        let ui = adaptive_ui_scale();
        let title_h = 20.0 * ui;
        let pad = 6.0 * ui;

        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::from_rgba(8, 10, 17, 255),
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(42, 58, 90, 255),
        );
        draw_text_ex(
            "MINIMAP",
            rect.x + 8.0 * ui,
            rect.y + 16.0 * ui,
            TextParams {
                font: self.view.text_font.as_ref(),
                font_size: scaled_font_size(16.0, 11, 36),
                color: Color::from_rgba(210, 222, 246, 255),
                ..Default::default()
            },
        );

        let content = Rect::new(
            rect.x + pad,
            rect.y + title_h,
            (rect.w - pad * 2.0).max(4.0),
            (rect.h - title_h - pad).max(4.0),
        );
        draw_rectangle(
            content.x,
            content.y,
            content.w,
            content.h,
            Color::from_rgba(10, 14, 24, 255),
        );
        draw_rectangle_lines(
            content.x,
            content.y,
            content.w,
            content.h,
            1.0,
            Color::from_rgba(52, 72, 108, 255),
        );

        if content.h <= 2.0 || content.w <= 2.0 {
            return MinimapRenderInfo {
                content_rect: content,
                highlight_rect: content,
                seek_start_ms: 0.0,
                seek_end_ms: duration_ms,
            };
        }

        let half_ms = duration_ms * 0.5;
        let pair_gap = (2.0 * ui).clamp(1.0, 5.0);
        let group_gap = (5.0 * ui).clamp(3.0, 10.0);
        let total_gap = pair_gap * 2.0 + group_gap;
        let col_w = ((content.w - total_gap) / 4.0).max(2.0);

        let ground_rect_1 = Rect::new(content.x, content.y, col_w, content.h);
        let sky_rect_1 = Rect::new(
            ground_rect_1.x + col_w + pair_gap,
            content.y,
            col_w,
            content.h,
        );
        let ground_rect_2 = Rect::new(
            sky_rect_1.x + col_w + group_gap,
            content.y,
            col_w,
            content.h,
        );
        let sky_rect_2 = Rect::new(
            ground_rect_2.x + col_w + pair_gap,
            content.y,
            col_w,
            content.h,
        );

        let left_group_rect = Rect::new(
            ground_rect_1.x,
            ground_rect_1.y,
            (sky_rect_1.x + sky_rect_1.w - ground_rect_1.x).max(1.0),
            ground_rect_1.h,
        );
        let right_group_rect = Rect::new(
            ground_rect_2.x,
            ground_rect_2.y,
            (sky_rect_2.x + sky_rect_2.w - ground_rect_2.x).max(1.0),
            ground_rect_2.h,
        );
        let active_right = visible.current_ms >= half_ms;
        let (active_group_rect, active_start_ms, active_end_ms) = if active_right {
            (right_group_rect, half_ms, duration_ms)
        } else {
            (left_group_rect, 0.0, half_ms)
        };

        let layout = MinimapDrawLayout {
            duration_ms,
            half_ms,
            ui,
            ground_rect_1,
            sky_rect_1,
            ground_rect_2,
            sky_rect_2,
            left_group_rect,
            right_group_rect,
            active_group_rect,
            active_start_ms,
            active_end_ms,
        };

        self.draw_minimap_page_columns(layout);
        self.draw_minimap_barlines(layout);
        self.draw_minimap_note_overview(layout);
        let active_highlight = self.draw_minimap_visible_highlight(layout, visible);

        MinimapRenderInfo {
            content_rect: layout.active_group_rect,
            highlight_rect: active_highlight,
            seek_start_ms: layout.active_start_ms,
            seek_end_ms: layout.active_end_ms,
        }
    }
}

impl FallingGroundEditor {
    fn draw_minimap_page_columns(&self, layout: MinimapDrawLayout) {
        let ui = layout.ui;
        for (g_rect, a_rect, g_label, a_label) in [
            (layout.ground_rect_1, layout.sky_rect_1, "G1", "A1"),
            (layout.ground_rect_2, layout.sky_rect_2, "G2", "A2"),
        ] {
            draw_rectangle(
                g_rect.x,
                g_rect.y,
                g_rect.w,
                g_rect.h,
                Color::from_rgba(12, 18, 28, 188),
            );
            draw_rectangle(
                a_rect.x,
                a_rect.y,
                a_rect.w,
                a_rect.h,
                Color::from_rgba(18, 14, 30, 188),
            );
            draw_rectangle_lines(
                g_rect.x,
                g_rect.y,
                g_rect.w,
                g_rect.h,
                1.0,
                Color::from_rgba(62, 86, 118, 144),
            );
            draw_rectangle_lines(
                a_rect.x,
                a_rect.y,
                a_rect.w,
                a_rect.h,
                1.0,
                Color::from_rgba(94, 84, 138, 144),
            );
            draw_text_ex(
                g_label,
                g_rect.x + 2.0 * ui,
                g_rect.y + 12.0 * ui,
                TextParams {
                    font: self.view.text_font.as_ref(),
                    font_size: scaled_font_size(10.0, 8, 20),
                    color: Color::from_rgba(186, 216, 245, 196),
                    ..Default::default()
                },
            );
            draw_text_ex(
                a_label,
                a_rect.x + 2.0 * ui,
                a_rect.y + 12.0 * ui,
                TextParams {
                    font: self.view.text_font.as_ref(),
                    font_size: scaled_font_size(10.0, 8, 20),
                    color: Color::from_rgba(214, 188, 246, 196),
                    ..Default::default()
                },
            );

            let ground_lane_w = g_rect.w / LANE_COUNT as f32;
            for lane in 1..LANE_COUNT {
                let x = g_rect.x + lane as f32 * ground_lane_w;
                draw_line(
                    x,
                    g_rect.y,
                    x,
                    g_rect.y + g_rect.h,
                    1.0,
                    Color::from_rgba(42, 56, 84, 132),
                );
            }
            for lane in 1..4 {
                let x = a_rect.x + lane as f32 * (a_rect.w / 4.0);
                draw_line(
                    x,
                    a_rect.y,
                    x,
                    a_rect.y + a_rect.h,
                    1.0,
                    Color::from_rgba(72, 64, 108, 128),
                );
            }
        }
    }
}

impl FallingGroundEditor {
    fn draw_minimap_barlines(&self, layout: MinimapDrawLayout) {
        let ui = layout.ui;
        for (group_rect, page_start_ms, page_end_ms) in [
            (layout.left_group_rect, 0.0_f32, layout.half_ms.max(0.001)),
            (
                layout.right_group_rect,
                layout.half_ms,
                layout.duration_ms.max(layout.half_ms + 0.001),
            ),
        ] {
            let center_ms = (page_start_ms + page_end_ms) * 0.5;
            let ahead_ms = (page_end_ms - center_ms).max(0.0);
            let behind_ms = (center_ms - page_start_ms).max(0.0);
            for barline in self
                .editor_state
                .timeline
                .visible_barlines(center_ms, ahead_ms, behind_ms, 16)
            {
                if barline.time_ms < page_start_ms || barline.time_ms > page_end_ms {
                    continue;
                }
                let y = self.minimap_segment_time_to_y(
                    barline.time_ms,
                    group_rect,
                    page_start_ms,
                    page_end_ms,
                );
                let (thickness, color) = match barline.kind {
                    BarLineKind::Measure => (1.3 * ui, Color::from_rgba(168, 190, 236, 170)),
                    BarLineKind::Beat => (1.0 * ui, Color::from_rgba(108, 128, 170, 124)),
                    BarLineKind::Subdivision => (0.8 * ui, Color::from_rgba(76, 96, 132, 92)),
                };
                draw_line(
                    group_rect.x,
                    y,
                    group_rect.x + group_rect.w,
                    y,
                    thickness.max(1.0),
                    color,
                );
            }
        }
    }
}

impl FallingGroundEditor {
    fn draw_minimap_note_overview(&self, layout: MinimapDrawLayout) {
        let ui = layout.ui;
        let thin = (1.05 * ui).max(1.0);
        let head_h = (2.8 * ui).clamp(1.0, 5.0);
        let flick_tip_h = (head_h * 0.35).max(0.8);

        // 分两次绘制，确保 Flick 始终压在 SkyArea 上层。
        for flick_pass in [false, true] {
            for note in &self.editor_state.notes {
                if flick_pass != (note.kind == GroundNoteKind::Flick) {
                    continue;
                }

                let note_time = note.time_ms.max(0.0);
                let on_right = note_time >= layout.half_ms;
                let (ground_rect, sky_rect, page_start_ms, page_end_ms) = if on_right {
                    (
                        layout.ground_rect_2,
                        layout.sky_rect_2,
                        layout.half_ms,
                        layout.duration_ms.max(layout.half_ms + 0.001),
                    )
                } else {
                    (
                        layout.ground_rect_1,
                        layout.sky_rect_1,
                        0.0_f32,
                        layout.half_ms.max(0.001),
                    )
                };

                let y_head = self.minimap_segment_time_to_y(
                    note_time,
                    ground_rect,
                    page_start_ms,
                    page_end_ms,
                );
                let lane_palette = lane_note_palette(note.lane.clamp(0, LANE_COUNT - 1));
                let ground_lane_w = ground_rect.w / LANE_COUNT as f32;
                let page_duration_ms = (page_end_ms - page_start_ms).max(0.001);

                match note.kind {
                    GroundNoteKind::Tap => {
                        let eff_w = ground_note_effective_width(note.lane, note.width);
                        let total_w = ground_lane_w * eff_w as f32;
                        let note_w = (total_w * 0.74).max(1.0);
                        let note_x = ground_rect.x
                            + ground_lane_w * note.lane as f32
                            + (total_w - note_w) * 0.5;
                        draw_rectangle(
                            note_x,
                            y_head - head_h * 0.5,
                            note_w,
                            head_h,
                            lane_palette.tap,
                        );
                    }
                    GroundNoteKind::Hold => {
                        let note_end = note.end_time_ms();
                        let eff_w = ground_note_effective_width(note.lane, note.width);
                        for (g_rect, start_ms, end_ms) in [
                            (layout.ground_rect_1, 0.0_f32, layout.half_ms.max(0.001)),
                            (
                                layout.ground_rect_2,
                                layout.half_ms,
                                layout.duration_ms.max(layout.half_ms + 0.001),
                            ),
                        ] {
                            if note_end < start_ms || note_time > end_ms {
                                continue;
                            }
                            let lane_w = g_rect.w / LANE_COUNT as f32;
                            let total_w = lane_w * eff_w as f32;
                            let lane_x = g_rect.x + lane_w * note.lane as f32;
                            let head_w = (total_w * 0.82).max(1.0);
                            let head_x = lane_x + (total_w - head_w) * 0.5;
                            let body_start = note_time.max(start_ms);
                            let body_end = note_end.min(end_ms);
                            let y0 = self
                                .minimap_segment_time_to_y(body_start, g_rect, start_ms, end_ms);
                            let y1 =
                                self.minimap_segment_time_to_y(body_end, g_rect, start_ms, end_ms);
                            let body_w = (head_w * 0.56).max(1.0);
                            let body_x = head_x + (head_w - body_w) * 0.5;
                            draw_rectangle(
                                body_x,
                                y0.min(y1),
                                body_w,
                                (y1 - y0).abs().max(1.0),
                                lane_palette.hold_body,
                            );

                            if note_time >= start_ms && note_time <= end_ms {
                                let y_head_local = self
                                    .minimap_segment_time_to_y(note_time, g_rect, start_ms, end_ms);
                                draw_rectangle(
                                    head_x,
                                    y_head_local - head_h * 0.55,
                                    head_w,
                                    head_h * 1.1,
                                    lane_palette.hold_head,
                                );
                            }
                            if note_end >= start_ms && note_end <= end_ms {
                                let y_tail_local = self
                                    .minimap_segment_time_to_y(note_end, g_rect, start_ms, end_ms);
                                draw_rectangle(
                                    head_x,
                                    y_tail_local - head_h * 0.45,
                                    head_w,
                                    head_h * 0.9,
                                    Color::from_rgba(
                                        (lane_palette.hold_head.r * 255.0) as u8,
                                        (lane_palette.hold_head.g * 255.0) as u8,
                                        (lane_palette.hold_head.b * 255.0) as u8,
                                        190,
                                    ),
                                );
                            }
                        }
                    }
                    GroundNoteKind::Flick => {
                        let center_x = sky_rect.x + note.center_x_norm * sky_rect.w;
                        let air_lane_w = sky_rect.w / 4.0;
                        let note_w = air_note_width(note, sky_rect.w)
                            .clamp(air_lane_w * 0.22, air_lane_w * 0.98);
                        let flick_color = if note.flick_right {
                            Color::from_rgba(112, 228, 156, 230)
                        } else {
                            Color::from_rgba(246, 232, 122, 230)
                        };
                        let bpm = self
                            .editor_state
                            .timeline
                            .point_at_time(note_time)
                            .bpm
                            .abs()
                            .max(0.001);
                        let subdiv_ms = 60_000.0 / bpm / 16.0;
                        let side_h = (subdiv_ms / page_duration_ms * sky_rect.h).max(head_h);
                        let side_x = if note.flick_right {
                            center_x + note_w * 0.46
                        } else {
                            center_x - note_w * 0.46
                        };
                        let tip_x = if note.flick_right {
                            center_x - note_w * 0.52
                        } else {
                            center_x + note_w * 0.52
                        };
                        let y_bottom = self.minimap_segment_time_to_y(
                            note_time,
                            sky_rect,
                            page_start_ms,
                            page_end_ms,
                        );
                        let y_top = y_bottom - side_h;
                        let y_tip_top = y_bottom - flick_tip_h;
                        const CURVE_STEPS: usize = 16;
                        let mut top_curve = [Vec2::new(0.0, 0.0); CURVE_STEPS + 1];
                        for i in 0..=CURVE_STEPS {
                            let t = i as f32 / CURVE_STEPS as f32;
                            let eased = ease_progress(Ease::SineOut, t);
                            let x = lerp(side_x, tip_x, t);
                            let y = lerp(y_top, y_tip_top, eased);
                            top_curve[i] = Vec2::new(x, y);
                        }
                        let base = Vec2::new(side_x, y_bottom);
                        for i in 0..CURVE_STEPS {
                            draw_triangle(
                                base,
                                top_curve[i],
                                top_curve[i + 1],
                                Color::new(flick_color.r, flick_color.g, flick_color.b, 0.52),
                            );
                        }
                        draw_triangle(
                            base,
                            top_curve[CURVE_STEPS],
                            Vec2::new(tip_x, y_bottom),
                            Color::new(flick_color.r, flick_color.g, flick_color.b, 0.52),
                        );
                        for i in 0..CURVE_STEPS {
                            let a = top_curve[i];
                            let b = top_curve[i + 1];
                            draw_line(a.x, a.y, b.x, b.y, thin, flick_color);
                        }
                        draw_line(side_x, y_bottom, tip_x, y_bottom, thin, flick_color);
                        draw_line(side_x, y_bottom, side_x, y_top, thin, flick_color);
                    }
                    GroundNoteKind::SkyArea => {
                        self.draw_minimap_skyarea_note(note, note_time, head_h, thin, layout);
                    }
                }
            }
        }
    }
}

impl FallingGroundEditor {
    fn draw_minimap_skyarea_note(
        &self,
        note: &GroundNote,
        note_time: f32,
        head_h: f32,
        thin: f32,
        layout: MinimapDrawLayout,
    ) {
        let note_end = note.end_time_ms();
        for (s_rect, start_ms, end_ms) in [
            (layout.sky_rect_1, 0.0_f32, layout.half_ms.max(0.001)),
            (
                layout.sky_rect_2,
                layout.half_ms,
                layout.duration_ms.max(layout.half_ms + 0.001),
            ),
        ] {
            if note_end < start_ms || note_time > end_ms {
                continue;
            }
            if let Some(shape) = note.skyarea_shape {
                let inter_start = note_time.max(start_ms);
                let inter_end = note_end.min(end_ms);
                if inter_end > inter_start + 0.000_1 && note.duration_ms > 0.0 {
                    let seg_count = 20;
                    for i in 0..seg_count {
                        let s0 = i as f32 / seg_count as f32;
                        let s1 = (i + 1) as f32 / seg_count as f32;
                        let t0 = lerp(inter_start, inter_end, s0);
                        let t1 = lerp(inter_start, inter_end, s1);
                        let p0 = ((t0 - note_time) / note.duration_ms).clamp(0.0, 1.0);
                        let p1 = ((t1 - note_time) / note.duration_ms).clamp(0.0, 1.0);

                        let y0 = self.minimap_segment_time_to_y(t0, s_rect, start_ms, end_ms);
                        let y1 = self.minimap_segment_time_to_y(t1, s_rect, start_ms, end_ms);
                        let l0 = lerp(
                            shape.start_left_norm,
                            shape.end_left_norm,
                            ease_progress(shape.left_ease, p0),
                        )
                        .clamp(0.0, 1.0);
                        let r0 = lerp(
                            shape.start_right_norm,
                            shape.end_right_norm,
                            ease_progress(shape.right_ease, p0),
                        )
                        .clamp(0.0, 1.0);
                        let l1 = lerp(
                            shape.start_left_norm,
                            shape.end_left_norm,
                            ease_progress(shape.left_ease, p1),
                        )
                        .clamp(0.0, 1.0);
                        let r1 = lerp(
                            shape.start_right_norm,
                            shape.end_right_norm,
                            ease_progress(shape.right_ease, p1),
                        )
                        .clamp(0.0, 1.0);

                        let lx0 = s_rect.x + l0 * s_rect.w;
                        let rx0 = s_rect.x + r0 * s_rect.w;
                        let lx1 = s_rect.x + l1 * s_rect.w;
                        let rx1 = s_rect.x + r1 * s_rect.w;
                        draw_triangle(
                            Vec2::new(lx0, y0),
                            Vec2::new(rx0, y0),
                            Vec2::new(rx1, y1),
                            Color::new(
                                AIR_SKYAREA_BODY_COLOR.r,
                                AIR_SKYAREA_BODY_COLOR.g,
                                AIR_SKYAREA_BODY_COLOR.b,
                                0.30,
                            ),
                        );
                        draw_triangle(
                            Vec2::new(lx0, y0),
                            Vec2::new(rx1, y1),
                            Vec2::new(lx1, y1),
                            Color::new(
                                AIR_SKYAREA_BODY_COLOR.r,
                                AIR_SKYAREA_BODY_COLOR.g,
                                AIR_SKYAREA_BODY_COLOR.b,
                                0.30,
                            ),
                        );
                    }
                }

                if note_time >= start_ms && note_time <= end_ms {
                    let y_head_local =
                        self.minimap_segment_time_to_y(note_time, s_rect, start_ms, end_ms);
                    let head_left = s_rect.x + shape.start_left_norm.clamp(0.0, 1.0) * s_rect.w;
                    let head_right = s_rect.x + shape.start_right_norm.clamp(0.0, 1.0) * s_rect.w;
                    draw_rectangle(
                        head_left,
                        y_head_local - head_h * 0.5,
                        (head_right - head_left).max(1.0),
                        head_h,
                        AIR_SKYAREA_HEAD_COLOR,
                    );
                }
                if note_end >= start_ms && note_end <= end_ms {
                    let y_tail_local =
                        self.minimap_segment_time_to_y(note_end, s_rect, start_ms, end_ms);
                    let tail_left = s_rect.x + shape.end_left_norm.clamp(0.0, 1.0) * s_rect.w;
                    let tail_right = s_rect.x + shape.end_right_norm.clamp(0.0, 1.0) * s_rect.w;
                    draw_rectangle(
                        tail_left,
                        y_tail_local - head_h * 0.5,
                        (tail_right - tail_left).max(1.0),
                        head_h,
                        AIR_SKYAREA_TAIL_COLOR,
                    );
                }
            } else {
                let x = s_rect.x + note.center_x_norm * s_rect.w;
                let y_head_local =
                    self.minimap_segment_time_to_y(note_time, s_rect, start_ms, end_ms);
                let y_tail_local =
                    self.minimap_segment_time_to_y(note_end, s_rect, start_ms, end_ms);
                let head_w = (s_rect.w / 4.0 * 0.64).max(1.0);
                draw_rectangle(
                    x - head_w * 0.5,
                    y_head_local - head_h * 0.5,
                    head_w,
                    head_h,
                    AIR_SKYAREA_HEAD_COLOR,
                );
                draw_rectangle(
                    x - head_w * 0.5,
                    y_tail_local - head_h * 0.5,
                    head_w,
                    head_h,
                    AIR_SKYAREA_TAIL_COLOR,
                );
                draw_line(
                    x,
                    y_head_local,
                    x,
                    y_tail_local,
                    thin,
                    AIR_SKYAREA_BODY_COLOR,
                );
            }
        }
    }
}

impl FallingGroundEditor {
    fn draw_minimap_visible_highlight(
        &self,
        layout: MinimapDrawLayout,
        visible: TimeWindowMs,
    ) -> Rect {
        let ui = layout.ui;
        let mut active_highlight = Rect::new(
            layout.active_group_rect.x,
            layout.active_group_rect.y,
            layout.active_group_rect.w,
            (2.0 * ui).max(1.0),
        );

        for (group_rect, page_start_ms, page_end_ms) in [
            (layout.left_group_rect, 0.0_f32, layout.half_ms.max(0.001)),
            (
                layout.right_group_rect,
                layout.half_ms,
                layout.duration_ms.max(layout.half_ms + 0.001),
            ),
        ] {
            let overlap_start = visible.start_ms.max(page_start_ms).min(page_end_ms);
            let overlap_end = visible.end_ms.max(page_start_ms).min(page_end_ms);
            if overlap_end < overlap_start {
                continue;
            }
            let y_top =
                self.minimap_segment_time_to_y(overlap_end, group_rect, page_start_ms, page_end_ms);
            let y_bottom = self.minimap_segment_time_to_y(
                overlap_start,
                group_rect,
                page_start_ms,
                page_end_ms,
            );
            let highlight_h = (y_bottom - y_top).abs().max((2.0 * ui).max(1.0));
            let highlight = Rect::new(group_rect.x, y_top.min(y_bottom), group_rect.w, highlight_h);
            draw_rectangle(
                highlight.x,
                highlight.y,
                highlight.w,
                highlight.h,
                Color::from_rgba(255, 255, 255, 28),
            );
            draw_rectangle_lines(
                highlight.x,
                highlight.y,
                highlight.w,
                highlight.h,
                (1.2 * ui).max(1.0),
                Color::from_rgba(255, 255, 255, 214),
            );
            if (page_start_ms - layout.active_start_ms).abs() < 0.5 {
                active_highlight = highlight;
            }
        }

        let current_y = self.minimap_segment_time_to_y(
            visible
                .current_ms
                .clamp(layout.active_start_ms, layout.active_end_ms),
            layout.active_group_rect,
            layout.active_start_ms,
            layout.active_end_ms,
        );
        draw_line(
            layout.active_group_rect.x,
            current_y,
            layout.active_group_rect.x + layout.active_group_rect.w,
            current_y,
            (1.0 * ui).max(1.0),
            Color::from_rgba(255, 238, 204, 182),
        );

        active_highlight
    }
}
