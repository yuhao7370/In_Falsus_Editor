use std::fs;
use std::path::Path;
use serde::Serialize;

/// 缓动类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Ease {
    Linear,  // 0
    SineOut, // 1
    SineIn,  // 2
}

impl Ease {
    pub fn from_value(v: i32) -> Self {
        match v {
            1 => Ease::SineOut,
            2 => Ease::SineIn,
            _ => Ease::Linear,
        }
    }

    pub fn to_value(self) -> i32 {
        match self {
            Ease::Linear => 0,
            Ease::SineOut => 1,
            Ease::SineIn => 2,
        }
    }
}

/// Flick 方向
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum FlickType {
    Right, // 4
    Left,  // 16
    Other(i32),
}

impl FlickType {
    pub fn from_value(v: i32) -> Self {
        match v {
            4 => FlickType::Right,
            16 => FlickType::Left,
            _ => FlickType::Other(v),
        }
    }

    pub fn to_value(self) -> i32 {
        match self {
            FlickType::Right => 4,
            FlickType::Left => 16,
            FlickType::Other(v) => v,
        }
    }
}

/// 谱面事件
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChartEvent {
    Chart {
        bpm: f64,
        beats: f64,
    },
    Tap {
        time: f64,
        width: f64,
        lane: i32,
    },
    Hold {
        time: f64,
        lane: i32,
        width: f64,
        duration: f64,
    },
    Flick {
        time: f64,
        x: f64,
        x_split: f64,
        width: f64,
        flick_type: FlickType,
    },
    SkyArea {
        time: f64,
        start_x: f64,
        start_x_split: f64,
        start_width: f64,
        end_x: f64,
        end_x_split: f64,
        end_width: f64,
        left_ease: Ease,
        right_ease: Ease,
        duration: f64,
        group_id: i32,
    },
    Bpm {
        time: f64,
        bpm: f64,
        beats: f64,
        unknown: f64,
    },
    Track {
        time: f64,
        speed: f64,
    },
    Lane {
        time: f64,
        lane: i32,
        enable: bool,
    },
    Beam {
        raw: String,
    },
    Unknown {
        raw: String,
    },
}

/// 谱面数据
#[derive(Debug, Clone, Serialize)]
pub struct Chart {
    pub events: Vec<ChartEvent>,
}

