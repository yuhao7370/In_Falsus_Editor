pub const TOP_BAR_HEIGHT: f32 = 32.0;
pub const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;

pub const ANDROID_BUILTIN_PROJECT_CHART: &str = "projects/alamode/alamode.spc";
pub const ANDROID_BUILTIN_PROJECT_AUDIO: &str = "projects/alamode/music.ogg";
pub const ANDROID_BUILTIN_PROJECT_IFFPROJ: &str = "projects/alamode/alamode.iffproj";

// Debug builds auto-open a local test chart/audio pair.
pub const DEV_MODE: bool = cfg!(debug_assertions);
pub const DEV_CHART_PATH: &str = "testchart/grievouslady2.spc";
pub const DEV_AUDIO_PATH: &str = "testchart/music.ogg";
