use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, anyhow, bail};
use opus::{Application, Bitrate, Channels};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, broadcast},
    task::JoinHandle,
};
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

use crate::{dsp::AudioBlock, stream::AudioStreamHub};

const WEBRTC_AUDIO_SAMPLE_RATE_HZ: u32 = 48_000;
const WEBRTC_AUDIO_CHANNELS: u16 = 1;
const DEFAULT_OPUS_BITRATE_BPS: i32 = 32_000;
const DEFAULT_OPUS_COMPLEXITY: i32 = 5;
const MAX_OPUS_PACKET_BYTES: usize = 4_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebRtcConfig {
    pub ice_servers: Vec<String>,
    pub audio: WebRtcAudioConfig,
    pub default_playback_mode: PlaybackMode,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            ice_servers: Vec::new(),
            audio: WebRtcAudioConfig::default(),
            default_playback_mode: PlaybackMode::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackMode {
    WebRtc,
    PcmDiagnostics,
    LegacyPcm,
}

impl Default for PlaybackMode {
    fn default() -> Self {
        Self::WebRtc
    }
}

impl PlaybackMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WebRtc => "webrtc",
            Self::PcmDiagnostics => "pcm-diagnostics",
            Self::LegacyPcm => "legacy-pcm",
        }
    }
}

impl std::str::FromStr for PlaybackMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "webrtc" => Ok(Self::WebRtc),
            "pcm" | "pcm-diagnostics" | "diagnostics" => Ok(Self::PcmDiagnostics),
            "legacy" | "legacy-pcm" => Ok(Self::LegacyPcm),
            _ => Err(format!("unsupported playback mode {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebRtcAudioConfig {
    pub enabled: bool,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frame_duration_ms: u64,
    pub opus_bitrate_bps: i32,
    pub opus_complexity: i32,
    pub silence_on_underrun: bool,
}

impl Default for WebRtcAudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate_hz: WEBRTC_AUDIO_SAMPLE_RATE_HZ,
            channels: WEBRTC_AUDIO_CHANNELS,
            frame_duration_ms: 20,
            opus_bitrate_bps: DEFAULT_OPUS_BITRATE_BPS,
            opus_complexity: DEFAULT_OPUS_COMPLEXITY,
            silence_on_underrun: true,
        }
    }
}

