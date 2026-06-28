use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    audio::PcmStats,
    dsp::DspStats,
    session::{GainMode, ReceiverSettings, SessionError, SessionSnapshot, SessionStats},
    stream::StreamStats,
};

use super::{ApiError, ApiState, devices::DeviceResponse};

pub async fn get(State(state): State<ApiState>) -> Result<Json<SessionResponse>, ApiError> {
    let session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to read RTL-SDR session")
    })?;

    Ok(Json(SessionResponse::from(session.snapshot())))
}

pub async fn connect(
    State(state): State<ApiState>,
    Json(request): Json<ConnectRequest>,
) -> Result<Json<SessionResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match session.connect(request.index) {
        Ok(_) => Ok(Json(SessionResponse::from(session.snapshot()))),
        Err(SessionError::AlreadyConnected) => {
            Err(ApiError::conflict("an RTL-SDR device is already connected"))
        }
        Err(SessionError::AlreadyReceiving) => {
            Err(ApiError::conflict("RTL-SDR reception is already running"))
        }
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, index = request.index, "failed to connect RTL-SDR device");
            Err(ApiError::internal("failed to connect RTL-SDR device"))
        }
        Err(SessionError::ReceiveThreadPanicked) => {
            unreachable!("connect cannot join a receive thread")
        }
        Err(SessionError::NotConnected) => unreachable!("connect cannot require a prior session"),
    }
}

pub async fn disconnect(State(state): State<ApiState>) -> Result<Json<SessionResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    if let Err(error) = session.disconnect() {
        tracing::error!(%error, "failed to stop RTL-SDR reception before disconnect");
        return Err(ApiError::internal("failed to disconnect RTL-SDR device"));
    }

    Ok(Json(SessionResponse::from(session.snapshot())))
}

pub async fn start(State(state): State<ApiState>) -> Result<Json<ReceiveStateResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match session.start_receiving(state.audio_stream.clone()) {
        Ok(()) => Ok(Json(ReceiveStateResponse { receiving: true })),
        Err(SessionError::NotConnected) => {
            Err(ApiError::conflict("no RTL-SDR device is connected"))
        }
        Err(SessionError::AlreadyReceiving) => {
            Err(ApiError::conflict("RTL-SDR reception is already running"))
        }
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, "failed to start RTL-SDR reception");
            Err(ApiError::internal("failed to start RTL-SDR reception"))
        }
        Err(SessionError::AlreadyConnected) => unreachable!("start cannot connect a session"),
        Err(SessionError::ReceiveThreadPanicked) => {
            unreachable!("start cannot join a receive thread")
        }
    }
}

pub async fn stop(State(state): State<ApiState>) -> Result<Json<ReceiveStateResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match session.stop_receiving() {
        Ok(()) => Ok(Json(ReceiveStateResponse { receiving: false })),
        Err(SessionError::ReceiveThreadPanicked) => {
            tracing::error!("RTL-SDR receive thread panicked while stopping");
            Err(ApiError::internal("failed to stop RTL-SDR reception"))
        }
        Err(SessionError::NotConnected) => unreachable!("stop is idempotent without a session"),
        Err(SessionError::AlreadyReceiving) => unreachable!("stop cannot start reception"),
        Err(SessionError::AlreadyConnected) => unreachable!("stop cannot connect a session"),
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, "failed to stop RTL-SDR reception");
            Err(ApiError::internal("failed to stop RTL-SDR reception"))
        }
    }
}

pub async fn stats(State(state): State<ApiState>) -> Result<Json<SessionStatsResponse>, ApiError> {
    let session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to read RTL-SDR session")
    })?;

    Ok(Json(SessionStatsResponse::from_stats(
        session.stats(),
        state.audio_stream.stats(),
    )))
}

pub async fn tune(
    State(state): State<ApiState>,
    Json(request): Json<TuneRequest>,
) -> Result<Json<TuneResponse>, ApiError> {
    if request.frequency_hz == 0 {
        return Err(ApiError::bad_request("frequency_hz must be greater than 0"));
    }

    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match session.set_center_frequency_hz(request.frequency_hz) {
        Ok(center_frequency_hz) => Ok(Json(TuneResponse {
            center_frequency_hz,
        })),
        Err(SessionError::NotConnected) => {
            Err(ApiError::conflict("no RTL-SDR device is connected"))
        }
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, "failed to tune RTL-SDR device");
            Err(ApiError::internal("failed to tune RTL-SDR device"))
        }
        Err(SessionError::AlreadyReceiving) => {
            Err(ApiError::conflict("RTL-SDR reception is already running"))
        }
        Err(SessionError::ReceiveThreadPanicked) => {
            unreachable!("tuning cannot join a receive thread")
        }
        Err(SessionError::AlreadyConnected) => unreachable!("tuning cannot create a session"),
    }
}

