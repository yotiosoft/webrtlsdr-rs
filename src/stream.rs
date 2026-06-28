//! Low-latency audio streaming transports.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::audio::PcmFrame;

const AUDIO_CHANNEL_CAPACITY: usize = 32;
const DEFAULT_AUDIO_SAMPLE_RATE_HZ: u32 = 48_000;
const WEBSOCKET_SEND_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub struct AudioStreamHub {
    sender: broadcast::Sender<PcmFrame>,
    stats: Arc<Mutex<StreamStats>>,
}

impl Default for AudioStreamHub {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(AUDIO_CHANNEL_CAPACITY);

        Self {
            sender,
            stats: Arc::new(Mutex::new(StreamStats::default())),
        }
    }
}

impl AudioStreamHub {
    pub fn publish(&self, frame: PcmFrame) {
        self.record_frame(frame.payload.len());

        if let Err(error) = self.sender.send(frame) {
            tracing::trace!(%error, "published PCM frame with no WebSocket subscribers");
        }
    }

    pub fn stats(&self) -> StreamStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_else(|error| {
                tracing::error!(%error, "audio stream stats mutex is poisoned");
                StreamStats {
                    last_error: Some("audio stream stats unavailable".to_string()),
                    ..StreamStats::default()
                }
            })
    }

    fn subscribe(&self) -> broadcast::Receiver<PcmFrame> {
        self.sender.subscribe()
    }

    fn record_client_connected(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.active_clients = stats.active_clients.saturating_add(1);
            stats.last_client_connected_unix_ms = unix_ms_now();
            stats.last_error = None;
        }
    }

    fn record_client_disconnected(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.active_clients = stats.active_clients.saturating_sub(1);
            stats.last_client_disconnected_unix_ms = unix_ms_now();
        }
    }

    fn record_frame(&self, bytes: usize) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.frames_broadcast = stats.frames_broadcast.saturating_add(1);
            stats.bytes_broadcast = stats.bytes_broadcast.saturating_add(bytes as u64);
            stats.last_error = None;
        }
    }

    fn record_lagged_subscriber(&self, frames: u64) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.frames_dropped = stats.frames_dropped.saturating_add(frames);
            stats.lagged_subscribers = stats.lagged_subscribers.saturating_add(1);
        }
    }

    fn record_send_drop(&self, frames: u64) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.frames_dropped = stats.frames_dropped.saturating_add(frames);
        }
    }

    fn record_error(&self, message: impl Into<String>) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.last_error = Some(message.into());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamStats {
    pub active_clients: usize,
    pub frames_broadcast: u64,
    pub bytes_broadcast: u64,
    pub frames_dropped: u64,
    pub lagged_subscribers: u64,
    pub last_client_connected_unix_ms: Option<u64>,
    pub last_client_disconnected_unix_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
struct AudioFormatMessage {
    #[serde(rename = "type")]
    message_type: &'static str,
    format: &'static str,
    channels: u8,
    sample_rate_hz: u32,
}

pub async fn serve_audio_websocket(socket: WebSocket, hub: AudioStreamHub) {
    hub.record_client_connected();
    let result = stream_audio(socket, hub.clone()).await;

    if let Err(error) = result {
        tracing::debug!(%error, "WebSocket audio stream ended with an error");
        hub.record_error(error);
    }

    hub.record_client_disconnected();
}

async fn stream_audio(mut socket: WebSocket, hub: AudioStreamHub) -> Result<(), String> {
    let metadata = AudioFormatMessage {
        message_type: "audio_format",
        format: "i16le",
        channels: 1,
        sample_rate_hz: DEFAULT_AUDIO_SAMPLE_RATE_HZ,
    };
    let metadata = serde_json::to_string(&metadata)
        .map_err(|error| format!("failed to encode audio metadata: {error}"))?;

    socket
        .send(Message::Text(metadata))
        .await
        .map_err(|error| format!("failed to send audio metadata: {error}"))?;

    let mut receiver = hub.subscribe();
    loop {
        tokio::select! {
            frame = receiver.recv() => {
                match frame {
                    Ok(frame) => {
                        let payload = frame.payload;
                        match tokio::time::timeout(
                            WEBSOCKET_SEND_TIMEOUT,
                            socket.send(Message::Binary(payload)),
                        )
                        .await
                        {
                            Ok(Ok(())) => {}
                            Ok(Err(error)) => {
                                return Err(format!("failed to send PCM frame: {error}"));
                            }
                            Err(_) => {
                                hub.record_send_drop(1);
                                return Err("timed out sending PCM frame".to_string());
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(frames)) => {
                        tracing::debug!(
                            frames,
                            "WebSocket audio subscriber lagged; dropping old frames"
                        );
                        hub.record_lagged_subscriber(frames);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err("audio stream broadcaster closed".to_string());
                    }
                }
            }
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Close(_))) | None => return Ok(()),
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        return Err(format!("failed to read WebSocket message: {error}"));
                    }
                }
            }
        }
    }
}

fn unix_ms_now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm_frame(sequence: u64) -> PcmFrame {
        PcmFrame {
            sequence,
            generated_at_unix_ms: Some(1),
            sample_rate_hz: 48_000,
            channels: 1,
            format: crate::audio::AudioSampleFormat::I16Le,
            samples: 2,
            payload: vec![0, 0, 1, 0],
        }
    }

    #[tokio::test]
    async fn subscriber_receives_published_frame() {
        let hub = AudioStreamHub::default();
        let mut subscriber = hub.subscribe();

        hub.publish(pcm_frame(7));

        let frame = subscriber.recv().await.expect("published frame");
        assert_eq!(frame.sequence, 7);
        assert_eq!(hub.stats().frames_broadcast, 1);
        assert_eq!(hub.stats().bytes_broadcast, 4);
    }

    #[test]
    fn publish_without_subscribers_updates_stats() {
        let hub = AudioStreamHub::default();

        hub.publish(pcm_frame(1));

        assert_eq!(hub.stats().frames_broadcast, 1);
        assert_eq!(hub.stats().bytes_broadcast, 4);
        assert_eq!(hub.stats().frames_dropped, 0);
    }

    #[test]
    fn dropped_frames_are_tracked() {
        let hub = AudioStreamHub::default();

        hub.record_lagged_subscriber(3);

        assert_eq!(hub.stats().frames_dropped, 3);
        assert_eq!(hub.stats().lagged_subscribers, 1);
    }
}
