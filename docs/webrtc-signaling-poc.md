# WebRTC Signaling PoC

Step 14 implements the first WebRTC + Opus migration checkpoint: HTTP signaling
between the browser and Rust server, plus browser-visible ICE and connection
state. It does not send SDR audio, encode Opus, or publish an RTP audio track.

## Endpoints

- `GET /api/webrtc/config`: returns configured ICE server URLs for the browser.
- `POST /api/webrtc/offer`: accepts a browser SDP offer and returns an SDP
  answer with a `session_id`.
- `DELETE /api/webrtc/sessions/:session_id`: closes the stored Rust peer
  connection for explicit cleanup.

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
3. Press Create WebRTC Session in the WebRTC PoC panel.
4. Confirm the offer request succeeds, an answer is returned, and the UI updates
   signaling, ICE gathering, ICE connection, peer connection, data channel,
   selected candidate pair, transport, and stats fields.
5. Press Close WebRTC Session and confirm the peer connection closes.

The browser creates a diagnostics data channel so the SDP contains a WebRTC media
section even though Step 14 has no audio track.

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

Step 15 should add an audio track, initially silent or dummy, then introduce Opus
encoding for 48 kHz mono frames before real SDR audio is routed through WebRTC.