impl Chart {
    /// 从 .spc 文件解析
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("读取文件失败: {e}"))?;
        Self::parse(&content)
    }

    /// 从字符串解析
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
        // 提取 typename 和参数部分
        let Some(paren_start) = line.find('(') else {
            return ChartEvent::Unknown { raw: line.to_string() };
        };
        let Some(paren_end) = line.rfind(')') else {
            return ChartEvent::Unknown { raw: line.to_string() };
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
            "beam" => ChartEvent::Beam { raw: line.to_string() },
            _ => ChartEvent::Unknown { raw: line.to_string() },
        }
    }

    fn parse_f64(s: &str) -> Option<f64> {
        s.trim().parse::<f64>().ok()
    }

    fn parse_i32(s: &str) -> Option<i32> {
        // 支持浮点数形式的整数如 "4.00"
        s.trim().parse::<f64>().ok().map(|v| v as i32)
    }

    fn parse_chart(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 2 {
            if let (Some(bpm), Some(beats)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
                return ChartEvent::Chart { bpm, beats };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
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
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    fn parse_hold(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 4 {
            if let (Some(time), Some(lane), Some(width), Some(duration)) = (
                Self::parse_f64(args[0]),
                Self::parse_i32(args[1]),
                Self::parse_f64(args[2]),
                Self::parse_f64(args[3]),
            ) {
                return ChartEvent::Hold { time, lane, width, duration };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
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
                    time, x, x_split, width,
                    flick_type: FlickType::from_value(ft),
                };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    fn parse_skyarea(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 10 {
            if let (
                Some(time), Some(sx), Some(sxs), Some(sw),
                Some(ex), Some(exs), Some(ew),
                Some(le), Some(re), Some(dur),
            ) = (
                Self::parse_f64(args[0]), Self::parse_f64(args[1]),
                Self::parse_f64(args[2]), Self::parse_f64(args[3]),
                Self::parse_f64(args[4]), Self::parse_f64(args[5]),
                Self::parse_f64(args[6]), Self::parse_i32(args[7]),
                Self::parse_i32(args[8]), Self::parse_f64(args[9]),
            ) {
                let group_id = if args.len() >= 11 {
                    Self::parse_i32(args[10]).unwrap_or(0)
                } else {
                    0
                };
                return ChartEvent::SkyArea {
                    time, start_x: sx, start_x_split: sxs, start_width: sw,
                    end_x: ex, end_x_split: exs, end_width: ew,
                    left_ease: Ease::from_value(le), right_ease: Ease::from_value(re),
                    duration: dur, group_id,
                };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    fn parse_bpm(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 2 {
            if let (Some(time), Some(bpm)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
                let beats = if args.len() >= 3 { Self::parse_f64(args[2]).unwrap_or(4.0) } else { 4.0 };
                let unknown = if args.len() >= 4 { Self::parse_f64(args[3]).unwrap_or(0.0) } else { 0.0 };
                return ChartEvent::Bpm { time, bpm, beats, unknown };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    fn parse_track(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 2 {
            if let (Some(time), Some(speed)) = (Self::parse_f64(args[0]), Self::parse_f64(args[1])) {
                return ChartEvent::Track { time, speed };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    fn parse_lane(args: &[&str], raw: &str) -> ChartEvent {
        if args.len() >= 3 {
            if let (Some(time), Some(lane), Some(enable)) = (
                Self::parse_f64(args[0]),
                Self::parse_i32(args[1]),
                Self::parse_i32(args[2]),
            ) {
                return ChartEvent::Lane { time, lane, enable: enable != 0 };
            }
        }
        ChartEvent::Unknown { raw: raw.to_string() }
    }

    /// 序列化为 .spc 格式字符串（V2/V3 写回）
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
                ChartEvent::Hold { time, lane, width, duration } => {
                    lines.push(format!("hold({},{},{},{})", fmt_num(*time), lane, fmt_num(*width), fmt_num(*duration)));
                }
                ChartEvent::Flick { time, x, x_split, width, flick_type } => {
                    lines.push(format!("flick({},{},{},{},{})",
                        fmt_num(*time), fmt_num(*x), fmt_num(*x_split), fmt_num(*width), flick_type.to_value()));
                }
                ChartEvent::SkyArea { time, start_x, start_x_split, start_width,
                    end_x, end_x_split, end_width, left_ease, right_ease, duration, group_id } => {
                    lines.push(format!("skyarea({},{},{},{},{},{},{},{},{},{},{})",
                        fmt_num(*time), fmt_num(*start_x), fmt_num(*start_x_split), fmt_num(*start_width),
                        fmt_num(*end_x), fmt_num(*end_x_split), fmt_num(*end_width),
                        left_ease.to_value(), right_ease.to_value(), fmt_num(*duration), group_id));
                }
                ChartEvent::Bpm { time, bpm, beats, unknown } => {
                    lines.push(format!("bpm({},{:.2},{:.2},{:.2})", fmt_num(*time), bpm, beats, unknown));
                }
                ChartEvent::Track { time, speed } => {
                    lines.push(format!("track({},{:.2})", fmt_num(*time), speed));
                }
                ChartEvent::Lane { time, lane, enable } => {
                    lines.push(format!("lane({},{},{})", fmt_num(*time), lane, if *enable { 1 } else { 0 }));
                }
                ChartEvent::Beam { raw } | ChartEvent::Unknown { raw } => {
                    unknowns.push(raw.clone());
                }
            }
        }

        lines.extend(unknowns);
        lines.join("\n") + "\n"
    }

    // --- 便捷查询方法 ---

    pub fn chart_info(&self) -> Option<(f64, f64)> {
        self.events.iter().find_map(|e| match e {
            ChartEvent::Chart { bpm, beats } => Some((*bpm, *beats)),
            _ => None,
        })
    }

    pub fn tap_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, ChartEvent::Tap { .. })).count()
    }

    pub fn hold_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, ChartEvent::Hold { .. })).count()
    }

    pub fn flick_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, ChartEvent::Flick { .. })).count()
    }

    pub fn skyarea_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, ChartEvent::SkyArea { .. })).count()
    }

    pub fn total_notes(&self) -> usize {
        self.tap_count() + self.hold_count() + self.flick_count()
    }

    /// 导出为 JSON 文件（indent=4）
    pub fn to_json_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("JSON 序列化失败: {e}"))?;
        fs::write(path, json).map_err(|e| format!("写入文件失败: {e}"))
    }
}

/// 格式化数值：整数不带小数点，浮点保留原样
fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}
