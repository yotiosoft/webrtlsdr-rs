mod devices;
mod rtc;
mod session;

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::{State, ws::WebSocketUpgrade},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use serde::Serialize;

use crate::{
    session::SessionState,
    stream::AudioStreamHub,
    webrtc::{WebRtcConfig, WebRtcSessionManager},
};

#[derive(Clone)]
pub struct ApiState {
    pub(crate) session: Arc<Mutex<SessionState>>,
    pub(crate) audio_stream: AudioStreamHub,
    pub(crate) webrtc: Arc<WebRtcSessionManager>,
}

impl ApiState {
    fn new(webrtc_config: WebRtcConfig) -> Self {
        let audio_stream = AudioStreamHub::default();
        Self {
            session: Arc::new(Mutex::new(SessionState::default())),
            audio_stream: audio_stream.clone(),
            webrtc: Arc::new(WebRtcSessionManager::new(webrtc_config, audio_stream)),
        }
    }
}

pub fn router(webrtc_config: WebRtcConfig) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/audio-worklet.js", get(audio_worklet_js))
        .route("/health", get(health))
        .route("/ws/audio", get(audio_websocket))
        .route("/ws/audio-v1", get(audio_v1_websocket))
        .route("/api/devices", get(devices::list))
        .route("/api/webrtc/config", get(rtc::config))
        .route("/api/webrtc/stats", get(rtc::stats))
        .route("/api/webrtc/offer", axum::routing::post(rtc::offer))
        .route(
            "/api/webrtc/sessions/:session_id",
            axum::routing::delete(rtc::close),
        )
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
        .with_state(ApiState::new(webrtc_config))
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../public/index.html"))
}

async fn app_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../public/app.js"),
    )
}

async fn audio_worklet_js() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../public/audio-worklet.js"),
    )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn audio_websocket(
    State(state): State<ApiState>,
    websocket: WebSocketUpgrade,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| {
        crate::stream::serve_audio_websocket(
            socket,
            state.audio_stream,
            crate::stream::AudioWebSocketProtocol::LegacyRawPcm,
        )
    })
}

async fn audio_v1_websocket(
    State(state): State<ApiState>,
    websocket: WebSocketUpgrade,
) -> impl IntoResponse {
    websocket.on_upgrade(|socket| {
        crate::stream::serve_audio_websocket(
            socket,
            state.audio_stream,
            crate::stream::AudioWebSocketProtocol::PcmFrameV1,
        )
    })
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
