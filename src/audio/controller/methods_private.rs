impl AudioController {
    fn do_play(&mut self, i18n: &I18n) {
        if let Some(p) = self.player.as_mut() {
            if let Err(e) = p.play() {
                self.status = format_error(&e, i18n);
                return;
            }
        }
        // Anchor = current paused position, timestamp = now.
        // anchor_pos is already correct from the last pause/seek.
        self.anchor_time = get_time();
        self.playing = true;
        self.status = i18n.t(TextKey::StatusPlaying).to_owned();
    }

    fn do_pause(&mut self, i18n: &I18n) {
        // Freeze position FIRST, then tell backend.
        let pos = self.current_sec();
        self.anchor_pos = pos;
        self.playing = false;

        if let Some(p) = self.player.as_mut() {
            if let Err(e) = p.pause() {
                self.status = format_error(&e, i18n);
                return;
            }
        }
        self.status = i18n.t(TextKey::StatusPaused).to_owned();
    }


}

