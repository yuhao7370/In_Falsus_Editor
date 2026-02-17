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
        // 与 visible_barlines 一致：在当前 BPM 段内按 sub_ms 网格吸附
        let idx = self.point_index_at_or_before_time(time_ms);
        let point = self.points[idx];
        let bpm = point.bpm.abs().max(0.001);
        let beat_ms = 60_000.0 / bpm;
        let sub_ms = beat_ms / division as f32;
        let offset = time_ms - point.time_ms;
        let n = (offset / sub_ms).round() as i32;
        let snapped = point.time_ms + n as f32 * sub_ms;
        // 如果 snap 结果超出当前段（进入下一段），则 clamp 到段边界
        if idx + 1 < self.points.len() && snapped > self.points[idx + 1].time_ms {
            // 比较段边界和前一个网格点，取更近的
            let boundary = self.points[idx + 1].time_ms;
            let prev = point.time_ms + (n - 1).max(0) as f32 * sub_ms;
            if (boundary - time_ms).abs() < (prev - time_ms).abs() {
                return boundary.max(0.0);
            }
            return prev.max(0.0);
        }
        snapped.max(0.0)
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

            let is_negative_bpm = point.bpm < 0.0;
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

                let beat = if is_negative_bpm {
                    point.start_beat - n as f32 / subdivision as f32
                } else {
                    point.start_beat + n as f32 / subdivision as f32
                };
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

                // Label in global beat space (continuous 0,1,2,3,4...),
                // instead of resetting every measure.
                // 1x/odd-x -> every whole beat
                // even-x(>=2) -> every half beat
                let label_step_beats = if subdivision > 1 && subdivision % 2 == 0 {
                    0.5
                } else {
                    1.0
                };
                let label_grid = (beat / label_step_beats).round();
                let show_measure_label = (beat / label_step_beats - label_grid).abs() < 0.001;
                let measure_pos = label_grid * label_step_beats;

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


// ─── TrackTimeline: 视觉位移积分（纯 track_speed）───
//
// 视觉速度完全由 track speed 决定，与 BPM 无关。
// BPM 只用于计算小节线的时间位置（BpmTimeline 负责）。
// 小节线和音符的 Y 坐标都通过 TrackTimeline 计算。
//
// visual_velocity(t) = track_speed(t)
// visual_position(t) = ∫ track_speed(τ) dτ
//
// 默认 track speed = 1.0（没有 track 事件时匀速）。
// 负 track speed 表示倒流（音符从下往上走）。

/// 一个 track speed 变化点。
#[derive(Debug, Clone, Copy)]
struct TrackPoint {
    time_ms: f32,
    speed: f32,
    /// 从 t=0 到此点的累计视觉位移（单位：拍）。
    start_visual_beat: f32,
}

/// Track 速度事件的原始数据。
#[derive(Debug, Clone)]
struct TrackSourceData {
    track_events: Vec<(f32, f32)>, // (time_ms, speed)
}

impl Default for TrackSourceData {
    fn default() -> Self {
        Self {
            track_events: Vec::new(),
        }
    }
}

/// 视觉位移时间轴：通过 track_speed 的分段积分，
/// 将时间映射到视觉位置（单位：毫秒等效）。
///
/// - 小节线位置由 BpmTimeline 决定（纯 BPM）
/// - 音符和小节线的 Y 坐标都由 TrackTimeline 决定
#[derive(Debug, Clone)]
struct TrackTimeline {
    points: Vec<TrackPoint>,
}

impl TrackTimeline {
    /// 从 track 事件构建视觉位移时间轴。
    /// bpm_timeline 参数保留以备将来扩展，当前不使用。
    fn from_source(_bpm_timeline: &BpmTimeline, source: TrackSourceData) -> Self {
        // 构建 track speed 查找表（按时间排序）
        let mut speed_points: Vec<(f32, f32)> = vec![(0.0, 1.0)];
        let mut sorted_events = source.track_events.clone();
        sorted_events.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (t, s) in sorted_events {
            if t <= 0.0 {
                speed_points[0].1 = s;
            } else if speed_points.last().map(|p| (p.0 - t).abs() < 0.000_1).unwrap_or(false) {
                speed_points.last_mut().unwrap().1 = s;
            } else {
                speed_points.push((t, s));
            }
        }

        // 构建 TrackPoint 列表，计算累计视觉位移
        let mut points = Vec::with_capacity(speed_points.len());
        points.push(TrackPoint {
            time_ms: 0.0,
            speed: speed_points[0].1,
            start_visual_beat: 0.0,
        });

        for i in 1..speed_points.len() {
            let prev_time = speed_points[i - 1].0;
            let prev_speed = speed_points[i - 1].1;
            let curr_time = speed_points[i].0;
            let dt = (curr_time - prev_time).max(0.0);
            let prev_visual = points.last().unwrap().start_visual_beat;

            // 积分：track_speed * dt_ms（纯 track speed，不含 BPM）
            let delta_visual = prev_speed * dt;

            points.push(TrackPoint {
                time_ms: curr_time,
                speed: speed_points[i].1,
                start_visual_beat: prev_visual + delta_visual,
            });
        }

        Self { points }
    }

    /// 计算从 t=0 到 time_ms 的视觉位移。
    fn visual_beat_at(&self, time_ms: f32) -> f32 {
        let idx = self.point_index_at_or_before(time_ms);
        let pt = self.points[idx];
        let dt = (time_ms - pt.time_ms).max(0.0);
        pt.start_visual_beat + pt.speed * dt
    }

    /// 从视觉位移反查时间（用于 pointer_to_time）。
    fn visual_beat_to_time(&self, target_vb: f32) -> f32 {
        if self.points.is_empty() {
            return 0.0;
        }

        // 找到 target_vb 所在的段
        let mut idx = 0;
        for i in 0..self.points.len() {
            if self.points[i].start_visual_beat <= target_vb + 0.000_1 {
                idx = i;
            } else {
                break;
            }
        }

        let pt = self.points[idx];
        let rate = pt.speed;

        if rate.abs() < 0.000_001 {
            // 速度为 0，无法反推，返回段起始时间
            return pt.time_ms;
        }

        pt.time_ms + (target_vb - pt.start_visual_beat) / rate
    }

    fn point_index_at_or_before(&self, time_ms: f32) -> usize {
        match self
            .points
            .binary_search_by(|p| p.time_ms.total_cmp(&time_ms))
        {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(next_idx) => next_idx.saturating_sub(1),
        }
    }
}



