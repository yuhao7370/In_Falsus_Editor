use std::fmt;
use std::path::Path;

use sasa::backend::cpal::{CpalBackend, CpalSettings};
use sasa::{AudioClip, AudioManager, Music, MusicParams};

const DEFAULT_TRACK_PATH: &str = "songs/alamode/music.ogg";

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

impl SongPlayer {
    pub fn new() -> Result<Self, PlayerError> {
        let audio_manager = AudioManager::new(CpalBackend::new(CpalSettings::default()))
            .map_err(|err| PlayerError::BackendInit(err.to_string()))?;

        Ok(Self {
            audio_manager,
            music: None,
            track_path: None,
            duration_sec: 0.0,
            state: PlaybackState::Empty,
            volume: 1.0,
            position_cache: 0.0,
            pending_event: None,
            skip_pause_check: false,
        })
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn duration_sec(&self) -> f32 {
        self.duration_sec
    }

    pub fn default_track_path() -> &'static str {
        DEFAULT_TRACK_PATH
    }

    pub fn load_default(&mut self, autoplay: bool) -> Result<(), PlayerError> {
        self.load_file(DEFAULT_TRACK_PATH, autoplay)
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>, autoplay: bool) -> Result<(), PlayerError> {
        let path_string = path.as_ref().to_string_lossy().to_string();
        let bytes = std::fs::read(path.as_ref()).map_err(|err| PlayerError::Io {
            path: path_string.clone(),
            message: err.to_string(),
        })?;

        let clip = AudioClip::new(bytes).map_err(|err| PlayerError::Decode(err.to_string()))?;
        let duration_sec = clip.length();
        let mut music = self.create_music_renderer(clip)?;

        if autoplay {
            music
                .play()
                .map_err(|err| PlayerError::StartPlayback(err.to_string()))?;
            self.state = PlaybackState::Playing;
            self.position_cache = 0.0;
            self.pending_event = Some(PlayerEvent::Started);
        } else {
            // sasa's Music may start playing by default; ensure it's paused.
            let _ = music.pause();
            self.state = PlaybackState::Ready;
            self.position_cache = 0.0;
            self.pending_event = Some(PlayerEvent::Loaded {
                path: path_string.clone(),
                duration_sec,
            });
        }

        self.music = Some(music);
        self.track_path = Some(path_string);
        self.duration_sec = duration_sec;
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), PlayerError> {
        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;
        music
            .play()
            .map_err(|err| PlayerError::StartPlayback(err.to_string()))?;
        self.state = PlaybackState::Playing;
        self.skip_pause_check = true;
        self.pending_event = Some(PlayerEvent::Started);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), PlayerError> {
        let pos = self.current_position_sec();
        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;
        music
            .pause()
            .map_err(|err| PlayerError::PausePlayback(err.to_string()))?;
        self.position_cache = pos;
        self.state = PlaybackState::Paused;
        self.pending_event = Some(PlayerEvent::Paused);
        Ok(())
    }

    pub fn toggle_play_pause(&mut self) -> Result<(), PlayerError> {
        if self.state == PlaybackState::Playing {
            self.pause()
        } else {
            self.play()
        }
    }

    pub fn stop(&mut self) -> Result<(), PlayerError> {
        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;
        music
            .pause()
            .map_err(|err| PlayerError::PausePlayback(err.to_string()))?;
        music
            .seek_to(0.0)
            .map_err(|err| PlayerError::Seek(err.to_string()))?;
        self.state = PlaybackState::Stopped;
        self.position_cache = 0.0;
        self.pending_event = Some(PlayerEvent::Stopped(StopReason::User));
        Ok(())
    }

    pub fn seek_to(&mut self, sec: f32) -> Result<(), PlayerError> {
        if !sec.is_finite() || sec < 0.0 || sec > self.duration_sec {
            return Err(PlayerError::InvalidSeek {
                requested: sec,
                duration: self.duration_sec,
            });
        }

        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;
        music
            .seek_to(sec)
            .map_err(|err| PlayerError::Seek(err.to_string()))?;
        self.position_cache = sec;

        if matches!(self.state, PlaybackState::Ready | PlaybackState::Stopped) {
            self.state = PlaybackState::Paused;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<(), PlayerError> {
        let volume = volume.clamp(0.0, 1.0);
        self.volume = volume;
        if let Some(music) = self.music.as_mut() {
            music
                .set_amplifier(volume)
                .map_err(|err| PlayerError::SetVolume(err.to_string()))?;
        }
        Ok(())
    }

    pub fn update(&mut self) -> Option<PlayerEvent> {
        if let Some(event) = self.pending_event.take() {
            return Some(event);
        }

        if self.audio_manager.consume_broken() {
            match self.audio_manager.start() {
                Ok(()) => return Some(PlayerEvent::BackendRecovered),
                Err(err) => {
                    self.state = PlaybackState::Error;
                    return Some(PlayerEvent::Error(PlayerError::BackendRecover(
                        err.to_string(),
                    )));
                }
            }
        }

        if self.state == PlaybackState::Playing {
            let Some(music) = self.music.as_mut() else {
                self.state = PlaybackState::Error;
                return Some(PlayerEvent::Error(PlayerError::NoTrackLoaded));
            };

            if music.paused() {
                // After play(), sasa may need one cycle before it actually
                // starts; skip this check once to avoid a false Paused event.
                if self.skip_pause_check {
                    self.skip_pause_check = false;
                    return None;
                }

                let position = music.position().clamp(0.0, self.duration_sec.max(0.0));
                let near_end = self.duration_sec > 0.0 && position >= (self.duration_sec - 0.02);
                if near_end {
                    self.state = PlaybackState::Paused;
                    self.position_cache = self.duration_sec.max(0.0);
                    return Some(PlayerEvent::Stopped(StopReason::EndOfTrack));
                } else {
                    self.state = PlaybackState::Paused;
                    self.position_cache = position;
                    return Some(PlayerEvent::Paused);
                }
            } else {
                // Backend is actually playing now, clear the flag.
                self.skip_pause_check = false;
            }
        }

        None
    }

    pub fn snapshot(&mut self) -> PlayerSnapshot {
        let position_sec = if self.state == PlaybackState::Playing {
            // Read live backend position during playback.
            let pos = self.current_position_sec();
            self.position_cache = pos;
            pos.clamp(0.0, self.duration_sec.max(0.0))
        } else {
            self.position_cache.clamp(0.0, self.duration_sec.max(0.0))
        };
        let progress = if self.duration_sec > 0.0 {
            (position_sec / self.duration_sec).clamp(0.0, 1.0)
        } else {
            0.0
        };

        PlayerSnapshot {
            state: self.state,
            track_path: self.track_path.clone(),
            duration_sec: self.duration_sec,
            position_sec,
            progress,
            volume: self.volume,
        }
    }

    fn create_music_renderer(&mut self, clip: AudioClip) -> Result<Music, PlayerError> {
        self.audio_manager
            .create_music(
                clip,
                MusicParams {
                    amplifier: self.volume,
                    ..Default::default()
                },
            )
            .map_err(|err| PlayerError::CreateMusic(err.to_string()))
    }

    fn current_position_sec(&self) -> f32 {
        self.music.as_ref().map(Music::position).unwrap_or(0.0)
    }
}
