//! Minimal DSP pipeline for converting RTL-SDR `u8` IQ samples into audio.

use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_AUDIO_SAMPLE_RATE_HZ: u32 = 48_000;
const IQ_CENTER: f32 = 127.5;
const IQ_SCALE: f32 = 127.5;
const DC_ALPHA: f32 = 0.001;
const AUDIO_GAIN: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspConfig {
    pub input_sample_rate_hz: u32,
    pub output_sample_rate_hz: u32,
    pub mode: DemodulationMode,
}

impl DspConfig {
    pub fn am(input_sample_rate_hz: u32) -> Self {
        Self {
            input_sample_rate_hz,
            output_sample_rate_hz: DEFAULT_AUDIO_SAMPLE_RATE_HZ,
            mode: DemodulationMode::Am,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemodulationMode {
    Am,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBlock {
    pub sample_rate_hz: u32,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DspStats {
    pub iq_bytes_processed: u64,
    pub audio_samples_produced: u64,
    pub audio_sample_rate_hz: u32,
    pub decimation_ratio: usize,
    pub last_processed_unix_ms: Option<u64>,
    pub audio_peak: f32,
    pub audio_rms: f32,
    pub last_error: Option<String>,
}

impl Default for DspStats {
    fn default() -> Self {
        Self {
            iq_bytes_processed: 0,
            audio_samples_produced: 0,
            audio_sample_rate_hz: 0,
            decimation_ratio: 1,
            last_processed_unix_ms: None,
            audio_peak: 0.0,
            audio_rms: 0.0,
            last_error: None,
        }
    }
}

#[derive(Debug)]
pub struct DspProcessor {
    config: DspConfig,
    decimation_ratio: usize,
    audio_sample_rate_hz: u32,
    dc_level: f32,
    low_pass_level: f32,
    decimation_phase: usize,
    stats: DspStats,
}

impl DspProcessor {
    pub fn new(config: DspConfig) -> Self {
        let decimation_ratio =
            decimation_ratio(config.input_sample_rate_hz, config.output_sample_rate_hz);
        let audio_sample_rate_hz = if decimation_ratio == 0 {
            config.input_sample_rate_hz
        } else {
            config.input_sample_rate_hz / decimation_ratio as u32
        };

        let stats = DspStats {
            audio_sample_rate_hz,
            decimation_ratio,
            ..DspStats::default()
        };

        Self {
            config,
            decimation_ratio,
            audio_sample_rate_hz,
            dc_level: 0.0,
            low_pass_level: 0.0,
            decimation_phase: 0,
            stats,
        }
    }

    pub fn process_iq_u8(&mut self, iq: &[u8]) -> AudioBlock {
        match self.config.mode {
            DemodulationMode::Am => self.process_am(iq),
        }
    }

    pub fn stats(&self) -> DspStats {
        self.stats.clone()
    }

    fn process_am(&mut self, iq: &[u8]) -> AudioBlock {
        let iq_len = iq.len() - (iq.len() % 2);
        let mut samples = Vec::with_capacity(iq_len / 2 / self.decimation_ratio.max(1));

        for pair in iq[..iq_len].chunks_exact(2) {
            let i = normalize_iq_byte(pair[0]);
            let q = normalize_iq_byte(pair[1]);
            let envelope = (i.mul_add(i, q * q)).sqrt();

            self.dc_level += DC_ALPHA * (envelope - self.dc_level);
            let demodulated = envelope - self.dc_level;
            let low_pass_alpha = self.low_pass_alpha();
            self.low_pass_level += low_pass_alpha * (demodulated - self.low_pass_level);

            if self.decimation_phase == 0 {
                let audio = (self.low_pass_level * AUDIO_GAIN).clamp(-1.0, 1.0);
                if audio.is_finite() {
                    samples.push(audio);
                }
            }

            self.decimation_phase = (self.decimation_phase + 1) % self.decimation_ratio.max(1);
        }

        self.record_success(iq_len, &samples);

        AudioBlock {
            sample_rate_hz: self.audio_sample_rate_hz,
            samples,
        }
    }

    fn record_success(&mut self, iq_bytes_processed: usize, samples: &[f32]) {
        let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        let rms = if samples.is_empty() {
            0.0
        } else {
            let sum_squares = samples.iter().map(|sample| sample * sample).sum::<f32>();
            (sum_squares / samples.len() as f32).sqrt()
        };

        self.stats.iq_bytes_processed = self
            .stats
            .iq_bytes_processed
            .saturating_add(iq_bytes_processed as u64);
        self.stats.audio_samples_produced = self
            .stats
            .audio_samples_produced
            .saturating_add(samples.len() as u64);
        self.stats.audio_sample_rate_hz = self.audio_sample_rate_hz;
        self.stats.decimation_ratio = self.decimation_ratio;
        self.stats.last_processed_unix_ms = unix_ms_now();
        self.stats.audio_peak = finite_or_zero(peak);
        self.stats.audio_rms = finite_or_zero(rms);
        self.stats.last_error = None;
    }

    fn low_pass_alpha(&self) -> f32 {
        // A light smoothing filter before integer decimation. This is intentionally
        // simple for Step 7; later steps can replace it with a real resampler.
        (1.0 / self.decimation_ratio.max(1) as f32).clamp(0.01, 1.0)
    }
}

fn decimation_ratio(input_sample_rate_hz: u32, output_sample_rate_hz: u32) -> usize {
    if input_sample_rate_hz == 0 || output_sample_rate_hz == 0 {
        return 1;
    }

    let rounded = (input_sample_rate_hz + output_sample_rate_hz / 2) / output_sample_rate_hz;
    rounded.max(1) as usize
}

fn normalize_iq_byte(byte: u8) -> f32 {
    (f32::from(byte) - IQ_CENTER) / IQ_SCALE
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

    #[test]
    fn empty_input_produces_empty_audio() {
        let mut dsp = DspProcessor::new(DspConfig::am(2_048_000));
        let audio = dsp.process_iq_u8(&[]);

        assert!(audio.samples.is_empty());
        assert_eq!(dsp.stats().iq_bytes_processed, 0);
    }

    #[test]
    fn odd_length_input_does_not_panic() {
        let mut dsp = DspProcessor::new(DspConfig::am(96_000));
        let audio = dsp.process_iq_u8(&[128, 128, 129]);

        assert_eq!(dsp.stats().iq_bytes_processed, 2);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn constant_iq_produces_finite_audio() {
        let mut dsp = DspProcessor::new(DspConfig::am(2_048_000));
        let iq = vec![180_u8; 16_384];
        let audio = dsp.process_iq_u8(&iq);

        assert!(!audio.samples.is_empty());
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
        assert!(dsp.stats().audio_peak.is_finite());
        assert!(dsp.stats().audio_rms.is_finite());
    }

    #[test]
    fn stats_track_audio_output() {
        let mut dsp = DspProcessor::new(DspConfig::am(96_000));
        let audio = dsp.process_iq_u8(&[128, 128, 180, 128, 128, 128, 64, 128]);
        let stats = dsp.stats();

        assert_eq!(audio.sample_rate_hz, 48_000);
        assert_eq!(stats.audio_samples_produced, audio.samples.len() as u64);
        assert!(stats.last_processed_unix_ms.is_some());
        assert!(stats.audio_peak >= 0.0);
        assert!(stats.audio_rms >= 0.0);
    }
}
