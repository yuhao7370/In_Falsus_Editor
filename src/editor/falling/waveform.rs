// 文件说明：音频波形数据处理与缓存实现。
// 主要功能：使用 Spectral Flux 分析音频，提取节拍/旋律起音强度用于频谱可视化。
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
        let duration_sec = clip.length();
        let bucket_count = bucket_count.max(256);

        if frames.is_empty() || duration_sec <= 0.0 {
            return Ok(Self {
                path: path.to_owned(),
                peaks: vec![0.0; bucket_count],
                duration_sec,
            });
        }

        // Convert to mono f32 samples
        let samples: Vec<f32> = frames.iter().map(|f| f.avg()).collect();
        let sample_rate = (samples.len() as f32 / duration_sec).round() as usize;

        // STFT parameters
        let fft_size = 2048;
        let hop_size = 1024;
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);

        // Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos())
            })
            .collect();

        // Frequency bin range: focus on 200Hz ~ 8000Hz (skip bass, keep drums/melody)
        let bin_lo = (200.0 * fft_size as f32 / sample_rate as f32).round() as usize;
        let bin_hi = (8000.0 * fft_size as f32 / sample_rate as f32)
            .round()
            .min((fft_size / 2) as f32) as usize;

        // Compute spectral flux
        let num_frames = if samples.len() >= fft_size {
            (samples.len() - fft_size) / hop_size + 1
        } else {
            0
        };

        let mut prev_mag = vec![0.0_f32; fft_size / 2 + 1];
        let mut flux_values: Vec<f32> = Vec::with_capacity(num_frames);
        let mut flux_times: Vec<f32> = Vec::with_capacity(num_frames);

        let mut buffer = vec![Complex::new(0.0_f32, 0.0_f32); fft_size];

        for frame_idx in 0..num_frames {
            let start = frame_idx * hop_size;
            // Apply window and fill buffer
            for i in 0..fft_size {
                let s = if start + i < samples.len() {
                    samples[start + i]
                } else {
                    0.0
                };
                buffer[i] = Complex::new(s * window[i], 0.0);
            }

            fft.process(&mut buffer);

            // Compute magnitude spectrum and spectral flux (positive differences only)
            let mut flux = 0.0_f32;
            for bin in bin_lo..=bin_hi.min(fft_size / 2) {
                let mag = (buffer[bin].re * buffer[bin].re + buffer[bin].im * buffer[bin].im).sqrt();
                let diff = mag - prev_mag[bin];
                if diff > 0.0 {
                    flux += diff;
                }
                prev_mag[bin] = mag;
            }

            let time_sec = (start + fft_size / 2) as f32 / sample_rate as f32;
            flux_values.push(flux);
            flux_times.push(time_sec);
        }

        // Downsample flux into buckets
        let mut peaks = vec![0.0_f32; bucket_count];
        for (i, &flux) in flux_values.iter().enumerate() {
            let t = flux_times[i] / duration_sec;
            let bucket = (t * bucket_count as f32) as usize;
            if bucket < bucket_count && flux > peaks[bucket] {
                peaks[bucket] = flux;
            }
        }

        // Normalize: scale so max = 1.0
        let max_val = peaks.iter().cloned().fold(0.0_f32, f32::max);
        if max_val > 1.0e-6 {
            let inv = 1.0 / max_val;
            for p in peaks.iter_mut() {
                *p *= inv;
            }
        }

        Ok(Self {
            path: path.to_owned(),
            peaks,
            duration_sec,
        })
    }
}
