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
            Self::BackendInit(message) => write!(f, "backend init failed: {message}"),
            Self::BackendRecover(message) => write!(f, "backend recover failed: {message}"),
            Self::Io { path, message } => write!(f, "read '{path}' failed: {message}"),
            Self::Decode(message) => write!(f, "decode failed: {message}"),
            Self::CreateMusic(message) => write!(f, "create music failed: {message}"),
            Self::StartPlayback(message) => write!(f, "start playback failed: {message}"),
            Self::PausePlayback(message) => write!(f, "pause playback failed: {message}"),
            Self::Seek(message) => write!(f, "seek failed: {message}"),
            Self::SetVolume(message) => write!(f, "set volume failed: {message}"),
            Self::NoTrackLoaded => write!(f, "no track loaded"),
            Self::InvalidSeek {
                requested,
                duration,
            } => write!(f, "invalid seek {requested:.3}s (duration {duration:.3}s)"),
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
    pub can_play: bool,
    pub can_pause: bool,
    pub can_stop: bool,
    pub can_seek: bool,
}

pub struct SongPlayer {
    audio_manager: AudioManager,
    music: Option<Music>,
    track_path: Option<String>,
    duration_sec: f32,
    state: PlaybackState,
    volume: f32,
    // When backend is paused, renderer may not refresh shared position immediately.
    // Keep a UI-side position override so seek is reflected right away.
    paused_position_override_sec: Option<f32>,
    pending_event: Option<PlayerEvent>,
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
            paused_position_override_sec: Some(0.0),
            pending_event: None,
        })
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
            self.paused_position_override_sec = None;
            self.pending_event = Some(PlayerEvent::Started);
        } else {
            self.state = PlaybackState::Ready;
            self.paused_position_override_sec = Some(0.0);
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
        self.paused_position_override_sec = None;
        self.pending_event = Some(PlayerEvent::Started);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), PlayerError> {
        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;
        music
            .pause()
            .map_err(|err| PlayerError::PausePlayback(err.to_string()))?;
        self.state = PlaybackState::Paused;
        self.paused_position_override_sec = Some(self.current_position_sec());
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
        self.paused_position_override_sec = Some(0.0);
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
        self.paused_position_override_sec = Some(sec);

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
                let position = music.position().clamp(0.0, self.duration_sec.max(0.0));
                let near_end = self.duration_sec > 0.0 && position >= (self.duration_sec - 0.02);
                if near_end {
                    if let Err(err) = music.seek_to(0.0) {
                        self.state = PlaybackState::Error;
                        return Some(PlayerEvent::Error(PlayerError::Seek(err.to_string())));
                    }
                    self.state = PlaybackState::Stopped;
                    self.paused_position_override_sec = Some(0.0);
                    return Some(PlayerEvent::Stopped(StopReason::EndOfTrack));
                }
            }
        }

        None
    }

    pub fn snapshot(&mut self) -> PlayerSnapshot {
        let backend_position = self.current_position_sec().clamp(0.0, self.duration_sec.max(0.0));
        let position_sec = if self.state == PlaybackState::Playing {
            self.paused_position_override_sec = None;
            backend_position
        } else {
            self.paused_position_override_sec
                .unwrap_or(backend_position)
                .clamp(0.0, self.duration_sec.max(0.0))
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
            can_play: self.music.is_some()
                && matches!(
                    self.state,
                    PlaybackState::Ready | PlaybackState::Paused | PlaybackState::Stopped
                ),
            can_pause: self.music.is_some() && self.state == PlaybackState::Playing,
            can_stop: self.music.is_some()
                && (self.state == PlaybackState::Playing
                    || self.state == PlaybackState::Paused
                    || position_sec > 0.001),
            can_seek: self.music.is_some() && self.duration_sec > 0.0,
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
