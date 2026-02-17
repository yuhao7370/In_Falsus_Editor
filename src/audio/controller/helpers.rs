fn format_error(error: &PlayerError, i18n: &I18n) -> String {
    match error {
        PlayerError::BackendInit(_) => i18n.with_detail(TextKey::StatusInitAudioFailed, error),
        PlayerError::Io { .. } => i18n.with_detail(TextKey::StatusReadAudioFailed, error),
        PlayerError::Decode(_) => i18n.with_detail(TextKey::StatusDecodeAudioFailed, error),
        PlayerError::CreateMusic(_) => i18n.with_detail(TextKey::StatusCreateMusicFailed, error),
        PlayerError::StartPlayback(_) | PlayerError::PausePlayback(_) => {
            i18n.with_detail(TextKey::StatusStartPlaybackFailed, error)
        }
        PlayerError::Seek(_) | PlayerError::InvalidSeek { .. } => {
            i18n.with_detail(TextKey::StatusSeekFailed, error)
        }
        PlayerError::NoTrackLoaded | PlayerError::SetVolume(_) => {
            i18n.with_detail(TextKey::StatusAudioUnavailable, error)
        }
        PlayerError::BackendRecover(_) => i18n.with_detail(TextKey::StatusBackendError, error),
    }
}

fn format_event(event: PlayerEvent, i18n: &I18n) -> String {
    match event {
        PlayerEvent::Loaded { path, duration_sec } => {
            format!(
                "{}: {} ({duration_sec:.2}s)",
                i18n.t(TextKey::StatusLoaded),
                path
            )
        }
        PlayerEvent::Started => i18n.t(TextKey::StatusPlaying).to_owned(),
        PlayerEvent::Paused => i18n.t(TextKey::StatusPaused).to_owned(),
        PlayerEvent::Stopped(StopReason::User) => i18n.t(TextKey::StatusStopped).to_owned(),
        PlayerEvent::Stopped(StopReason::EndOfTrack) => {
            i18n.t(TextKey::StatusPlaybackEnded).to_owned()
        }
        PlayerEvent::BackendRecovered => i18n.t(TextKey::StatusBackendRecovered).to_owned(),
        PlayerEvent::Error(e) => format_error(&e, i18n),
    }
}

fn normalize_wheel_units(raw: f32) -> f32 {
    if raw.abs() <= 10.0 {
        raw * 120.0
    } else {
        raw
    }
}

