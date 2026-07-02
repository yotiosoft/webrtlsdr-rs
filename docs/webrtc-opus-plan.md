# WebRTC + Opus Plan

Research date: 2026-06-28 JST.

Implementation update: Step 16 now connects the 48 kHz mono SDR/DSP audio stream to a 20 ms Opus encoder and WebRTC audio track. PCM WebSocket routes remain diagnostics/fallback.

## Goal

Move the default browser listening path from raw PCM over WebSocket to WebRTC
audio with Opus, while keeping the existing PCM path for diagnostics and
fallback. Step 13 is design only: do not add WebRTC, Opus, RTP, or DSP
implementation in this step.

The target media pipeline is:

```text
RTL-SDR
  -> Rust SDR/DSP pipeline
  -> 48 kHz mono audio frames
  -> Opus encoder
  -> WebRTC audio track
  -> RTCPeerConnection
  -> browser MediaStream
  -> <audio>
```

WebRTC should become the standard playback route because browsers already have
jitter buffering, clock handling, packet loss concealment, codec negotiation,
ICE, SRTP, RTCP, and stats APIs. The raw PCM path should stay as an explicit
diagnostic and comparison route.

## Current PCM Path

The project currently exposes:

- `GET /ws/audio`: legacy raw signed 16-bit little-endian mono PCM.
- `GET /ws/audio-v1`: PCM Frame Protocol v1 with sequence, PTS, frame metadata,
  jitter/gap visibility, and client diagnostics.

Keep both paths. Their roles after WebRTC is added:

- DSP and PCM pipeline validation.
- WebRTC outage comparison.
- Protocol/debug tooling where raw sample visibility matters.
- Fallback for browsers or environments where WebRTC cannot connect.

The UI should eventually default to `WebRTC`, with `PCM v1` and `Legacy PCM`
under a diagnostics or legacy mode.

## Recommended Architecture

Add a separate WebRTC media subsystem beside the existing REST and PCM
WebSocket routes.

```text
                  +-----------------------------+
RTL-SDR session ->| SDR read + DSP + resampler  |
                  +-------------+---------------+
                                |
                         48 kHz mono f32/i16 PCM
                                |
              +-----------------+-----------------+
              |                                   |
      PCM broadcaster                       WebRTC publisher
  /ws/audio, /ws/audio-v1              Opus -> RTP/SRTP track
              |                                   |
       diagnostic clients                  browser RTCPeerConnection
```

Recommended server components:

- `webrtc` module: owns WebRTC API setup, peer connection lifecycle, ICE server
  config, tracks, and stats snapshots.
- `opus` module: converts 48 kHz mono audio frames into Opus payloads when Step
  15+ starts real audio media.
- `audio_frame` boundary: keep a small internal frame type such as 20 ms,
  48 kHz, mono, `960` samples. This lets PCM WebSocket and WebRTC consume the
  same DSP output without coupling transport details to DSP.
- `signaling` routes: HTTP JSON endpoints for SDP and, later, ICE/session
  operations.

Signaling and media transport must stay separate:

- Signaling is HTTP under axum and Apache reverse proxy paths.
- Media uses ICE-selected UDP/TCP candidates and does not flow through the HTTP
  reverse proxy unless a TURN server relays it.

## Dependency Candidates

### Rust WebRTC Stack

Use `webrtc-rs` / crate `webrtc` as the first candidate.

Observed crate metadata from `cargo search` and `cargo info` on 2026-06-28:

| Crate | Version observed | License | Notes |
| --- | --- | --- | --- |
| `webrtc` | latest shown by `cargo search`: `0.20.0-beta.2` | `MIT/Apache-2.0` | Pure Rust WebRTC API. Beta latest should be treated as API-spike material first. |
| `webrtc` | `cargo info webrtc`: `0.8.0`, latest `0.20.0-beta.2` | `MIT/Apache-2.0` | Cargo displayed this as the default info result. Older API may differ significantly. |
| `webrtc@0.14.0` | `0.14.0`, latest `0.20.0-beta.2` | `MIT OR Apache-2.0` | A non-beta version worth evaluating if `0.20` churn is too high. |

Sources:

- <https://crates.io/crates/webrtc>
- <https://docs.rs/webrtc>
- <https://github.com/webrtc-rs/webrtc>
- <https://webrtc.rs>

Expected API areas to verify during Step 14:

- `RTCPeerConnection`: create peer connections, set local/remote descriptions,
  create answers, observe ICE/connection state, close sessions.
- `RTCConfiguration` and ICE servers: configure `urls`, optional username, and
  credential.
