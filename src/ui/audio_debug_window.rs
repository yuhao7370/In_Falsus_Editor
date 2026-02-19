use crate::audio::controller::AudioDebugSnapshot;
use egui_macroquad::egui;

fn debug_row(ui: &mut egui::Ui, mono: &egui::FontId, label: &str, value: String, warn: bool) {
    let label_color = egui::Color32::from_rgb(160, 160, 170);
    let value_color = egui::Color32::from_rgb(220, 220, 230);
    let warn_color = egui::Color32::from_rgb(255, 180, 60);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).font(mono.clone()).color(label_color));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let color = if warn { warn_color } else { value_color };
            ui.label(egui::RichText::new(value).font(mono.clone()).color(color));
        });
    });
}

/// Draw the audio debug floating window.
pub fn draw_audio_debug_window(
    ctx: &egui::Context,
    open: &mut bool,
    snapshot: &AudioDebugSnapshot,
) {
    if !*open {
        return;
    }

    // 使用控制器预计算的平滑速度（0.5s 窗口均值），而非单帧瞬时值
    let estimated_speed = snapshot.estimated_speed;
    let speed_abnormal = snapshot.is_playing_ctrl && estimated_speed > 1.5;

    let mut is_open = *open;
    egui::Window::new("🔊 Audio Debug")
        .open(&mut is_open)
        .resizable(false)
        .collapsible(true)
        .default_size(egui::vec2(340.0, 0.0))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
        .frame(
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 12, 16, 240))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 55)))
                .inner_margin(egui::Margin::same(10))
                .corner_radius(egui::CornerRadius::same(6)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            let mono = egui::TextStyle::Monospace.resolve(ui.style());

            debug_row(ui, &mono, "State", snapshot.playback_state.clone(), false);
            debug_row(ui, &mono, "Backend", format!("{:.4}s", snapshot.backend_position), false);
            debug_row(ui, &mono, "Controller", format!("{:.4}s", snapshot.controller_position), false);
            debug_row(ui, &mono, "Anchor Pos", format!("{:.4}s", snapshot.anchor_pos), false);
            debug_row(ui, &mono, "Anchor Time", format!("{:.4}s", snapshot.anchor_time), false);
            debug_row(ui, &mono, "Duration", format!("{:.3}s", snapshot.duration_sec), false);

            ui.separator();

            debug_row(ui, &mono, "Δ Pos/Frame", format!("{:.6}s", snapshot.pos_delta_per_frame), false);
            debug_row(ui, &mono, "Est. Speed", format!("{:.3}x", estimated_speed), speed_abnormal);

            // 始终预留警告行空间，避免窗口高度抖动导致闪烁
            ui.horizontal(|ui| {
                if speed_abnormal {
                    ui.label(
                        egui::RichText::new("⚠ 播放速度异常！可能是采样率不匹配")
                            .color(egui::Color32::from_rgb(255, 180, 60))
                            .small(),
                    );
                } else {
                    // 占位：用透明文字保持行高一致
                    ui.label(
                        egui::RichText::new("　")
                            .color(egui::Color32::TRANSPARENT)
                            .small(),
                    );
                }
            });

            ui.separator();

            debug_row(
                ui, &mono, "Volume",
                format!(
                    "{:.0}% (M:{:.0}% × Mus:{:.0}%)",
                    snapshot.effective_volume * 100.0,
                    snapshot.master_volume * 100.0,
                    snapshot.music_volume * 100.0
                ),
                false,
            );
            debug_row(
                ui, &mono, "FPS",
                format!("{:.1} ({:.2}ms)", snapshot.fps, snapshot.delta_time * 1000.0),
                false,
            );
            debug_row(
                ui, &mono, "Backend",
                if snapshot.has_backend { "OK".to_string() } else { "MISSING".to_string() },
                !snapshot.has_backend,
            );

            if !snapshot.track_path.is_empty() {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("Track: {}", &snapshot.track_path))
                        .font(mono)
                        .color(egui::Color32::from_rgb(140, 180, 220))
                        .small(),
                );
            }
        });
    *open = is_open;
}
