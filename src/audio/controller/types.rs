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
    volume: f32,
}

