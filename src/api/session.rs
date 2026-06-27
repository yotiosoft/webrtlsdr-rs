use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::session::{GainMode, ReceiverSettings, SessionError, SessionSnapshot};

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
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, index = request.index, "failed to connect RTL-SDR device");
            Err(ApiError::internal("failed to connect RTL-SDR device"))
        }
        Err(SessionError::NotConnected) => unreachable!("connect cannot require a prior session"),
    }
}

pub async fn disconnect(State(state): State<ApiState>) -> Result<Json<SessionResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    session.disconnect();

    Ok(Json(SessionResponse::from(session.snapshot())))
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
pub struct SessionResponse {
    connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<DeviceResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<SettingsResponse>,
}

impl From<SessionSnapshot> for SessionResponse {
    fn from(snapshot: SessionSnapshot) -> Self {
        Self {
            connected: snapshot.connected(),
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
