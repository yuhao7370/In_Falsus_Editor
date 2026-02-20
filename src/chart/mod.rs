use std::fs;
use std::path::Path;

use serde::Serialize;

/// SPC 协议中 `Ease::Linear` 的编码值。
const EASE_LINEAR_CODE: i32 = 0;
/// SPC 协议中 `Ease::SineOut` 的编码值。
const EASE_SINE_OUT_CODE: i32 = 1;
/// SPC 协议中 `Ease::SineIn` 的编码值。
const EASE_SINE_IN_CODE: i32 = 2;

/// SPC 协议中 `FlickType::Right` 的编码值。
const FLICK_RIGHT_CODE: i32 = 4;
/// SPC 协议中 `FlickType::Left` 的编码值。
const FLICK_LEFT_CODE: i32 = 16;

/// `bpm(...)` 缺省拍号值。
const DEFAULT_BPM_BEATS: f64 = -1.0;
/// `bpm(...)` 第四个保留字段的缺省值。
const DEFAULT_BPM_UNKNOWN: i32 = -1;
/// `skyarea(...)` 缺省 `group_id`。
const DEFAULT_SKYAREA_GROUP_ID: i32 = -1;

include!("codec/types.rs");
include!("codec/methods_parse.rs");
include!("codec/methods_export.rs");
include!("codec/utils.rs");
