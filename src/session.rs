use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::sdr::{self, OpenedDevice, SdrDeviceInfo, SdrError};

const IQ_BLOCK_BYTES: usize = 16 * 16_384;

#[derive(Debug, Default)]
pub struct SessionState {
    connected: Option<OpenedDevice>,
    receiver: Option<ReceiveHandle>,
    settings: Option<ReceiverSettings>,
}

impl SessionState {
    pub fn connect(&mut self, index: u32) -> Result<SdrDeviceInfo, SessionError> {
        if self.connected.is_some() || self.receiver.is_some() {
            return Err(SessionError::AlreadyConnected);
        }

        let device = sdr::open_device(index).map_err(SessionError::Sdr)?;
        let info = device.info().clone();
        self.connected = Some(device);
        self.settings = Some(ReceiverSettings::default());

        Ok(info)
    }

    pub fn disconnect(&mut self) -> Result<(), SessionError> {
        self.stop_receiving()?;
        self.connected = None;
        self.settings = None;

        Ok(())
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            device: self
                .connected
                .as_ref()
                .map(|device| device.info().clone())
                .or_else(|| self.receiver.as_ref().map(|receiver| receiver.info.clone())),
            settings: self.settings.clone(),
            receiving: self.is_receiving(),
        }
    }

    pub fn set_center_frequency_hz(&mut self, frequency_hz: u32) -> Result<u32, SessionError> {
        if self.receiver.is_some() {
            return Err(SessionError::AlreadyReceiving);
        }
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
        if self.receiver.is_some() {
            return Err(SessionError::AlreadyReceiving);
        }
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
        if self.receiver.is_some() {
            return Err(SessionError::AlreadyReceiving);
        }
        let device = self.connected.as_mut().ok_or(SessionError::NotConnected)?;

        device.set_auto_gain().map_err(SessionError::Sdr)?;

        if let Some(settings) = &mut self.settings {
            settings.gain_mode = GainMode::Auto;
        }

        Ok(())
    }

    pub fn set_manual_gain_tenths_db(&mut self, gain_tenths_db: i32) -> Result<i32, SessionError> {
        if self.receiver.is_some() {
            return Err(SessionError::AlreadyReceiving);
        }
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

    pub fn start_receiving(&mut self) -> Result<(), SessionError> {
        if self.receiver.is_some() {
            return Err(SessionError::AlreadyReceiving);
        }

        let device = self.connected.take().ok_or(SessionError::NotConnected)?;
        self.receiver = Some(ReceiveHandle::spawn(device));

        Ok(())
    }

    pub fn stop_receiving(&mut self) -> Result<(), SessionError> {
        let Some(receiver) = self.receiver.take() else {
            return Ok(());
        };

        let device = receiver.stop()?;
        self.connected = Some(device);

        Ok(())
    }

    pub fn stats(&self) -> SessionStats {
        let receiver_stats = self.receiver.as_ref().map(ReceiveHandle::stats);

        SessionStats {
            connected: self.connected.is_some() || self.receiver.is_some(),
            receiving: receiver_stats
                .as_ref()
                .map(|stats| stats.receiving)
                .unwrap_or(false),
            blocks_read: receiver_stats
                .as_ref()
                .map(|stats| stats.blocks_read)
                .unwrap_or(0),
            bytes_read: receiver_stats
                .as_ref()
                .map(|stats| stats.bytes_read)
                .unwrap_or(0),
            last_block_bytes: receiver_stats
                .as_ref()
                .and_then(|stats| stats.last_block_bytes),
            last_block_unix_ms: receiver_stats
                .as_ref()
                .and_then(|stats| stats.last_block_unix_ms),
            last_error: receiver_stats.and_then(|stats| stats.last_error),
        }
    }

    fn is_receiving(&self) -> bool {
        self.receiver
            .as_ref()
            .map(|receiver| receiver.stats().receiving)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub device: Option<SdrDeviceInfo>,
    pub settings: Option<ReceiverSettings>,
    pub receiving: bool,
}

impl SessionSnapshot {
    pub fn connected(&self) -> bool {
        self.device.is_some()
    }
}

#[derive(Debug)]
pub enum SessionError {
    AlreadyConnected,
    AlreadyReceiving,
    NotConnected,
    ReceiveThreadPanicked,
    Sdr(SdrError),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyConnected => write!(formatter, "an RTL-SDR device is already connected"),
            Self::AlreadyReceiving => write!(formatter, "RTL-SDR reception is already running"),
            Self::NotConnected => write!(formatter, "no RTL-SDR device is connected"),
            Self::ReceiveThreadPanicked => write!(formatter, "RTL-SDR receive thread panicked"),
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionStats {
    pub connected: bool,
    pub receiving: bool,
    pub blocks_read: u64,
    pub bytes_read: u64,
    pub last_block_bytes: Option<usize>,
    pub last_block_unix_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug)]
struct ReceiveHandle {
    info: SdrDeviceInfo,
    stop_requested: Arc<AtomicBool>,
    stats: Arc<Mutex<SessionStats>>,
    thread: JoinHandle<OpenedDevice>,
}

impl ReceiveHandle {
    fn spawn(mut device: OpenedDevice) -> Self {
        let info = device.info().clone();
        let stop_requested = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Mutex::new(SessionStats {
            connected: true,
            receiving: true,
            ..SessionStats::default()
        }));

        let thread_stop_requested = Arc::clone(&stop_requested);
        let thread_stats = Arc::clone(&stats);
        let thread = thread::spawn(move || {
            receive_loop(&mut device, thread_stop_requested, thread_stats);
            device
        });

        Self {
            info,
            stop_requested,
            stats,
            thread,
        }
    }

    fn stop(self) -> Result<OpenedDevice, SessionError> {
        self.stop_requested.store(true, Ordering::Release);

        let device = self
            .thread
            .join()
            .map_err(|_| SessionError::ReceiveThreadPanicked)?;

        if let Ok(mut stats) = self.stats.lock() {
            stats.receiving = false;
        }

        Ok(device)
    }

    fn stats(&self) -> SessionStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_else(|error| {
                tracing::error!(%error, "receive stats mutex is poisoned");
                SessionStats {
                    connected: true,
                    last_error: Some("receive stats unavailable".to_string()),
                    ..SessionStats::default()
                }
            })
    }
}

