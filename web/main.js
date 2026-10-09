// The page around the wasm build: pick a disc image, read it here, hand its
// bytes to the game. Nothing is sent anywhere. See docs/tools/web.md.
import init, { start } from "./pkg/oag_web.js";

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

let booted = false;
async function boot(file) {
  if (booted) return;
  if (!navigator.gpu) {
    say("This browser has no WebGPU, which the game draws with.");
    return;
  }
  booted = true;
  try {
    say(`Reading ${file.name} (${(file.size / 1e6).toFixed(0)} MB)...`);
    const bytes = new Uint8Array(await file.arrayBuffer());
    say("Starting...");
    await init();
    picker.hidden = true;
    canvas.focus();
    // `?log=debug` (or `trace`) widens what reaches the console.
    const log = new URLSearchParams(location.search).get("log") ?? undefined;
    await start(file.name, bytes, log);
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
