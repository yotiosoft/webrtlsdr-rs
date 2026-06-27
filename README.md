# WebRTLSDR

Rust backend skeleton for a future browser-controlled RTL-SDR server.

## Run

```sh
cargo run
```

The server listens on `127.0.0.1:3000` by default.

Override the listen address with environment variables:

```sh
WEBRTLSDR_HOST=0.0.0.0 WEBRTLSDR_PORT=3000 cargo run
```

Health check:

```sh
curl http://127.0.0.1:3000/health
```