- Local audio track: use `TrackLocalStaticSample` for encoded samples if the
  crate handles RTP packetization for the selected codec, or
  `TrackLocalStaticRTP` if WebRTLSDR produces RTP packets itself.
- SDP offer/answer: browser creates the offer, server creates the answer.
- Stats: verify available sender/peer stats in the selected version; browser
  `RTCPeerConnection.getStats()` should be used regardless.

Production recommendation:

- Step 14 should spike against `webrtc@0.20.0-beta.2` only if its API examples
  are clearer and it integrates cleanly with the current Rust toolchain.
- If beta API churn blocks progress, fall back to the newest non-beta release
  that supports the required audio-track and signaling APIs.
- Pin the chosen exact version before Step 14 is merged; do not use a loose
  dependency range for WebRTC.

### Opus Encoding Options

| Option | Dependency | License | Difficulty | Raspberry Pi 5 CPU expectation | Fit |
| --- | --- | --- | --- | --- | --- |
| Rust crate `opus` over system `libopus` | `opus = "0.3.1"` plus `libopus` dev/runtime package | crate `MIT/Apache-2.0`; libopus is BSD-style | Low to medium | Low for 48 kHz mono 20 ms frames; libopus is mature and optimized | Recommended first real encoder path |
| Direct FFI | `opus-sys = "0.2.1"` plus `libopus` | crate `MIT`; libopus BSD-style | Medium | Low, same native encoder | Use only if safe wrapper lacks needed controls |
| Vendored/head bindings | e.g. `opus-head-sys = "0.3.0"` | verify before use | Medium | Low | Useful if distro libopus is too old, but increases build complexity |
| WebRTC helper encoder | Possibly inside `webrtc` crate or related crates | follows selected crate | Unknown | Potentially low | Investigate, but do not assume it exists or exposes a stable server encoder |
| GStreamer pipeline | `gstreamer = "0.24.5"` observed, latest `0.25.2`; system GStreamer plugins | Rust bindings `MIT OR Apache-2.0`; plugin licenses vary | High | Reasonable, but heavier process/runtime footprint | Good later for complex pipelines, not first WebRTC MVP |

Observed crate metadata:

- `opus = "0.3.1"`: safe Rust bindings for libopus, license
  `MIT/Apache-2.0`, repository <https://github.com/SpaceManiac/opus-rs>,
  crates.io <https://crates.io/crates/opus/0.3.1>.
- `opus-sys = "0.2.1"`: bindings to libopus, license `MIT`, repository
  <https://github.com/lgvz/rust-opus>, crates.io
  <https://crates.io/crates/opus-sys/0.2.1>.
- `gstreamer = "0.24.5"` observed, latest `0.25.2`, license
  `MIT OR Apache-2.0`, homepage <https://gstreamer.freedesktop.org>,
  crates.io <https://crates.io/crates/gstreamer>.

Opus/libopus sources:

- <https://opus-codec.org/>
- <https://opus-codec.org/license/>
- <https://opus-codec.org/downloads/>
- RFC 6716: <https://www.rfc-editor.org/rfc/rfc6716>

Recommendation:

- Use `opus` + system `libopus` for the first real audio implementation.
- Start with mono, 48 kHz, 20 ms frames, application mode chosen for general
  audio unless AM voice-only tuning proves better.
- Keep encoder controls visible in config or constants: bitrate, complexity,
  VBR/CBR, FEC, DTX, and packet loss percentage.

## License Notes

- `webrtc` crate: MIT/Apache-2.0 or MIT OR Apache-2.0 depending on version
  metadata. Compatible with this project, but verify the exact selected version.
- `opus` crate: MIT/Apache-2.0.
- `opus-sys` crate: MIT.
- `libopus`: BSD-style license with patent grant language; use the official
  license page as the release checklist source.
- GStreamer Rust bindings: MIT OR Apache-2.0. GStreamer plugins may have
  different licenses, so a GStreamer-based encoder path needs plugin-level
  license review.

Before merging Step 14 or Step 15 dependency changes, run a dependency license
check on the exact lockfile.

## Signaling Design

Start with one HTTP endpoint:

```text
POST /api/webrtc/offer
Content-Type: application/json

request:
{
  "sdp": "...",
  "type": "offer"
}

response:
{
  "session_id": "optional-server-session-id",
  "sdp": "...",
  "type": "answer"
}
```

For the first PoC, prefer non-trickle ICE:

- Browser creates the peer connection and offer.
- Browser waits for ICE gathering to complete or near-complete.
- Browser posts the complete SDP offer.
- Server creates the peer connection, attaches a dummy or silent audio track,
  sets the remote description, creates an answer, sets the local description,
  waits for local ICE gathering, and returns the complete answer.

