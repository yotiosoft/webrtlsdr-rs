mod devices;
mod session;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Serialize;

use crate::{session::SessionState, stream::AudioStreamHub};

#[derive(Clone)]
pub struct ApiState {
    pub(crate) session: Arc<Mutex<SessionState>>,
    pub(crate) audio_stream: AudioStreamHub,
}

impl Default for ApiState {
    fn default() -> Self {
        Self {
            session: Arc::new(Mutex::new(SessionState::default())),
            audio_stream: AudioStreamHub::default(),
        }
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ws/audio", get(audio_websocket))
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
        .route("/api/session/start", axum::routing::post(session::start))
        .route("/api/session/stop", axum::routing::post(session::stop))
        .route("/api/session/stats", get(session::stats))
        .route("/api/session/tune", axum::routing::post(session::tune))
        .route(
            "/api/session/sample-rate",
            axum::routing::post(session::sample_rate),
        )
        .route("/api/session/gain", axum::routing::post(session::gain))
        .with_state(ApiState::default())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn audio_websocket(
    State(state): State<ApiState>,
    websocket: WebSocketUpgrade,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| crate::stream::serve_audio_websocket(socket, state.audio_stream))
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
    pub fn bad_request(message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }

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
