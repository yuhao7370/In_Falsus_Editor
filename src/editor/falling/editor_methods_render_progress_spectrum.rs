impl FallingGroundEditor {
    fn draw_vertical_progress(&self, rect: Rect, current_sec: f32, duration_sec: f32) {
        if rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::from_rgba(12, 16, 24, 255));
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::from_rgba(44, 54, 84, 255),
        );

        let duration = self.estimate_duration(duration_sec).max(0.001);
        let progress = (current_sec / duration).clamp(0.0, 1.0);
        let fill_h = rect.h * progress;
        if fill_h > 0.5 {
            draw_rectangle(
                rect.x + 2.0,
                rect.y + rect.h - fill_h,
                (rect.w - 4.0).max(1.0),
                fill_h,
                Color::from_rgba(74, 134, 210, 165),
            );
        }

        let playhead_y = rect.y + rect.h - progress * rect.h;
        draw_line(
            rect.x,
            playhead_y,
            rect.x + rect.w,
            playhead_y,
            2.0,
            Color::from_rgba(255, 96, 96, 255),
        );
        if self.waveform_seek_active {
            let seek_progress = (self.waveform_seek_sec / duration).clamp(0.0, 1.0);
            let seek_y = rect.y + rect.h - seek_progress * rect.h;
            draw_line(
                rect.x,
                seek_y,
                rect.x + rect.w,
                seek_y,
                1.6,
                Color::from_rgba(255, 220, 80, 255),
            );
        }

        draw_text_ex(
            "AUDIO",
            rect.x + 2.0,
            rect.y + 16.0,
            TextParams {
                font_size: 14,
                color: Color::from_rgba(176, 200, 236, 255),
                ..Default::default()
            },
        );
    }

    fn draw_falling_spectrum(&self, rect: Rect, current_ms: f32, judge_y: f32, tint: Color) {
        let Some(waveform) = &self.waveform else {
            return;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 || rect.h <= 4.0 || rect.w <= 4.0 {
            return;
        }

        let pixels_per_sec = (self.scroll_speed * rect.h).max(1.0);
        let mut y = rect.y;
        while y <= rect.y + rect.h {
            let dt_ms = (judge_y - y) / pixels_per_sec * 1000.0;
            let time_sec = (current_ms + dt_ms).max(0.0) / 1000.0;
            let amp = self.sample_waveform_amp(time_sec).powf(0.82);
            if amp > 0.015 {
                let alpha = (amp * 116.0).clamp(10.0, 128.0) as u8;
                let main_color = Color::new(tint.r, tint.g, tint.b, alpha as f32 / 255.0);
                let edge = (rect.w * (0.5 - 0.46 * amp)).clamp(0.0, rect.w * 0.45);
                draw_line(
                    rect.x + edge,
                    y,
                    rect.x + rect.w - edge,
                    y,
                    1.0,
                    main_color,
                );
            }
            y += 2.0;
        }
    }

    fn sample_waveform_amp(&self, sec: f32) -> f32 {
        let Some(waveform) = &self.waveform else {
            return 0.0;
        };
        if waveform.peaks.is_empty() || waveform.duration_sec <= 0.0 {
            return 0.0;
        }
        let len = waveform.peaks.len();
        if len == 1 {
            return waveform.peaks[0].clamp(0.0, 1.0);
        }
        let t = (sec / waveform.duration_sec).clamp(0.0, 1.0);
        let pos = t * (len as f32 - 1.0);
        let i0 = pos.floor() as usize;
        let i1 = (i0 + 1).min(len - 1);
        let f = pos - i0 as f32;
        lerp(waveform.peaks[i0], waveform.peaks[i1], f).clamp(0.0, 1.0)
    }



}

