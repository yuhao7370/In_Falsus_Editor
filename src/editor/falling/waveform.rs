#[derive(Debug, Clone)]
struct Waveform {
    path: String,
    peaks: Vec<f32>,
    duration_sec: f32,
}

impl Waveform {
    fn from_audio_file(path: &str, bucket_count: usize) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|err| format!("failed to read audio: {err}"))?;
        let clip = AudioClip::new(bytes).map_err(|err| format!("failed to decode audio: {err}"))?;
        let frames = clip.frames();
        let bucket_count = bucket_count.max(256);
        let mut peaks = vec![0.0_f32; bucket_count];

        if !frames.is_empty() {
            let frame_count = frames.len();
            for (idx, frame) in frames.iter().enumerate() {
                let bucket = idx * bucket_count / frame_count;
                let amp = frame.avg().abs().min(1.0);
                if amp > peaks[bucket] {
                    peaks[bucket] = amp;
                }
            }
        }

        Ok(Self {
            path: path.to_owned(),
            peaks,
            duration_sec: clip.length(),
        })
    }
}

