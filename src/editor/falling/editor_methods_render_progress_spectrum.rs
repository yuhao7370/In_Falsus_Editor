// 文件说明：进度条与频谱渲染实现。
// 主要功能：绘制播放进度、频谱信息及其交互反馈。
impl FallingGroundEditor {
    fn draw_falling_spectrum(&self, rect: Rect, current_ms: f32, judge_y: f32, tint: Color) {
        let Some(waveform) = &self.view.waveform else {
            return;
        };
        if waveform.peaks.is_empty()
            || waveform.duration_sec <= 0.0
            || rect.h <= 4.0
            || rect.w <= 4.0
        {
            return;
        }

        let pixels_per_sec = (self.view.scroll_speed * rect.h).max(1.0);
        let mut y = rect.y;
        while y <= rect.y + rect.h {
            let dt_ms = (judge_y - y) / pixels_per_sec * 1000.0;
            let time_sec = (current_ms + dt_ms).max(0.0) / 1000.0;
            let raw = self.sample_waveform_amp(time_sec);
            // Apply sqrt curve to expand low-mid range visibility
            let amp = raw.sqrt();
            if amp > 0.03 {
                let alpha = (amp * 128.0).clamp(12.0, 140.0) as u8;
                let main_color = Color::new(tint.r, tint.g, tint.b, alpha as f32 / 255.0);
                let width_ratio = 0.06 + 0.88 * amp;
                let edge = (rect.w * (0.5 - 0.5 * width_ratio)).clamp(0.0, rect.w * 0.48);
                draw_line(rect.x + edge, y, rect.x + rect.w - edge, y, 1.0, main_color);
            }
            y += 2.0;
        }
    }

    fn sample_waveform_amp(&self, sec: f32) -> f32 {
        let Some(waveform) = &self.view.waveform else {
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
