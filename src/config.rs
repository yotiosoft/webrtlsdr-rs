use std::{env, net::SocketAddr};

use crate::webrtc::{WebRtcAudioConfig, WebRtcConfig};

#[derive(Debug, Clone)]
pub struct Config {
    listen_addr: SocketAddr,
    webrtc: WebRtcConfig,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let host = env::var("WEBRTLSDR_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("WEBRTLSDR_PORT")
            .ok()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(3000);

        let listen_addr = format!("{host}:{port}").parse()?;
        let ice_servers = env::var("WEBRTLSDR_WEBRTC_ICE_SERVERS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let audio = WebRtcAudioConfig {
            enabled: env_bool("WEBRTLSDR_WEBRTC_AUDIO_ENABLED", true)?,
            opus_bitrate_bps: env_i32("WEBRTLSDR_WEBRTC_OPUS_BITRATE_BPS", 32_000)?,
            opus_complexity: env_i32("WEBRTLSDR_WEBRTC_OPUS_COMPLEXITY", 5)?,
            ..WebRtcAudioConfig::default()
        };

        Ok(Self {
            listen_addr,
            webrtc: WebRtcConfig { ice_servers, audio },
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn webrtc(&self) -> WebRtcConfig {
        self.webrtc.clone()
    }
}

fn env_i32(name: &str, default: i32) -> anyhow::Result<i32> {
    env::var(name)
        .ok()
        .map(|value| value.parse())
        .transpose()
        .map(|value| value.unwrap_or(default))
        .map_err(Into::into)
}

fn env_bool(name: &str, default: bool) -> anyhow::Result<bool> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => anyhow::bail!("{name} must be a boolean value, got {value}"),
        },
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_webrtc_ice_servers_from_env_like_string() {
        let ice_servers = " stun:stun.example.test:3478, turn:turn.example.test:3478 "
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(
            ice_servers,
            vec![
                "stun:stun.example.test:3478".to_string(),
                "turn:turn.example.test:3478".to_string(),
            ]
        );
    }

    #[test]
    fn default_webrtc_audio_config_matches_step_16() {
        let audio = WebRtcAudioConfig::default();
        assert!(audio.enabled);
        assert_eq!(audio.sample_rate_hz, 48_000);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.frame_duration_ms, 20);
        assert_eq!(audio.opus_bitrate_bps, 32_000);
        assert_eq!(audio.opus_complexity, 5);
    }
}
