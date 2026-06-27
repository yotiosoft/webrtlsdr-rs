use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::session::{SessionError, SessionSnapshot};

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
        Ok(device) => Ok(Json(SessionResponse {
            connected: true,
            device: Some(DeviceResponse::from(device)),
        })),
        Err(SessionError::AlreadyConnected) => {
            Err(ApiError::conflict("an RTL-SDR device is already connected"))
        }
        Err(SessionError::Sdr(error)) => {
            tracing::error!(%error, index = request.index, "failed to connect RTL-SDR device");
            Err(ApiError::internal("failed to connect RTL-SDR device"))
        }
    }
}

pub async fn disconnect(State(state): State<ApiState>) -> Result<Json<SessionResponse>, ApiError> {
    let mut session = state.session.lock().map_err(|error| {
        tracing::error!(%error, "session mutex is poisoned");
        ApiError::internal("failed to update RTL-SDR session")
    })?;

    session.disconnect();

    Ok(Json(SessionResponse {
        connected: false,
        device: None,
    }))
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    index: u32,
}

#[derive(Serialize)]
pub struct SessionResponse {
    connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    device: Option<DeviceResponse>,
}

impl From<SessionSnapshot> for SessionResponse {
    fn from(snapshot: SessionSnapshot) -> Self {
        Self {
            connected: snapshot.connected(),
            device: snapshot.device.map(DeviceResponse::from),
        }
    }
}
