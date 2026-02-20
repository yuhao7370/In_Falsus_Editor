/// 缓动类型（用于 SkyArea 左右边界插值）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Ease {
    Linear,
    SineOut,
    SineIn,
}

impl Ease {
    pub fn from_value(v: i32) -> Self {
        match v {
            EASE_SINE_OUT_CODE => Ease::SineOut,
            EASE_SINE_IN_CODE => Ease::SineIn,
            _ => Ease::Linear,
        }
    }

    pub fn to_value(self) -> i32 {
        match self {
            Ease::Linear => EASE_LINEAR_CODE,
            Ease::SineOut => EASE_SINE_OUT_CODE,
            Ease::SineIn => EASE_SINE_IN_CODE,
        }
    }
}

/// Flick 方向。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum FlickType {
    Right,
    Left,
    Other(i32),
}

impl FlickType {
    pub fn from_value(v: i32) -> Self {
        match v {
            FLICK_RIGHT_CODE => FlickType::Right,
            FLICK_LEFT_CODE => FlickType::Left,
            _ => FlickType::Other(v),
        }
    }

    pub fn to_value(self) -> i32 {
        match self {
            FlickType::Right => FLICK_RIGHT_CODE,
            FlickType::Left => FLICK_LEFT_CODE,
            FlickType::Other(v) => v,
        }
    }
}

/// 谱面事件。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChartEvent {
    Chart { bpm: f64, beats: f64 },
    Tap { time: f64, width: f64, lane: i32 },
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
        unknown: i32,
    },
    Track { time: f64, speed: f64 },
    Lane { time: f64, lane: i32, enable: bool },
    Beam { raw: String },
    Unknown { raw: String },
}

/// 完整谱面数据。
#[derive(Debug, Clone, Serialize)]
pub struct Chart {
    pub events: Vec<ChartEvent>,
}
