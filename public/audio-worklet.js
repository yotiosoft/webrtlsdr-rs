class PcmPlayerProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const capacity = Math.max(4096, options.processorOptions?.capacitySamples || Math.floor(sampleRate * 0.5));
    this.buffer = new Float32Array(capacity);
    this.capacity = capacity;
    this.readIndex = 0;
    this.writeIndex = 0;
    this.available = 0;
    this.underruns = 0;
    this.overflows = 0;
    this.droppedSamples = 0;
    this.receivedSamples = 0;
    this.playedSamples = 0;
    this.framesUntilStats = 0;
    this.startThresholdSamples = options.processorOptions?.startThresholdSamples || Math.floor(sampleRate * 0.2);
    this.targetBufferedSamples = options.processorOptions?.targetBufferedSamples || Math.floor(sampleRate * 0.35);
    this.lowWatermarkSamples = this.capacity;
    this.highWatermarkSamples = 0;
    this.playing = false;

    this.port.onmessage = (event) => {
      if (event.data?.type === "samples" && event.data.samples instanceof Float32Array) {
        this.push(event.data.samples);
      } else if (event.data?.type === "reset") {
        this.reset();
      }
    };
  }

  push(samples) {
    let incoming = samples;
    this.receivedSamples += incoming.length;

    if (incoming.length > this.capacity) {
      const drop = incoming.length - this.capacity;
      incoming = incoming.subarray(drop);
      this.droppedSamples += drop;
      this.overflows += 1;
    }

    const overflow = Math.max(0, this.available + incoming.length - this.capacity);
    if (overflow > 0) {
      const drop = Math.min(this.available, overflow);
      this.readIndex = (this.readIndex + drop) % this.capacity;
      this.available -= drop;
      this.droppedSamples += drop;
      this.overflows += 1;
    }

    for (let index = 0; index < incoming.length; index += 1) {
      this.buffer[this.writeIndex] = incoming[index];
      this.writeIndex = (this.writeIndex + 1) % this.capacity;
    }
    this.available += incoming.length;
    if (this.available < this.lowWatermarkSamples) this.lowWatermarkSamples = this.available;
    if (this.available > this.highWatermarkSamples) this.highWatermarkSamples = this.available;
  }

  reset() {
    this.readIndex = 0;
    this.writeIndex = 0;
    this.available = 0;
    this.underruns = 0;
    this.overflows = 0;
    this.droppedSamples = 0;
    this.receivedSamples = 0;
    this.playedSamples = 0;
    this.lowWatermarkSamples = this.capacity;
    this.highWatermarkSamples = 0;
    this.playing = false;
  }

  process(_inputs, outputs) {
    const output = outputs[0];
    const frames = output[0]?.length || 0;

    for (let frame = 0; frame < frames; frame += 1) {
      let sample = 0;
      if (!this.playing && this.available >= this.startThresholdSamples) {
        this.playing = true;
      }

      if (this.playing && this.available > 0) {
        sample = this.buffer[this.readIndex];
        this.readIndex = (this.readIndex + 1) % this.capacity;
        this.available -= 1;
        this.playedSamples += 1;
        if (this.available < this.lowWatermarkSamples) this.lowWatermarkSamples = this.available;
      } else {
        if (this.playing) {
          this.underruns += 1;
          this.playing = false;
        }
      }

      for (let channel = 0; channel < output.length; channel += 1) {
        output[channel][frame] = sample;
      }
    }

    this.framesUntilStats -= frames;
    if (this.framesUntilStats <= 0) {
      this.framesUntilStats = Math.floor(sampleRate / 2);
      this.port.postMessage({
        type: "stats",
        bufferedSamples: this.available,
        capacitySamples: this.capacity,
        initialBufferSamples: this.startThresholdSamples,
        targetBufferSamples: this.targetBufferedSamples,
        underruns: this.underruns,
        overflows: this.overflows,
        droppedSamples: this.droppedSamples,
        receivedSamples: this.receivedSamples,
        playedSamples: this.playedSamples,
        lowWatermarkSamples: this.lowWatermarkSamples === this.capacity ? this.available : this.lowWatermarkSamples,
        highWatermarkSamples: this.highWatermarkSamples,
        playing: this.playing,
      });
    }

    return true;
  }
}

registerProcessor("pcm-player", PcmPlayerProcessor);
