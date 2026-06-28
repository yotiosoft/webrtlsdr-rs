# WebRTC Signaling and Silent Audio PoC

Step 14 implemented HTTP signaling between the browser and Rust server, plus
browser-visible ICE and connection state. Step 15 adds a server-side silent Opus
audio track so the browser can verify remote track delivery, `<audio>` element
attachment, and inbound audio RTP stats before SDR audio is connected.

## Endpoints

- `GET /api/webrtc/config`: returns configured ICE server URLs for the browser.
- `GET /api/webrtc/stats`: returns active WebRTC sessions, silent audio frames
  and bytes sent, the last send error, and peer connection states.
- `POST /api/webrtc/offer`: accepts a browser SDP offer and returns an SDP
  answer with a `session_id`.
- `DELETE /api/webrtc/sessions/:session_id`: closes the stored Rust peer
  connection and aborts its silent audio send task for explicit cleanup.

`POST /api/webrtc/offer` request:

```json
{
  "type": "offer",
  "sdp": "v=0..."
}
```

response:

```json
{
  "type": "answer",
  "sdp": "v=0...",
  "session_id": "rtc_1"
}
```

## Browser Check

1. Start the server with `cargo run`.
2. Open `http://127.0.0.1:3000/` or the Apache-mounted `/rtl-sdr/` URL.
3. Press Start WebRTC Audio in the WebRTC PoC panel.
4. Confirm the offer request succeeds, an answer is returned, and the UI updates
   signaling, ICE gathering, ICE connection, peer connection, data channel,
   selected candidate pair, transport, and stats fields.
5. Confirm Remote Track becomes `audio:live`, the `<audio>` element has a remote
   stream, and Inbound Packets / Bytes increases. If autoplay is rejected, press
   Play WebRTC Audio and confirm the rejection or playback state is visible.
6. Press Stop WebRTC Audio and confirm the peer connection closes. Repeat start
   and stop to verify reconnect cleanup.

The browser creates a diagnostics data channel and a recvonly audio transceiver,
so the server answer can attach the silent Opus track.

## ICE Configuration

The LAN MVP defaults to no ICE servers. Optional STUN/TURN URLs can be provided
with a comma-separated environment variable:

```sh
WEBRTLSDR_WEBRTC_ICE_SERVERS=stun:stun.l.google.com:19302 cargo run
```

For remote networks, VPN boundaries, NATs, or restrictive firewalls, TURN may be
required. Do not assume Apache HTTP proxying carries WebRTC media; it only carries
static assets and signaling. ICE candidates must be reachable by the browser.

## Reverse Proxy Notes

The frontend uses base-path-aware relative URLs, so signaling works from `/` and
from `/rtl-sdr/` when Apache forwards the mounted path to axum. The WebRTC ICE
path remains peer-to-peer or TURN-relayed according to the negotiated candidates.

## Next Step

Step 16 should replace the fixed silent Opus packet with 48 kHz mono SDR audio
frames encoded as Opus, while keeping the PCM WebSocket path available for
fallback diagnostics.
