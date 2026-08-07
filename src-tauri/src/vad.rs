use voice_activity_detector::VoiceActivityDetector;

const VAD_SAMPLE_RATE: i64 = 16_000;
const VAD_CHUNK_SIZE: usize = 512;
const SPEECH_THRESHOLD: f32 = 0.4;
const ENDPOINT_TRAILING_SILENCE_FRAMES: usize = 20;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct VadPushResult {
    pub speech_detected: bool,
    pub events: Vec<VadEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VadEvent {
    /// Frame-end offset, in samples, relative to the current input slice.
    pub end_offset: usize,
    pub kind: VadEventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadEventKind {
    Speech,
    Endpoint,
}

#[derive(Debug)]
struct EndpointTracker {
    speech_started: bool,
    trailing_silence_frames: usize,
    endpoint_silence_frames: usize,
}

impl EndpointTracker {
    fn new(endpoint_silence_frames: usize) -> Self {
        Self {
            speech_started: false,
            trailing_silence_frames: 0,
            endpoint_silence_frames: endpoint_silence_frames.max(1),
        }
    }

    fn observe(&mut self, is_speech: bool) -> bool {
        if is_speech {
            self.speech_started = true;
            self.trailing_silence_frames = 0;
            return false;
        }

        if !self.speech_started {
            return false;
        }

        self.trailing_silence_frames += 1;
        if self.trailing_silence_frames < self.endpoint_silence_frames {
            return false;
        }

        self.speech_started = false;
        self.trailing_silence_frames = 0;
        true
    }
}

/// Stateful Silero VAD for incremental 16 kHz mono audio.
///
/// The model carries recurrent state between frames. Rebuilding the detector
/// for every recorder poll resets that state and makes short/quiet speech much
/// easier to miss.
pub struct StreamingVad {
    detector: VoiceActivityDetector,
    pending: Vec<f32>,
    endpoint_tracker: EndpointTracker,
}

impl StreamingVad {
    pub fn new_16k() -> Result<Self, String> {
        let detector = VoiceActivityDetector::builder()
            .sample_rate(VAD_SAMPLE_RATE)
            .chunk_size(VAD_CHUNK_SIZE)
            .build()
            .map_err(|e| format!("Failed to initialize streaming VAD: {e:?}"))?;
        Ok(Self {
            detector,
            pending: Vec::with_capacity(VAD_CHUNK_SIZE * 2),
            endpoint_tracker: EndpointTracker::new(ENDPOINT_TRAILING_SILENCE_FRAMES),
        })
    }

    pub fn push(&mut self, samples: &[f32]) -> bool {
        self.push_with_endpoints(samples).speech_detected
    }

    pub fn push_with_endpoints(&mut self, samples: &[f32]) -> VadPushResult {
        let pending_before_push = self.pending.len();
        self.pending.extend(
            samples
                .iter()
                .map(|sample| if sample.is_finite() { *sample } else { 0.0 }),
        );
        let complete_len = self.pending.len() / VAD_CHUNK_SIZE * VAD_CHUNK_SIZE;
        let mut result = VadPushResult::default();
        for (frame_index, frame) in self.pending[..complete_len]
            .chunks_exact(VAD_CHUNK_SIZE)
            .enumerate()
        {
            let is_speech = self.detector.predict(frame.iter().copied()) > SPEECH_THRESHOLD;
            result.speech_detected |= is_speech;
            let endpoint = self.endpoint_tracker.observe(is_speech);
            let frame_end = (frame_index + 1) * VAD_CHUNK_SIZE;
            let end_offset = frame_end
                .saturating_sub(pending_before_push)
                .min(samples.len());
            if is_speech {
                result.events.push(VadEvent {
                    end_offset,
                    kind: VadEventKind::Speech,
                });
            } else if endpoint {
                result.events.push(VadEvent {
                    end_offset,
                    kind: VadEventKind::Endpoint,
                });
            }
        }
        self.pending.drain(..complete_len);
        result
    }

    /// Evaluates the final incomplete frame before a recording is finalized.
    ///
    /// Padding with silence preserves all captured samples while preventing a
    /// short trailing phoneme from being treated as an unexamined silent tail.
    pub fn finish_pending_has_speech(&mut self) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        let mut frame = [0.0_f32; VAD_CHUNK_SIZE];
        let pending_len = self.pending.len().min(VAD_CHUNK_SIZE);
        frame[..pending_len].copy_from_slice(&self.pending[..pending_len]);
        self.pending.clear();
        self.detector.predict(frame.iter().copied()) > SPEECH_THRESHOLD
    }
}

/// Returns true if any 16 kHz mono frame in `samples` exceeds the speech probability threshold.
pub fn has_speech(samples: &[f32], sample_rate: i64) -> bool {
    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(sample_rate)
        .chunk_size(512usize)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Aura Dev Log ERROR: Failed to build VAD in has_speech: {:?}",
                e
            );
            return true; // Fallback to true so we don't discard audio on error
        }
    };

    let chunk_size = if sample_rate == 8000 { 256 } else { 512 };
    for chunk in samples.chunks(chunk_size) {
        let prob = vad.predict(chunk.iter().copied());
        if prob > SPEECH_THRESHOLD {
            return true;
        }
    }
    false
}

