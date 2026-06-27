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
  connectedState: document.querySelector("#connectedState"),
  receivingState: document.querySelector("#receivingState"),
  audioState: document.querySelector("#audioState"),
  message: document.querySelector("#message"),
  wsMetric: document.querySelector("#wsMetric"),
  framesMetric: document.querySelector("#framesMetric"),
  bytesMetric: document.querySelector("#bytesMetric"),
  bufferMetric: document.querySelector("#bufferMetric"),
  underrunMetric: document.querySelector("#underrunMetric"),
  droppedMetric: document.querySelector("#droppedMetric"),
  contextMetric: document.querySelector("#contextMetric"),
  formatMetric: document.querySelector("#formatMetric"),
  clientsMetric: document.querySelector("#clientsMetric"),
};

const state = {
  devices: [],
  session: { connected: false, receiving: false },
  busy: false,
  socket: null,
  audioContext: null,
  workletNode: null,
  audio: {
    wsState: "disconnected",
    frames: 0,
    bytes: 0,
    bufferedSamples: 0,
    underruns: 0,
    droppedSamples: 0,
    format: "unknown",
  },
};

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

function renderAudio() {
  const running = state.audioContext?.state === "running";
  const audioOpen = state.socket || state.audioContext;

  setPill(elements.audioState, running ? "Audio running" : "Audio stopped", running ? "good" : "");
  elements.startAudioButton.disabled = state.busy || Boolean(audioOpen);
  elements.stopAudioButton.disabled = state.busy || !audioOpen;
  elements.wsMetric.textContent = state.audio.wsState;
  elements.framesMetric.textContent = state.audio.frames.toLocaleString();
  elements.bytesMetric.textContent = state.audio.bytes.toLocaleString();
  elements.bufferMetric.textContent = state.audio.bufferedSamples.toLocaleString();
  elements.underrunMetric.textContent = state.audio.underruns.toLocaleString();
  elements.droppedMetric.textContent = state.audio.droppedSamples.toLocaleString();
  elements.contextMetric.textContent = state.audioContext?.state || "closed";
  elements.formatMetric.textContent = state.audio.format;
}

function renderStats(stats) {
  if (stats?.stream) {
    elements.clientsMetric.textContent = String(stats.stream.active_clients);
  }
}

function render() {
  renderDevices();
  renderSession();
  renderAudio();
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
    bufferedSamples: 0,
    underruns: 0,
    droppedSamples: 0,
    format: "unknown",
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
    state.workletNode = new AudioWorkletNode(state.audioContext, "pcm-player", {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [2],
      processorOptions: {
        capacitySamples: Math.floor(state.audioContext.sampleRate * 4),
        startThresholdSamples: Math.floor(state.audioContext.sampleRate * 0.75),
        targetBufferedSamples: Math.floor(state.audioContext.sampleRate * 1.25),
      },
    });
    state.workletNode.port.onmessage = (event) => {
      if (event.data?.type !== "stats") return;
      state.audio.bufferedSamples = event.data.bufferedSamples || 0;
      state.audio.underruns = event.data.underruns || 0;
      state.audio.droppedSamples = event.data.droppedSamples || 0;
      renderAudio();
    };
    state.workletNode.connect(state.audioContext.destination);
  }

  if (state.audioContext.state !== "running") {
    await state.audioContext.resume();
  }
}

function pcmI16LeToFloat32(buffer) {
  const view = new DataView(buffer);
  const samples = new Float32Array(Math.floor(buffer.byteLength / 2));
  for (let index = 0; index < samples.length; index += 1) {
    samples[index] = view.getInt16(index * 2, true) / 32768;
  }
  return samples;
}

async function startAudio() {
  resetAudioStats();
  state.audio.wsState = "connecting";
  renderAudio();
  try {
    await ensureAudioContext(48000);
  } catch (error) {
    state.audio.wsState = "disconnected";
    renderAudio();
    throw error;
  }

  const socket = new WebSocket(websocketUrl("ws/audio").href);
  state.socket = socket;
  socket.binaryType = "arraybuffer";

  socket.addEventListener("open", () => {
    state.audio.wsState = "connected";
    renderAudio();
  });

  socket.addEventListener("message", async (event) => {
    try {
      if (typeof event.data === "string") {
        const metadata = JSON.parse(event.data);
        state.audio.format = `${metadata.format}, ${metadata.channels} ch, ${metadata.sample_rate_hz} Hz`;
        await ensureAudioContext(metadata.sample_rate_hz);
        renderAudio();
        return;
      }

      const samples = pcmI16LeToFloat32(event.data);
      state.audio.frames += 1;
      state.audio.bytes += event.data.byteLength;
      state.workletNode?.port.postMessage({ type: "samples", samples }, [samples.buffer]);
      renderAudio();
    } catch (error) {
      console.error(error);
      setMessage(error.message);
    }
  });

  socket.addEventListener("close", () => {
    state.audio.wsState = "disconnected";
    if (state.socket === socket) state.socket = null;
    renderAudio();
  });

  socket.addEventListener("error", () => {
    state.audio.wsState = "error";
    setMessage("WebSocket audio connection failed");
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

window.addEventListener("beforeunload", () => {
  if (state.socket && state.socket.readyState < WebSocket.CLOSING) {
    state.socket.close();
  }
});

setInterval(refreshStats, 2000);
await loadDevices();