impl WebRtcAudioConfig {
    pub fn samples_per_frame(&self) -> anyhow::Result<usize> {
        if !matches!(self.frame_duration_ms, 10 | 20 | 40) {
            bail!("WebRTC frame duration must be 10, 20, or 40 ms");
        }
        Ok((u64::from(self.sample_rate_hz) * self.frame_duration_ms / 1_000) as usize)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.sample_rate_hz != WEBRTC_AUDIO_SAMPLE_RATE_HZ {
            bail!("WebRTC audio sample rate must be 48000 Hz");
        }
        if self.channels != WEBRTC_AUDIO_CHANNELS {
            bail!("WebRTC audio must be mono");
        }
        self.samples_per_frame()?;
        if !(6_000..=510_000).contains(&self.opus_bitrate_bps) {
            bail!("Opus bitrate must be between 6000 and 510000 bps");
        }
        if !(0..=10).contains(&self.opus_complexity) {
            bail!("Opus complexity must be between 0 and 10");
        }
        Ok(())
    }
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
    pub default_playback_mode: &'static str,
    pub audio: WebRtcAudioConfigResponse,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WebRtcAudioConfigResponse {
    pub enabled: bool,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frame_duration_ms: u64,
    pub opus_bitrate_bps: i32,
    pub opus_complexity: i32,
    pub silence_on_underrun: bool,
    pub samples_per_frame: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct WebRtcStatsResponse {
    pub active_sessions: usize,
    pub audio_frames_sent: u64,
    pub audio_bytes_sent: u64,
    pub frames_encoded: u64,
    pub opus_bytes_encoded: u64,
    pub frames_sent: u64,
    pub send_errors: u64,
    pub underrun_silence_frames: u64,
    pub encoder_errors: u64,
    pub encode_errors: u64,
    pub late_frames: u64,
    pub source_lagged_blocks: u64,
    pub encode_time_total_us: u64,
    pub encode_time_max_us: u64,
    pub encode_time_avg_ms: f64,
    pub encode_time_p95_ms: f64,
    pub send_interval_avg_ms: f64,
    pub send_interval_jitter_ms: f64,
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
    audio_stream: AudioStreamHub,
    next_session: Mutex<u64>,
    sessions: Arc<Mutex<HashMap<String, WebRtcSession>>>,
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
    frames_encoded: AtomicU64,
    opus_bytes_encoded: AtomicU64,
    frames_sent: AtomicU64,
    bytes_sent: AtomicU64,
    send_errors: AtomicU64,
    underrun_silence_frames: AtomicU64,
    encoder_errors: AtomicU64,
    source_lagged_blocks: AtomicU64,
    encode_time_total_us: AtomicU64,
    encode_time_max_us: AtomicU64,
    late_frames: AtomicU64,
    send_interval_count: AtomicU64,
    send_interval_total_us: AtomicU64,
    send_interval_squared_us: AtomicU64,
    encode_time_samples_us: std::sync::Mutex<VecDeque<u64>>,
    last_send_error: Mutex<Option<String>>,
}

impl WebRtcSessionManager {
    pub fn new(config: WebRtcConfig, audio_stream: AudioStreamHub) -> Self {
        Self {
            config,
            audio_stream,
            next_session: Mutex::new(1),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            audio_stats: Arc::new(WebRtcAudioStats::default()),
        }
    }

    pub fn config_response(&self) -> WebRtcConfigResponse {
        WebRtcConfigResponse {
            ice_servers: self.config.ice_servers.clone(),
            default_playback_mode: self.config.default_playback_mode.as_str(),
            audio: WebRtcAudioConfigResponse {
                enabled: self.config.audio.enabled,
                sample_rate_hz: self.config.audio.sample_rate_hz,
                channels: self.config.audio.channels,
                frame_duration_ms: self.config.audio.frame_duration_ms,
                opus_bitrate_bps: self.config.audio.opus_bitrate_bps,
                opus_complexity: self.config.audio.opus_complexity,
                silence_on_underrun: self.config.audio.silence_on_underrun,
                samples_per_frame: self.config.audio.samples_per_frame().unwrap_or(0),
            },
        }
    }

    pub async fn stats_response(&self) -> WebRtcStatsResponse {
        let sessions = self.sessions.lock().await;
        let frames_encoded = self.audio_stats.frames_encoded.load(Ordering::Relaxed);
        let encode_total_us = self
            .audio_stats
            .encode_time_total_us
            .load(Ordering::Relaxed);
        let interval_count = self.audio_stats.send_interval_count.load(Ordering::Relaxed);
        let interval_total_us = self
            .audio_stats
            .send_interval_total_us
            .load(Ordering::Relaxed);
        let interval_squared_us = self
            .audio_stats
            .send_interval_squared_us
            .load(Ordering::Relaxed);
        let interval_avg_us = ratio(interval_total_us, interval_count);
        let interval_variance =
            ratio(interval_squared_us, interval_count) - interval_avg_us * interval_avg_us;
        WebRtcStatsResponse {
            active_sessions: sessions.len(),
            audio_frames_sent: self.audio_stats.frames_sent.load(Ordering::Relaxed),
            audio_bytes_sent: self.audio_stats.bytes_sent.load(Ordering::Relaxed),
            frames_encoded,
            opus_bytes_encoded: self.audio_stats.opus_bytes_encoded.load(Ordering::Relaxed),
            frames_sent: self.audio_stats.frames_sent.load(Ordering::Relaxed),
            send_errors: self.audio_stats.send_errors.load(Ordering::Relaxed),
            underrun_silence_frames: self
                .audio_stats
                .underrun_silence_frames
                .load(Ordering::Relaxed),
            encoder_errors: self.audio_stats.encoder_errors.load(Ordering::Relaxed),
            encode_errors: self.audio_stats.encoder_errors.load(Ordering::Relaxed),
            late_frames: self.audio_stats.late_frames.load(Ordering::Relaxed),
            source_lagged_blocks: self
                .audio_stats
                .source_lagged_blocks
                .load(Ordering::Relaxed),
            encode_time_total_us: self
                .audio_stats
                .encode_time_total_us
                .load(Ordering::Relaxed),
            encode_time_max_us: self.audio_stats.encode_time_max_us.load(Ordering::Relaxed),
            encode_time_avg_ms: ratio(encode_total_us, frames_encoded) / 1_000.0,
            encode_time_p95_ms: encode_time_p95_us(&self.audio_stats) / 1_000.0,
            send_interval_avg_ms: interval_avg_us / 1_000.0,
            send_interval_jitter_ms: interval_variance.max(0.0).sqrt() / 1_000.0,
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
        let (audio_track, audio_sender) = self.add_audio_track(&peer_connection).await?;
        let state_session_id = session_id.clone();
        let state_sessions = Arc::clone(&self.sessions);

        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let session_id = state_session_id.clone();
            let sessions = Arc::clone(&state_sessions);
            Box::pin(async move {
                tracing::info!(%session_id, ?state, "WebRTC peer connection state changed");
                if matches!(
                    state,
                    RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed
                ) {
                    if let Err(error) = close_session_entry(&sessions, &session_id).await {
                        tracing::warn!(%session_id, %error, "failed to clean up terminal WebRTC session");
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

        let audio_send_task = spawn_sdr_audio_sender(
            session_id.clone(),
            Arc::clone(&audio_track),
            self.audio_stream.subscribe_audio(),
            self.config.audio.clone(),
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
        close_session_entry(&self.sessions, session_id).await
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

    async fn add_audio_track(
        &self,
        peer_connection: &Arc<RTCPeerConnection>,
    ) -> anyhow::Result<(Arc<TrackLocalStaticSample>, Arc<RTCRtpSender>)> {
        let track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_string(),
                clock_rate: WEBRTC_AUDIO_SAMPLE_RATE_HZ,
                channels: WEBRTC_AUDIO_CHANNELS,
                sdp_fmtp_line: "minptime=10;useinbandfec=0".to_string(),
                rtcp_feedback: vec![],
            },
            "sdr-audio".to_string(),
            "webrtlsdr".to_string(),
        ));

        let sender = peer_connection
            .add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .context("failed to add WebRTC audio track")?;
        Ok((track, sender))
    }
}

fn ratio(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn encode_time_p95_us(stats: &WebRtcAudioStats) -> f64 {
    let Ok(samples) = stats.encode_time_samples_us.lock() else {
        return 0.0;
    };
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort_unstable();
    let index = ((sorted.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
    sorted[index] as f64
}

async fn close_session_entry(
    sessions: &Arc<Mutex<HashMap<String, WebRtcSession>>>,
    session_id: &str,
) -> anyhow::Result<bool> {
    let session = sessions.lock().await.remove(session_id);
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

fn spawn_sdr_audio_sender(
    session_id: String,
    audio_track: Arc<TrackLocalStaticSample>,
    mut audio_receiver: broadcast::Receiver<AudioBlock>,
    audio_config: WebRtcAudioConfig,
    audio_stats: Arc<WebRtcAudioStats>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let frame_duration = Duration::from_millis(audio_config.frame_duration_ms);
        let frame_samples = match audio_config.samples_per_frame() {
            Ok(samples) => samples,
            Err(error) => {
                audio_stats.encoder_errors.fetch_add(1, Ordering::Relaxed);
                record_audio_error(&audio_stats, error.to_string()).await;
                return;
            }
        };
        let mut ticker = tokio::time::interval(frame_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut frame_builder = WebRtcAudioFrameBuilder::new(frame_samples);
        let mut last_send_at: Option<Instant> = None;
        let mut encoder = match OpusAudioEncoder::new(&audio_config) {
            Ok(encoder) => encoder,
            Err(error) => {
                record_audio_error(
                    &audio_stats,
                    format!("failed to create Opus encoder: {error}"),
                )
                .await;
                audio_stats.encoder_errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        loop {
            ticker.tick().await;
            let loop_started = Instant::now();
            drain_audio_blocks(&mut audio_receiver, &mut frame_builder, &audio_stats).await;
            let (frame, was_silence) = frame_builder.next_frame_or_silence();
            if was_silence {
                audio_stats
                    .underrun_silence_frames
                    .fetch_add(1, Ordering::Relaxed);
                if !audio_config.silence_on_underrun {
                    continue;
                }
            }

            let encode_start = Instant::now();
            let encoded = match encoder.encode(&frame) {
                Ok(encoded) => encoded,
                Err(error) => {
                    let error = error.to_string();
                    tracing::warn!(%session_id, %error, "failed to encode WebRTC Opus audio frame");
                    audio_stats.encoder_errors.fetch_add(1, Ordering::Relaxed);
                    record_audio_error(&audio_stats, error).await;
                    continue;
                }
            };
            let elapsed_us = u64::try_from(encode_start.elapsed().as_micros()).unwrap_or(u64::MAX);
            audio_stats
                .encode_time_total_us
                .fetch_add(elapsed_us, Ordering::Relaxed);
            update_atomic_max(&audio_stats.encode_time_max_us, elapsed_us);
            if let Ok(mut samples) = audio_stats.encode_time_samples_us.lock() {
                if samples.len() == 512 {
                    samples.pop_front();
                }
                samples.push_back(elapsed_us);
            }

            let sample = Sample {
                data: encoded.clone().into(),
                duration: frame_duration,
                ..Default::default()
            };

            audio_stats.frames_encoded.fetch_add(1, Ordering::Relaxed);
            audio_stats
                .opus_bytes_encoded
                .fetch_add(encoded.len() as u64, Ordering::Relaxed);

            match audio_track.write_sample(&sample).await {
                Ok(()) => {
                    let now = Instant::now();
                    if let Some(previous) = last_send_at.replace(now) {
                        let interval_us = u64::try_from(now.duration_since(previous).as_micros())
                            .unwrap_or(u64::MAX);
                        audio_stats
                            .send_interval_count
                            .fetch_add(1, Ordering::Relaxed);
                        audio_stats
                            .send_interval_total_us
                            .fetch_add(interval_us, Ordering::Relaxed);
                        audio_stats
                            .send_interval_squared_us
                            .fetch_add(interval_us.saturating_mul(interval_us), Ordering::Relaxed);
                    }
                    audio_stats.frames_sent.fetch_add(1, Ordering::Relaxed);
                    audio_stats
                        .bytes_sent
                        .fetch_add(encoded.len() as u64, Ordering::Relaxed);
                }
                Err(error) => {
                    let error = error.to_string();
                    tracing::warn!(%session_id, %error, "failed to send WebRTC Opus audio frame");
                    audio_stats.send_errors.fetch_add(1, Ordering::Relaxed);
                    record_audio_error(&audio_stats, error).await;
                }
            }
            if loop_started.elapsed() > frame_duration {
                audio_stats.late_frames.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
}

async fn drain_audio_blocks(
    receiver: &mut broadcast::Receiver<AudioBlock>,
    frame_builder: &mut WebRtcAudioFrameBuilder,
    audio_stats: &Arc<WebRtcAudioStats>,
) {
    loop {
        match receiver.try_recv() {
            Ok(block) => {
                if let Err(error) = frame_builder.push_block(&block) {
                    audio_stats.encoder_errors.fetch_add(1, Ordering::Relaxed);
                    record_audio_error(audio_stats, error).await;
                }
            }
            Err(broadcast::error::TryRecvError::Empty) => return,
            Err(broadcast::error::TryRecvError::Lagged(blocks)) => {
                audio_stats
                    .source_lagged_blocks
                    .fetch_add(blocks, Ordering::Relaxed);
            }
            Err(broadcast::error::TryRecvError::Closed) => return,
        }
    }
}

async fn record_audio_error(audio_stats: &Arc<WebRtcAudioStats>, error: String) {
    *audio_stats.last_send_error.lock().await = Some(error);
}

fn update_atomic_max(value: &AtomicU64, candidate: u64) {
    let mut current = value.load(Ordering::Relaxed);
    while candidate > current {
        match value.compare_exchange(current, candidate, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(next) => current = next,
        }
    }
}

struct WebRtcAudioFrameBuilder {
    pending: Vec<f32>,
    frame_samples: usize,
}

impl WebRtcAudioFrameBuilder {
    fn new(frame_samples: usize) -> Self {
        Self {
            pending: Vec::with_capacity(frame_samples * 2),
            frame_samples,
        }
    }

    fn push_block(&mut self, block: &AudioBlock) -> Result<(), String> {
        if block.sample_rate_hz != WEBRTC_AUDIO_SAMPLE_RATE_HZ {
            return Err(format!(
                "WebRTC audio requires {} Hz, got {} Hz",
                WEBRTC_AUDIO_SAMPLE_RATE_HZ, block.sample_rate_hz
            ));
        }

        self.pending.extend(
            block
                .samples
                .iter()
                .map(|sample| sanitize_audio_sample(*sample)),
        );
        Ok(())
    }

    fn next_frame_or_silence(&mut self) -> (Vec<f32>, bool) {
        if self.pending.len() < self.frame_samples {
            return (vec![0.0; self.frame_samples], true);
        }

        let frame = self.pending.drain(..self.frame_samples).collect::<Vec<_>>();
        (frame, false)
    }
}

fn sanitize_audio_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

struct OpusAudioEncoder {
    encoder: opus::Encoder,
    output: Vec<u8>,
    frame_samples: usize,
}

impl OpusAudioEncoder {
    fn new(config: &WebRtcAudioConfig) -> anyhow::Result<Self> {
        if !config.enabled {
            bail!("WebRTC audio is disabled");
        }
        if config.sample_rate_hz != WEBRTC_AUDIO_SAMPLE_RATE_HZ {
            bail!(
                "unsupported WebRTC audio sample rate {}",
                config.sample_rate_hz
            );
        }
        if config.channels != WEBRTC_AUDIO_CHANNELS {
            bail!("unsupported WebRTC audio channel count {}", config.channels);
        }
        config.validate()?;

        let mut encoder =
            opus::Encoder::new(config.sample_rate_hz, Channels::Mono, Application::Audio)
                .context("failed to initialize libopus encoder")?;
        encoder
            .set_bitrate(Bitrate::Bits(config.opus_bitrate_bps))
            .context("failed to set Opus bitrate")?;
        encoder
            .set_complexity(config.opus_complexity)
            .context("failed to set Opus complexity")?;

        Ok(Self {
            encoder,
            output: vec![0; MAX_OPUS_PACKET_BYTES],
            frame_samples: config.samples_per_frame()?,
        })
    }

    fn encode(&mut self, frame: &[f32]) -> anyhow::Result<Vec<u8>> {
        if frame.len() != self.frame_samples {
            bail!("Opus frame must contain {} samples", self.frame_samples);
        }
        let bytes = self
            .encoder
            .encode_float(frame, &mut self.output)
            .context("libopus encode_float failed")?;
        Ok(self.output[..bytes].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_FRAME_SAMPLES: usize = 960;

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
        let manager = WebRtcSessionManager::new(WebRtcConfig::default(), AudioStreamHub::default());
        assert!(!manager.close("missing").await.unwrap());
        assert_eq!(manager.len().await, 0);
    }

    #[test]
    fn audio_frame_builder_extracts_exact_20ms_frame() {
        let mut builder = WebRtcAudioFrameBuilder::new(TEST_FRAME_SAMPLES);
        builder
            .push_block(&AudioBlock {
                sample_rate_hz: WEBRTC_AUDIO_SAMPLE_RATE_HZ,
                samples: vec![0.25; TEST_FRAME_SAMPLES],
            })
            .unwrap();

        let (frame, silence) = builder.next_frame_or_silence();
        assert!(!silence);
        assert_eq!(frame.len(), TEST_FRAME_SAMPLES);
        assert!(frame.iter().all(|sample| *sample == 0.25));
    }

    #[test]
    fn audio_frame_builder_keeps_remainder() {
        let mut builder = WebRtcAudioFrameBuilder::new(TEST_FRAME_SAMPLES);
        builder
            .push_block(&AudioBlock {
                sample_rate_hz: WEBRTC_AUDIO_SAMPLE_RATE_HZ,
                samples: vec![0.5; TEST_FRAME_SAMPLES + 10],
            })
            .unwrap();

        let (_, silence) = builder.next_frame_or_silence();
        assert!(!silence);
        let (frame, silence) = builder.next_frame_or_silence();
        assert!(silence);
        assert_eq!(frame, vec![0.0; TEST_FRAME_SAMPLES]);
        assert_eq!(builder.pending.len(), 10);
    }

    #[test]
    fn audio_frame_builder_can_extract_multiple_frames() {
        let mut builder = WebRtcAudioFrameBuilder::new(TEST_FRAME_SAMPLES);
        builder
            .push_block(&AudioBlock {
                sample_rate_hz: WEBRTC_AUDIO_SAMPLE_RATE_HZ,
                samples: vec![0.1; TEST_FRAME_SAMPLES * 2],
            })
            .unwrap();

        assert!(!builder.next_frame_or_silence().1);
        assert!(!builder.next_frame_or_silence().1);
        assert!(builder.next_frame_or_silence().1);
    }

    #[test]
    fn audio_frame_builder_rejects_non_48khz_audio() {
        let mut builder = WebRtcAudioFrameBuilder::new(TEST_FRAME_SAMPLES);
        let error = builder
            .push_block(&AudioBlock {
                sample_rate_hz: 44_100,
                samples: vec![0.0; 10],
            })
            .unwrap_err();
        assert!(error.contains("48000 Hz"));
    }

    #[test]
    fn opus_encoder_encodes_silence() {
        let mut encoder = OpusAudioEncoder::new(&WebRtcAudioConfig::default()).unwrap();
        let packet = encoder.encode(&vec![0.0; TEST_FRAME_SAMPLES]).unwrap();
        assert!(!packet.is_empty());
    }

    #[test]
    fn supported_frame_durations_compute_samples() {
        for (duration, samples) in [(10, 480), (20, 960), (40, 1920)] {
            let config = WebRtcAudioConfig {
                frame_duration_ms: duration,
                ..Default::default()
            };
            assert_eq!(config.samples_per_frame().unwrap(), samples);
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn rejects_invalid_audio_tuning() {
        let config = WebRtcAudioConfig {
            frame_duration_ms: 30,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        let config = WebRtcAudioConfig {
            opus_complexity: 11,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn stats_helpers_compute_average_and_p95() {
        let stats = WebRtcAudioStats::default();
        stats
            .encode_time_samples_us
            .lock()
            .unwrap()
            .extend([100, 200, 300, 400, 5_000]);
        assert_eq!(ratio(1_000, 4), 250.0);
        assert_eq!(ratio(1_000, 0), 0.0);
        assert_eq!(encode_time_p95_us(&stats), 5_000.0);
    }
}
