use std::{env, net::SocketAddr};

#[derive(Debug, Clone)]
pub struct Config {
    listen_addr: SocketAddr,
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

        Ok(Self { listen_addr })
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }
}