This is slower to connect but much easier to debug under `/rtl-sdr/`.

Add these only after the one-shot offer/answer path works:

```text
POST   /api/webrtc/ice
GET    /api/webrtc/session/:id
DELETE /api/webrtc/session/:id
```

Future trickle ICE request shape:

```json
{
  "session_id": "...",
  "candidate": {
    "candidate": "...",
    "sdpMid": "0",
    "sdpMLineIndex": 0
  }
}
```

Session lifecycle:

- Create a session per browser playback attempt.
- Store peer connection, created timestamp, last state, and stats handle.
- Close sessions on explicit stop, failed/disconnected timeout, or browser page
  unload best-effort call.
- Limit concurrent sessions initially to protect the Raspberry Pi.

## Browser Playback Design

Add WebRTC as a playback mode without deleting the PCM modes.

Browser flow:

```text
on user presses Play WebRTC:
  create RTCPeerConnection({ iceServers })
  set pc.ontrack = attach first audio stream to <audio>
  set pc.oniceconnectionstatechange = update UI state
  set pc.onconnectionstatechange = update UI state
  create transceiver or receiver for audio
  createOffer()
  setLocalDescription(offer)
  wait for ICE gathering if non-trickle
  POST offer to base-path-aware /api/webrtc/offer
  setRemoteDescription(answer)
  call audio.play() from the user gesture flow
```

UI state to show:

- Mode: `WebRTC`, `PCM v1`, `Legacy PCM`.
- Signaling state.
- ICE gathering state.
- ICE connection state.
- Peer connection state.
- Remote audio track received: yes/no.
- Browser stats: packets received, packets lost, jitter, audio level if
  available, bytes received, selected candidate pair, round-trip time if
  reported.

Autoplay:

- Start WebRTC from a user action.
- Create or reveal an `<audio controls>` element.
- Set `audio.srcObject = event.streams[0]`.
- Call `audio.play()` and surface any rejection as UI state.

Browser sources:

- RTCPeerConnection: <https://developer.mozilla.org/docs/Web/API/RTCPeerConnection>
- `getStats()`: <https://developer.mozilla.org/docs/Web/API/RTCPeerConnection/getStats>
- WebRTC API: <https://developer.mozilla.org/docs/Web/API/WebRTC_API>

## Apache Reverse Proxy Notes

The application must continue to work when mounted below `/rtl-sdr/`.

Signaling:

- Use existing base-path-aware frontend URL helpers or relative URLs.
- From a page under `/rtl-sdr/`, `fetch("api/webrtc/offer", ...)` or the
  project's existing base URL helper is safer than hard-coding
  `/api/webrtc/offer`.
- Apache only proxies the HTTP signaling request and static frontend assets.

Media:

- WebRTC media does not normally traverse the HTTP reverse proxy.
- ICE candidates advertise addresses and ports that the browser connects to
  directly.
- On LAN, host candidates may be enough.
- Across NAT, VPN, firewall, or public Internet, STUN and usually TURN become
  necessary.
- HTTPS under Apache provides a secure context, which is helpful for browser
  WebRTC APIs and required for many modern media APIs outside localhost.

Deployment risk:

- If Apache terminates HTTPS and forwards to axum on localhost, the SDP still
  needs ICE candidates that are reachable by the browser.
- If the Rust process is bound only to `127.0.0.1`, host ICE candidates may not
  be reachable from other LAN clients. Step 14 should explicitly test from a
  second LAN device.

## STUN/TURN Policy

MVP policy:

- LAN-only Step 14 can start with no ICE servers.
- If host candidates are unreliable across target browsers, add public STUN as
  a configurable option.

Example config shape:

```toml
[webrtc]
ice_servers = ["stun:stun.l.google.com:19302"]
```

Future external access:

- Plan for TURN. STUN discovers candidates, but TURN relays media when direct
  connectivity fails.
- TURN credentials must not be hard-coded into frontend assets if they are
  long-lived.
- Prefer short-lived TURN credentials or a private LAN/VPN deployment model for
  early releases.

Privacy:

- ICE can expose local/network candidate information to the peer. This project
  is primarily self-hosted, but the UI and docs should not imply that WebRTC is
  only an HTTP stream through Apache.

## Audio Frame Format

Use Opus at 48 kHz mono.

Default frame:

```text
sample_rate_hz = 48000
channels = 1
frame_duration = 20 ms
samples_per_frame = 960
```

Frame duration trade-off:

