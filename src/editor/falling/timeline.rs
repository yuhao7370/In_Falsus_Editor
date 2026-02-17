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

impl BpmTimeline {
    fn from_chart(chart: &Chart) -> Self {
        let mut base_bpm = 120.0_f32;
        let mut base_beats = 4.0_f32;

        for event in &chart.events {
            if let ChartEvent::Chart { bpm, beats } = event {
                base_bpm = *bpm as f32;
                base_beats = (*beats as f32).max(1.0);
                break;
            }
        }

        let mut points = vec![BpmPoint {
            time_ms: 0.0,
            bpm: base_bpm,
            beats_per_measure: base_beats,
            start_beat: 0.0,
        }];

        let mut bpm_events = Vec::new();
        for event in &chart.events {
            if let ChartEvent::Bpm {
                time,
                bpm,
                beats,
                ..
            } = event
            {
                bpm_events.push((*time as f32, *bpm as f32, (*beats as f32).max(1.0)));
            }
        }

        bpm_events.sort_by(|a, b| a.0.total_cmp(&b.0));

        for (time_ms, bpm, beats_per_measure) in bpm_events {
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
        let mut active = self.points[0];
        for point in &self.points {
            if point.time_ms <= time_ms {
                active = *point;
            } else {
                break;
            }
        }
        active
    }

    fn time_to_beat(&self, time_ms: f32) -> f32 {
        let point = self.point_at_time(time_ms);
        let bpm = point.bpm.abs().max(0.001);
        point.start_beat + (time_ms - point.time_ms) / 60_000.0 * bpm
    }

    fn beat_to_time(&self, beat: f32) -> f32 {
        for idx in 0..self.points.len() {
            let point = self.points[idx];
            let next_beat = if idx + 1 < self.points.len() {
                self.points[idx + 1].start_beat
            } else {
                f32::INFINITY
            };
            if beat < next_beat {
                let bpm = point.bpm.abs().max(0.001);
                return point.time_ms + (beat - point.start_beat) * 60_000.0 / bpm;
            }
        }

        let point = *self.points.last().unwrap_or(&BpmPoint {
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

        for idx in 0..self.points.len() {
            let point = self.points[idx];
            let segment_start = point.time_ms;
            let segment_end = if idx + 1 < self.points.len() {
                self.points[idx + 1].time_ms
            } else {
                end_ms + 60_000.0
            };

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

                output.push(BarLine { time_ms: line_time_ms, kind });
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
                    }
                    continue;
                }
            }
            deduped.push(line);
        }
        deduped
    }
}



