#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Empty,
    Ready,
    Playing,
    Paused,
    Stopped,
    Error,
}

impl PlaybackState {
    /// Whether the player is in a state that can transition to Playing.
    pub fn can_play(self) -> bool {
        matches!(self, Self::Ready | Self::Paused | Self::Stopped)
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Playing | Self::Paused)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    User,
    EndOfTrack,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Loaded { path: String, duration_sec: f32 },
    Started,
    Paused,
    Stopped(StopReason),
    BackendRecovered,
    Error(PlayerError),
}

#[derive(Debug, Clone)]
pub enum PlayerError {
    BackendInit(String),
    BackendRecover(String),
    Io { path: String, message: String },
    Decode(String),
    CreateMusic(String),
    StartPlayback(String),
    PausePlayback(String),
    Seek(String),
    SetVolume(String),
    NoTrackLoaded,
    InvalidSeek { requested: f32, duration: f32 },
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendInit(msg) => write!(f, "backend init failed: {msg}"),
            Self::BackendRecover(msg) => write!(f, "backend recover failed: {msg}"),
            Self::Io { path, message } => write!(f, "read '{path}' failed: {message}"),
            Self::Decode(msg) => write!(f, "decode failed: {msg}"),
            Self::CreateMusic(msg) => write!(f, "create music failed: {msg}"),
            Self::StartPlayback(msg) => write!(f, "start playback failed: {msg}"),
            Self::PausePlayback(msg) => write!(f, "pause playback failed: {msg}"),
            Self::Seek(msg) => write!(f, "seek failed: {msg}"),
            Self::SetVolume(msg) => write!(f, "set volume failed: {msg}"),
            Self::NoTrackLoaded => write!(f, "no track loaded"),
            Self::InvalidSeek { requested, duration } => {
                write!(f, "invalid seek {requested:.3}s (duration {duration:.3}s)")
            }
        }
    }
}

impl std::error::Error for PlayerError {}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub state: PlaybackState,
    pub track_path: Option<String>,
    pub duration_sec: f32,
    pub position_sec: f32,
    pub progress: f32,
    pub volume: f32,
}

impl PlayerSnapshot {
    pub fn can_play(&self) -> bool {
        self.state.can_play()
    }
    pub fn can_pause(&self) -> bool {
        self.state == PlaybackState::Playing
    }
    pub fn can_stop(&self) -> bool {
        self.state.is_active() || self.position_sec > 0.001
    }
    pub fn can_seek(&self) -> bool {
        self.duration_sec > 0.0
    }
}

pub struct SongPlayer {
    audio_manager: AudioManager,
    music: Option<Music>,
    track_path: Option<String>,
    duration_sec: f32,
    state: PlaybackState,
    volume: f32,
    /// Cached position for when backend can't report accurately (paused/seek).
    position_cache: f32,
    pending_event: Option<PlayerEvent>,
    /// After play(), sasa may briefly still report `music.paused() == true`.
    /// Skip the end-of-track / unexpected-pause check for one update cycle.
    skip_pause_check: bool,
}


