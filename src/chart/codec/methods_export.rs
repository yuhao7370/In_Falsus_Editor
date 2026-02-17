impl Chart {
/// 序列化为 `.spc` 文本格式。
pub fn to_spc(&self) -> String {
    let mut lines = Vec::new();
    let mut unknowns = Vec::new();

    for event in &self.events {
        match event {
            ChartEvent::Chart { bpm, beats } => {
                lines.push(format!("chart({:.2},{:.2})", bpm, beats));
            }
            ChartEvent::Tap { time, width, lane } => {
                lines.push(format!("tap({},{},{})", fmt_num(*time), fmt_num(*width), lane));
            }
            ChartEvent::Hold {
                time,
                lane,
                width,
                duration,
            } => {
                lines.push(format!(
                    "hold({},{},{},{})",
                    fmt_num(*time),
                    lane,
                    fmt_num(*width),
                    fmt_num(*duration)
                ));
            }
            ChartEvent::Flick {
                time,
                x,
                x_split,
                width,
                flick_type,
            } => {
                lines.push(format!(
                    "flick({},{},{},{},{})",
                    fmt_num(*time),
                    fmt_num(*x),
                    fmt_num(*x_split),
                    fmt_num(*width),
                    flick_type.to_value()
                ));
            }
            ChartEvent::SkyArea {
                time,
                start_x,
                start_x_split,
                start_width,
                end_x,
                end_x_split,
                end_width,
                left_ease,
                right_ease,
                duration,
                group_id,
            } => {
                lines.push(format!(
                    "skyarea({},{},{},{},{},{},{},{},{},{},{})",
                    fmt_num(*time),
                    fmt_num(*start_x),
                    fmt_num(*start_x_split),
                    fmt_num(*start_width),
                    fmt_num(*end_x),
                    fmt_num(*end_x_split),
                    fmt_num(*end_width),
                    left_ease.to_value(),
                    right_ease.to_value(),
                    fmt_num(*duration),
                    group_id
                ));
            }
            ChartEvent::Bpm {
                time,
                bpm,
                beats,
                unknown,
            } => {
                lines.push(format!(
                    "bpm({},{:.2},{:.2},{:.2})",
                    fmt_num(*time),
                    bpm,
                    beats,
                    unknown
                ));
            }
            ChartEvent::Track { time, speed } => {
                lines.push(format!("track({},{:.2})", fmt_num(*time), speed));
            }
            ChartEvent::Lane { time, lane, enable } => {
                lines.push(format!(
                    "lane({},{},{})",
                    fmt_num(*time),
                    lane,
                    if *enable { 1 } else { 0 }
                ));
            }
            ChartEvent::Beam { raw } | ChartEvent::Unknown { raw } => {
                unknowns.push(raw.clone());
            }
        }
    }

    lines.extend(unknowns);
    lines.join("\n") + "\n"
}

pub fn chart_info(&self) -> Option<(f64, f64)> {
    self.events.iter().find_map(|e| match e {
        ChartEvent::Chart { bpm, beats } => Some((*bpm, *beats)),
        _ => None,
    })
}

pub fn tap_count(&self) -> usize {
    self.events
        .iter()
        .filter(|e| matches!(e, ChartEvent::Tap { .. }))
        .count()
}

pub fn hold_count(&self) -> usize {
    self.events
        .iter()
        .filter(|e| matches!(e, ChartEvent::Hold { .. }))
        .count()
}

pub fn flick_count(&self) -> usize {
    self.events
        .iter()
        .filter(|e| matches!(e, ChartEvent::Flick { .. }))
        .count()
}

pub fn skyarea_count(&self) -> usize {
    self.events
        .iter()
        .filter(|e| matches!(e, ChartEvent::SkyArea { .. }))
        .count()
}

pub fn total_notes(&self) -> usize {
    self.tap_count() + self.hold_count() + self.flick_count()
}

/// 导出到 JSON 文件（pretty 格式）。
pub fn to_json_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(self).map_err(|e| format!("JSON 序列化失败: {e}"))?;
    fs::write(path, json).map_err(|e| format!("写入文件失败: {e}"))
}


}

