// 文件说明：界面通用辅助函数集合。
// 主要功能：提供插值、裁剪、命中检测等基础 UI 工具。
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn ease_progress(ease: Ease, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match ease {
        Ease::Linear => t,
        Ease::SineOut => (t * std::f32::consts::FRAC_PI_2).sin(),
        Ease::SineIn => 1.0 - (t * std::f32::consts::FRAC_PI_2).cos(),
    }
}

fn point_in_rect(x: f32, y: f32, rect: Rect) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

