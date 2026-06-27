mod devices;
mod session;

use std::sync::{Arc, Mutex};

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::session::SessionState;

#[derive(Clone, Default)]
pub struct ApiState {
    session: Arc<Mutex<SessionState>>,
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/devices", get(devices::list))
        .route("/api/session", get(session::get))
        .route(
            "/api/session/connect",
            axum::routing::post(session::connect),
        )
        .route(
            "/api/session/disconnect",
            axum::routing::post(session::disconnect),
        )
        .with_state(ApiState::default())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    pub fn conflict(message: &'static str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message,
        }
    }

    pub fn internal(message: &'static str) -> Self {
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
