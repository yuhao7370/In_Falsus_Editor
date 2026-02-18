use sasa::backend::cpal::{CpalBackend, CpalSettings};
use sasa::{AudioClip, AudioManager, Music, MusicParams};

const ASSETS_TAP: &str = "assets/tap.wav";
const ASSETS_ARC: &str = "assets/arc.wav";

/// Default maximum concurrent hitsound voices.
const DEFAULT_MAX_VOICES: usize = 8;

pub struct HitSoundPlayer {
    audio_manager: AudioManager,
    tap_clip: Option<AudioClip>,
    arc_clip: Option<AudioClip>,
    voices: Vec<Music>,
    tap_volume: f32,
    arc_volume: f32,
    master_volume: f32,
    max_voices: usize,
    enabled: bool,
}

impl HitSoundPlayer {
    pub fn new() -> Self {
        let audio_manager =
            AudioManager::new(CpalBackend::new(CpalSettings::default())).unwrap_or_else(|e| {
                eprintln!("[hitsound] backend init failed: {e}");
                AudioManager::new(CpalBackend::new(CpalSettings::default())).unwrap()
            });

        let tap_clip = Self::load_clip(ASSETS_TAP);
        let arc_clip = Self::load_clip(ASSETS_ARC);

        Self {
            audio_manager,
            tap_clip,
            arc_clip,
            voices: Vec::new(),
            tap_volume: 1.0,
            arc_volume: 1.0,
            master_volume: 1.0,
            max_voices: DEFAULT_MAX_VOICES,
            enabled: true,
        }
    }

    fn load_clip(path: &str) -> Option<AudioClip> {
        match std::fs::read(path) {
            Ok(bytes) => match AudioClip::new(bytes) {
                Ok(clip) => Some(clip),
                Err(e) => {
                    eprintln!("[hitsound] decode '{path}' failed: {e}");
                    None
                }
            },
            Err(e) => {
                eprintln!("[hitsound] read '{path}' failed: {e}");
                None
            }
        }
    }

    pub fn play_tap(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(clip) = &self.tap_clip {
            self.play_clip(clip.clone(), self.tap_volume * self.master_volume);
        }
    }

    pub fn play_arc(&mut self) {
        if !self.enabled {
            return;
        }
        if let Some(clip) = &self.arc_clip {
            self.play_clip(clip.clone(), self.arc_volume * self.master_volume);
        }
    }

    fn play_clip(&mut self, clip: AudioClip, volume: f32) {
        let music = self.audio_manager.create_music(
            clip,
            MusicParams {
                amplifier: volume,
                ..Default::default()
            },
        );
        match music {
            Ok(mut m) => {
                let _ = m.play();
                self.voices.push(m);
                // Evict oldest voices if over limit — explicitly pause before removing
                while self.voices.len() > self.max_voices {
                    let mut old = self.voices.remove(0);
                    let _ = old.pause();
                }
            }
            Err(e) => {
                eprintln!("[hitsound] create music failed: {e}");
            }
        }
    }

    /// Call once per frame.
    /// We intentionally do NOT remove paused voices here, because sasa's
    /// Music::paused() can return true before the audio thread has started
    /// processing the play command, which would kill the sound prematurely.
    /// Voices are only evicted when max_voices is exceeded in play_clip().
    pub fn update(&mut self) {
        // no-op: cleanup is handled by max_voices eviction in play_clip()
    }

    // ── Volume control ──

    pub fn set_tap_volume(&mut self, volume: f32) {
        self.tap_volume = volume.clamp(0.0, 2.0);
    }

    pub fn tap_volume(&self) -> f32 {
        self.tap_volume
    }

    pub fn set_arc_volume(&mut self, volume: f32) {
        self.arc_volume = volume.clamp(0.0, 2.0);
    }

    pub fn arc_volume(&self) -> f32 {
        self.arc_volume
    }

    /// Set both tap and arc volume at once.
    pub fn set_volume(&mut self, volume: f32) {
        let v = volume.clamp(0.0, 2.0);
        self.tap_volume = v;
        self.arc_volume = v;
    }

    // ── Master volume ──

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    // ── Max voices ──

    pub fn set_max_voices(&mut self, max: usize) {
        self.max_voices = max.max(1);
    }

    pub fn max_voices(&self) -> usize {
        self.max_voices
    }

    // ── Enable/disable ──

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            // Stop all active voices immediately
            for voice in self.voices.iter_mut() {
                let _ = voice.pause();
            }
            self.voices.clear();
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Tracks playback position to detect note head crossings.
pub struct HitSoundTrigger {
    prev_sec: f32,
    was_playing: bool,
}

impl HitSoundTrigger {
    pub fn new() -> Self {
        Self {
            prev_sec: 0.0,
            was_playing: false,
        }
    }

    /// Call once per frame.
    /// `note_heads`: slice of `(time_ms, is_ground)`.
    pub fn tick(
        &mut self,
        current_sec: f32,
        is_playing: bool,
        note_heads: &[(f32, bool)],
        player: &mut HitSoundPlayer,
    ) {
        if !is_playing {
            if self.was_playing {
                self.prev_sec = current_sec;
            }
            self.was_playing = false;
            return;
        }

        if !self.was_playing {
            self.prev_sec = current_sec;
            self.was_playing = true;
            return;
        }

        // Detect seek: large backward jump or unreasonably large forward jump
        let delta = current_sec - self.prev_sec;
        if delta < -0.01 || delta > 0.5 {
            self.prev_sec = current_sec;
            return;
        }

        let prev_ms = self.prev_sec * 1000.0;
        let curr_ms = current_sec * 1000.0;

        for &(time_ms, is_ground) in note_heads {
            if time_ms > prev_ms && time_ms <= curr_ms {
                if is_ground {
                    player.play_tap();
                } else {
                    player.play_arc();
                }
            }
        }

        self.prev_sec = current_sec;
    }

    /// Force reset (call on seek from outside).
    pub fn reset(&mut self, sec: f32) {
        self.prev_sec = sec;
    }
}
