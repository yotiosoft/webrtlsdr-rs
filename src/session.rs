use std::fmt;

use crate::sdr::{self, OpenedDevice, SdrDeviceInfo, SdrError};

#[derive(Debug, Default)]
pub struct SessionState {
    connected: Option<OpenedDevice>,
    settings: Option<ReceiverSettings>,
}

impl SessionState {
    pub fn connect(&mut self, index: u32) -> Result<SdrDeviceInfo, SessionError> {
        if self.connected.is_some() {
            return Err(SessionError::AlreadyConnected);
        }

        let device = sdr::open_device(index).map_err(SessionError::Sdr)?;
        let info = device.info().clone();
        self.connected = Some(device);
        self.settings = Some(ReceiverSettings::default());

        Ok(info)
    }

    pub fn disconnect(&mut self) {
        self.connected = None;
        self.settings = None;
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            device: self.connected.as_ref().map(|device| device.info().clone()),
            settings: self.settings.clone(),
        }
    }

    pub fn set_center_frequency_hz(&mut self, frequency_hz: u32) -> Result<u32, SessionError> {
        let device = self.connected.as_mut().ok_or(SessionError::NotConnected)?;

        device
            .set_center_frequency_hz(frequency_hz)
            .map_err(SessionError::Sdr)?;
        let actual_frequency_hz = device.center_frequency_hz();

        if let Some(settings) = &mut self.settings {
            settings.center_frequency_hz = Some(actual_frequency_hz);
        }

        Ok(actual_frequency_hz)
    }

    pub fn set_sample_rate_hz(&mut self, sample_rate_hz: u32) -> Result<u32, SessionError> {
        let device = self.connected.as_mut().ok_or(SessionError::NotConnected)?;

        device
            .set_sample_rate_hz(sample_rate_hz)
            .map_err(SessionError::Sdr)?;
        let actual_sample_rate_hz = device.sample_rate_hz();

        if let Some(settings) = &mut self.settings {
            settings.sample_rate_hz = Some(actual_sample_rate_hz);
        }

        Ok(actual_sample_rate_hz)
    }

    pub fn set_auto_gain(&mut self) -> Result<(), SessionError> {
        let device = self.connected.as_mut().ok_or(SessionError::NotConnected)?;

        device.set_auto_gain().map_err(SessionError::Sdr)?;

        if let Some(settings) = &mut self.settings {
            settings.gain_mode = GainMode::Auto;
        }

        Ok(())
    }

    pub fn set_manual_gain_tenths_db(&mut self, gain_tenths_db: i32) -> Result<i32, SessionError> {
        let device = self.connected.as_mut().ok_or(SessionError::NotConnected)?;

        device
            .set_manual_gain_tenths_db(gain_tenths_db)
            .map_err(SessionError::Sdr)?;
        let actual_gain_tenths_db = device.tuner_gain_tenths_db();

        if let Some(settings) = &mut self.settings {
            settings.gain_mode = GainMode::Manual {
                gain_tenths_db: actual_gain_tenths_db,
            };
        }

        Ok(actual_gain_tenths_db)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub device: Option<SdrDeviceInfo>,
    pub settings: Option<ReceiverSettings>,
}

impl SessionSnapshot {
    pub fn connected(&self) -> bool {
        self.device.is_some()
    }
}

#[derive(Debug)]
pub enum SessionError {
    AlreadyConnected,
    NotConnected,
    Sdr(SdrError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConnected => write!(formatter, "an RTL-SDR device is already connected"),
            Self::NotConnected => write!(formatter, "no RTL-SDR device is connected"),
            Self::Sdr(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReceiverSettings {
    pub center_frequency_hz: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub gain_mode: GainMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GainMode {
    #[default]
    Auto,
    Manual {
        gain_tenths_db: i32,
    },
}
