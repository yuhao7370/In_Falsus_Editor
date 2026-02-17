use std::fmt;
use std::path::Path;

use sasa::backend::cpal::{CpalBackend, CpalSettings};
use sasa::{AudioClip, AudioManager, Music, MusicParams};


include!("player/types.rs");
include!("player/methods_control.rs");
include!("player/methods_runtime.rs");