fn receive_loop(
    device: &mut OpenedDevice,
    stop_requested: Arc<AtomicBool>,
    stats: Arc<Mutex<SessionStats>>,
) {
    if let Err(error) = device.reset_buffer() {
        record_receive_error(&stats, &error);
        return;
    }

    let mut buffer = vec![0_u8; IQ_BLOCK_BYTES];
    while !stop_requested.load(Ordering::Acquire) {
        match device.read_sync(&mut buffer) {
            Ok(0) => {
                let message = "RTL-SDR read returned zero bytes";
                tracing::warn!(index = device.info().index, message);
                record_receive_message(&stats, message);
                break;
            }
            Ok(n_read) => record_receive_block(&stats, n_read),
            Err(error) => {
                tracing::error!(%error, "failed to read RTL-SDR IQ samples");
                record_receive_error(&stats, &error);
                break;
            }
        }
    }

    if let Ok(mut stats) = stats.lock() {
        stats.receiving = false;
    }
}

fn record_receive_block(stats: &Mutex<SessionStats>, n_read: usize) {
    let last_block_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok());

    if let Ok(mut stats) = stats.lock() {
        stats.blocks_read = stats.blocks_read.saturating_add(1);
        stats.bytes_read = stats.bytes_read.saturating_add(n_read as u64);
        stats.last_block_bytes = Some(n_read);
        stats.last_block_unix_ms = last_block_unix_ms;
    }
}

fn record_receive_error(stats: &Mutex<SessionStats>, error: &SdrError) {
    record_receive_message(stats, &error.to_string());
}

fn record_receive_message(stats: &Mutex<SessionStats>, message: &str) {
    if let Ok(mut stats) = stats.lock() {
        stats.receiving = false;
        stats.last_error = Some(message.to_string());
    }
}
