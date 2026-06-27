use axum::Json;
use serde::Serialize;

use crate::sdr::{self, SdrDeviceInfo};

use super::ApiError;

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
pub(crate) struct ListDevicesResponse {
    devices: Vec<DeviceResponse>,
}

#[derive(Serialize)]
pub(crate) struct DeviceResponse {
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
