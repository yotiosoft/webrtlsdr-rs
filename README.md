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
milliseconds, and the last receive error if one occurred. It also includes DSP
statistics under `dsp`: processed IQ bytes, produced AM audio samples, approximate
audio sample rate, decimation ratio, audio peak/RMS, the last DSP timestamp, and
the last DSP error if one occurred. The `pcm` section reports server-side
conversion from internal `f32` audio into signed 16-bit little-endian mono PCM:
frames, samples, total bytes, last frame size, pre-clamp peak, clipped samples,
and the last PCM error if one occurred.

For a quick DSP smoke test with RTL-SDR hardware, start reception and call the
stats endpoint twice a few seconds apart:

```sh
curl http://127.0.0.1:3000/api/session/stats
sleep 2
curl http://127.0.0.1:3000/api/session/stats
```

During reception, `bytes_read`, `dsp.audio_samples_produced`,
`pcm.frames_produced`, `pcm.bytes_produced`, `stream.frames_broadcast`, and
`stream.bytes_broadcast` should increase. `dsp.audio_peak`, `dsp.audio_rms`,
and `pcm.peak_before_clamp` should update as IQ blocks are AM-demodulated into
internal `f32` audio samples and converted to PCM.

The server also exposes raw mono signed 16-bit little-endian PCM over
WebSocket:

```sh
websocat ws://127.0.0.1:3000/ws/audio
```

On connect, the first message is text JSON metadata:

```json
{"type":"audio_format","format":"i16le","channels":1,"sample_rate_hz":48000}
```

After reception starts, subsequent WebSocket messages are binary PCM payloads
with no JSON wrapper. The `stream` section of `/api/session/stats` reports
active WebSocket clients, frames and bytes broadcast, dropped frames from lagging
clients, the last connect/disconnect timestamps, and the last stream error.

## Browser Playback

The server serves a minimal browser UI at:

```sh
http://127.0.0.1:3000/
```

To try live audio from a browser, run `cargo run`, open the URL, reload devices,
select an RTL-SDR, connect, tune a frequency such as `100000000`, apply the
sample rate and gain settings, start receiving, then press Start Audio. The page
connects to `/ws/audio`, reads the initial PCM metadata message, converts binary
signed 16-bit little-endian mono PCM frames to `Float32Array` samples, and plays
them through an AudioWorklet. The audio panel shows WebSocket state, received
frames and bytes, buffered samples, underruns, and dropped samples.

AudioWorklet requires a secure browser context. For development from another
host, use an SSH tunnel and open the app as localhost:

```sh
ssh -L 3000:127.0.0.1:3000 user@server
```

Then open `http://127.0.0.1:3000/` in the local browser. Direct access to
`http://server-ip:3000/` is not enough for AudioWorklet unless the app is served
over HTTPS.

When serving under an Apache subpath such as `https://example.com/rtl-sdr/`,
proxy the whole path to this server and keep the trailing slash:

```apache
RedirectMatch 301 ^/rtl-sdr$ /rtl-sdr/
ProxyPass /rtl-sdr/ws/audio ws://127.0.0.1:3000/ws/audio
ProxyPassReverse /rtl-sdr/ws/audio ws://127.0.0.1:3000/ws/audio
ProxyPass /rtl-sdr/ http://127.0.0.1:3000/
ProxyPassReverse /rtl-sdr/ http://127.0.0.1:3000/
```

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
