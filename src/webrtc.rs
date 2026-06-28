use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use anyhow::{Context, anyhow, bail};
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};
use webrtc::{
    api::{
        APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MediaEngine},
    },
    data_channel::RTCDataChannel,
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    media::Sample,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::{rtp_codec::RTCRtpCodecCapability, rtp_sender::RTCRtpSender},
    track::track_local::{TrackLocal, track_local_static_sample::TrackLocalStaticSample},
};

const SILENT_AUDIO_FRAME_DURATION: Duration = Duration::from_millis(20);
const OPUS_SILENCE_PACKET: &[u8] = &[0xf8, 0xff, 0xfe];

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

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WebRtcStatsResponse {
    pub active_sessions: usize,
    pub audio_frames_sent: u64,
    pub audio_bytes_sent: u64,
    pub last_audio_send_error: Option<String>,
    pub peer_connections: Vec<WebRtcPeerConnectionStats>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WebRtcPeerConnectionStats {
    pub session_id: String,
    pub peer_connection_state: String,
    pub created_at_unix_ms: Option<u128>,
}

#[derive(Default)]
pub struct WebRtcSessionManager {
    config: WebRtcConfig,
    next_session: Mutex<u64>,
    sessions: Mutex<HashMap<String, WebRtcSession>>,
    audio_stats: Arc<WebRtcAudioStats>,
}

struct WebRtcSession {
    peer_connection: Arc<RTCPeerConnection>,
    audio_send_task: JoinHandle<()>,
    rtcp_read_task: JoinHandle<()>,
    created_at: SystemTime,
    connection_state: RTCPeerConnectionState,
}

#[derive(Default)]
struct WebRtcAudioStats {
    frames_sent: AtomicU64,
    bytes_sent: AtomicU64,
    last_send_error: Mutex<Option<String>>,
}

impl WebRtcSessionManager {
    pub fn new(config: WebRtcConfig) -> Self {
        Self {
            config,
            next_session: Mutex::new(1),
            sessions: Mutex::new(HashMap::new()),
            audio_stats: Arc::new(WebRtcAudioStats::default()),
        }
    }

    pub fn config_response(&self) -> WebRtcConfigResponse {
        WebRtcConfigResponse {
            ice_servers: self.config.ice_servers.clone(),
        }
    }

    pub async fn stats_response(&self) -> WebRtcStatsResponse {
        let sessions = self.sessions.lock().await;
        WebRtcStatsResponse {
            active_sessions: sessions.len(),
            audio_frames_sent: self.audio_stats.frames_sent.load(Ordering::Relaxed),
            audio_bytes_sent: self.audio_stats.bytes_sent.load(Ordering::Relaxed),
            last_audio_send_error: self.audio_stats.last_send_error.lock().await.clone(),
            peer_connections: sessions
                .iter()
                .map(|(session_id, session)| WebRtcPeerConnectionStats {
                    session_id: session_id.clone(),
                    peer_connection_state: format!(
                        "{:?}",
                        session.peer_connection.connection_state()
                    ),
                    created_at_unix_ms: session
                        .created_at
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                        .map(|duration| duration.as_millis()),
                })
                .collect(),
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
        let (audio_track, audio_sender) = self.add_silent_audio_track(&peer_connection).await?;
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

        let audio_send_task = spawn_silent_audio_sender(
            session_id.clone(),
            Arc::clone(&audio_track),
            Arc::clone(&self.audio_stats),
        );
        let rtcp_read_task = spawn_rtcp_reader(session_id.clone(), audio_sender);

        self.sessions.lock().await.insert(
            session_id.clone(),
            WebRtcSession {
                peer_connection,
                audio_send_task,
                rtcp_read_task,
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
            session.audio_send_task.abort();
            session.rtcp_read_task.abort();
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

    async fn add_silent_audio_track(
        &self,
        peer_connection: &Arc<RTCPeerConnection>,
    ) -> anyhow::Result<(Arc<TrackLocalStaticSample>, Arc<RTCRtpSender>)> {
        let track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_string(),
                clock_rate: 48_000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_string(),
                rtcp_feedback: vec![],
            },
            "silent-audio".to_string(),
            "webrtlsdr".to_string(),
        ));

        let sender = peer_connection
            .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .context("failed to add WebRTC silent audio track")?;
        Ok((track, sender))
    }
}

fn spawn_rtcp_reader(session_id: String, sender: Arc<RTCRtpSender>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match sender.read_rtcp().await {
                Ok((_packets, _attributes)) => {}
                Err(error) => {
                    tracing::debug!(%session_id, %error, "WebRTC RTCP reader stopped");
                    return;
                }
            }
        }
    })
}

fn spawn_silent_audio_sender(
    session_id: String,
    audio_track: Arc<TrackLocalStaticSample>,
    audio_stats: Arc<WebRtcAudioStats>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SILENT_AUDIO_FRAME_DURATION);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let sample = Sample {
                data: OPUS_SILENCE_PACKET.to_vec().into(),
                duration: SILENT_AUDIO_FRAME_DURATION,
                ..Default::default()
            };

            match audio_track.write_sample(&sample).await {
                Ok(()) => {
                    audio_stats.frames_sent.fetch_add(1, Ordering::Relaxed);
                    audio_stats
                        .bytes_sent
                        .fetch_add(OPUS_SILENCE_PACKET.len() as u64, Ordering::Relaxed);
                }
                Err(error) => {
                    let error = error.to_string();
                    tracing::warn!(%session_id, %error, "failed to send WebRTC silent audio frame");
                    *audio_stats.last_send_error.lock().await = Some(error);
                }
            }
        }
    })
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

    #[test]
    fn silent_audio_frame_uses_20ms_opus_packet() {
        assert_eq!(SILENT_AUDIO_FRAME_DURATION, Duration::from_millis(20));
        assert_eq!(OPUS_SILENCE_PACKET, &[0xf8, 0xff, 0xfe]);
    }
}