/// Returns samples with leading and trailing silence (non-speech) trimmed, preserving a small margin.
pub fn trim_silence(samples: &[f32], sample_rate: i64) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(sample_rate)
        .chunk_size(512usize)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Aura Dev Log ERROR: Failed to build VAD in trim_silence: {:?}",
                e
            );
            return samples.to_vec(); // Fallback to returning original samples on error
        }
    };

    let chunk_size = if sample_rate == 8000 { 256 } else { 512 };
    let mut speech_indices = Vec::new();

    for (i, chunk) in samples.chunks(chunk_size).enumerate() {
        let prob = vad.predict(chunk.iter().copied());
        if prob > SPEECH_THRESHOLD {
            speech_indices.push(i);
        }
    }

    if speech_indices.is_empty() {
        // If no speech was detected, return the original audio to be safe and avoid erasing dictation
        return samples.to_vec();
    }

    let first_speech_chunk = speech_indices[0];
    let last_speech_chunk = speech_indices[speech_indices.len() - 1];

    // Margin: 200ms = 0.2 * sample_rate
    let margin_samples = (0.2 * sample_rate as f32) as usize;

    let start_sample = (first_speech_chunk * chunk_size).saturating_sub(margin_samples);
    let end_sample = ((last_speech_chunk + 1) * chunk_size + margin_samples).min(samples.len());

    samples[start_sample..end_sample].to_vec()
}

/// Reads a WAV file, trims the leading and trailing silence using Silero VAD, and overwrites the file.
pub fn trim_wav_file(path: &str) -> Result<(), String> {
    // 1. Read i16 samples from the WAV file
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| format!("Failed to open WAV file for VAD trimming: {}", e))?;
    let spec = reader.spec();

    // Ensure the file is indeed 16kHz mono as expected
    if spec.sample_rate != 16000 || spec.channels != 1 {
        return Err(format!(
            "Unsupported WAV format for VAD: channels={}, sample_rate={}",
            spec.channels, spec.sample_rate
        ));
    }

    let samples_i16: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<i16>, hound::Error>>()
        .map_err(|e| format!("Failed to read WAV samples: {}", e))?;

    // 2. Convert to f32
    let samples_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

    // 3. Trim silence
    let trimmed_f32 = trim_silence(&samples_f32, 16000);

    // 4. Convert back to i16
    let mut trimmed_i16 = Vec::with_capacity(trimmed_f32.len());
    for &sample in &trimmed_f32 {
        let clamped = sample.clamp(-1.0, 1.0);
        let s = if clamped >= 0.0 {
            (clamped * i16::MAX as f32) as i16
        } else {
            (clamped * 32768.0) as i16
        };
        trimmed_i16.push(s);
    }

    // 5. Overwrite the WAV file
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("Failed to create WAV writer for VAD trimming: {}", e))?;

    for &sample in &trimmed_i16 {
        writer
            .write_sample(sample)
            .map_err(|e| format!("Failed to write trimmed sample: {}", e))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("Failed to finalize trimmed WAV file: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_speech_silence() {
        let silence = vec![0.0f32; 16000]; // 1 second of silence
        assert!(!has_speech(&silence, 16000));
    }

    #[test]
    fn test_trim_silence_no_speech() {
        let silence = vec![0.0f32; 16000];
        let trimmed = trim_silence(&silence, 16000);
        assert_eq!(trimmed.len(), silence.len());
    }

    #[test]
    fn endpoint_tracker_ignores_silence_before_speech() {
        let mut tracker = EndpointTracker::new(3);

        assert!(!(0..10).any(|_| tracker.observe(false)));
    }

    #[test]
    fn endpoint_tracker_emits_after_trailing_silence() {
        let mut tracker = EndpointTracker::new(3);

        assert!(!tracker.observe(true));
        assert!(!tracker.observe(false));
        assert!(!tracker.observe(false));
        assert!(tracker.observe(false));
        assert!(!tracker.observe(false));
    }

    #[test]
    fn endpoint_tracker_resets_silence_when_speech_resumes() {
        let mut tracker = EndpointTracker::new(3);

        assert!(!tracker.observe(true));
        assert!(!tracker.observe(false));
        assert!(!tracker.observe(false));
        assert!(!tracker.observe(true));
        assert!(!tracker.observe(false));
        assert!(!tracker.observe(false));
        assert!(tracker.observe(false));
    }

    #[test]
    fn endpoint_tracker_handles_multiple_segments() {
        let mut tracker = EndpointTracker::new(2);
        let frames = [
            true, false, false, false, true, true, false, false, true, false, false,
        ];

        let endpoint_count = frames
            .into_iter()
            .filter(|is_speech| tracker.observe(*is_speech))
            .count();

        assert_eq!(endpoint_count, 3);
    }

    #[test]
    fn final_partial_silence_frame_is_checked_and_consumed() {
        let mut vad = StreamingVad::new_16k().expect("streaming VAD");
        let result = vad.push_with_endpoints(&[0.0; 127]);
        assert!(result.events.is_empty());

        assert!(!vad.finish_pending_has_speech());
        assert!(!vad.finish_pending_has_speech());
    }
}
