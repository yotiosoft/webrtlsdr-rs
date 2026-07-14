# WebRTC audio tuning and Raspberry Pi 5 measurements

## Configuration

WebRTC audio is fixed at 48 kHz mono. The DSP produces 48 kHz mono blocks and
the sender rejects a mismatched rate rather than silently resampling it. The
recommended balanced settings are:

```sh
WEBRTLSDR_WEBRTC_FRAME_DURATION_MS=20
WEBRTLSDR_WEBRTC_OPUS_BITRATE_BPS=32000
WEBRTLSDR_WEBRTC_OPUS_COMPLEXITY=5
WEBRTLSDR_WEBRTC_SILENCE_ON_UNDERRUN=true
```

Frame duration may be 10, 20, or 40 ms, corresponding to 480, 960, or 1920
samples. Bitrate and complexity are applied when the Opus encoder for a new
session is created. Environment changes require a server restart; close and
reopen browser audio after restarting. Silence-on-underrun preserves real-time
pacing. Disabling it is intended only for diagnosis.

Suggested comparisons:

| Profile | Frame | Bitrate | Complexity | Trade-off |
|---|---:|---:|---:|---|
| Low latency | 10 ms | 32–48 kbps | 3–5 | More wakeups and overhead |
| Balanced | 20 ms | 32 kbps | 5 | Recommended default |
| Low CPU / robust | 40 ms | 24–32 kbps | 3 | More packetization latency |

## Raspberry Pi 5 procedure

Use only WebRTC playback while measuring; running PCM diagnostics at the same
time intentionally adds DSP fan-out, WebSocket, and AudioWorklet load.

1. Start reception and WebRTC playback, then allow 60 seconds for warm-up.
2. Record process CPU and RSS with `pidstat -p $(pgrep -n webrtlsdr-rs) 1 60`
   or `top -p $(pgrep -n webrtlsdr-rs)`.
3. Save `curl http://127.0.0.1:3000/api/webrtc/stats`. Record encode avg/p95,
   send interval jitter, late frames, underrun silence, and errors.
4. Record browser loss, jitter, concealed samples, and whether audio dropouts
   were audible.
5. Record `vcgencmd measure_temp` and `vcgencmd get_throttled` before and after.
6. Repeat with 20/32k/5, 20/24k/3, 40/24k/3, and 10/48k/5. Restart the server
   between configurations and keep station, gain, browser, and duration fixed.

| Frame / bitrate / complexity | CPU | RSS | encode avg/p95 | send jitter | browser jitter/loss | concealed delta | temp/throttled | audible gaps |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| 20 ms / 32k / 5 | | | | | | | | |
| 20 ms / 24k / 3 | | | | | | | | |
| 40 ms / 24k / 3 | | | | | | | | |
| 10 ms / 48k / 5 | | | | | | | | |

## Interpreting symptoms

- Packet loss is missing RTP packets. Sustained loss points to the network or
  receiver, not encoder CPU alone.
- Jitter is variation in packet arrival time. Rising jitter with concealment
  indicates that the browser jitter buffer cannot fully absorb that variation.
- Concealed samples/events are browser-generated replacement audio. Compare
  deltas over the same measurement interval because these counters accumulate.
- Inserted/removed samples show browser playout-rate correction and can expose
  clock drift or unstable arrival pacing.
- Encode p95 approaching the frame duration or increasing late frames indicates
  CPU scheduling/encoder pressure. Reduce complexity or use a longer frame.
- Underrun silence means the sender did not have a complete DSP frame on time.
  Check SDR/DSP production, source lag, CPU throttling, and sample-rate errors.
- DSP `audio_peak` and `audio_rms` are available from `/api/session/stats`.
  Persistent peaks near 1.0 or increasing PCM clip counts suggest excessive RF
  or audio level; reduce tuner gain. Very low RMS suggests mistuning or low gain.

For an A/B comparison, run WebRTC alone and record its statistics, then stop it
and run PCM diagnostics alone with the same receiver settings. Compare audible
dropouts, DSP peak/RMS, PCM underruns and arrival jitter, and process CPU. Do not
interpret simultaneous playback as a baseline CPU comparison.
