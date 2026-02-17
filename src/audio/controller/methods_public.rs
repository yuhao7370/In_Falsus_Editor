impl AudioController {
    pub fn new(i18n: &I18n, default_track_path: &str) -> Self {
        let (mut player, status) = match SongPlayer::new() {
            Ok(mut p) => {
                let s = if let Err(e) = p.load_file(default_track_path, false) {
                    format_error(&e, i18n)
                } else {
                    format!(
                        "{}: {}",
                        i18n.t(TextKey::StatusLoaded),
                        default_track_path
                    )
                };
                (Some(p), s)
            }
            Err(e) => (None, format_error(&e, i18n)),
        };

        // Read initial metadata from player.
        let (duration_sec, track_path, volume) = match &mut player {
            Some(p) => {
                let snap = p.snapshot();
                (snap.duration_sec, snap.track_path.clone(), snap.volume)
            }
            None => (0.0, None, 1.0),
        };

        Self {
            player,
            status,
            anchor_pos: 0.0,
            anchor_time: get_time(),
            playing: false,
            duration_sec,
            track_path,
            volume,
        }
    }

    // Per-frame update

    /// Call once per frame. Polls player events (end-of-track, backend
    /// recovery, etc.), syncs metadata, and re-anchors our position
    /// to the backend's real `Music::position()` so we never drift.
    pub fn tick(&mut self, i18n: &I18n) {
        // Pre-compute self-timed position before borrowing player mutably.
        let cur = self.current_sec();

        if let Some(p) = self.player.as_mut() {
            // 1. Poll events from the backend.
            if let Some(event) = p.update() {
                match &event {
                    PlayerEvent::Stopped(StopReason::EndOfTrack) => {
                        self.playing = false;
                        self.anchor_pos = self.duration_sec;
                    }
                    PlayerEvent::Paused => {
                        if self.playing {
                            self.anchor_pos = cur;
                            self.playing = false;
                        }
                    }
                    PlayerEvent::Error(_) => {
                        self.playing = false;
                    }
                    _ => {}
                }
                self.status = format_event(event, i18n);
            }

            // 2. Sync metadata.
            let snap = p.snapshot();
            self.duration_sec = snap.duration_sec;
            self.track_path = snap.track_path;
            self.volume = snap.volume;

            // 3. Re-anchor to backend position every frame to prevent drift.
            if self.playing {
                let backend_pos = snap.position_sec;
                if backend_pos.is_finite() && backend_pos >= 0.0 {
                    self.anchor_pos = backend_pos;
                    self.anchor_time = get_time();
                }
            }
        }
    }

    // Getters (always reflect this-frame state)

    /// Current playback position in seconds, computed from the
    /// anchor pair. Guaranteed to be consistent within a single frame
    /// regardless of when play/pause/seek happened.
    pub fn current_sec(&self) -> f32 {
        if self.playing {
            let elapsed = (get_time() - self.anchor_time) as f32;
            (self.anchor_pos + elapsed).clamp(0.0, self.duration_sec)
        } else {
            self.anchor_pos.clamp(0.0, self.duration_sec)
        }
    }

    pub fn duration_sec(&self) -> f32 {
        self.duration_sec
    }

    pub fn track_path(&self) -> Option<&str> {
        self.track_path.as_deref()
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn has_player(&self) -> bool {
        self.player.is_some()
    }

    // Actions

    pub fn toggle_play_pause(&mut self, i18n: &I18n) {
        if self.playing {
            self.do_pause(i18n);
        } else {
            self.do_play(i18n);
        }
    }

    pub fn seek_to(&mut self, sec: f32, i18n: &I18n) {
        let sec = sec.clamp(0.0, self.duration_sec);
        if let Some(p) = self.player.as_mut() {
            if let Err(e) = p.seek_to(sec) {
                self.status = format_error(&e, i18n);
                return;
            }
        }
        // Update anchor atomically.
        self.anchor_pos = sec;
        self.anchor_time = get_time();
    }

    pub fn set_volume(&mut self, volume: f32, i18n: &I18n) {
        if let Some(p) = self.player.as_mut() {
            match p.set_volume(volume) {
                Ok(()) => {
                    self.volume = volume.clamp(0.0, 1.0);
                    self.status = format!(
                        "{}: {:.0}%",
                        i18n.t(TextKey::StatusVolumeUpdated),
                        self.volume * 100.0
                    );
                }
                Err(e) => self.status = format_error(&e, i18n),
            }
        }
    }

    // Input handling

    pub fn handle_keyboard(&mut self, i18n: &I18n) -> bool {
        let space = is_key_pressed(KeyCode::Space);
        if space {
            self.toggle_play_pause(i18n);
        }

        space
    }

    pub fn handle_wheel_seek(
        &mut self,
        mq_wheel_y: f32,
        egui_wheel_y: f32,
        space_consumed: bool,
        i18n: &I18n,
    ) {
        if self.duration_sec <= 0.0 || self.playing || space_consumed {
            return;
        }

        let wheel_units = if mq_wheel_y.abs() > f32::EPSILON {
            normalize_wheel_units(mq_wheel_y)
        } else {
            normalize_wheel_units(egui_wheel_y)
        };

        if wheel_units.abs() <= f32::EPSILON {
            return;
        }

        // Ctrl+wheel is handled elsewhere (flow speed adjustment), skip seek.
        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            return;
        }

        let denom = if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
            WHEEL_SEEK_DIV_ALT
        } else {
            WHEEL_SEEK_DIV_DEFAULT
        };
        let pos = self.current_sec();
        let delta = (wheel_units / denom) * WHEEL_SEEK_SPEED_MULT;
        let target = (pos + delta).clamp(0.0, self.duration_sec);
        self.seek_to(target, i18n);
    }

    pub fn handle_editor_seek(&mut self, sec: f32, i18n: &I18n) {
        self.seek_to(sec, i18n);
    }


}

