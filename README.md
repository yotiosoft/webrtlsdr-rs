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

List connected RTL-SDR devices:

```sh
curl http://127.0.0.1:3000/api/devices
```

The response is JSON:

```json
{
  "devices": [
    {
      "index": 0,
      "name": "Generic RTL2832U OEM",
      "manufacturer": "Realtek",
      "product": "RTL2838UHIDIR",
      "serial": "00000001"
    }
  ]
}
```

When no RTL-SDR devices are connected, the endpoint returns an empty array:

```json
{
  "devices": []
}
```

Check the current RTL-SDR session:

```sh
curl http://127.0.0.1:3000/api/session
```

Connect the first RTL-SDR device:

```sh
curl -X POST http://127.0.0.1:3000/api/session/connect \
  -H 'content-type: application/json' \
  -d '{"index":0}'
```

Disconnect the current RTL-SDR device. This is safe to call even when no device
is connected:

```sh
curl -X POST http://127.0.0.1:3000/api/session/disconnect
```

Connecting while another device is already open returns `409 Conflict`.

Tune the connected RTL-SDR device:

```sh
curl -X POST http://127.0.0.1:3000/api/session/tune \
  -H 'content-type: application/json' \
  -d '{"frequency_hz":100000000}'
```

Set the sample rate:

```sh
curl -X POST http://127.0.0.1:3000/api/session/sample-rate \
  -H 'content-type: application/json' \
  -d '{"sample_rate_hz":2048000}'
```

Set tuner gain to automatic mode:

```sh
curl -X POST http://127.0.0.1:3000/api/session/gain \
  -H 'content-type: application/json' \
  -d '{"mode":"auto"}'
```

Set tuner gain manually. Manual gain uses librtlsdr's 0.1 dB integer unit, so
`280` means 28.0 dB:

```sh
curl -X POST http://127.0.0.1:3000/api/session/gain \
  -H 'content-type: application/json' \
  -d '{"mode":"manual","gain_tenths_db":280}'
```

Setting APIs return `409 Conflict` when no RTL-SDR device is connected.

Start reading raw interleaved `u8` IQ samples from the connected device:

```sh
curl -X POST http://127.0.0.1:3000/api/session/start
```

Check receive statistics:

```sh
curl http://127.0.0.1:3000/api/session/stats
```

The response includes whether reception is active, the number of blocks read,
the total bytes read, the last block size, the last block timestamp as Unix
milliseconds, and the last receive error if one occurred.

Stop reception. This is safe to call even when reception is not running:

```sh
curl -X POST http://127.0.0.1:3000/api/session/stop
```

Disconnecting also stops the receive loop before closing the RTL-SDR device.
Tuning, sample-rate, and gain changes return `409 Conflict` while reception is
running because the receive thread owns the device handle.

## RTL-SDR native dependency

The Rust server links directly to `librtlsdr` for RTL-SDR device discovery and
open/close session management and synchronous IQ sample reads.
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
