const elements = {
  reloadDevicesButton: document.querySelector("#reloadDevicesButton"),
  deviceSelect: document.querySelector("#deviceSelect"),
  connectButton: document.querySelector("#connectButton"),
  disconnectButton: document.querySelector("#disconnectButton"),
  frequencyInput: document.querySelector("#frequencyInput"),
  sampleRateInput: document.querySelector("#sampleRateInput"),
  gainModeSelect: document.querySelector("#gainModeSelect"),
  gainInput: document.querySelector("#gainInput"),
  applySettingsButton: document.querySelector("#applySettingsButton"),
  startButton: document.querySelector("#startButton"),
  stopButton: document.querySelector("#stopButton"),
  startAudioButton: document.querySelector("#startAudioButton"),
  stopAudioButton: document.querySelector("#stopAudioButton"),
  audioProtocolSelect: document.querySelector("#audioProtocolSelect"),
  connectedState: document.querySelector("#connectedState"),
  receivingState: document.querySelector("#receivingState"),
  audioState: document.querySelector("#audioState"),
  webrtcState: document.querySelector("#webrtcState"),
  message: document.querySelector("#message"),
  createWebRtcButton: document.querySelector("#createWebRtcButton"),
  playWebRtcAudioButton: document.querySelector("#playWebRtcAudioButton"),
  closeWebRtcButton: document.querySelector("#closeWebRtcButton"),
  webrtcAudioElement: document.querySelector("#webrtcAudioElement"),
  webrtcSessionMetric: document.querySelector("#webrtcSessionMetric"),
  webrtcSignalingMetric: document.querySelector("#webrtcSignalingMetric"),
  webrtcIceGatheringMetric: document.querySelector("#webrtcIceGatheringMetric"),
  webrtcIceConnectionMetric: document.querySelector("#webrtcIceConnectionMetric"),
  webrtcPeerConnectionMetric: document.querySelector("#webrtcPeerConnectionMetric"),
  webrtcDataChannelMetric: document.querySelector("#webrtcDataChannelMetric"),
  webrtcRemoteTrackMetric: document.querySelector("#webrtcRemoteTrackMetric"),
  webrtcAudioElementMetric: document.querySelector("#webrtcAudioElementMetric"),
  webrtcMutedMetric: document.querySelector("#webrtcMutedMetric"),
  webrtcPlaybackMetric: document.querySelector("#webrtcPlaybackMetric"),
  webrtcInboundMetric: document.querySelector("#webrtcInboundMetric"),
  webrtcInboundQualityMetric: document.querySelector("#webrtcInboundQualityMetric"),
  webrtcConcealmentMetric: document.querySelector("#webrtcConcealmentMetric"),
  webrtcAudioLevelMetric: document.querySelector("#webrtcAudioLevelMetric"),
  webrtcServerSessionsMetric: document.querySelector("#webrtcServerSessionsMetric"),
  webrtcServerAudioMetric: document.querySelector("#webrtcServerAudioMetric"),
  webrtcServerAudioErrorMetric: document.querySelector("#webrtcServerAudioErrorMetric"),
  webrtcCandidatePairMetric: document.querySelector("#webrtcCandidatePairMetric"),
  webrtcTransportMetric: document.querySelector("#webrtcTransportMetric"),
  webrtcBytesMetric: document.querySelector("#webrtcBytesMetric"),
  webrtcRttMetric: document.querySelector("#webrtcRttMetric"),
  webrtcIceServersMetric: document.querySelector("#webrtcIceServersMetric"),
  webrtcErrorMetric: document.querySelector("#webrtcErrorMetric"),
  wsMetric: document.querySelector("#wsMetric"),
  protocolMetric: document.querySelector("#protocolMetric"),
  framesMetric: document.querySelector("#framesMetric"),
  bytesMetric: document.querySelector("#bytesMetric"),
  bufferMetric: document.querySelector("#bufferMetric"),
  targetBufferMetric: document.querySelector("#targetBufferMetric"),
  initialBufferMetric: document.querySelector("#initialBufferMetric"),
  watermarkMetric: document.querySelector("#watermarkMetric"),
  underrunMetric: document.querySelector("#underrunMetric"),
  overflowMetric: document.querySelector("#overflowMetric"),
  droppedMetric: document.querySelector("#droppedMetric"),
  receivedSamplesMetric: document.querySelector("#receivedSamplesMetric"),
  lastSequenceMetric: document.querySelector("#lastSequenceMetric"),
  sequenceGapMetric: document.querySelector("#sequenceGapMetric"),
  outOfOrderMetric: document.querySelector("#outOfOrderMetric"),
  invalidFrameMetric: document.querySelector("#invalidFrameMetric"),
  jitterMetric: document.querySelector("#jitterMetric"),
  serverPtsMetric: document.querySelector("#serverPtsMetric"),
  clientReceivedMetric: document.querySelector("#clientReceivedMetric"),
  driftMetric: document.querySelector("#driftMetric"),
  playedSamplesMetric: document.querySelector("#playedSamplesMetric"),
  contextMetric: document.querySelector("#contextMetric"),
  formatMetric: document.querySelector("#formatMetric"),
  serverRateMetric: document.querySelector("#serverRateMetric"),
  browserRateMetric: document.querySelector("#browserRateMetric"),
  audioErrorMetric: document.querySelector("#audioErrorMetric"),
  clientsMetric: document.querySelector("#clientsMetric"),
  streamBroadcastMetric: document.querySelector("#streamBroadcastMetric"),
  streamDropMetric: document.querySelector("#streamDropMetric"),
};

