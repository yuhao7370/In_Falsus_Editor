fn extract_timeline_events(chart: &Chart) -> Vec<TimelineEvent> {
    let mut events = Vec::new();

    for event in &chart.events {
        match event {
            ChartEvent::Chart { bpm, beats } => events.push(TimelineEvent {
                time_ms: 0.0,
                label: format!("chart {:.2}/{:.2}", bpm, beats),
                color: Color::from_rgba(126, 210, 255, 255),
            }),
            ChartEvent::Bpm { time, bpm, beats, .. } => events.push(TimelineEvent {
                time_ms: *time as f32,
                label: format!("bpm {:.2} (beats {:.2})", bpm, beats),
                color: Color::from_rgba(124, 226, 255, 255),
            }),
            ChartEvent::Track { time, speed } => {
                let color = if *speed >= 0.0 {
                    Color::from_rgba(150, 240, 170, 255)
                } else {
                    Color::from_rgba(255, 168, 128, 255)
                };
                events.push(TimelineEvent {
                    time_ms: *time as f32,
                    label: format!("track x{:.2}", speed),
                    color,
                });
            }
            ChartEvent::Lane { time, lane, enable } => events.push(TimelineEvent {
                time_ms: *time as f32,
                label: format!("lane {} {}", lane, if *enable { "on" } else { "off" }),
                color: Color::from_rgba(232, 198, 124, 255),
            }),
            _ => {}
        }
    }

    events.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms).then_with(|| a.label.cmp(&b.label)));
    events
}

fn extract_ground_notes(chart: &Chart) -> Vec<GroundNote> {
    let mut notes = Vec::new();
    let mut next_id = 1_u64;

    for event in &chart.events {
        match event {
            ChartEvent::Tap { time, width, lane } => {
                if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                    notes.push(GroundNote {
                        id: next_id,
                        kind: GroundNoteKind::Tap,
                        lane: *lane as usize,
                        time_ms: *time as f32,
                        duration_ms: 0.0,
                        width: (*width as f32).max(0.4),
                        flick_right: true,
                        skyarea_shape: None,
                    });
                    next_id += 1;
                }
            }
            ChartEvent::Hold {
                time,
                lane,
                width,
                duration,
            } => {
                if *lane >= 0 && (*lane as usize) < LANE_COUNT {
                    notes.push(GroundNote {
                        id: next_id,
                        kind: GroundNoteKind::Hold,
                        lane: *lane as usize,
                        time_ms: *time as f32,
                        duration_ms: (*duration as f32).max(0.0),
                        width: (*width as f32).max(0.4),
                        flick_right: true,
                        skyarea_shape: None,
                    });
                    next_id += 1;
                }
            }
            ChartEvent::Flick {
                time,
                x,
                x_split,
                width,
                flick_type,
            } => {
                notes.push(GroundNote {
                    id: next_id,
                    kind: GroundNoteKind::Flick,
                    lane: lane_from_normalized_x((*x as f32) / (*x_split as f32).max(1.0)),
                    time_ms: *time as f32,
                    duration_ms: 0.0,
                    width: normalized_width_to_air_ratio((*width as f32) / (*x_split as f32).max(1.0)),
                    flick_right: !matches!(flick_type, FlickType::Left),
                    skyarea_shape: None,
                });
                next_id += 1;
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
                ..
            } => {
                let start_split = (*start_x_split as f32).max(1.0);
                let end_split = (*end_x_split as f32).max(1.0);
                // skyarea 的 X 语义是中心点，left/right 由中心点和宽度展开。
                let start_center = (*start_x as f32) / start_split;
                let end_center = (*end_x as f32) / end_split;
                let start_half = ((*start_width as f32) / start_split).abs() * 0.5;
                let end_half = ((*end_width as f32) / end_split).abs() * 0.5;

                let start_left = (start_center - start_half).clamp(0.0, 1.0);
                let start_right = (start_center + start_half).clamp(0.0, 1.0);
                let end_left = (end_center - end_half).clamp(0.0, 1.0);
                let end_right = (end_center + end_half).clamp(0.0, 1.0);

                let avg_width_norm = (((*start_width as f32) / start_split).abs()
                    + ((*end_width as f32) / end_split).abs())
                    * 0.5;
                notes.push(GroundNote {
                    id: next_id,
                    kind: GroundNoteKind::SkyArea,
                    lane: lane_from_normalized_x((start_center + end_center) * 0.5),
                    time_ms: *time as f32,
                    duration_ms: (*duration as f32).max(0.0),
                    width: normalized_width_to_air_ratio(avg_width_norm),
                    flick_right: true,
                    skyarea_shape: Some(SkyAreaShape {
                        start_left_norm: start_left,
                        start_right_norm: start_right,
                        end_left_norm: end_left,
                        end_right_norm: end_right,
                        left_ease: *left_ease,
                        right_ease: *right_ease,
                    }),
                });
                next_id += 1;
            }
            _ => {}
        }
    }

    notes.sort_by(|a, b| {
        a.time_ms
            .total_cmp(&b.time_ms)
            .then_with(|| a.lane.cmp(&b.lane))
            .then_with(|| a.id.cmp(&b.id))
    });
    notes
}

fn lane_from_normalized_x(norm_x: f32) -> usize {
    let central = (norm_x.clamp(0.0, 0.999_9) * 4.0).floor() as usize;
    (central + 1).min(LANE_COUNT - 1)
}

fn normalized_width_to_air_ratio(width_norm: f32) -> f32 {
    width_norm.abs().clamp(0.05, 1.0)
}

fn lane_to_air_x_norm(lane: usize) -> f32 {
    let lane4 = lane.clamp(1, 4);
    ((lane4 as f32) - 0.5) / 4.0
}

fn air_x_to_lane(x_norm: f32) -> usize {
    ((x_norm.clamp(0.0, 0.999_9) * 4.0).floor() as usize + 1).clamp(1, 4)
}

fn air_split_rect(rect: Rect) -> Rect {
    let lane_w = rect.w / LANE_COUNT as f32;
    Rect::new(rect.x + lane_w, rect.y, lane_w * 4.0, rect.h)
}

fn air_note_width(note: &GroundNote, total_width: f32) -> f32 {
    let width_norm = match note.kind {
        GroundNoteKind::Flick => note.width.clamp(0.05, 1.0),
        GroundNoteKind::SkyArea => note.width.clamp(0.05, 1.0),
        _ => note.width.clamp(0.05, 1.0),
    };
    width_norm * total_width
}


