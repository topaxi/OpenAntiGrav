// Sound on the page: the AudioContext and the worklet that plays the game's
// mix (audio-worklet.js), which the module asks for through `oagAudioOpen`
// and `oagAudioConnect` (oag_audio::output::web). See docs/tools/web.md,
// "Sound".
//
// `?audio=off` keeps the game silent (no context at all). `?audiotap=SECONDS`
// records that long of exactly what the worklet hands the browser, from its
// first quantum, for `oagAudioTapTake()` (scripts/web-screenshot.py
// --audio-wav) to collect.

const params = new URLSearchParams(location.search);
const off = params.get("audio") === "off";
const tapSeconds = Number(params.get("audiotap") || 0);
let context = null;

// The game's mix is rendered at the context's own rate, so the module reads
// it from here before it builds its mixer. 0 means no sound.
window.oagAudioOpen = () => {
  if (off || typeof AudioContext === "undefined") return 0;
  context = new AudioContext({ latencyHint: "interactive" });
  // Allowed at once where the page already had a click (the picker's); a
  // browser that wants another gesture gets the next key press or click.
  context.resume().catch(() => {});
  return context.sampleRate;
};

window.oagAudioConnect = (memory, header, samples, capacity) => {
  const tapSamples = Math.round(tapSeconds * context.sampleRate) * 2;
  context.audioWorklet.addModule(new URL("./audio-worklet.js", import.meta.url)).then(() => {
    const node = new AudioWorkletNode(context, "oag-ring", {
      numberOfInputs: 0,
      outputChannelCount: [2],
      processorOptions: { memory, header, samples, capacity, tapSamples },
    });
    node.connect(context.destination);
    if (tapSamples > 0) collect(node);
    console.log(`audio: worklet started, ${context.sampleRate} Hz, context ${context.state}`);
  }).catch((why) => console.error(`audio: the worklet would not start (${why}); running silent`));
};

// The browser starts a context suspended until a gesture; every key press or
// click until it runs is one.
const wake = () => {
  if (context?.state === "suspended" && !document.hidden) context.resume().catch(() => {});
};
for (const type of ["keydown", "pointerdown", "touchend"]) window.addEventListener(type, wake, true);

// A hidden tab is silent and costs nothing: the context stops pulling, the
// ring stays full and the render worker idles until it is shown again.
document.addEventListener("visibilitychange", () => {
  if (!context) return;
  (document.hidden ? context.suspend() : context.resume()).catch(() => {});
});
window.oagAudioSetPaused = (paused) => {
  if (context) (paused ? context.suspend() : context.resume()).catch(() => {});
};

const tap = { chunks: [], done: false, underruns: 0, quanta: 0 };
function collect(node) {
  node.port.onmessage = ({ data }) => {
    tap.chunks.push(data.chunk);
    tap.done = data.done;
    tap.underruns = data.underruns;
    tap.quanta = data.quanta;
  };
}

// The recording so far, interleaved stereo f32 as base64, with the worklet's
// counters and the context's state.
window.oagAudioTapTake = () => {
  const length = tap.chunks.reduce((n, c) => n + c.length, 0);
  const all = new Float32Array(length);
  let at = 0;
  for (const chunk of tap.chunks) { all.set(chunk, at); at += chunk.length; }
  const bytes = new Uint8Array(all.buffer);
  let text = "";
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return {
    rate: context?.sampleRate ?? 0,
    state: context?.state ?? "none",
    currentTime: context?.currentTime ?? 0,
    done: tap.done,
    underruns: tap.underruns,
    quanta: tap.quanta,
    samples: btoa(text),
  };
};
