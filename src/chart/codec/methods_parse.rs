impl Chart {
pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("读取文件失败: {e}"))?;
    Self::parse(&content)
}

/// 从 `.spc` 文本解析为 Chart。
pub fn parse(content: &str) -> Result<Self, String> {
    let mut events = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        events.push(Self::parse_line(line));
    }
    Ok(Chart { events })
}

fn parse_line(line: &str) -> ChartEvent {
    // 提取 `name(arg1,arg2,...)` 的函数名与参数体。
    let Some(paren_start) = line.find('(') else {
        return ChartEvent::Unknown {
            raw: line.to_string(),
        };
    };
    let Some(paren_end) = line.rfind(')') else {
        return ChartEvent::Unknown {
            raw: line.to_string(),
        };
    };

    let name = &line[..paren_start];
    let args_str = &line[paren_start + 1..paren_end];
    let args: Vec<&str> = args_str.split(',').collect();

    match name {
        "chart" => Self::parse_chart(&args, line),
        "tap" => Self::parse_tap(&args, line),
        "hold" => Self::parse_hold(&args, line),
        "flick" => Self::parse_flick(&args, line),
        "skyarea" => Self::parse_skyarea(&args, line),
        "bpm" => Self::parse_bpm(&args, line),
        "track" => Self::parse_track(&args, line),
        "lane" => Self::parse_lane(&args, line),
        "beam" => ChartEvent::Beam {
            raw: line.to_string(),
        },
        _ => ChartEvent::Unknown {
            raw: line.to_string(),
        },
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

fn parse_i32(s: &str) -> Option<i32> {
    // 支持 "4.00" 这类浮点表示的整数。
    s.trim().parse::<f64>().ok().map(|v| v as i32)
}

fn parse_chart(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 2 {
        if let (Some(bpm), Some(beats)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
            return ChartEvent::Chart { bpm, beats };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_tap(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 3 {
        if let (Some(time), Some(width), Some(lane)) = (
            Self::parse_f64(args[0]),
            Self::parse_f64(args[1]),
            Self::parse_i32(args[2]),
        ) {
            return ChartEvent::Tap { time, width, lane };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_hold(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 4 {
        if let (Some(time), Some(lane), Some(width), Some(duration)) = (
            Self::parse_f64(args[0]),
            Self::parse_i32(args[1]),
            Self::parse_f64(args[2]),
            Self::parse_f64(args[3]),
        ) {
            return ChartEvent::Hold {
                time,
                lane,
                width,
                duration,
            };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_flick(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 5 {
        if let (Some(time), Some(x), Some(x_split), Some(width), Some(ft)) = (
            Self::parse_f64(args[0]),
            Self::parse_f64(args[1]),
            Self::parse_f64(args[2]),
            Self::parse_f64(args[3]),
            Self::parse_i32(args[4]),
        ) {
            return ChartEvent::Flick {
                time,
                x,
                x_split,
                width,
                flick_type: FlickType::from_value(ft),
            };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_skyarea(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 10 {
        if let (
            Some(time),
            Some(start_x),
            Some(start_x_split),
            Some(start_width),
            Some(end_x),
            Some(end_x_split),
            Some(end_width),
            Some(left_ease),
            Some(right_ease),
            Some(duration),
        ) = (
            Self::parse_f64(args[0]),
            Self::parse_f64(args[1]),
            Self::parse_f64(args[2]),
            Self::parse_f64(args[3]),
            Self::parse_f64(args[4]),
            Self::parse_f64(args[5]),
            Self::parse_f64(args[6]),
            Self::parse_i32(args[7]),
            Self::parse_i32(args[8]),
            Self::parse_f64(args[9]),
        ) {
            let group_id = if args.len() >= 11 {
                Self::parse_i32(args[10]).unwrap_or(DEFAULT_SKYAREA_GROUP_ID)
            } else {
                DEFAULT_SKYAREA_GROUP_ID
            };
            return ChartEvent::SkyArea {
                time,
                start_x,
                start_x_split,
                start_width,
                end_x,
                end_x_split,
                end_width,
                left_ease: Ease::from_value(left_ease),
                right_ease: Ease::from_value(right_ease),
                duration,
                group_id,
            };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_bpm(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 2 {
        if let (Some(time), Some(bpm)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
            let beats = if args.len() >= 3 {
                Self::parse_f64(args[2]).unwrap_or(DEFAULT_BPM_BEATS)
            } else {
                DEFAULT_BPM_BEATS
            };
            let unknown = if args.len() >= 4 {
                Self::parse_f64(args[3]).unwrap_or(DEFAULT_BPM_UNKNOWN)
            } else {
                DEFAULT_BPM_UNKNOWN
            };
            return ChartEvent::Bpm {
                time,
                bpm,
                beats,
                unknown,
            };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_track(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 2 {
        if let (Some(time), Some(speed)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
            return ChartEvent::Track { time, speed };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}

fn parse_lane(args: &[&str], raw: &str) -> ChartEvent {
    if args.len() >= 3 {
        if let (Some(time), Some(lane), Some(enable)) = (
            Self::parse_f64(args[0]),
            Self::parse_i32(args[1]),
            Self::parse_i32(args[2]),
        ) {
            return ChartEvent::Lane {
                time,
                lane,
                enable: enable != 0,
            };
        }
    }
    ChartEvent::Unknown {
        raw: raw.to_string(),
    }
}


}

