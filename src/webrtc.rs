use std::{collections::HashMap, sync::Arc, time::SystemTime};

use anyhow::{Context, anyhow, bail};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use webrtc::{
    api::{
        APIBuilder, interceptor_registry::register_default_interceptors, media_engine::MediaEngine,
    },
    data_channel::RTCDataChannel,
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebRtcConfig {
    pub ice_servers: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionDescriptionDto {
    pub sdp: String,
    #[serde(rename = "type")]
    pub sdp_type: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OfferRequest {
    pub sdp: String,
    #[serde(rename = "type")]
    pub sdp_type: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct OfferResponse {
    pub sdp: String,
    #[serde(rename = "type")]
    pub sdp_type: String,
    pub session_id: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WebRtcConfigResponse {
    pub ice_servers: Vec<String>,
}

#[derive(Default)]
pub struct WebRtcSessionManager {
    config: WebRtcConfig,
    next_session: Mutex<u64>,
    sessions: Mutex<HashMap<String, WebRtcSession>>,
}

struct WebRtcSession {
    peer_connection: Arc<RTCPeerConnection>,
    created_at: SystemTime,
    connection_state: RTCPeerConnectionState,
}

impl WebRtcSessionManager {
    pub fn new(config: WebRtcConfig) -> Self {
        Self {
            config,
            next_session: Mutex::new(1),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn config_response(&self) -> WebRtcConfigResponse {
        WebRtcConfigResponse {
            ice_servers: self.config.ice_servers.clone(),
        }
    }

    pub async fn create_answer(&self, offer: OfferRequest) -> anyhow::Result<OfferResponse> {
        if offer.sdp_type != "offer" {
            bail!("WebRTC signaling request type must be offer");
        }
        if offer.sdp.trim().is_empty() {
            bail!("WebRTC offer SDP is empty");
        }

        let session_id = self.allocate_session_id().await;
        let peer_connection = self.new_peer_connection().await?;
        let state_session_id = session_id.clone();
        let state_peer_connection = Arc::clone(&peer_connection);

        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let session_id = state_session_id.clone();
            let peer_connection = Arc::clone(&state_peer_connection);
            Box::pin(async move {
                tracing::info!(%session_id, ?state, "WebRTC peer connection state changed");
                if matches!(
                    state,
                    RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed
                ) {
                    if let Err(error) = peer_connection.close().await {
                        tracing::warn!(%session_id, %error, "failed to close WebRTC peer connection after terminal state");
                    }
                }
            })
        }));

        peer_connection.on_data_channel(Box::new(move |data_channel: Arc<RTCDataChannel>| {
            tracing::info!(label = %data_channel.label(), "WebRTC data channel opened by browser");
            Box::pin(async move {})
        }));

        let remote_offer = RTCSessionDescription::offer(offer.sdp)
            .context("failed to build WebRTC offer description")?;
        peer_connection
            .set_remote_description(remote_offer)
            .await
            .context("failed to set WebRTC remote offer")?;

        let answer = peer_connection
            .create_answer(None)
            .await
            .context("failed to create WebRTC answer")?;
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        peer_connection
            .set_local_description(answer)
            .await
            .context("failed to set WebRTC local answer")?;
        let _ = gather_complete.recv().await;

        let local_description = peer_connection
            .local_description()
            .await
            .ok_or_else(|| anyhow!("WebRTC local answer was not set"))?;

        self.sessions.lock().await.insert(
            session_id.clone(),
            WebRtcSession {
                peer_connection,
                created_at: SystemTime::now(),
                connection_state: RTCPeerConnectionState::New,
            },
        );

        Ok(OfferResponse {
            sdp: local_description.sdp,
            sdp_type: "answer".to_string(),
            session_id,
        })
    }

    pub async fn close(&self, session_id: &str) -> anyhow::Result<bool> {
        let session = self.sessions.lock().await.remove(session_id);
        if let Some(session) = session {
            tracing::info!(%session_id, created_at = ?session.created_at, state = ?session.connection_state, "closing WebRTC session");
            session
                .peer_connection
                .close()
                .await
                .context("failed to close WebRTC peer connection")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[cfg(test)]
    async fn len(&self) -> usize {
        self.sessions.lock().await.len()
    }

    async fn allocate_session_id(&self) -> String {
        let mut next_session = self.next_session.lock().await;
        let session_id = format!("rtc_{}", *next_session);
        *next_session += 1;
        session_id
    }

    async fn new_peer_connection(&self) -> anyhow::Result<Arc<RTCPeerConnection>> {
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .context("failed to register WebRTC default codecs")?;
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)
            .context("failed to register WebRTC default interceptors")?;
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();
        let config = RTCConfiguration {
            ice_servers: self
                .config
                .ice_servers
                .iter()
                .map(|url| RTCIceServer {
                    urls: vec![url.clone()],
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };

        api.new_peer_connection(config)
            .await
            .map(Arc::new)
            .context("failed to create WebRTC peer connection")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_request_deserializes_browser_shape() {
        let request: OfferRequest =
            serde_json::from_str(r#"{"type":"offer","sdp":"v=0"}"#).unwrap();
        assert_eq!(request.sdp_type, "offer");
        assert_eq!(request.sdp, "v=0");
    }

    #[test]
    fn offer_response_serializes_browser_shape() {
        let response = OfferResponse {
            sdp: "v=0".to_string(),
            sdp_type: "answer".to_string(),
            session_id: "rtc_1".to_string(),
        };
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["type"], "answer");
        assert_eq!(json["sdp"], "v=0");
        assert_eq!(json["session_id"], "rtc_1");
    }

    #[tokio::test]
    async fn close_unknown_session_is_noop() {
        let manager = WebRtcSessionManager::new(WebRtcConfig::default());
        assert!(!manager.close("missing").await.unwrap());
        assert_eq!(manager.len().await, 0);
    }
}
