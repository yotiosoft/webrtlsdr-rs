use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

use crate::sdr::{self, SdrDeviceInfo};

pub async fn list() -> Result<Json<ListDevicesResponse>, ApiError> {
    let devices = sdr::list_devices().map_err(|error| {
        tracing::error!(%error, "failed to list RTL-SDR devices");
        ApiError::internal("failed to list RTL-SDR devices")
    })?;

    Ok(Json(ListDevicesResponse {
        devices: devices.into_iter().map(DeviceResponse::from).collect(),
    }))
}

#[derive(Serialize)]
pub struct ListDevicesResponse {
    devices: Vec<DeviceResponse>,
}

#[derive(Serialize)]
struct DeviceResponse {
    index: u32,
    name: String,
    manufacturer: String,
    product: String,
    serial: String,
}

impl From<SdrDeviceInfo> for DeviceResponse {
    fn from(device: SdrDeviceInfo) -> Self {
        Self {
            index: device.index,
            name: device.name,
            manufacturer: device.manufacturer,
            product: device.product,
            serial: device.serial,
        }
    }
}

pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    fn internal(message: &'static str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorResponse {
            error: self.message,
        });

        (self.status, body).into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}
