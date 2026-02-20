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

    #[allow(dead_code)]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    #[allow(dead_code)]
    pub fn duration_sec(&self) -> f32 {
        self.duration_sec
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<(), PlayerError> {
        let path_string = path.as_ref().to_string_lossy().to_string();
        let bytes = std::fs::read(path.as_ref()).map_err(|err| PlayerError::Io {
            path: path_string.clone(),
            message: err.to_string(),
        })?;

        let clip = AudioClip::new(bytes).map_err(|err| PlayerError::Decode(err.to_string()))?;
        let duration_sec = clip.length();
        let mut music = self.create_music_renderer(clip)?;

        // sasa's Music may start playing by default; ensure it's paused.
        let _ = music.pause();
        self.state = PlaybackState::Ready;
        self.position_cache = 0.0;
        self.pending_event = Some(PlayerEvent::Loaded {
            path: path_string.clone(),
            duration_sec,
        });

        self.music = Some(music);
        self.track_path = Some(path_string);
        self.duration_sec = duration_sec;
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), PlayerError> {
        let music = self.music.as_mut().ok_or(PlayerError::NoTrackLoaded)?;

        // Always re-seek to position_cache before resuming.
        // This is critical because sasa processes commands asynchronously
        // via a ring buffer. If a previous seek_to() hasn't been consumed
        // by the audio thread yet, sending Resume could cause the renderer
        // to run one cycle at the OLD index (possibly past the clip end)
        // and immediately re-pause. By always issuing SeekTo right before
        // Resume, we guarantee the FIFO order: SeekTo -> Resume.
        let target = if self.state == PlaybackState::Stopped {
            0.0
        } else {
            self.position_cache
        };
        music
            .seek_to(target)
            .map_err(|err| PlayerError::Seek(err.to_string()))?;
        self.position_cache = target;

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

    #[allow(dead_code)]
    pub fn toggle_play_pause(&mut self) -> Result<(), PlayerError> {
        if self.state == PlaybackState::Playing {
            self.pause()
        } else {
            self.play()
        }
    }

    #[allow(dead_code)]
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

    /// 安装已解码的 AudioClip（跳过文件读取和解码步骤，用于异步加载流程）。
    pub fn install_clip(&mut self, clip: AudioClip, path: &str) -> Result<(), PlayerError> {
        let path_string = path.to_string();
        let duration_sec = clip.length();
        let mut music = self.create_music_renderer(clip)?;

        let _ = music.pause();
        self.state = PlaybackState::Ready;
        self.position_cache = 0.0;
        self.pending_event = Some(PlayerEvent::Loaded {
            path: path_string.clone(),
            duration_sec,
        });

        self.music = Some(music);
        self.track_path = Some(path_string);
        self.duration_sec = duration_sec;
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



}

