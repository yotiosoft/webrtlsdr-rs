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

## RTL-SDR native dependency

The Rust server links directly to `librtlsdr` for RTL-SDR device discovery.
Install the development package before building on Linux, for example:

```sh
sudo apt install librtlsdr-dev
```

At runtime, the `librtlsdr` shared library must also be loadable by the dynamic
linker. The build script uses `pkg-config --libs librtlsdr` when available and
falls back to linking `-lrtlsdr` with a warning if `pkg-config` cannot find it.

To manually check RTL-SDR enumeration on a machine with `librtlsdr` available,
run:

```sh
cargo test list_devices_with_librtlsdr -- --ignored --nocapture
```

`librtlsdr` is licensed under GPL-2.0-or-later. This project is expected to be
published as GPL-3.0-or-later in the future.
