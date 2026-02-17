// 文件说明：BPM 时间轴模型与节拍换算实现。
// 主要功能：处理毫秒与拍点转换、吸附和可见拍线生成。
#[derive(Debug, Clone, Copy)]
struct BpmPoint {
    time_ms: f32,
    bpm: f32,
    beats_per_measure: f32,
    start_beat: f32,
}

#[derive(Debug, Clone, Copy)]
struct BarLine {
    time_ms: f32,
    kind: BarLineKind,
    measure_pos: f32,
    show_measure_label: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarLineKind {
    Measure,
    Beat,
    Subdivision,
}

impl BarLineKind {
    fn priority(self) -> u8 {
        match self {
            Self::Measure => 3,
            Self::Beat => 2,
            Self::Subdivision => 1,
        }
    }
}

#[derive(Debug, Clone)]
struct BpmTimeline {
    points: Vec<BpmPoint>,
}

#[derive(Debug, Clone)]
struct BpmSourceData {
    base_bpm: f32,
    base_beats_per_measure: f32,
    bpm_events: Vec<(f32, f32, f32)>,
}

impl Default for BpmSourceData {
    fn default() -> Self {
        Self {
            base_bpm: 120.0,
            base_beats_per_measure: 4.0,
            bpm_events: Vec::new(),
        }
    }
}

impl BpmTimeline {
    fn from_source(mut source: BpmSourceData) -> Self {
        let mut points = vec![BpmPoint {
            time_ms: 0.0,
            bpm: source.base_bpm,
            beats_per_measure: source.base_beats_per_measure,
            start_beat: 0.0,
        }];

        source.bpm_events.sort_by(|a, b| a.0.total_cmp(&b.0));

        for (time_ms, bpm, beats_per_measure) in source.bpm_events {
            if time_ms <= 0.0 {
                points[0].bpm = bpm;
                points[0].beats_per_measure = beats_per_measure;
                continue;
            }

            if let Some(last) = points.last_mut() {
                if (last.time_ms - time_ms).abs() < 0.000_1 {
                    last.bpm = bpm;
                    last.beats_per_measure = beats_per_measure;
                    continue;
                }
            }

            points.push(BpmPoint {
                time_ms,
                bpm,
                beats_per_measure,
                start_beat: 0.0,
            });
        }

        points.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));

        for idx in 1..points.len() {
            let previous = points[idx - 1];
            let dt_ms = (points[idx].time_ms - previous.time_ms).max(0.0);
            let bpm = previous.bpm.abs().max(0.001);
            points[idx].start_beat = previous.start_beat + dt_ms / 60_000.0 * bpm;
        }

        Self { points }
    }

    fn point_at_time(&self, time_ms: f32) -> BpmPoint {
        let idx = self.point_index_at_or_before_time(time_ms);
        self.points[idx]
    }

    fn time_to_beat(&self, time_ms: f32) -> f32 {
        let point = self.point_at_time(time_ms);
        let bpm = point.bpm.abs().max(0.001);
        point.start_beat + (time_ms - point.time_ms) / 60_000.0 * bpm
    }

    fn beat_to_time(&self, beat: f32) -> f32 {
        let idx = self.point_index_at_or_before_beat(beat);
        let point = self.points.get(idx).copied().unwrap_or(BpmPoint {
            time_ms: 0.0,
            bpm: 120.0,
            beats_per_measure: 4.0,
            start_beat: 0.0,
        });
        let bpm = point.bpm.abs().max(0.001);
        point.time_ms + (beat - point.start_beat) * 60_000.0 / bpm
    }

    fn snap_time_ms(&self, time_ms: f32, division: u32) -> f32 {
        if division == 0 {
            return time_ms.max(0.0);
        }
        let beat = self.time_to_beat(time_ms);
        let snapped = (beat * division as f32).round() / division as f32;
        self.beat_to_time(snapped).max(0.0)
    }

    fn visible_barlines(
        &self,
        current_ms: f32,
        ahead_ms: f32,
        behind_ms: f32,
        subdivision: u32,
    ) -> Vec<BarLine> {
        let start_ms = current_ms - behind_ms;
        let end_ms = current_ms + ahead_ms;
        let mut output = Vec::new();
        let subdivision = subdivision.max(1);
        let subdivision_i = subdivision as i32;
        let first_idx = self.point_index_at_or_before_time(start_ms);
        for idx in first_idx..self.points.len() {
            let point = self.points[idx];
            let segment_start = point.time_ms;
            let segment_end = if idx + 1 < self.points.len() {
                self.points[idx + 1].time_ms
            } else {
                end_ms + 60_000.0
            };
            if segment_start > end_ms + 0.001 {
                break;
            }

            let visible_start = segment_start.max(start_ms);
            let visible_end = segment_end.min(end_ms);
            if visible_end < visible_start {
                continue;
            }

            let bpm = point.bpm.abs().max(0.001);
            let beat_ms = 60_000.0 / bpm;
            let sub_ms = beat_ms / subdivision as f32;
            let beats_per_measure = point.beats_per_measure.max(1.0);

            let n_start = ((visible_start - segment_start) / sub_ms).floor() as i32 - 2;
            let n_end = ((visible_end - segment_start) / sub_ms).ceil() as i32 + 2;

            for n in n_start..=n_end {
                if n < 0 {
                    continue;
                }
                let line_time_ms = segment_start + n as f32 * sub_ms;
                if line_time_ms < visible_start - 0.001 || line_time_ms > visible_end + 0.001 {
                    continue;
                }

                let beat = point.start_beat + n as f32 / subdivision as f32;
                let measure_phase = beat / beats_per_measure;
                let is_measure = (measure_phase - measure_phase.round()).abs() < 0.001;
                let is_beat = n % subdivision_i == 0;
                let kind = if is_measure {
                    BarLineKind::Measure
                } else if is_beat {
                    BarLineKind::Beat
                } else {
                    BarLineKind::Subdivision
                };

                let measure_pos = beat / beats_per_measure;
                let half_step = (measure_pos * 2.0).round();
                let show_measure_label = (measure_pos * 2.0 - half_step).abs() < 0.001;

                output.push(BarLine {
                    time_ms: line_time_ms,
                    kind,
                    measure_pos,
                    show_measure_label,
                });
            }
        }

        output.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then_with(|| b.kind.priority().cmp(&a.kind.priority()))
        });

        let mut deduped: Vec<BarLine> = Vec::with_capacity(output.len());
        for line in output {
            if let Some(last) = deduped.last_mut() {
                if (last.time_ms - line.time_ms).abs() < 0.001 {
                    if line.kind.priority() > last.kind.priority() {
                        last.kind = line.kind;
                        last.measure_pos = line.measure_pos;
                        last.show_measure_label = line.show_measure_label;
                    }
                    continue;
                }
            }
            deduped.push(line);
        }
        deduped
    }

    fn point_index_at_or_before_time(&self, time_ms: f32) -> usize {
        match self
            .points
            .binary_search_by(|point| point.time_ms.total_cmp(&time_ms))
        {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(next_idx) => next_idx.saturating_sub(1),
        }
    }

    fn point_index_at_or_before_beat(&self, beat: f32) -> usize {
        match self
            .points
            .binary_search_by(|point| point.start_beat.total_cmp(&beat))
        {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(next_idx) => next_idx.saturating_sub(1),
        }
    }
}



