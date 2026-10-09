// The page around the wasm build: pick a disc image, read it here, hand it to
// the game. Nothing is sent anywhere. See docs/tools/web.md.
import init, { start } from "./pkg/oag_web.js";
// Sound: the AudioContext and its worklet (audio.js).
import "./audio.js";

// The same glue again, for the workers `oagSpawnWorker` starts. build-web.sh
// rewrites both names to the content-hashed directory.
const GLUE = new URL("./pkg/oag_web.js", import.meta.url).href;

const picker = document.getElementById("picker");
const status = document.getElementById("status");
const fileInput = document.getElementById("file");
const openHandle = document.getElementById("open-handle");
const resume = document.getElementById("resume");
const canvas = document.getElementById("oag-canvas");

const say = (text) => { status.textContent = text; };

// The File System Access API is Chromium-only: an optional extra that
// remembers the picked file across reloads, never the only way in.
const DB = "oag-web", STORE = "handles", KEY = "image";
const canRemember = "showOpenFilePicker" in window && "indexedDB" in window;

function db() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function remembered(handle) {
  const store = (await db()).transaction(STORE, handle ? "readwrite" : "readonly").objectStore(STORE);
  return new Promise((resolve, reject) => {
    const request = handle ? store.put(handle, KEY) : store.get(KEY);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

// A panic or a lost GPU device would otherwise leave a white canvas. The wasm
// side calls this (`gpu::web::fatal`); the first cause stays, since later ones
// are usually fallout. Added to the page here so the HTML stays plain.
let fatalShown = false;
window.oagFatal = (cause, software) => {
  if (fatalShown) return;
  fatalShown = true;
  const box = document.createElement("div");
  box.setAttribute("role", "alert");
  box.style.cssText = "position:fixed;inset:0;z-index:10;overflow:auto;padding:2rem;"
    + "background:#101418;color:#eee;font:16px/1.5 system-ui,sans-serif;";
  const add = (tag, text) => {
    const el = document.createElement(tag);
    el.textContent = text;
    box.append(el);
    return el;
  };
  add("h2", "OpenAntiGrav stopped");
  add("p", "The game hit an error and cannot go on. Reloading the page starts it again.");
  if (software) {
    add("p", "Your browser gave WebGPU a software (CPU) adapter, which is slow and fails on "
      + "things a graphics card does not. On Linux, start Chrome with "
      + "--enable-unsafe-webgpu --enable-features=Vulkan (or enable those in chrome://flags), "
      + "and check chrome://gpu for the WebGPU line. Details: docs/tools/web.md, \"Troubleshooting\".");
  }
  add("pre", cause.split("\n").slice(0, 3).join("\n").slice(0, 700)).style.cssText = "white-space:pre-wrap;word-break:break-word;background:#1c232b;padding:1rem;";
  add("p", "The full text is in the browser console (F12).");
  document.body.append(box);
};
// The player quit. winit's web loop cannot be started a second time in one
// page and wasm memory never shrinks, so the page reloads and shows the picker
// again; a Chromium remembered handle then offers "Play <name> again". Settings
// and records are already in localStorage (written as they change, synchronously).
window.oagQuit = () => { setTimeout(() => location.reload(), 0); };
// The picked image, which every worker reads for itself (worker.js).
let imageFile = null;

// Threads (docs/tools/web.md, "Threads"): the module asks for a worker running
// `oag_worker_entry(id)` on its own shared memory. Called from Rust
// (`oag_thread::web::spawn`); throws if the browser refuses.
window.oagSpawnWorker = (module, memory, id) => {
  const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
  worker.onmessage = ({ data }) => {
    if (data?.fatal) window.oagFatal(data.fatal, false);
  };
  worker.onerror = (event) => {
    event.preventDefault();
    window.oagFatal(`a worker stopped: ${event.message}`, false);
  };
  worker.postMessage({ glue: GLUE, module, memory, id, file: imageFile });
};

// Reads `file` a slice at a time, synchronously, for the game's disc readers
// on this page's thread: a synchronous XMLHttpRequest on a blob URL of
// `file.slice(..)`. The game runs here and its readers cannot wait for a
// promise; a race load reads on a worker instead (worker.js). No Range header
// is involved, and no byte leaves the tab: a blob URL names memory this tab
// already holds.
// Firefox runs a nested event loop inside a synchronous request and delivers
// `pagehide` from it: a reload mid-read then reached winit's handler while a
// winit handler was still on the stack, and its runner panicked ("RefCell
// already borrowed", winit 0.30.13). Registered before the game starts, so
// it runs first, and it keeps the event from winit only while a read is on
// the stack: the page is going away, and settings are already saved. A race
// load reads on a worker and no longer hits this; menus still read here.
let readsInFlight = 0;
window.addEventListener("pagehide", (event) => {
  if (readsInFlight > 0) event.stopImmediatePropagation();
}, true);

function slicedReader(file) {
  return {
    size: file.size,
    read(offset, length) {
      const url = URL.createObjectURL(file.slice(offset, offset + length));
      readsInFlight++;
      try {
        const request = new XMLHttpRequest();
        request.open("GET", url, false);
        // Bytes as one char each (U+0000-00FF or U+F780-F7FF): the only binary
        // response a synchronous request on a page may have.
        request.overrideMimeType("text/plain; charset=x-user-defined");
        request.send();
        if (request.status !== 200) throw new Error(`read at ${offset}: status ${request.status}`);
        const text = request.responseText;
        const bytes = new Uint8Array(text.length);
        for (let i = 0; i < text.length; i++) bytes[i] = text.charCodeAt(i) & 0xff;
        return bytes;
      } finally {
        readsInFlight--;
        URL.revokeObjectURL(url);
      }
    },
  };
}

// Whether `slicedReader` returns the right bytes in this browser: one 64 KiB
// slice from the middle, against the same slice read the asynchronous way.
async function slicesWork(file) {
  try {
    const length = Math.min(file.size, 65536);
    const offset = Math.floor((file.size - length) / 2);
    const got = slicedReader(file).read(offset, length);
    const want = new Uint8Array(await file.slice(offset, offset + length).arrayBuffer());
    return got.length === want.length && got.every((byte, i) => byte === want[i]);
  } catch (error) {
    console.warn("sliced reads unavailable:", error);
    return false;
  }
}

let booted = false;
async function boot(file) {
  if (booted) return;
  if (!navigator.gpu) {
    say("This browser has no WebGPU, which the game draws with.");
    return;
  }
  // The module's memory is shared between threads, which a page may only
  // create when its server sends COOP/COEP (web/_headers).
  if (!window.crossOriginIsolated) {
    say("This page was served without cross-origin isolation (COOP/COEP headers), "
      + "which the game's threads need. See docs/tools/web.md, \"Hosting\".");
    return;
  }
  booted = true;
  imageFile = file;
  try {
    const params = new URLSearchParams(location.search);
    let image;
    // `?read=memory` takes the fallback on purpose, to test it.
    if (params.get("read") !== "memory" && await slicesWork(file)) {
      image = slicedReader(file);
    } else {
      // Today's fallback: the whole file in memory, which wasm32 caps.
      console.warn("reading the whole image into memory instead of in slices");
      say(`Reading ${file.name} (${(file.size / 1e6).toFixed(0)} MB)...`);
      image = new Uint8Array(await file.arrayBuffer());
    }
    say("Starting...");
    await init();
    picker.hidden = true;
    canvas.focus();
    // `?log=debug` (or `trace`) widens what reaches the console.
    const log = params.get("log") ?? undefined;
    await start(file.name, image, log);
  } catch (error) {
    booted = false;
    picker.hidden = false;
    say(`Could not start: ${error}`);
    console.error(error);
  }
}

fileInput.addEventListener("change", () => {
  const file = fileInput.files?.[0];
  if (file) boot(file);
});

if (canRemember) {
  openHandle.hidden = false;
  openHandle.addEventListener("click", async () => {
    try {
      const [handle] = await window.showOpenFilePicker({
        types: [{ description: "Disc image", accept: { "application/octet-stream": [".chd", ".iso"] } }],
      });
      await remembered(handle).catch(() => {});
      boot(await handle.getFile());
    } catch (error) {
      if (error.name !== "AbortError") say(`${error}`);
    }
  });
  remembered().then((handle) => {
    if (!handle) return;
    resume.textContent = `Play ${handle.name} again`;
    resume.hidden = false;
    resume.addEventListener("click", async () => {
      // A remembered handle needs the player's say-so again after a reload.
      if ((await handle.queryPermission()) !== "granted"
        && (await handle.requestPermission()) !== "granted") {
        say("Permission to read the remembered image was not given.");
        return;
      }
      boot(await handle.getFile());
    });
  }).catch(() => {});
}

// For a test harness, which hands a File in without the dialog.
window.oagBoot = boot;