const state = {
  devices: [],
  session: { connected: false, receiving: false },
  busy: false,
  socket: null,
  audioContext: null,
  workletNode: null,
  webrtc: {
    peerConnection: null,
    dataChannel: null,
    remoteStream: null,
    remoteTrackState: "none",
    sessionId: null,
    iceServers: [],
    signalingState: "closed",
    iceGatheringState: "new",
    iceConnectionState: "new",
    connectionState: "closed",
    dataChannelState: "closed",
    selectedCandidatePair: "-",
    transportState: "-",
    bytesSent: 0,
    bytesReceived: 0,
    roundTripTimeMs: null,
    inboundPacketsReceived: 0,
    inboundBytesReceived: 0,
    inboundPacketsLost: 0,
    inboundJitterMs: null,
    concealedSamples: null,
    concealmentEvents: null,
    audioLevel: null,
    serverActiveSessions: 0,
    serverAudioFramesSent: 0,
    serverAudioBytesSent: 0,
    serverAudioError: null,
    lastError: null,
  },
  audio: {
    wsState: "disconnected",
    frames: 0,
    bytes: 0,
    sampleRate: null,
    contextSampleRate: null,
    bufferedSamples: 0,
    targetBufferSamples: 0,
    initialBufferSamples: 0,
    lowWatermarkSamples: 0,
    highWatermarkSamples: 0,
    underruns: 0,
    overflows: 0,
    droppedSamples: 0,
    receivedSamples: 0,
    playedSamples: 0,
    protocol: "-",
    expectedSequence: null,
    lastSequence: null,
    sequenceGaps: 0,
    outOfOrderFrames: 0,
    invalidFrames: 0,
    lastArrivalMs: null,
    firstArrivalMs: null,
    jitterAvgMs: null,
    jitterMaxMs: 0,
    serverPtsMs: null,
    clientReceivedMs: null,
    estimatedDriftMs: null,
    format: "unknown",
    lastError: null,
  },
};

const AUDIO_INITIAL_BUFFER_SECONDS = 0.75;
const AUDIO_TARGET_BUFFER_SECONDS = 1.25;
const AUDIO_CAPACITY_SECONDS = 4;
const AUDIO_RENDER_INTERVAL_MS = 250;
const PCM_V1_HEADER_LEN = 36;
const PCM_V1_MAGIC = "WPCM";
const PCM_V1_VERSION = 1;
const PCM_FORMAT_I16LE = 1;
let audioRenderTimer = null;
let webRtcStatsTimer = null;

const appBaseUrl = new URL("./", import.meta.url);

function appUrl(path) {
  return new URL(path.replace(/^\/+/, ""), appBaseUrl);
}

function websocketUrl(path) {
  const url = appUrl(path);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url;
}

async function api(path, options = {}) {
  const response = await fetch(appUrl(path).href, {
    ...options,
    headers: {
      ...(options.body ? { "content-type": "application/json" } : {}),
      ...options.headers,
    },
  });
  const text = await response.text();
  const data = text ? JSON.parse(text) : {};
  if (!response.ok) {
    throw new Error(data.error || `${response.status} ${response.statusText}`);
  }
  return data;
}

function post(path, body) {
  return api(path, { method: "POST", body: body ? JSON.stringify(body) : undefined });
}

function setMessage(message = "") {
  elements.message.textContent = message;
}

function setBusy(busy) {
  state.busy = busy;
  render();
}

function selectedDeviceIndex() {
  const value = elements.deviceSelect.value;
  return value === "" ? null : Number(value);
}

