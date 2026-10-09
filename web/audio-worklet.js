// The browser's audio thread: copies the game's mix out of a ring in the
// module's shared memory, which a worker keeps filled (oag_audio::output::web,
// the ring's layout in crates/audio/src/output/shared_ring.rs). No wasm runs
// here and nothing here waits: what the ring lacks plays as silence, and the
// quantum is counted. See docs/tools/web.md, "Sound".

// Header words (shared_ring.rs, `word`).
const READ = 0, WRITE = 1, UNDERRUNS = 2, QUANTA = 3;
// Frames a gap ramps down over from the last frame played, so a starved
// quantum is a short fade and not a click (oag_audio's DECLICK_FRAMES).
const DECLICK_FRAMES = 64;
// Frames of output the debug tap posts to the page at a time.
const TAP_CHUNK = 8192;

class OagRing extends AudioWorkletProcessor {
  constructor({ processorOptions: { memory, header, samples, capacity, tapSamples } }) {
    super();
    // Shared memory never moves, so views made now stay valid when the
    // module's memory grows: the ring was allocated before this started.
    this.words = new Int32Array(memory.buffer, header, 4);
    this.samples = new Float32Array(memory.buffer, samples, capacity);
    this.mask = capacity - 1;
    this.tail = [0, 0];
    // `?audiotap=SECONDS`: exactly what this hands the browser, posted to the
    // page, for checking a run without speakers.
    this.tapLeft = tapSamples;
    this.tap = tapSamples > 0 ? new Float32Array(TAP_CHUNK * 2) : null;
    this.tapAt = 0;
  }

  process(_inputs, outputs) {
    const left = outputs[0][0];
    const right = outputs[0][1] ?? left;
    const frames = left.length;
    const words = this.words;
    const read = Atomics.load(words, READ);
    const write = Atomics.load(words, WRITE);
    const have = Math.min(frames, ((write - read) >>> 0) >>> 1);
    const samples = this.samples, mask = this.mask;
    let at = read;
    for (let f = 0; f < have; f++) {
      left[f] = samples[at & mask];
      right[f] = samples[(at + 1) & mask];
      at += 2;
    }
    if (have > 0) this.tail = [left[have - 1], right[have - 1]];
    if (have < frames) {
      const ramp = Math.min(frames - have, DECLICK_FRAMES);
      for (let f = have; f < frames; f++) {
        const gain = f - have < ramp ? 1 - (f - have + 1) / ramp : 0;
        left[f] = this.tail[0] * gain;
        right[f] = this.tail[1] * gain;
      }
      this.tail = [0, 0];
      // A stream that has not produced its first sample is starting, not
      // starving.
      if (write !== 0) Atomics.add(words, UNDERRUNS, 1);
    }
    Atomics.add(words, QUANTA, 1);
    Atomics.store(words, READ, (read + have * 2) | 0);
    if (this.tap) this.record(left, right, frames);
    return true;
  }

  record(left, right, frames) {
    for (let f = 0; f < frames && this.tapLeft > 0; f++) {
      this.tap[this.tapAt++] = left[f];
      this.tap[this.tapAt++] = right[f];
      this.tapLeft -= 2;
      if (this.tapAt === this.tap.length || this.tapLeft === 0) {
        const chunk = this.tap.slice(0, this.tapAt);
        this.port.postMessage({
          chunk,
          done: this.tapLeft === 0,
          underruns: Atomics.load(this.words, UNDERRUNS),
          quanta: Atomics.load(this.words, QUANTA),
        }, [chunk.buffer]);
        this.tapAt = 0;
      }
    }
    if (this.tapLeft === 0) this.tap = null;
  }
}

registerProcessor("oag-ring", OagRing);
