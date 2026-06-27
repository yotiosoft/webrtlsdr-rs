use std::fmt;

use crate::sdr::{self, OpenedDevice, SdrDeviceInfo, SdrError};

#[derive(Debug, Default)]
pub struct SessionState {
    connected: Option<OpenedDevice>,
}

impl SessionState {
    pub fn connect(&mut self, index: u32) -> Result<SdrDeviceInfo, SessionError> {
        if self.connected.is_some() {
            return Err(SessionError::AlreadyConnected);
        }

        let device = sdr::open_device(index).map_err(SessionError::Sdr)?;
        let info = device.info().clone();
        self.connected = Some(device);

        Ok(info)
    }

    pub fn disconnect(&mut self) {
        self.connected = None;
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            device: self.connected.as_ref().map(|device| device.info().clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub device: Option<SdrDeviceInfo>,
}

impl SessionSnapshot {
    pub fn connected(&self) -> bool {
        self.device.is_some()
    }
}

#[derive(Debug)]
pub enum SessionError {
    AlreadyConnected,
    Sdr(SdrError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConnected => write!(formatter, "an RTL-SDR device is already connected"),
            Self::Sdr(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for SessionError {}
