use std::{env, net::SocketAddr};

use crate::webrtc::WebRtcConfig;

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

        Ok(Self {
            listen_addr,
            webrtc: WebRtcConfig { ice_servers },
        })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn webrtc(&self) -> WebRtcConfig {
        self.webrtc.clone()
    }
}

#[cfg(test)]
mod tests {
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
}