function numberFromInput(input, name) {
  const value = Number(input.value);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${name} must be greater than 0`);
  }
  return Math.trunc(value);
}

function setPill(element, text, tone) {
  element.textContent = text;
  element.className = `pill ${tone || ""}`.trim();
}

function renderDevices() {
  const current = elements.deviceSelect.value;
  elements.deviceSelect.replaceChildren();

  if (state.devices.length === 0) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "No RTL-SDR devices found";
    elements.deviceSelect.append(option);
    return;
  }

  for (const device of state.devices) {
    const option = document.createElement("option");
    option.value = String(device.index);
    option.textContent = `${device.index}: ${device.name} ${device.serial ? `(${device.serial})` : ""}`;
    elements.deviceSelect.append(option);
  }

  if ([...elements.deviceSelect.options].some((option) => option.value === current)) {
    elements.deviceSelect.value = current;
  }
}

function renderSession() {
  const { connected, receiving } = state.session;
  setPill(elements.connectedState, connected ? "Device connected" : "Device disconnected", connected ? "good" : "");
  setPill(elements.receivingState, receiving ? "Receiving running" : "Receiving stopped", receiving ? "good" : "");

  const canConnect = !state.busy && !connected && selectedDeviceIndex() !== null;
  const canConfigure = !state.busy && connected && !receiving;
  elements.connectButton.disabled = !canConnect;
  elements.disconnectButton.disabled = state.busy || !connected;
  elements.applySettingsButton.disabled = !canConfigure;
  elements.startButton.disabled = state.busy || !connected || receiving;
  elements.stopButton.disabled = state.busy || !receiving;
  elements.deviceSelect.disabled = state.busy || connected;
  elements.reloadDevicesButton.disabled = state.busy;
  elements.gainInput.disabled = elements.gainModeSelect.value !== "manual";
}

function syncSettingsInputs() {
  const { settings } = state.session;
  if (settings?.center_frequency_hz) elements.frequencyInput.value = settings.center_frequency_hz;
  if (settings?.sample_rate_hz) elements.sampleRateInput.value = settings.sample_rate_hz;
  if (settings?.gain_mode) elements.gainModeSelect.value = settings.gain_mode;
  if (settings?.gain_tenths_db !== undefined) elements.gainInput.value = settings.gain_tenths_db;
}

function samplesToMs(samples, sampleRate) {
  if (!sampleRate) return "-";
  return `${Math.round((samples / sampleRate) * 1000).toLocaleString()} ms`;
}

function queueAudioRender() {
  if (audioRenderTimer !== null) return;
  audioRenderTimer = window.setTimeout(() => {
    audioRenderTimer = null;
    renderAudio();
  }, AUDIO_RENDER_INTERVAL_MS);
}

function renderAudio() {
  const running = state.audioContext?.state === "running";
  const audioOpen = state.socket || state.audioContext;
  const sampleRate = state.audio.contextSampleRate || state.audio.sampleRate;

  setPill(elements.audioState, running ? "Audio running" : "Audio stopped", running ? "good" : "");
  elements.startAudioButton.disabled = state.busy || Boolean(audioOpen);
  elements.stopAudioButton.disabled = state.busy || !audioOpen;
  elements.audioProtocolSelect.disabled = Boolean(audioOpen);
  elements.wsMetric.textContent = state.audio.wsState;
  elements.protocolMetric.textContent = state.audio.protocol;
  elements.framesMetric.textContent = state.audio.frames.toLocaleString();
  elements.bytesMetric.textContent = state.audio.bytes.toLocaleString();
  elements.bufferMetric.textContent = samplesToMs(state.audio.bufferedSamples, sampleRate);
  elements.targetBufferMetric.textContent = samplesToMs(state.audio.targetBufferSamples, sampleRate);
  elements.initialBufferMetric.textContent = samplesToMs(state.audio.initialBufferSamples, sampleRate);
  elements.watermarkMetric.textContent = `${samplesToMs(state.audio.lowWatermarkSamples, sampleRate)} / ${samplesToMs(state.audio.highWatermarkSamples, sampleRate)}`;
  elements.underrunMetric.textContent = state.audio.underruns.toLocaleString();
  elements.overflowMetric.textContent = state.audio.overflows.toLocaleString();
  elements.droppedMetric.textContent = state.audio.droppedSamples.toLocaleString();
  elements.receivedSamplesMetric.textContent = state.audio.receivedSamples.toLocaleString();
  elements.lastSequenceMetric.textContent = state.audio.lastSequence === null ? "-" : state.audio.lastSequence.toString();
  elements.sequenceGapMetric.textContent = state.audio.sequenceGaps.toLocaleString();
  elements.outOfOrderMetric.textContent = state.audio.outOfOrderFrames.toLocaleString();
  elements.invalidFrameMetric.textContent = state.audio.invalidFrames.toLocaleString();
  elements.jitterMetric.textContent = state.audio.jitterAvgMs === null ? "-" : `${state.audio.jitterAvgMs.toFixed(1)} / ${state.audio.jitterMaxMs.toFixed(1)} ms`;
  elements.serverPtsMetric.textContent = state.audio.serverPtsMs === null ? "-" : `${Math.round(state.audio.serverPtsMs).toLocaleString()} ms`;
  elements.clientReceivedMetric.textContent = state.audio.clientReceivedMs === null ? "-" : `${Math.round(state.audio.clientReceivedMs).toLocaleString()} ms`;
  elements.driftMetric.textContent = state.audio.estimatedDriftMs === null ? "-" : `${Math.round(state.audio.estimatedDriftMs).toLocaleString()} ms`;
  elements.playedSamplesMetric.textContent = state.audio.playedSamples.toLocaleString();
  elements.contextMetric.textContent = state.audioContext?.state || "closed";
  elements.formatMetric.textContent = state.audio.format;
  elements.serverRateMetric.textContent = state.audio.sampleRate ? state.audio.sampleRate.toLocaleString() : "-";
  elements.browserRateMetric.textContent = state.audio.contextSampleRate ? state.audio.contextSampleRate.toLocaleString() : "-";
  elements.audioErrorMetric.textContent = state.audio.lastError || "-";
}



function renderWebRtc() {
  const rtc = state.webrtc;
  const open = Boolean(rtc.peerConnection);
  const connected = rtc.connectionState === "connected" || rtc.iceConnectionState === "connected";
  const connecting = open && !connected && rtc.connectionState !== "failed";
  const tone = connected ? "good" : rtc.connectionState === "failed" ? "bad" : "";
  setPill(elements.webrtcState, connected ? "WebRTC connected" : connecting ? "WebRTC connecting" : "WebRTC closed", tone);
  elements.createWebRtcButton.disabled = state.busy || open;
  elements.playWebRtcAudioButton.disabled = state.busy || !open || !rtc.remoteStream;
  elements.closeWebRtcButton.disabled = state.busy || !open;
  elements.webrtcSessionMetric.textContent = rtc.sessionId || "-";
  elements.webrtcSignalingMetric.textContent = rtc.signalingState;
  elements.webrtcIceGatheringMetric.textContent = rtc.iceGatheringState;
  elements.webrtcIceConnectionMetric.textContent = rtc.iceConnectionState;
  elements.webrtcPeerConnectionMetric.textContent = rtc.connectionState;
  elements.webrtcDataChannelMetric.textContent = rtc.dataChannelState;
  elements.webrtcRemoteTrackMetric.textContent = rtc.remoteTrackState;
  elements.webrtcAudioElementMetric.textContent = `ready:${elements.webrtcAudioElement.readyState} network:${elements.webrtcAudioElement.networkState}`;
  elements.webrtcMutedMetric.textContent = `${elements.webrtcAudioElement.muted} / ${elements.webrtcAudioElement.volume.toFixed(2)}`;
  elements.webrtcPlaybackMetric.textContent = elements.webrtcAudioElement.paused ? "paused" : "playing";
  elements.webrtcInboundMetric.textContent = `${rtc.inboundPacketsReceived.toLocaleString()} / ${rtc.inboundBytesReceived.toLocaleString()}`;
  elements.webrtcInboundQualityMetric.textContent = `${rtc.inboundPacketsLost.toLocaleString()} / ${rtc.inboundJitterMs === null ? "-" : `${rtc.inboundJitterMs.toFixed(2)} ms`}`;
  elements.webrtcConcealmentMetric.textContent = rtc.concealedSamples === null ? "-" : `${rtc.concealedSamples.toLocaleString()} / ${(rtc.concealmentEvents || 0).toLocaleString()}`;
  elements.webrtcAudioLevelMetric.textContent = rtc.audioLevel === null ? "-" : rtc.audioLevel.toFixed(4);
  elements.webrtcServerSessionsMetric.textContent = rtc.serverActiveSessions.toLocaleString();
  elements.webrtcServerAudioMetric.textContent = `${rtc.serverAudioFramesSent.toLocaleString()} / ${rtc.serverAudioBytesSent.toLocaleString()}`;
  elements.webrtcServerAudioErrorMetric.textContent = rtc.serverAudioError || "-";
  elements.webrtcCandidatePairMetric.textContent = rtc.selectedCandidatePair;
  elements.webrtcTransportMetric.textContent = rtc.transportState;
  elements.webrtcBytesMetric.textContent = `${rtc.bytesSent.toLocaleString()} / ${rtc.bytesReceived.toLocaleString()}`;
  elements.webrtcRttMetric.textContent = rtc.roundTripTimeMs === null ? "-" : `${rtc.roundTripTimeMs.toFixed(1)} ms`;
  elements.webrtcIceServersMetric.textContent = rtc.iceServers.length ? rtc.iceServers.join(", ") : "none";
  elements.webrtcErrorMetric.textContent = rtc.lastError || "-";
}

function resetWebRtcStats() {
  state.webrtc.signalingState = "closed";
  state.webrtc.iceGatheringState = "new";
  state.webrtc.iceConnectionState = "new";
  state.webrtc.connectionState = "closed";
  state.webrtc.dataChannelState = "closed";
  state.webrtc.remoteTrackState = "none";
  state.webrtc.selectedCandidatePair = "-";
  state.webrtc.transportState = "-";
  state.webrtc.bytesSent = 0;
  state.webrtc.bytesReceived = 0;
  state.webrtc.roundTripTimeMs = null;
  state.webrtc.inboundPacketsReceived = 0;
  state.webrtc.inboundBytesReceived = 0;
  state.webrtc.inboundPacketsLost = 0;
  state.webrtc.inboundJitterMs = null;
  state.webrtc.concealedSamples = null;
  state.webrtc.concealmentEvents = null;
  state.webrtc.audioLevel = null;
  state.webrtc.serverActiveSessions = 0;
  state.webrtc.serverAudioFramesSent = 0;
  state.webrtc.serverAudioBytesSent = 0;
  state.webrtc.serverAudioError = null;
  state.webrtc.lastError = null;
}

function updateWebRtcStateFromPeerConnection(peerConnection) {
  state.webrtc.signalingState = peerConnection.signalingState;
  state.webrtc.iceGatheringState = peerConnection.iceGatheringState;
  state.webrtc.iceConnectionState = peerConnection.iceConnectionState;
  state.webrtc.connectionState = peerConnection.connectionState;
  renderWebRtc();
}

function waitForIceGatheringComplete(peerConnection) {
  if (peerConnection.iceGatheringState === "complete") return Promise.resolve();
  return new Promise((resolve) => {
    const onStateChange = () => {
      updateWebRtcStateFromPeerConnection(peerConnection);
      if (peerConnection.iceGatheringState === "complete") {
        peerConnection.removeEventListener("icegatheringstatechange", onStateChange);
        resolve();
      }
    };
    peerConnection.addEventListener("icegatheringstatechange", onStateChange);
  });
}

async function refreshWebRtcStats() {
  const peerConnection = state.webrtc.peerConnection;
  if (!peerConnection) return;

  try {
    const stats = await peerConnection.getStats();
    let selectedPair = null;
    let transport = null;
    let inboundAudio = null;
    const reports = new Map();
    stats.forEach((report) => reports.set(report.id, report));

    stats.forEach((report) => {
      if (report.type === "transport") {
        transport = report;
        if (report.selectedCandidatePairId) selectedPair = reports.get(report.selectedCandidatePairId) || selectedPair;
      }
      if (report.type === "candidate-pair" && (report.selected || report.nominated || report.state === "succeeded")) {
        selectedPair = report;
      }
      if (report.type === "inbound-rtp" && (report.kind === "audio" || report.mediaType === "audio")) {
        inboundAudio = report;
      }
    });

    if (selectedPair) {
      const local = reports.get(selectedPair.localCandidateId);
      const remote = reports.get(selectedPair.remoteCandidateId);
      const localLabel = local ? `${local.candidateType || "local"}/${local.protocol || "?"}/${local.address || local.ip || "?"}:${local.port || "?"}` : "local";
      const remoteLabel = remote ? `${remote.candidateType || "remote"}/${remote.protocol || "?"}/${remote.address || remote.ip || "?"}:${remote.port || "?"}` : "remote";
      state.webrtc.selectedCandidatePair = `${localLabel} -> ${remoteLabel}`;
      state.webrtc.bytesSent = selectedPair.bytesSent || 0;
      state.webrtc.bytesReceived = selectedPair.bytesReceived || 0;
      state.webrtc.roundTripTimeMs = typeof selectedPair.currentRoundTripTime === "number" ? selectedPair.currentRoundTripTime * 1000 : null;
    }

    if (inboundAudio) {
      state.webrtc.inboundPacketsReceived = inboundAudio.packetsReceived || 0;
      state.webrtc.inboundBytesReceived = inboundAudio.bytesReceived || 0;
      state.webrtc.inboundPacketsLost = inboundAudio.packetsLost || 0;
      state.webrtc.inboundJitterMs = typeof inboundAudio.jitter === "number" ? inboundAudio.jitter * 1000 : null;
      state.webrtc.concealedSamples = typeof inboundAudio.concealedSamples === "number" ? inboundAudio.concealedSamples : null;
      state.webrtc.concealmentEvents = typeof inboundAudio.concealmentEvents === "number" ? inboundAudio.concealmentEvents : null;
      state.webrtc.audioLevel = typeof inboundAudio.audioLevel === "number" ? inboundAudio.audioLevel : null;
    }

    try {
      const serverStats = await api("api/webrtc/stats");
      state.webrtc.serverActiveSessions = serverStats.active_sessions || 0;
      state.webrtc.serverAudioFramesSent = serverStats.audio_frames_sent || 0;
      state.webrtc.serverAudioBytesSent = serverStats.audio_bytes_sent || 0;
      state.webrtc.serverAudioError = serverStats.last_audio_send_error || null;
    } catch (error) {
      state.webrtc.serverAudioError = error.message;
    }

    state.webrtc.transportState = transport ? `${transport.dtlsState || "dtls:?"} / ${transport.iceState || "ice:?"}` : "-";
    renderWebRtc();
  } catch (error) {
    console.warn(error);
    state.webrtc.lastError = error.message;
    renderWebRtc();
  }
}

function startWebRtcStatsTimer() {
  if (webRtcStatsTimer !== null) return;
  webRtcStatsTimer = window.setInterval(refreshWebRtcStats, 1000);
}

function stopWebRtcStatsTimer() {
  if (webRtcStatsTimer === null) return;
  window.clearInterval(webRtcStatsTimer);
  webRtcStatsTimer = null;
}

async function loadWebRtcConfig() {
  const config = await api("api/webrtc/config");
  state.webrtc.iceServers = config.ice_servers || [];
  renderWebRtc();
}

async function playWebRtcAudio() {
  if (!elements.webrtcAudioElement.srcObject) return;
  try {
    await elements.webrtcAudioElement.play();
  } catch (error) {
    state.webrtc.lastError = error.message;
  }
  renderWebRtc();
}

async function createWebRtcSession() {
  resetWebRtcStats();
  const config = await api("api/webrtc/config");
  state.webrtc.iceServers = config.ice_servers || [];
  const iceServers = state.webrtc.iceServers.map((url) => ({ urls: url }));
  const peerConnection = new RTCPeerConnection({ iceServers });
  peerConnection.addTransceiver("audio", { direction: "recvonly" });
  const dataChannel = peerConnection.createDataChannel("diagnostics");
  state.webrtc.peerConnection = peerConnection;
  state.webrtc.dataChannel = dataChannel;
  state.webrtc.dataChannelState = dataChannel.readyState;

  const update = () => updateWebRtcStateFromPeerConnection(peerConnection);
  peerConnection.addEventListener("signalingstatechange", update);
  peerConnection.addEventListener("icegatheringstatechange", update);
  peerConnection.addEventListener("iceconnectionstatechange", update);
  peerConnection.addEventListener("connectionstatechange", update);
  peerConnection.addEventListener("track", (event) => {
    const stream = event.streams[0] || new MediaStream([event.track]);
    state.webrtc.remoteStream = stream;
    state.webrtc.remoteTrackState = `${event.track.kind}:${event.track.readyState}`;
    elements.webrtcAudioElement.srcObject = stream;
    event.track.addEventListener("ended", () => {
      state.webrtc.remoteTrackState = `${event.track.kind}:ended`;
      renderWebRtc();
    });
    playWebRtcAudio();
    renderWebRtc();
  });
  dataChannel.addEventListener("open", () => {
    state.webrtc.dataChannelState = dataChannel.readyState;
    dataChannel.send("ping");
    renderWebRtc();
  });
  dataChannel.addEventListener("close", () => {
    state.webrtc.dataChannelState = dataChannel.readyState;
    renderWebRtc();
  });
  dataChannel.addEventListener("error", () => {
    state.webrtc.dataChannelState = dataChannel.readyState;
    state.webrtc.lastError = "WebRTC data channel error";
    renderWebRtc();
  });

  try {
    update();
    const offer = await peerConnection.createOffer();
    await peerConnection.setLocalDescription(offer);
    await waitForIceGatheringComplete(peerConnection);
    const localDescription = peerConnection.localDescription;
    const answer = await post("api/webrtc/offer", {
      type: localDescription.type,
      sdp: localDescription.sdp,
    });
    state.webrtc.sessionId = answer.session_id;
    await peerConnection.setRemoteDescription({ type: answer.type, sdp: answer.sdp });
    update();
    startWebRtcStatsTimer();
  } catch (error) {
    peerConnection.close();
    state.webrtc.peerConnection = null;
    state.webrtc.dataChannel = null;
    state.webrtc.remoteStream = null;
    elements.webrtcAudioElement.srcObject = null;
    state.webrtc.sessionId = null;
    resetWebRtcStats();
    state.webrtc.lastError = error.message;
    renderWebRtc();
    throw error;
  }
}

async function closeWebRtcSession() {
  const { peerConnection, sessionId } = state.webrtc;
  stopWebRtcStatsTimer();
  state.webrtc.peerConnection = null;
  state.webrtc.dataChannel = null;
  state.webrtc.remoteStream = null;
  elements.webrtcAudioElement.pause();
  elements.webrtcAudioElement.srcObject = null;
  state.webrtc.sessionId = null;
  resetWebRtcStats();
  if (peerConnection) peerConnection.close();
  if (sessionId) {
    await api(`api/webrtc/sessions/${encodeURIComponent(sessionId)}`, { method: "DELETE" });
  }
  renderWebRtc();
}

function renderStats(stats) {
  if (stats?.stream) {
    elements.clientsMetric.textContent = String(stats.stream.active_clients);
    const framesSent = stats.stream.frames_sent ?? stats.stream.frames_broadcast ?? 0;
    const bytesSent = stats.stream.bytes_sent ?? stats.stream.bytes_broadcast ?? 0;
    const droppedFrames = stats.stream.dropped_frames ?? stats.stream.frames_dropped ?? 0;
    elements.streamBroadcastMetric.textContent = `${framesSent.toLocaleString()} / ${bytesSent.toLocaleString()}`;
    elements.streamDropMetric.textContent = `${droppedFrames.toLocaleString()} / ${(stats.stream.lagged_subscribers || 0).toLocaleString()}`;
  }
}

function render() {
  renderDevices();
  renderSession();
  renderAudio();
  renderWebRtc();
}

async function refreshSession() {
  state.session = await api("api/session");
  syncSettingsInputs();
  render();
}

async function refreshStats() {
  try {
    renderStats(await api("api/session/stats"));
  } catch (error) {
    console.warn(error);
  }
}

async function loadDevices() {
  setBusy(true);
  try {
    setMessage("");
    const response = await api("api/devices");
    state.devices = response.devices || [];
    await refreshSession();
  } catch (error) {
    console.error(error);
    setMessage(error.message);
  } finally {
    setBusy(false);
  }
}

async function runAction(action) {
  setBusy(true);
  try {
    setMessage("");
    await action();
    await refreshSession();
    await refreshStats();
  } catch (error) {
    console.error(error);
    setMessage(error.message);
  } finally {
    setBusy(false);
  }
}

async function applySettings() {
  const frequency_hz = numberFromInput(elements.frequencyInput, "Frequency");
  const sample_rate_hz = numberFromInput(elements.sampleRateInput, "Sample rate");
  await post("api/session/tune", { frequency_hz });
  await post("api/session/sample-rate", { sample_rate_hz });

  if (elements.gainModeSelect.value === "manual") {
    const gain_tenths_db = Math.trunc(Number(elements.gainInput.value));
    if (!Number.isFinite(gain_tenths_db)) {
      throw new Error("Manual gain must be a number");
    }
    await post("api/session/gain", { mode: "manual", gain_tenths_db });
  } else {
    await post("api/session/gain", { mode: "auto" });
  }
}

function resetAudioStats() {
  state.audio = {
    wsState: "disconnected",
    frames: 0,
    bytes: 0,
    sampleRate: null,
    contextSampleRate: null,
    bufferedSamples: 0,
    targetBufferSamples: 0,
    initialBufferSamples: 0,
    lowWatermarkSamples: 0,
    highWatermarkSamples: 0,
    underruns: 0,
    overflows: 0,
    droppedSamples: 0,
    receivedSamples: 0,
    playedSamples: 0,
    protocol: "-",
    expectedSequence: null,
    lastSequence: null,
    sequenceGaps: 0,
    outOfOrderFrames: 0,
    invalidFrames: 0,
    lastArrivalMs: null,
    firstArrivalMs: null,
    jitterAvgMs: null,
    jitterMaxMs: 0,
    serverPtsMs: null,
    clientReceivedMs: null,
    estimatedDriftMs: null,
    format: "unknown",
    lastError: null,
  };
}

async function ensureAudioContext(sampleRate) {
  if (!state.audioContext) {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext;
    if (!AudioContextClass) {
      throw new Error("Web Audio API is not available in this browser");
    }

    state.audioContext = new AudioContextClass({ sampleRate });
    if (!state.audioContext.audioWorklet) {
      const context = state.audioContext;
      state.audioContext = null;
      await context.close();
      if (!window.isSecureContext) {
        throw new Error(
          "AudioWorklet requires a secure context. Use an SSH tunnel and open http://127.0.0.1:3000/, or serve this app over HTTPS.",
        );
      }
      throw new Error("AudioWorklet is not available in this browser. Try a current Chrome, Edge, Firefox, or Safari.");
    }

    await state.audioContext.audioWorklet.addModule(appUrl("audio-worklet.js").href);
    const capacitySamples = Math.floor(state.audioContext.sampleRate * AUDIO_CAPACITY_SECONDS);
    const startThresholdSamples = Math.floor(state.audioContext.sampleRate * AUDIO_INITIAL_BUFFER_SECONDS);
    const targetBufferedSamples = Math.floor(state.audioContext.sampleRate * AUDIO_TARGET_BUFFER_SECONDS);
    state.audio.contextSampleRate = state.audioContext.sampleRate;
    state.audio.initialBufferSamples = startThresholdSamples;
    state.audio.targetBufferSamples = targetBufferedSamples;

    state.workletNode = new AudioWorkletNode(state.audioContext, "pcm-player", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [2],
      processorOptions: {
        capacitySamples,
        startThresholdSamples,
        targetBufferedSamples,
      },
    });
    state.workletNode.port.onmessage = (event) => {
      if (event.data?.type !== "stats") return;
      state.audio.bufferedSamples = event.data.bufferedSamples || 0;
      state.audio.initialBufferSamples = event.data.initialBufferSamples || state.audio.initialBufferSamples;
      state.audio.targetBufferSamples = event.data.targetBufferSamples || state.audio.targetBufferSamples;
      state.audio.lowWatermarkSamples = event.data.lowWatermarkSamples || 0;
      state.audio.highWatermarkSamples = event.data.highWatermarkSamples || 0;
      state.audio.underruns = event.data.underruns || 0;
      state.audio.overflows = event.data.overflows || 0;
      state.audio.droppedSamples = event.data.droppedSamples || 0;
      state.audio.receivedSamples = event.data.receivedSamples || state.audio.receivedSamples;
      state.audio.playedSamples = event.data.playedSamples || 0;
      queueAudioRender();
    };
    state.workletNode.connect(state.audioContext.destination);
  }

  if (state.audioContext.state !== "running") {
    await state.audioContext.resume();
  }
}

function pcmI16LeToFloat32(buffer, byteOffset = 0, byteLength = buffer.byteLength - byteOffset) {
  const view = new DataView(buffer, byteOffset, byteLength);
  const samples = new Float32Array(Math.floor(byteLength / 2));
  for (let index = 0; index < samples.length; index += 1) {
    samples[index] = view.getInt16(index * 2, true) / 32768;
  }
  return samples;
}

function readU64Le(view, offset) {
  return view.getBigUint64(offset, true);
}

function addSequenceGap(gap) {
  const capped = gap > BigInt(Number.MAX_SAFE_INTEGER) ? Number.MAX_SAFE_INTEGER : Number(gap);
  state.audio.sequenceGaps += capped;
}

function parsePcmV1Frame(buffer) {
  if (buffer.byteLength < PCM_V1_HEADER_LEN) {
    throw new Error("PCM v1 frame shorter than header");
  }

  const view = new DataView(buffer);
  const magic = String.fromCharCode(view.getUint8(0), view.getUint8(1), view.getUint8(2), view.getUint8(3));
  const version = view.getUint8(4);
  const headerLen = view.getUint8(5);
  const format = view.getUint8(6);
  const channels = view.getUint8(7);
  const sampleRate = view.getUint32(8, true);
  const sampleCount = view.getUint32(12, true);
  const sequence = readU64Le(view, 16);
  const ptsSamples = readU64Le(view, 24);
  const payloadBytes = view.getUint32(32, true);

  if (magic !== PCM_V1_MAGIC) throw new Error("PCM v1 bad magic");
  if (version !== PCM_V1_VERSION) throw new Error(`PCM v1 unsupported version ${version}`);
  if (headerLen < PCM_V1_HEADER_LEN || headerLen > buffer.byteLength) throw new Error("PCM v1 invalid header length");
  if (format !== PCM_FORMAT_I16LE) throw new Error(`PCM v1 unsupported format ${format}`);
  if (channels !== 1) throw new Error(`PCM v1 unsupported channels ${channels}`);
  if (payloadBytes !== sampleCount * channels * 2) throw new Error("PCM v1 payload_bytes mismatch");
  if (buffer.byteLength !== headerLen + payloadBytes) throw new Error("PCM v1 message length mismatch");

  const now = performance.now();
  if (state.audio.firstArrivalMs === null) state.audio.firstArrivalMs = now;
  if (state.audio.expectedSequence !== null) {
    if (sequence > state.audio.expectedSequence) {
      addSequenceGap(sequence - state.audio.expectedSequence);
    } else if (sequence < state.audio.expectedSequence) {
      state.audio.outOfOrderFrames += 1;
    }
  }

  if (state.audio.lastArrivalMs !== null) {
    const expectedMs = (sampleCount / sampleRate) * 1000;
    const arrivalIntervalMs = now - state.audio.lastArrivalMs;
    const jitterMs = Math.abs(arrivalIntervalMs - expectedMs);
    state.audio.jitterAvgMs = state.audio.jitterAvgMs === null ? jitterMs : state.audio.jitterAvgMs * 0.9 + jitterMs * 0.1;
    state.audio.jitterMaxMs = Math.max(state.audio.jitterMaxMs, jitterMs);
  }

  state.audio.lastArrivalMs = now;
  state.audio.expectedSequence = sequence + 1n;
  state.audio.lastSequence = sequence;
  state.audio.serverPtsMs = (Number(ptsSamples) / sampleRate) * 1000;
  state.audio.clientReceivedMs = now - state.audio.firstArrivalMs;
  state.audio.estimatedDriftMs = state.audio.clientReceivedMs - state.audio.serverPtsMs;

  return {
    samples: pcmI16LeToFloat32(buffer, headerLen, payloadBytes),
    sampleRate,
    sampleCount,
    payloadBytes,
  };
}

async function startAudio() {
  resetAudioStats();
  state.audio.wsState = "connecting";
  state.audio.lastError = null;
  renderAudio();
  try {
    await ensureAudioContext(48000);
  } catch (error) {
    state.audio.wsState = "disconnected";
    state.audio.lastError = error.message;
    renderAudio();
    throw error;
  }

  const selectedProtocol = elements.audioProtocolSelect.value;
  const endpointPath = selectedProtocol === "v1" ? "ws/audio-v1" : "ws/audio";
  const endpointUrl = websocketUrl(endpointPath).href;
  const socket = new WebSocket(endpointUrl);
  let socketOpened = false;
  state.audio.protocol = selectedProtocol === "v1" ? "pcm-v1" : "legacy-raw";
  state.socket = socket;
  socket.binaryType = "arraybuffer";

  socket.addEventListener("open", () => {
    socketOpened = true;
    state.audio.wsState = "connected";
    renderAudio();
  });

  socket.addEventListener("message", async (event) => {
    try {
      if (typeof event.data === "string") {
        const metadata = JSON.parse(event.data);
        if (metadata.type === "audio_protocol") {
          state.audio.sampleRate = metadata.audio?.sample_rate_hz || state.audio.sampleRate;
          state.audio.protocol = `${metadata.protocol} v${metadata.version}`;
          state.audio.format = `${metadata.audio?.format}, ${metadata.audio?.channels} ch, ${metadata.audio?.sample_rate_hz} Hz`;
          await ensureAudioContext(metadata.audio?.sample_rate_hz || 48000);
        } else {
          state.audio.sampleRate = metadata.sample_rate_hz || state.audio.sampleRate;
          state.audio.protocol = "legacy-raw";
          state.audio.format = `${metadata.format}, ${metadata.channels} ch, ${metadata.sample_rate_hz} Hz`;
          await ensureAudioContext(metadata.sample_rate_hz);
        }
        renderAudio();
        return;
      }

      let samples;
      let payloadBytes = event.data.byteLength;
      if (selectedProtocol === "v1") {
        try {
          const frame = parsePcmV1Frame(event.data);
          samples = frame.samples;
          payloadBytes = frame.payloadBytes;
          state.audio.sampleRate = frame.sampleRate;
        } catch (error) {
          state.audio.invalidFrames += 1;
          state.audio.lastError = error.message;
          queueAudioRender();
          return;
        }
      } else {
        samples = pcmI16LeToFloat32(event.data);
      }

      state.audio.frames += 1;
      state.audio.bytes += payloadBytes;
      state.audio.receivedSamples += samples.length;
      state.workletNode?.port.postMessage({ type: "samples", samples }, [samples.buffer]);
      queueAudioRender();
    } catch (error) {
      console.error(error);
      state.audio.lastError = error.message;
      setMessage(error.message);
      renderAudio();
    }
  });

  socket.addEventListener("close", () => {
    if (state.socket !== socket) return;
    state.socket = null;
    state.audio.wsState = "disconnected";
    renderAudio();
  });

  socket.addEventListener("error", () => {
    if (!socketOpened && selectedProtocol === "v1") {
      if (state.socket === socket) state.socket = null;
      elements.audioProtocolSelect.value = "legacy";
      state.audio.wsState = "connecting";
      state.audio.lastError = `PCM v1 WebSocket failed at ${endpointUrl}; retrying legacy PCM`;
      setMessage(state.audio.lastError);
      renderAudio();
      startAudio().catch((error) => {
        console.error(error);
        state.audio.wsState = "error";
        state.audio.lastError = error.message;
        setMessage(error.message);
        renderAudio();
      });
      return;
    }

    state.audio.wsState = "error";
    state.audio.lastError = `WebSocket audio connection failed at ${endpointUrl}`;
    setMessage(state.audio.lastError);
    renderAudio();
  });
}

async function stopAudio() {
  const socket = state.socket;
  state.socket = null;
  if (socket && socket.readyState < WebSocket.CLOSING) {
    socket.close();
  }

  state.workletNode?.port.postMessage({ type: "reset" });
  state.workletNode?.disconnect();
  state.workletNode = null;

  if (state.audioContext) {
    await state.audioContext.close();
    state.audioContext = null;
  }

  state.audio.contextSampleRate = null;
  state.audio.wsState = "disconnected";
  renderAudio();
  await refreshStats();
}

elements.reloadDevicesButton.addEventListener("click", loadDevices);
elements.deviceSelect.addEventListener("change", render);
elements.gainModeSelect.addEventListener("change", render);
elements.connectButton.addEventListener("click", () =>
  runAction(async () => {
    const index = selectedDeviceIndex();
    if (index === null) throw new Error("Select an RTL-SDR device first");
    await post("api/session/connect", { index });
  }),
);
elements.disconnectButton.addEventListener("click", () =>
  runAction(async () => {
    await stopAudio();
    await post("api/session/disconnect");
  }),
);
elements.applySettingsButton.addEventListener("click", () => runAction(applySettings));
elements.startButton.addEventListener("click", () => runAction(() => post("api/session/start")));
elements.stopButton.addEventListener("click", () =>
  runAction(async () => {
    await stopAudio();
    await post("api/session/stop");
  }),
);
elements.startAudioButton.addEventListener("click", () => runAction(startAudio));
elements.stopAudioButton.addEventListener("click", () => runAction(stopAudio));
elements.createWebRtcButton.addEventListener("click", () => runAction(createWebRtcSession));
elements.playWebRtcAudioButton.addEventListener("click", () => runAction(playWebRtcAudio));
elements.closeWebRtcButton.addEventListener("click", () => runAction(closeWebRtcSession));
elements.webrtcAudioElement.addEventListener("play", renderWebRtc);
elements.webrtcAudioElement.addEventListener("pause", renderWebRtc);
elements.webrtcAudioElement.addEventListener("volumechange", renderWebRtc);
elements.webrtcAudioElement.addEventListener("loadedmetadata", renderWebRtc);

window.addEventListener("beforeunload", () => {
  if (state.socket && state.socket.readyState < WebSocket.CLOSING) {
    state.socket.close();
  }
  if (state.webrtc.peerConnection) {
    state.webrtc.peerConnection.close();
  }
  elements.webrtcAudioElement.srcObject = null;
  if (state.webrtc.sessionId) {
    fetch(appUrl(`api/webrtc/sessions/${encodeURIComponent(state.webrtc.sessionId)}`).href, {
      method: "DELETE",
      keepalive: true,
    }).catch(() => {});
  }
});

setInterval(refreshStats, 2000);
await loadWebRtcConfig().catch((error) => console.warn(error));
await loadDevices();