pub async fn sample_rate(
    State(state): State<ApiState>,
    Json(request): Json<SampleRateRequest>,
) -> Result<Json<SampleRateResponse>, ApiError> {
    if request.sample_rate_hz == 0 {
        return Err(ApiError::bad_request(
            "sample_rate_hz must be greater than 0",
        ));
    }

    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match session.set_sample_rate_hz(request.sample_rate_hz) {
        Ok(sample_rate_hz) => Ok(Json(SampleRateResponse { sample_rate_hz })),
        Err(SessionError::NotConnected) => {
            Err(ApiError::conflict("no RTL-SDR device is connected"))
        }
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, "failed to set RTL-SDR sample rate");
            Err(ApiError::internal("failed to set RTL-SDR sample rate"))
        }
        Err(SessionError::AlreadyReceiving) => {
            Err(ApiError::conflict("RTL-SDR reception is already running"))
        }
        Err(SessionError::ReceiveThreadPanicked) => {
            unreachable!("sample-rate setting cannot join a receive thread")
        }
        Err(SessionError::AlreadyConnected) => {
            unreachable!("sample-rate setting cannot create a session")
        }
    }
}

pub async fn gain(
    State(state): State<ApiState>,
    Json(request): Json<GainRequest>,
) -> Result<Json<GainResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    match request.mode {
        GainModeRequest::Auto => match session.set_auto_gain() {
            Ok(()) => Ok(Json(GainResponse {
                gain_mode: GainModeResponse::Auto,
                gain_tenths_db: None,
            })),
            Err(SessionError::NotConnected) => {
                Err(ApiError::conflict("no RTL-SDR device is connected"))
            }
            Err(SessionError::Sdr(error)) => {
                tracing::error!(%error, "failed to set RTL-SDR auto gain");
                Err(ApiError::internal("failed to set RTL-SDR gain"))
            }
            Err(SessionError::AlreadyReceiving) => {
                Err(ApiError::conflict("RTL-SDR reception is already running"))
            }
            Err(SessionError::ReceiveThreadPanicked) => {
                unreachable!("gain setting cannot join a receive thread")
            }
            Err(SessionError::AlreadyConnected) => {
                unreachable!("gain setting cannot create a session")
            }
        },
        GainModeRequest::Manual => {
            let gain_tenths_db = request.gain_tenths_db.ok_or_else(|| {
                ApiError::bad_request("gain_tenths_db is required for manual gain")
            })?;

            match session.set_manual_gain_tenths_db(gain_tenths_db) {
                Ok(gain_tenths_db) => Ok(Json(GainResponse {
                    gain_mode: GainModeResponse::Manual,
                    gain_tenths_db: Some(gain_tenths_db),
                })),
                Err(SessionError::NotConnected) => {
                    Err(ApiError::conflict("no RTL-SDR device is connected"))
                }
                Err(SessionError::Sdr(error)) => {
                    tracing::error!(%error, gain_tenths_db, "failed to set RTL-SDR manual gain");
                    Err(ApiError::internal("failed to set RTL-SDR gain"))
                }
                Err(SessionError::AlreadyReceiving) => {
                    Err(ApiError::conflict("RTL-SDR reception is already running"))
                }
                Err(SessionError::ReceiveThreadPanicked) => {
                    unreachable!("gain setting cannot join a receive thread")
                }
                Err(SessionError::AlreadyConnected) => {
                    unreachable!("gain setting cannot create a session")
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    index: u32,
}

#[derive(Deserialize)]
pub struct TuneRequest {
    frequency_hz: u32,
}

#[derive(Serialize)]
pub struct TuneResponse {
    center_frequency_hz: u32,
}

#[derive(Deserialize)]
pub struct SampleRateRequest {
    sample_rate_hz: u32,
}

#[derive(Serialize)]
pub struct SampleRateResponse {
    sample_rate_hz: u32,
}

#[derive(Deserialize)]
pub struct GainRequest {
    mode: GainModeRequest,
    gain_tenths_db: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GainModeRequest {
    Auto,
    Manual,
}

#[derive(Serialize)]
pub struct GainResponse {
    #[serde(rename = "gain_mode")]
    gain_mode: GainModeResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    gain_tenths_db: Option<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GainModeResponse {
    Auto,
    Manual,
}

#[derive(Serialize)]
pub struct ReceiveStateResponse {
    receiving: bool,
}

#[derive(Serialize)]
pub struct SessionResponse {
    connected: bool,
    receiving: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<DeviceResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<SettingsResponse>,
}

impl From<SessionSnapshot> for SessionResponse {
    fn from(snapshot: SessionSnapshot) -> Self {
        Self {
            connected: snapshot.connected(),
            receiving: snapshot.receiving,
            device: snapshot.device.map(DeviceResponse::from),
            settings: snapshot.settings.map(SettingsResponse::from),
        }
    }
}

#[derive(Serialize)]
pub struct SettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    center_frequency_hz: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_rate_hz: Option<u32>,
    gain_mode: GainModeResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    gain_tenths_db: Option<i32>,
}

impl From<ReceiverSettings> for SettingsResponse {
    fn from(settings: ReceiverSettings) -> Self {
        let (gain_mode, gain_tenths_db) = match settings.gain_mode {
            GainMode::Auto => (GainModeResponse::Auto, None),
            GainMode::Manual { gain_tenths_db } => (GainModeResponse::Manual, Some(gain_tenths_db)),
        };

        Self {
            center_frequency_hz: settings.center_frequency_hz,
            sample_rate_hz: settings.sample_rate_hz,
            gain_mode,
            gain_tenths_db,
        }
    }
}

#[derive(Serialize)]
pub struct SessionStatsResponse {
    connected: bool,
    receiving: bool,
    blocks_read: u64,
    bytes_read: u64,
    last_block_bytes: Option<usize>,
    last_block_unix_ms: Option<u64>,
    dsp: DspStatsResponse,
    pcm: PcmStatsResponse,
    stream: StreamStatsResponse,
    last_error: Option<String>,
}

impl From<SessionStats> for SessionStatsResponse {
    fn from(stats: SessionStats) -> Self {
        Self::from_stats(stats, StreamStats::default())
    }
}

impl SessionStatsResponse {
    fn from_stats(stats: SessionStats, stream: StreamStats) -> Self {
        Self {
            connected: stats.connected,
            receiving: stats.receiving,
            blocks_read: stats.blocks_read,
            bytes_read: stats.bytes_read,
            last_block_bytes: stats.last_block_bytes,
            last_block_unix_ms: stats.last_block_unix_ms,
            dsp: DspStatsResponse::from(stats.dsp),
            pcm: PcmStatsResponse::from(stats.pcm),
            stream: StreamStatsResponse::from(stream),
            last_error: stats.last_error,
        }
    }
}

#[derive(Serialize)]
pub struct DspStatsResponse {
    iq_bytes_processed: u64,
    audio_samples_produced: u64,
    audio_sample_rate_hz: u32,
    decimation_ratio: usize,
    last_processed_unix_ms: Option<u64>,
    audio_peak: f32,
    audio_rms: f32,
    last_error: Option<String>,
}

impl From<DspStats> for DspStatsResponse {
    fn from(stats: DspStats) -> Self {
        Self {
            iq_bytes_processed: stats.iq_bytes_processed,
            audio_samples_produced: stats.audio_samples_produced,
            audio_sample_rate_hz: stats.audio_sample_rate_hz,
            decimation_ratio: stats.decimation_ratio,
            last_processed_unix_ms: stats.last_processed_unix_ms,
            audio_peak: stats.audio_peak,
            audio_rms: stats.audio_rms,
            last_error: stats.last_error,
        }
    }
}

#[derive(Serialize)]
pub struct PcmStatsResponse {
    frames_produced: u64,
    samples_produced: u64,
    bytes_produced: u64,
    last_frame_bytes: usize,
    last_sequence: Option<u64>,
    last_pts_samples: Option<u64>,
    peak_before_clamp: f32,
    clipped_samples: u64,
    last_error: Option<String>,
}

impl From<PcmStats> for PcmStatsResponse {
    fn from(stats: PcmStats) -> Self {
        Self {
            frames_produced: stats.frames_produced,
            samples_produced: stats.samples_produced,
            bytes_produced: stats.bytes_produced,
            last_frame_bytes: stats.last_frame_bytes,
            last_sequence: stats.last_sequence,
            last_pts_samples: stats.last_pts_samples,
            peak_before_clamp: stats.peak_before_clamp,
            clipped_samples: stats.clipped_samples,
            last_error: stats.last_error,
        }
    }
}

#[derive(Serialize)]
pub struct StreamStatsResponse {
    active_clients: usize,
    frames_sent: u64,
    bytes_sent: u64,
    samples_sent: u64,
    last_sequence: Option<u64>,
    last_pts_samples: Option<u64>,
    dropped_frames: u64,
    lagged_subscribers: u64,
    frames_broadcast: u64,
    bytes_broadcast: u64,
    frames_dropped: u64,
    last_client_connected_unix_ms: Option<u64>,
    last_client_disconnected_unix_ms: Option<u64>,
    last_error: Option<String>,
}

impl From<StreamStats> for StreamStatsResponse {
    fn from(stats: StreamStats) -> Self {
        Self {
            active_clients: stats.active_clients,
            frames_sent: stats.frames_sent,
            bytes_sent: stats.bytes_sent,
            samples_sent: stats.samples_sent,
            last_sequence: stats.last_sequence,
            last_pts_samples: stats.last_pts_samples,
            dropped_frames: stats.dropped_frames,
            lagged_subscribers: stats.lagged_subscribers,
            frames_broadcast: stats.frames_broadcast,
            bytes_broadcast: stats.bytes_broadcast,
            frames_dropped: stats.frames_dropped,
            last_client_connected_unix_ms: stats.last_client_connected_unix_ms,
            last_client_disconnected_unix_ms: stats.last_client_disconnected_unix_ms,
            last_error: stats.last_error,
        }
    }
}
