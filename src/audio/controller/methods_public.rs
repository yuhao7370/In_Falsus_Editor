impl AudioController {
    pub fn new(i18n: &I18n, default_track_path: &str) -> Self {
        let (mut player, status) = match SongPlayer::new() {
            Ok(mut p) => {
                let s = if let Err(e) = p.load_file(default_track_path) {
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
        let (duration_sec, track_path, music_volume) = match &mut player {
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
            music_volume,
            master_volume: 1.0,
            hitsound_player: HitSoundPlayer::new(),
            hitsound_trigger: HitSoundTrigger::new(),
            prev_backend_pos: 0.0,
            pos_delta_per_frame: 0.0,
            speed_accum_pos: 0.0,
            speed_accum_time: 0.0,
            estimated_speed: 0.0,
        }
    }

    /// 创建一个不加载任何音频文件的空控制器（仅初始化音频后端）。
    pub fn new_empty(i18n: &I18n) -> Self {
        let (player, status) = match SongPlayer::new() {
            Ok(p) => (Some(p), String::new()),
            Err(e) => (None, format_error(&e, i18n)),
        };

        Self {
            player,
            status,
            anchor_pos: 0.0,
            anchor_time: get_time(),
            playing: false,
            duration_sec: 0.0,
            track_path: None,
            music_volume: 1.0,
            master_volume: 1.0,
            hitsound_player: HitSoundPlayer::new(),
            hitsound_trigger: HitSoundTrigger::new(),
            prev_backend_pos: 0.0,
            pos_delta_per_frame: 0.0,
            speed_accum_pos: 0.0,
            speed_accum_time: 0.0,
            estimated_speed: 0.0,
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
            // Note: we don't sync volume from backend; we manage it ourselves.

            // 3. Re-anchor to backend position every frame to prevent drift.
            if self.playing {
                let backend_pos = snap.position_sec;
                if backend_pos.is_finite() && backend_pos >= 0.0 {
                    // 计算每帧后端位置变化量（用于调试）
                    self.pos_delta_per_frame = backend_pos - self.prev_backend_pos;
                    self.prev_backend_pos = backend_pos;

                    // 累计位置和时间，每 0.5s 刷新一次平滑速度
                    let dt = get_frame_time() as f64;
                    self.speed_accum_pos += self.pos_delta_per_frame;
                    self.speed_accum_time += dt;
                    const SPEED_WINDOW: f64 = 0.5;
                    if self.speed_accum_time >= SPEED_WINDOW {
                        self.estimated_speed = self.speed_accum_pos / self.speed_accum_time as f32;
                        self.speed_accum_pos = 0.0;
                        self.speed_accum_time = 0.0;
                    }

                    self.anchor_pos = backend_pos;
                    self.anchor_time = get_time();
                }
            } else {
                self.pos_delta_per_frame = 0.0;
                self.estimated_speed = 0.0;
                self.speed_accum_pos = 0.0;
                self.speed_accum_time = 0.0;
            }
        }

        // 4. Clean up finished hitsound voices.
        self.hitsound_player.update();
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

    pub fn music_volume(&self) -> f32 {
        self.music_volume
    }

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Effective volume = music_volume × master_volume
    pub fn effective_volume(&self) -> f32 {
        self.music_volume * self.master_volume
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

    pub fn set_music_volume(&mut self, volume: f32, i18n: &I18n) {
        self.music_volume = volume.clamp(0.0, 1.0);
        self.apply_effective_volume(i18n);
    }

    pub fn set_master_volume(&mut self, volume: f32, i18n: &I18n) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.hitsound_player.set_master_volume(self.master_volume);
        self.apply_effective_volume(i18n);
    }

    fn apply_effective_volume(&mut self, i18n: &I18n) {
        let effective = self.effective_volume();
        if let Some(p) = self.player.as_mut() {
            match p.set_volume(effective) {
                Ok(()) => {
                    self.status = format!(
                        "{}: {:.0}%",
                        i18n.t(TextKey::StatusVolumeUpdated),
                        effective * 100.0
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

    pub fn load_audio_file(&mut self, path: &str, i18n: &I18n) {
        if let Some(p) = self.player.as_mut() {
            // Stop current playback first
            let _ = p.pause();
            if let Err(e) = p.load_file(path) {
                self.status = format_error(&e, i18n);
                return;
            }
            let snap = p.snapshot();
            self.duration_sec = snap.duration_sec;
            self.track_path = snap.track_path;
            self.music_volume = snap.volume;
            self.anchor_pos = 0.0;
            self.anchor_time = get_time();
            self.playing = false;
            self.status = format!("{}: {}", i18n.t(TextKey::StatusLoaded), path);
        } else {
            self.status = i18n.t(TextKey::StatusAudioUnavailable).to_owned();
        }
    }

    /// 从已读取的字节加载音频（避免重复读文件，用于异步加载流程）。
    pub fn load_audio_from_bytes(&mut self, bytes: Vec<u8>, path: &str, i18n: &I18n) {
        if let Some(p) = self.player.as_mut() {
            let _ = p.pause();
            if let Err(e) = p.load_from_bytes(bytes, path) {
                self.status = format_error(&e, i18n);
                return;
            }
            let snap = p.snapshot();
            self.duration_sec = snap.duration_sec;
            self.track_path = snap.track_path;
            self.music_volume = snap.volume;
            self.anchor_pos = 0.0;
            self.anchor_time = get_time();
            self.playing = false;
            self.status = format!("{}: {}", i18n.t(TextKey::StatusLoaded), path);
        } else {
            self.status = i18n.t(TextKey::StatusAudioUnavailable).to_owned();
        }
    }

    /// 安装已在后台线程解码完成的 AudioClip（不阻塞主线程）。
    pub fn install_decoded_audio(&mut self, clip: sasa::AudioClip, path: &str, i18n: &I18n) {
        if let Some(p) = self.player.as_mut() {
            let _ = p.pause();
            if let Err(e) = p.install_clip(clip, path) {
                self.status = format_error(&e, i18n);
                return;
            }
            let snap = p.snapshot();
            self.duration_sec = snap.duration_sec;
            self.track_path = snap.track_path;
            self.music_volume = snap.volume;
            self.anchor_pos = 0.0;
            self.anchor_time = get_time();
            self.playing = false;
            self.status = format!("{}: {}", i18n.t(TextKey::StatusLoaded), path);
        } else {
            self.status = i18n.t(TextKey::StatusAudioUnavailable).to_owned();
        }
    }

    pub fn handle_editor_seek(&mut self, sec: f32, i18n: &I18n) {
        self.seek_to(sec, i18n);
        self.hitsound_trigger.reset(sec);
    }

    // Hitsound

    /// Trigger hitsounds for note heads that were crossed this frame.
    /// `note_heads`: slice of `(time_ms, is_ground)`.
    pub fn trigger_hitsounds(&mut self, note_heads: &[(f32, bool)]) {
        let current_sec = self.current_sec();
        let is_playing = self.playing;
        self.hitsound_trigger.tick(
            current_sec,
            is_playing,
            note_heads,
            &mut self.hitsound_player,
        );
    }

    pub fn hitsound_tap_volume(&self) -> f32 {
        self.hitsound_player.tap_volume()
    }

    pub fn set_hitsound_tap_volume(&mut self, volume: f32) {
        self.hitsound_player.set_tap_volume(volume);
    }

    pub fn hitsound_arc_volume(&self) -> f32 {
        self.hitsound_player.arc_volume()
    }

    pub fn set_hitsound_arc_volume(&mut self, volume: f32) {
        self.hitsound_player.set_arc_volume(volume);
    }

    pub fn set_hitsound_enabled(&mut self, enabled: bool) {
        self.hitsound_player.set_enabled(enabled);
    }

    pub fn hitsound_enabled(&self) -> bool {
        self.hitsound_player.enabled()
    }

    pub fn set_hitsound_max_voices(&mut self, max: usize) {
        self.hitsound_player.set_max_voices(max);
    }

    pub fn hitsound_max_voices(&self) -> usize {
        self.hitsound_player.max_voices()
    }

    pub fn set_hitsound_delay_ms(&mut self, ms: i32) {
        self.hitsound_trigger.set_delay_ms(ms);
    }

    pub fn hitsound_delay_ms(&self) -> i32 {
        self.hitsound_trigger.delay_ms()
    }

    /// 生成音频调试快照，供调试窗口实时显示。
    pub fn debug_snapshot(&self) -> AudioDebugSnapshot {
        let dt = get_frame_time();
        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };

        let backend_position = self.player.as_ref()
            .and_then(|_p| {
                // 直接读 snapshot 中缓存的 position
                // 注意：这里不能调 p.snapshot() 因为需要 &mut
                // 用 prev_backend_pos 代替（tick 中已更新）
                Some(self.prev_backend_pos)
            })
            .unwrap_or(0.0);

        let state_str = if self.player.is_none() {
            "No Backend".to_string()
        } else if self.playing {
            "Playing".to_string()
        } else if self.anchor_pos > 0.001 {
            "Paused".to_string()
        } else {
            "Ready".to_string()
        };

        AudioDebugSnapshot {
            playback_state: state_str,
            backend_position,
            controller_position: self.current_sec(),
            anchor_pos: self.anchor_pos,
            anchor_time: self.anchor_time,
            duration_sec: self.duration_sec,
            effective_volume: self.effective_volume(),
            music_volume: self.music_volume,
            master_volume: self.master_volume,
            is_playing_ctrl: self.playing,
            pos_delta_per_frame: self.pos_delta_per_frame,
            estimated_speed: self.estimated_speed,
            fps,
            delta_time: dt,
            has_backend: self.player.is_some(),
            track_path: self.track_path.clone().unwrap_or_default(),
        }
    }
}

