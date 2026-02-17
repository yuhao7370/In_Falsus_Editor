pub enum GroundEditorAction {
    SeekTo(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteKind {
    Tap,
    Hold,
}

#[derive(Debug, Clone)]
struct GroundNote {
    id: u64,
    lane: usize,
    time_ms: f64,
    duration_ms: f64,
}

impl GroundNote {
    fn kind(&self) -> NoteKind {
        if self.duration_ms > 1.0 {
            NoteKind::Hold
        } else {
            NoteKind::Tap
        }
    }

    fn end_ms(&self) -> f64 {
        self.time_ms + self.duration_ms.max(0.0)
    }
}

#[derive(Debug, Clone)]
struct DragState {
    note_id: u64,
    pointer_origin: egui::Pos2,
    note_origin_time_ms: f64,
    note_origin_lane: usize,
}

#[derive(Debug, Clone)]
struct WaveformData {
    peaks: Vec<f32>,
    duration_sec: f32,
}

impl WaveformData {
    fn from_audio_file(path: &str, peak_count: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|err| format!("failed to read audio: {err}"))?;
        let clip = AudioClip::new(bytes).map_err(|err| format!("failed to decode audio: {err}"))?;
        let frames = clip.frames();
        let duration_sec = clip.length();
        let peak_count = peak_count.max(256);
        let mut peaks = vec![0.0_f32; peak_count];

        if !frames.is_empty() {
            let frame_count = frames.len();
            for (index, frame) in frames.iter().enumerate() {
                let bucket = index * peak_count / frame_count;
                let amp = frame.avg().abs().min(1.0);
                if amp > peaks[bucket] {
                    peaks[bucket] = amp;
                }
            }
        }

        Ok(Self {
            peaks,
            duration_sec,
        })
    }
}

pub struct GroundEditor {
    notes: Vec<GroundNote>,
    next_note_id: u64,
    selected_note_id: Option<u64>,
    drag_state: Option<DragState>,
    chart_path: String,
    status_message: String,
    base_bpm: f64,
    pixels_per_second: f32,
    snap_enabled: bool,
    snap_division: u32,
    waveform: Option<WaveformData>,
    waveform_error: Option<String>,
    last_audio_path: Option<String>,
}