| Duration | Samples at 48 kHz | Pros | Cons | Recommendation |
| --- | ---: | --- | --- | --- |
| 10 ms | 480 | Lower packetization latency | More packets, more CPU and RTP overhead | Later low-latency option |
| 20 ms | 960 | Common Opus/WebRTC balance | Slightly more latency than 10 ms | Default |
| 40 ms | 1920 | Fewer packets, lower overhead | Higher latency and larger loss unit | Avoid for live tuning unless CPU requires it |

The DSP output should be normalized into exact frame boundaries before entering
the Opus encoder. Any resampling to 48 kHz should happen before the WebRTC
publisher and remain shared/testable outside WebRTC.

## Comparison

| Path | Implementation difficulty | Dependencies | License notes | Pi 5 CPU expectation | Latency | Dropout tolerance | Browser compatibility | Future WBFM/NBFM/SSB/waterfall fit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| WebRTC + Opus with `webrtc` + `opus` | Medium | `webrtc`, `opus`, `libopus` | permissive; verify exact lockfile | Low to moderate | Low | Good: jitter buffer, PLC, RTCP | Strong in modern browsers | Good for audio; independent data channels/stats can coexist with waterfall |
| WebRTC + Opus with direct `opus-sys` | Medium-high | `webrtc`, `opus-sys`, `libopus` | permissive | Low to moderate | Low | Good | Strong | Good, but more unsafe/FFI surface |
| WebRTC + GStreamer | High | GStreamer runtime/plugins and Rust bindings | plugin-dependent | Moderate | Low to medium | Good if pipeline is tuned | Strong if SDP/media integration works | Strong for future media processing, heavier operationally |
| WebSocket PCM v1 | Already implemented | axum WebSocket, AudioWorklet | existing project deps | Low server CPU, high bandwidth | Medium | Weak: no built-in PLC/jitter buffer | Good where AudioWorklet works | Excellent diagnostics, poor WAN audio transport |
| Legacy raw PCM WebSocket | Already implemented | axum WebSocket, AudioWorklet | existing project deps | Low server CPU, high bandwidth | Medium | Weak and hard to observe | Good where AudioWorklet works | Keep only for compatibility/debug |

## Migration Plan

1. Step 14: signaling PoC with WebRTC session creation and a silent/dummy audio
   track. No SDR audio is required.
2. Step 15: add Opus encoder around fixed 48 kHz mono frames and feed a local
   WebRTC audio track.
3. Step 16: integrate the real DSP PCM stream, add backpressure/session limits,
   and compare WebRTC stats with PCM v1 diagnostics.
4. Step 17: make WebRTC the default UI playback mode; move PCM modes under
   diagnostics/legacy.
5. Later: TURN config, external-network deployment docs, bitrate controls,
   WBFM/NBFM/SSB-specific audio settings, and optional data channel/stats
   transport.

## Risks

- `webrtc` crate API churn: latest observed version is beta. Mitigate by
  pinning an exact version and keeping Step 14 narrow.
- Encoded sample vs RTP packet track choice: `TrackLocalStaticSample` may be
  easier, but Step 14 must verify how Opus samples are timestamped and
  packetized in the selected version.
- SDP codec negotiation: server must advertise Opus for audio and reject or
  ignore unsupported codecs.
- ICE reachability: reverse proxy success does not imply media path success.
- Multiple listeners: each peer may need its own encoder packet stream or a
  carefully shared encoded-frame fanout with correct timestamps.
- Clocking: WebRTC sender pacing must match the 20 ms audio frame cadence.
- Raspberry Pi thermal/CPU budget: Opus is likely fine for mono audio, but
  multiple listeners and future DSP modes need measurement.
- Browser autoplay: audio startup must remain inside a user gesture.
- License drift: GStreamer plugin choices and exact crate versions need review
  before dependency merge.

## Step 14 Tasks

- Add an axum `POST /api/webrtc/offer` endpoint.
- Add a `webrtc` module that can create and close one peer connection per
  request/session.
- Choose and pin the initial `webrtc` crate version for the spike.
- Configure ICE servers from app config, defaulting to an empty list for LAN
  MVP.
- Implement non-trickle SDP offer/answer exchange.
- Add a silent or dummy audio track; real SDR audio is not required yet.
- Add browser `RTCPeerConnection` PoC in the existing UI.
- Add WebRTC playback mode while preserving `PCM v1` and `Legacy PCM`.
- Display signaling, ICE gathering, ICE connection, and peer connection state.
- Display basic browser `getStats()` values when connected.
- Verify URLs under both root mount and `/rtl-sdr/` base path.
- Test from localhost and, if possible, another LAN client.
- Do not remove or rewrite the existing PCM WebSocket path.
