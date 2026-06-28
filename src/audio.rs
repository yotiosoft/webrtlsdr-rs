//! Audio frame conversion for browser-friendly PCM output.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::dsp::AudioBlock;

const MONO_CHANNELS: u8 = 1;
const PCM_BYTES_PER_SAMPLE: usize = 2;
pub const PCM_FRAME_V1_MAGIC: &[u8; 4] = b"WPCM";
pub const PCM_FRAME_V1_VERSION: u8 = 1;
pub const PCM_FRAME_V1_HEADER_LEN: u8 = 36;
pub const PCM_FRAME_V1_FORMAT_I16LE: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSampleFormat {
    I16Le,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PcmFrame {
    pub sequence: u64,
    pub pts_samples: u64,
    pub generated_at_unix_ms: Option<u64>,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub format: AudioSampleFormat,
    pub samples: usize,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PcmStats {
    pub frames_produced: u64,
    pub samples_produced: u64,
    pub bytes_produced: u64,
    pub last_frame_bytes: usize,
    pub last_sequence: Option<u64>,
    pub last_pts_samples: Option<u64>,
    pub peak_before_clamp: f32,
    pub clipped_samples: u64,
    pub last_error: Option<String>,
}

impl Default for PcmStats {
    fn default() -> Self {
        Self {
            frames_produced: 0,
            samples_produced: 0,
            bytes_produced: 0,
            last_frame_bytes: 0,
            last_sequence: None,
            last_pts_samples: None,
            peak_before_clamp: 0.0,
            clipped_samples: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct PcmEncoder {
    next_sequence: u64,
    next_pts_samples: u64,
    stats: PcmStats,
}

impl PcmEncoder {
    pub fn encode_block(&mut self, audio: &AudioBlock) -> Option<PcmFrame> {
        if audio.samples.is_empty() {
            return None;
        }

        let mut payload = Vec::with_capacity(audio.samples.len() * PCM_BYTES_PER_SAMPLE);
        let mut clipped_samples = 0_u64;
        let mut peak_before_clamp = 0.0_f32;

        for sample in &audio.samples {
            let sanitized = if sample.is_finite() { *sample } else { 0.0 };
            peak_before_clamp = peak_before_clamp.max(sanitized.abs());

            if !(-1.0..=1.0).contains(&sanitized) {
                clipped_samples = clipped_samples.saturating_add(1);
            }

            let pcm = sample_to_i16_le(sanitized);
            payload.extend_from_slice(&pcm);
        }

        let frame = PcmFrame {
            sequence: self.next_sequence,
            pts_samples: self.next_pts_samples,
            generated_at_unix_ms: unix_ms_now(),
            sample_rate_hz: audio.sample_rate_hz,
            channels: MONO_CHANNELS,
            format: AudioSampleFormat::I16Le,
            samples: audio.samples.len(),
            payload,
        };

        self.next_sequence = self.next_sequence.saturating_add(1);
        self.next_pts_samples = self
            .next_pts_samples
            .saturating_add(audio.samples.len() as u64);
        self.record_success(&frame, peak_before_clamp, clipped_samples);

        Some(frame)
    }

    pub fn stats(&self) -> PcmStats {
        self.stats.clone()
    }

    fn record_success(&mut self, frame: &PcmFrame, peak_before_clamp: f32, clipped_samples: u64) {
        self.stats.frames_produced = self.stats.frames_produced.saturating_add(1);
        self.stats.samples_produced = self
            .stats
            .samples_produced
            .saturating_add(frame.samples as u64);
        self.stats.bytes_produced = self
            .stats
            .bytes_produced
            .saturating_add(frame.payload.len() as u64);
        self.stats.last_frame_bytes = frame.payload.len();
        self.stats.last_sequence = Some(frame.sequence);
        self.stats.last_pts_samples = Some(frame.pts_samples);
        self.stats.peak_before_clamp = finite_or_zero(peak_before_clamp);
        self.stats.clipped_samples = self.stats.clipped_samples.saturating_add(clipped_samples);
        self.stats.last_error = None;
    }
}

impl PcmFrame {
    pub fn payload_bytes(&self) -> Option<u32> {
        u32::try_from(self.payload.len()).ok()
    }

    pub fn protocol_v1_message(&self) -> Option<Vec<u8>> {
        let sample_count = u32::try_from(self.samples).ok()?;
        let payload_bytes = self.payload_bytes()?;
        let mut message =
            Vec::with_capacity(usize::from(PCM_FRAME_V1_HEADER_LEN) + self.payload.len());

        message.extend_from_slice(PCM_FRAME_V1_MAGIC);
        message.push(PCM_FRAME_V1_VERSION);
        message.push(PCM_FRAME_V1_HEADER_LEN);
        message.push(format_code(self.format));
        message.push(self.channels);
        message.extend_from_slice(&self.sample_rate_hz.to_le_bytes());
        message.extend_from_slice(&sample_count.to_le_bytes());
        message.extend_from_slice(&self.sequence.to_le_bytes());
        message.extend_from_slice(&self.pts_samples.to_le_bytes());
        message.extend_from_slice(&payload_bytes.to_le_bytes());
        message.extend_from_slice(&self.payload);

        Some(message)
    }
}

fn format_code(format: AudioSampleFormat) -> u8 {
    match format {
        AudioSampleFormat::I16Le => PCM_FRAME_V1_FORMAT_I16LE,
    }
}

fn sample_to_i16_le(sample: f32) -> [u8; PCM_BYTES_PER_SAMPLE] {
    let value = if sample <= -1.0 {
        i16::MIN
    } else if sample >= 1.0 {
        i16::MAX
    } else {
        (sample * f32::from(i16::MAX)).round() as i16
    };

    value.to_le_bytes()
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn unix_ms_now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_block(samples: Vec<f32>) -> AudioBlock {
        AudioBlock {
            sample_rate_hz: 48_000,
            samples,
        }
    }

    #[test]
    fn converts_unit_range_to_i16_le() {
        let mut encoder = PcmEncoder::default();
        let frame = encoder
            .encode_block(&audio_block(vec![-1.0, 0.0, 1.0]))
            .expect("non-empty audio should produce a frame");

        let mut expected = Vec::new();
        expected.extend_from_slice(&i16::MIN.to_le_bytes());
        expected.extend_from_slice(&0_i16.to_le_bytes());
        expected.extend_from_slice(&i16::MAX.to_le_bytes());

        assert_eq!(frame.payload, expected);
        assert_eq!(frame.pts_samples, 0);
        assert_eq!(frame.samples, 3);
        assert_eq!(frame.sample_rate_hz, 48_000);
        assert_eq!(frame.channels, MONO_CHANNELS);
        assert_eq!(frame.format, AudioSampleFormat::I16Le);
    }

    #[test]
    fn clamps_out_of_range_values_and_tracks_clips() {
        let mut encoder = PcmEncoder::default();
        let frame = encoder
            .encode_block(&audio_block(vec![-1.5, 1.25]))
            .expect("non-empty audio should produce a frame");

        let mut expected = Vec::new();
        expected.extend_from_slice(&i16::MIN.to_le_bytes());
        expected.extend_from_slice(&i16::MAX.to_le_bytes());

        assert_eq!(frame.payload, expected);
        assert_eq!(encoder.stats().clipped_samples, 2);
        assert_eq!(encoder.stats().peak_before_clamp, 1.5);
    }

    #[test]
    fn treats_nan_and_infinity_as_zero() {
        let mut encoder = PcmEncoder::default();
        let frame = encoder
            .encode_block(&audio_block(vec![
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ]))
            .expect("non-empty audio should produce a frame");

        assert_eq!(frame.payload, vec![0, 0, 0, 0, 0, 0]);
        assert_eq!(encoder.stats().clipped_samples, 0);
        assert_eq!(encoder.stats().peak_before_clamp, 0.0);
    }

    #[test]
    fn preserves_little_endian_order() {
        let mut encoder = PcmEncoder::default();
        let frame = encoder
            .encode_block(&audio_block(vec![0.5]))
            .expect("non-empty audio should produce a frame");

        assert_eq!(frame.payload, (16_384_i16).to_le_bytes());
    }

    #[test]
    fn empty_input_does_not_produce_frame() {
        let mut encoder = PcmEncoder::default();

        assert!(encoder.encode_block(&audio_block(Vec::new())).is_none());
        assert_eq!(encoder.stats().frames_produced, 0);
    }

    #[test]
    fn sequence_number_increments() {
        let mut encoder = PcmEncoder::default();

        let first = encoder
            .encode_block(&audio_block(vec![0.0]))
            .expect("first frame");
        let second = encoder
            .encode_block(&audio_block(vec![0.0]))
            .expect("second frame");

        assert_eq!(first.sequence, 0);
        assert_eq!(second.sequence, 1);
        assert_eq!(first.pts_samples, 0);
        assert_eq!(second.pts_samples, 1);
        assert_eq!(encoder.stats().frames_produced, 2);
        assert_eq!(encoder.stats().samples_produced, 2);
        assert_eq!(encoder.stats().bytes_produced, 4);
        assert_eq!(encoder.stats().last_frame_bytes, 2);
        assert_eq!(encoder.stats().last_sequence, Some(1));
        assert_eq!(encoder.stats().last_pts_samples, Some(1));
    }

    #[test]
    fn encodes_pcm_frame_protocol_v1_header() {
        let mut encoder = PcmEncoder::default();
        let frame = encoder
            .encode_block(&audio_block(vec![0.0, 0.5]))
            .expect("frame");
        let message = frame.protocol_v1_message().expect("v1 message");

        assert_eq!(&message[0..4], PCM_FRAME_V1_MAGIC);
        assert_eq!(message[4], PCM_FRAME_V1_VERSION);
        assert_eq!(message[5], PCM_FRAME_V1_HEADER_LEN);
        assert_eq!(message[6], PCM_FRAME_V1_FORMAT_I16LE);
        assert_eq!(message[7], MONO_CHANNELS);
        assert_eq!(
            u32::from_le_bytes(message[8..12].try_into().unwrap()),
            48_000
        );
        assert_eq!(u32::from_le_bytes(message[12..16].try_into().unwrap()), 2);
        assert_eq!(u64::from_le_bytes(message[16..24].try_into().unwrap()), 0);
        assert_eq!(u64::from_le_bytes(message[24..32].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(message[32..36].try_into().unwrap()), 4);
        assert_eq!(&message[36..], frame.payload.as_slice());
    }
}
