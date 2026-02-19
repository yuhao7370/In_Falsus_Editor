/// High-level audio controller.
///
/// Position strategy:
///   - We maintain `playing` ourselves (zero-frame-lag).
///   - While playing, position = `anchor_pos + (now - anchor_time)`.
///   - Each frame in `tick()`, we re-anchor to the backend's real
///     `Music::position()` so we never drift.
///   - On pause/seek, `anchor_pos` is set directly.
pub struct AudioController {
    player: Option<SongPlayer>,
    pub status: String,

    /// Position (seconds) at the moment we last anchored.
    anchor_pos: f32,
    /// `get_time()` timestamp corresponding to `anchor_pos`.
    anchor_time: f64,
    /// Synchronous playing flag (set immediately, no frame lag).
    playing: bool,

    duration_sec: f32,
    track_path: Option<String>,
    /// Dirty flag: only sync track_path/duration from backend when a load occurs.
    metadata_dirty: bool,
    music_volume: f32,
    master_volume: f32,

    hitsound_player: HitSoundPlayer,
    hitsound_trigger: HitSoundTrigger,

    /// 上一帧后端位置，用于计算位置漂移速率
    prev_backend_pos: f32,
    /// 每帧后端位置变化量（用于调试检测倍速异常）
    pos_delta_per_frame: f32,
    /// 平滑估算播放速度（滚动窗口累计）
    speed_accum_pos: f32,
    speed_accum_time: f64,
    estimated_speed: f32,
}

/// 每帧音频状态快照，供编辑器和 UI 组件使用。
/// 由 `AudioController::frame_snapshot()` 生成，避免同一帧内多次调用 getter。
#[derive(Debug, Clone)]
pub struct FrameContext {
    pub current_sec: f32,
    pub duration_sec: f32,
    pub track_path: Option<String>,
    pub is_playing: bool,
}

/// 音频调试快照，供调试窗口显示。
#[derive(Debug, Clone)]
pub struct AudioDebugSnapshot {
    pub playback_state: String,
    pub backend_position: f32,
    pub controller_position: f32,
    pub anchor_pos: f32,
    pub anchor_time: f64,
    pub duration_sec: f32,
    pub effective_volume: f32,
    pub music_volume: f32,
    pub master_volume: f32,
    pub is_playing_ctrl: bool,
    pub pos_delta_per_frame: f32,
    pub estimated_speed: f32,
    pub fps: f32,
    pub delta_time: f32,
    pub has_backend: bool,
    pub track_path: String,
}

