use crate::audio::player::{PlayerError, PlayerEvent, SongPlayer, StopReason};
use crate::i18n::{I18n, TextKey};
use macroquad::prelude::*;

/// 默认滚轮 seek 灵敏度分母（普通模式）。
const WHEEL_SEEK_DIV_DEFAULT: f32 = 12_000.0;
/// Ctrl 精细调整时使用更大的分母（步进更小）。
const WHEEL_SEEK_DIV_CTRL: f32 = 60_000.0;
/// Alt 快速调整时使用更小的分母（步进更大）。
const WHEEL_SEEK_DIV_ALT: f32 = 3_000.0;
/// 最终滚轮位移倍率。
const WHEEL_SEEK_SPEED_MULT: f32 = 3.0;

include!("controller/types.rs");
include!("controller/methods_public.rs");
include!("controller/methods_private.rs");
include!("controller/helpers.rs");
