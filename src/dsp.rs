//! Stateful RTL-SDR IQ demodulation. Every mode emits finite 48 kHz mono audio.

use std::{
    f32::consts::PI,
    time::{SystemTime, UNIX_EPOCH},
};

const AUDIO_RATE: u32 = 48_000;
const IQ_CENTER: f32 = 127.5;
const IQ_SCALE: f32 = 127.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemodulationMode {
    Am,
    Wbfm,
    Nbfm,
    Usb,
    Lsb,
}

impl DemodulationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Am => "am",
            Self::Wbfm => "wbfm",
            Self::Nbfm => "nbfm",
            Self::Usb => "usb",
            Self::Lsb => "lsb",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DspConfig {
    pub input_sample_rate_hz: u32,
    pub output_sample_rate_hz: u32,
    pub mode: DemodulationMode,
    pub channel_bandwidth_hz: u32,
    pub audio_lowpass_hz: u32,
    pub deemphasis_us: Option<u32>,
    pub squelch_threshold: Option<f32>,
    pub bfo_offset_hz: i32,
}

impl DspConfig {
    pub fn preset(mode: DemodulationMode, input_sample_rate_hz: u32) -> Self {
        let (bw, lp, deemphasis, bfo) = match mode {
            DemodulationMode::Am => (10_000, 5_000, None, 0),
            DemodulationMode::Wbfm => (180_000, 15_000, Some(75), 0),
            DemodulationMode::Nbfm => (12_500, 3_500, None, 0),
            DemodulationMode::Usb => (3_000, 3_000, None, 1_500),
            DemodulationMode::Lsb => (3_000, 3_000, None, -1_500),
        };
        Self {
            input_sample_rate_hz,
            output_sample_rate_hz: AUDIO_RATE,
            mode,
            channel_bandwidth_hz: bw,
            audio_lowpass_hz: lp,
            deemphasis_us: deemphasis,
            squelch_threshold: None,
            bfo_offset_hz: bfo,
        }
    }

