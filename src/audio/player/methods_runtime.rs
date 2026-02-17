impl SongPlayer {
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
                    // Use Stopped (not Paused) so that play() knows to
                    // seek back to 0 before resuming 鈥?the sasa renderer's
                    // internal index is past the clip and would immediately
                    // re-pause if we just sent Resume.
                    self.state = PlaybackState::Stopped;
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

                // sasa may NOT auto-pause when the clip ends; it might
                // just stop advancing position. Check for near-end here too.
                let position = music.position().clamp(0.0, self.duration_sec.max(0.0));
                let near_end = self.duration_sec > 0.0
                    && position >= (self.duration_sec - 0.02);
                if near_end {
                    let _ = music.pause();
                    self.state = PlaybackState::Stopped;
                    self.position_cache = self.duration_sec.max(0.0);
                    return Some(PlayerEvent::Stopped(StopReason::EndOfTrack));
                }
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

