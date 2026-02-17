use std::fmt;
use std::path::Path;

use sasa::backend::cpal::{CpalBackend, CpalSettings};
use sasa::{AudioClip, AudioManager, Music, MusicParams};

/// 程序启动时默认尝试加载的音频文件路径。
const DEFAULT_TRACK_PATH: &str = "songs/alamode/music.ogg";

include!("player/types.rs");
include!("player/methods_control.rs");
include!("player/methods_runtime.rs");