    pub fn am(input_sample_rate_hz: u32) -> Self {
        Self::preset(DemodulationMode::Am, input_sample_rate_hz)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.input_sample_rate_hz == 0 || self.output_sample_rate_hz == 0 {
            return Err("sample rates must be greater than zero");
        }
        if self.channel_bandwidth_hz == 0 || self.channel_bandwidth_hz > self.input_sample_rate_hz {
            return Err("channel bandwidth must be within the input sample rate");
        }
        if self.audio_lowpass_hz == 0 || self.audio_lowpass_hz >= self.output_sample_rate_hz / 2 {
            return Err("audio low-pass must be below the audio Nyquist frequency");
        }
        if self
            .squelch_threshold
            .is_some_and(|v| !v.is_finite() || v < 0.0 || v > 1.0)
        {
            return Err("squelch threshold must be between 0 and 1");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBlock {
    pub sample_rate_hz: u32,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DspStats {
    pub mode: &'static str,
    pub input_sample_rate_hz: u32,
    pub channel_bandwidth_hz: u32,
    pub iq_bytes_processed: u64,
    pub audio_samples_produced: u64,
    pub audio_sample_rate_hz: u32,
    pub decimation_ratio: usize,
    pub last_processed_unix_ms: Option<u64>,
    pub audio_peak: f32,
    pub audio_rms: f32,
    pub squelch_open: bool,
    pub demodulator_errors: u64,
    pub last_error: Option<String>,
}

impl Default for DspStats {
    fn default() -> Self {
        Self {
            mode: "am",
            input_sample_rate_hz: 0,
            channel_bandwidth_hz: 0,
            iq_bytes_processed: 0,
            audio_samples_produced: 0,
            audio_sample_rate_hz: 0,
            decimation_ratio: 1,
            last_processed_unix_ms: None,
            audio_peak: 0.0,
            audio_rms: 0.0,
            squelch_open: true,
            demodulator_errors: 0,
            last_error: None,
        }
    }
}

#[derive(Debug)]
pub struct DspProcessor {
    config: DspConfig,
    output_phase: u32,
    prev_i: f32,
    prev_q: f32,
    have_prev: bool,
    dc: f32,
    rf_i: f32,
    rf_q: f32,
    audio_lp: f32,
    deemphasis: f32,
    bfo_phase: f32,
    stats: DspStats,
}

impl DspProcessor {
    pub fn new(config: DspConfig) -> Self {
        let ratio = decimation_ratio(config.input_sample_rate_hz, config.output_sample_rate_hz);
        let stats = DspStats {
            mode: config.mode.as_str(),
            input_sample_rate_hz: config.input_sample_rate_hz,
            channel_bandwidth_hz: config.channel_bandwidth_hz,
            audio_sample_rate_hz: config.output_sample_rate_hz,
            decimation_ratio: ratio,
            ..DspStats::default()
        };
        Self {
            config,
            output_phase: 0,
            prev_i: 0.0,
            prev_q: 0.0,
            have_prev: false,
            dc: 0.0,
            rf_i: 0.0,
            rf_q: 0.0,
            audio_lp: 0.0,
            deemphasis: 0.0,
            bfo_phase: 0.0,
            stats,
        }
    }

    #[allow(dead_code)] // Public DSP lifecycle hook; mode changes currently rebuild the processor.
    pub fn reset(&mut self) {
        let config = self.config;
        *self = Self::new(config);
    }
    pub fn stats(&self) -> DspStats {
        self.stats.clone()
    }

    pub fn process_iq_u8(&mut self, iq: &[u8]) -> AudioBlock {
        let len = iq.len() - iq.len() % 2;
        let mut out = Vec::with_capacity(len / 2 / self.stats.decimation_ratio.max(1) + 1);
        let rf_alpha = one_pole_alpha(
            self.config.channel_bandwidth_hz as f32 * 0.5,
            self.config.input_sample_rate_hz as f32,
        );
        for pair in iq[..len].chunks_exact(2) {
            let i = normalize_iq_byte(pair[0]);
            let q = normalize_iq_byte(pair[1]);
            self.rf_i += rf_alpha * (i - self.rf_i);
            self.rf_q += rf_alpha * (q - self.rf_q);
            let raw = match self.config.mode {
                DemodulationMode::Am => {
                    let env = self.rf_i.hypot(self.rf_q);
                    self.dc += 0.001 * (env - self.dc);
                    (env - self.dc) * 8.0
                }
                DemodulationMode::Wbfm | DemodulationMode::Nbfm => {
                    let v = if self.have_prev {
                        (self.rf_i * self.prev_q - self.rf_q * self.prev_i)
                            .atan2(self.rf_i * self.prev_i + self.rf_q * self.prev_q)
                    } else {
                        0.0
                    };
                    self.prev_i = self.rf_i;
                    self.prev_q = self.rf_q;
                    self.have_prev = true;
                    let deviation = if self.config.mode == DemodulationMode::Wbfm {
                        75_000.0
                    } else {
                        5_000.0
                    };
                    v * self.config.input_sample_rate_hz as f32 / (2.0 * PI * deviation)
                }
                DemodulationMode::Usb | DemodulationMode::Lsb => {
                    let step = 2.0 * PI * self.config.bfo_offset_hz as f32
                        / self.config.input_sample_rate_hz as f32;
                    let v = self.rf_i * self.bfo_phase.cos() - self.rf_q * self.bfo_phase.sin();
                    self.bfo_phase = (self.bfo_phase + step).rem_euclid(2.0 * PI);
                    v * 2.0
                }
            };
            let lp = one_pole_alpha(
                self.config.audio_lowpass_hz as f32,
                self.config.input_sample_rate_hz as f32,
            );
            self.audio_lp += lp * (raw - self.audio_lp);
            self.output_phase = self
                .output_phase
                .saturating_add(self.config.output_sample_rate_hz);
            if self.output_phase >= self.config.input_sample_rate_hz {
                self.output_phase -= self.config.input_sample_rate_hz;
                let mut sample = self.audio_lp;
                if let Some(us) = self.config.deemphasis_us {
                    let a = 1.0
                        - (-1.0 / (self.config.output_sample_rate_hz as f32 * us as f32 * 1e-6))
                            .exp();
                    self.deemphasis += a * (sample - self.deemphasis);
                    sample = self.deemphasis;
                }
                if sample.is_finite() {
                    out.push(sample.clamp(-1.0, 1.0));
                } else {
                    self.stats.demodulator_errors += 1;
                }
            }
        }
        let rms = if out.is_empty() {
            0.0
        } else {
            (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt()
        };
        let open = self
            .config
            .squelch_threshold
            .is_none_or(|threshold| rms >= threshold);
        if !open {
            out.fill(0.0);
        }
        self.stats.iq_bytes_processed += len as u64;
        self.stats.audio_samples_produced += out.len() as u64;
        self.stats.last_processed_unix_ms = unix_ms_now();
        self.stats.audio_peak = out.iter().copied().map(f32::abs).fold(0.0, f32::max);
        self.stats.audio_rms = if open { rms } else { 0.0 };
        self.stats.squelch_open = open;
        AudioBlock {
            sample_rate_hz: self.config.output_sample_rate_hz,
            samples: out,
        }
    }
}

fn one_pole_alpha(cutoff: f32, rate: f32) -> f32 {
    (1.0 - (-2.0 * PI * cutoff.min(rate * 0.45) / rate.max(1.0)).exp()).clamp(0.00001, 1.0)
}
fn decimation_ratio(input: u32, output: u32) -> usize {
    if input == 0 || output == 0 {
        1
    } else {
        ((input + output / 2) / output).max(1) as usize
    }
}
fn normalize_iq_byte(byte: u8) -> f32 {
    (f32::from(byte) - IQ_CENTER) / IQ_SCALE
}
fn unix_ms_now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_bounds() {
        assert_eq!(normalize_iq_byte(0), -1.0);
        assert_eq!(normalize_iq_byte(255), 1.0);
    }
    #[test]
    fn all_modes_are_finite() {
        for mode in [
            DemodulationMode::Am,
            DemodulationMode::Wbfm,
            DemodulationMode::Nbfm,
            DemodulationMode::Usb,
            DemodulationMode::Lsb,
        ] {
            let mut d = DspProcessor::new(DspConfig::preset(mode, 96_000));
            let a = d.process_iq_u8(&[128, 128, 180, 100].repeat(1000));
            assert!(a.samples.iter().all(|x| x.is_finite()));
        }
    }
    #[test]
    fn fractional_clock() {
        let mut c = DspConfig::am(1000);
        c.output_sample_rate_hz = 300;
        c.channel_bandwidth_hz = 500;
        c.audio_lowpass_hz = 100;
        let mut d = DspProcessor::new(c);
        assert_eq!(d.process_iq_u8(&[128; 2000]).samples.len(), 300);
    }
    #[test]
    fn fm_quadrature_detects_rotation() {
        let mut d = DspProcessor::new(DspConfig::preset(DemodulationMode::Nbfm, 48_000));
        let iq = [255, 128, 128, 255, 0, 128, 128, 0];
        let a = d.process_iq_u8(&iq);
        assert!(a.samples.iter().skip(1).any(|x| x.abs() > 0.01));
    }
    #[test]
    fn reset_clears_state() {
        let mut d = DspProcessor::new(DspConfig::am(96_000));
        d.process_iq_u8(&[255, 128].repeat(100));
        d.reset();
        assert_eq!(d.stats().iq_bytes_processed, 0);
    }
    #[test]
    fn config_validation() {
        let mut c = DspConfig::am(96_000);
        assert!(c.validate().is_ok());
        c.audio_lowpass_hz = 24_000;
        assert!(c.validate().is_err());
    }
}
