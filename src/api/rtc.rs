use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::{
    api::{ApiError, ApiState},
    webrtc::{OfferRequest, OfferResponse, WebRtcConfigResponse, WebRtcStatsResponse},
};

pub async fn config(State(state): State<ApiState>) -> Json<WebRtcConfigResponse> {
    Json(state.webrtc.config_response())
}

pub async fn stats(State(state): State<ApiState>) -> Json<WebRtcStatsResponse> {
    Json(state.webrtc.stats_response().await)
}

pub async fn offer(
    State(state): State<ApiState>,
    Json(request): Json<OfferRequest>,
) -> Result<Json<OfferResponse>, ApiError> {
    state
        .webrtc
        .create_answer(request)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::warn!(%error, "failed to create WebRTC answer");
            ApiError::bad_request("failed to create WebRTC answer")
        })
}

pub async fn close(
    State(state): State<ApiState>,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .webrtc
        .close(&session_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|error| {
            tracing::warn!(%session_id, %error, "failed to close WebRTC session");
            ApiError::internal("failed to close WebRTC session")
        })
}
