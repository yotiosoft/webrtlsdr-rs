mod devices;

use axum::{Json, Router, routing::get};
use serde::Serialize;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/devices", get(devices::list))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}
