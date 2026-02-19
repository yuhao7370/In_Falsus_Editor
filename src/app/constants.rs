pub const TOP_BAR_HEIGHT: f32 = 32.0;
pub const EGUI_MENU_BASE_HEIGHT: f32 = 32.0;

/// 开发模式：debug 构建自动加载指定谱面和音频，release 构建为空编辑器。
pub const DEV_MODE: bool = cfg!(debug_assertions);
// pub const DEV_CHART_PATH: &str = "songs/alamode/alamode3.spc";
// pub const DEV_AUDIO_PATH: &str = "songs/alamode/music.ogg";
pub const DEV_CHART_PATH: &str = "testchart/grievouslady2.spc";
pub const DEV_AUDIO_PATH: &str = "testchart/music.ogg";
