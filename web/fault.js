// What the player sees when the game cannot go on, and what stops the module
// once it cannot be trusted. One funnel for every failure class: a Rust panic,
// a wasm trap, a GPU error, a missing browser feature, a module that would not
// load, a disc image that would not open. Plain DOM and no wasm, because the
// module may be dead or never have loaded. See docs/tools/web.md,
// "Troubleshooting".
//
// The Rust side reaches this through `window.oagFatal(kind, detail)` and
// `window.oagNotice(kind, detail)` (`gpu::web::fatal`, `notice`; the panic
// hook), and a worker through a message `main.js` forwards. Every `kind` the
// module can send has a row in KINDS (`crates/game/tests/web_fault_kinds.rs`).

const GLUE_HASH = (/\/pkg\/([0-9a-f]{16})\//.exec(import.meta.url) ?? [])[1];

const REPO = "https://github.com/topaxi/OpenAntiGrav";
const RELOAD = "Reload the page and pick the disc image again.";
const REPORT = `If it keeps happening, press "Copy details" and report it at ${REPO}/issues.`;

// The browser, as far as advice differs by it.
function browser() {
  const ua = navigator.userAgent;
  const linux = /Linux/.test(ua) && !/Android/.test(ua);
  if (/Firefox\//.test(ua)) return { name: "Firefox", linux };
  if (/Edg\//.test(ua)) return { name: "Edge", linux };
  if (/Chrome\/|Chromium\//.test(ua)) return { name: "Chrome", linux };
  if (/Safari\//.test(ua)) return { name: "Safari", linux };
  return { name: "this browser", linux };
}

// How to get WebGPU, by browser (docs/tools/web.md, "Browser support").
function webgpuAdvice() {
  const { name, linux } = browser();
  if (name === "Firefox") {
    return linux
      ? "In Firefox on Linux, open about:config and set dom.webgpu.enabled and "
        + "gfx.webgpu.ignore-blocklist to true, then restart Firefox."
      : "Firefox 141 or newer has WebGPU on Windows; elsewhere, open about:config and set "
        + "dom.webgpu.enabled and gfx.webgpu.ignore-blocklist to true.";
  }
  if (name === "Safari") return "Safari 26 or newer has WebGPU. Update Safari, or use Chrome or Edge.";
  if (linux) {
    return "Chrome and Chromium on Linux need to be started with "
      + "--enable-unsafe-webgpu --enable-features=Vulkan (or enable those in chrome://flags); "
      + "chrome://gpu should then say WebGPU is hardware accelerated.";
  }
  return "Use Chrome or Edge 113 or newer, Safari 26 or newer, or Firefox 141 or newer on Windows, "
    + "and update your graphics driver.";
}

// Every failure the page can show. A `fatal` row stops the page and offers a
// reload; the others leave the page as it is and can be dismissed (a caller may
// make a row fatal: a disc read that fails mid-race). `what` is one plain
// sentence, `next` what the player can do.
const KINDS = {
  "panic": {
    title: "OpenAntiGrav stopped",
    what: "The game hit a bug and stopped itself (a Rust panic). Your disc image was not changed.",
    next: () => [RELOAD, REPORT],
    fatal: true,
  },
  "trap": {
    title: "OpenAntiGrav stopped",
    what: "The browser stopped the game's code (a WebAssembly trap), and the page cannot go on.",
    next: () => [RELOAD, REPORT],
    fatal: true,
  },
  "out-of-memory": {
    title: "OpenAntiGrav ran out of memory",
    what: "The game used all the memory a browser tab may have (4 GiB at most, less on some browsers and phones).",
    next: () => ["Close other tabs and programs, then reload the page and pick the disc image again.",
      "A computer copes better than a phone. A PSP disc needs less than a PS3 one.", REPORT],
    fatal: true,
  },
  "gpu-out-of-memory": {
    title: "The graphics card ran out of memory",
    what: "The browser reports that the graphics card has no memory left for the game.",
    next: () => ["Close other tabs and programs that use the graphics card, then reload the page.", REPORT],
    fatal: true,
  },
  "gpu-internal": {
    title: "The browser's graphics layer failed",
    what: "WebGPU reported an internal error. That is the browser or the graphics driver, not your disc.",
    next: () => ["Reload the page. If it repeats, update the browser and the graphics driver.", REPORT],
    fatal: true,
  },
  "gpu-lost": {
    title: "The graphics card connection was lost",
    what: "The browser lost the graphics device (a driver reset, a GPU crash, or the card was switched).",
    next: () => ["Reload the page.", "If it keeps happening, update the graphics driver and the browser.", REPORT],
    fatal: true,
  },
  "gpu-validation": {
    title: "The graphics card rejected a drawing command",
    what: "WebGPU reported an invalid command. The game keeps running, but part of the picture may be missing or wrong.",
    next: () => ["If the screen is black or garbled, reload the page.", REPORT],
    fatal: false,
  },
  "no-webgpu": {
    title: "This browser has no WebGPU",
    what: "The game draws with WebGPU, and this browser does not offer it.",
    next: () => [webgpuAdvice()],
    fatal: true,
  },
  "no-adapter": {
    title: "The browser gave the game no graphics card",
    what: "This browser has WebGPU, but it found no graphics adapter to give the game (the card or driver is on its blocklist, or hardware acceleration is off).",
    next: () => [webgpuAdvice(), "Make sure hardware acceleration is on in the browser's settings."],
    fatal: true,
  },
  "device-request": {
    title: "The graphics card refused the game",
    what: "The graphics adapter would not give the game a device with the features and limits it needs.",
    next: () => ["Update the browser and the graphics driver, or try another browser.", REPORT],
    fatal: true,
  },
  "no-isolation": {
    title: "This page is missing its security headers",
    what: "The game runs threads, which a browser allows only on a page served with the Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy headers. This copy of the page came without them.",
    next: () => ["If you opened a file from disk or a plain web server, use the hosted page, or serve this folder with scripts/serve-web.py.",
      "An in-app browser (inside a chat or social app) or an extension that rewrites headers can strip them: open the page in Chrome, Edge, Firefox or Safari directly."],
    fatal: true,
  },
  "module-load": {
    title: "The game could not be downloaded",
    what: "The page could not fetch its program (the connection dropped, the address changed, or the file is missing).",
    next: () => ["Check your connection, then reload the page.",
      "If it still fails, the site may be mid-update: wait a few minutes and try again."],
    fatal: true,
  },
  "wasm-unsupported": {
    title: "This browser cannot run the game",
    what: "The browser refused to compile the game's WebAssembly (it lacks threads, shared memory or a newer instruction).",
    next: () => ["Update the browser, or use a current Chrome, Edge, Firefox or Safari on a computer.", REPORT],
    fatal: true,
  },
  "wasm-memory": {
    title: "The browser would not give the game memory",
    what: "The game asks for a large block of shared memory up front and the browser refused (the tab or device is short of memory).",
    next: () => ["Close other tabs and programs, then reload the page.",
      "On a phone or tablet, use a computer: the game needs more memory than most phone browsers give a tab."],
    fatal: true,
  },
  "disc-unsupported": {
    title: "That file is not a disc image the game can play",
    what: "The game opened the file but found no game it supports on it.",
    next: () => ["Pick a .chd or .iso of Wipeout Pulse, Pure or HD / Fury: the table on this page says which discs get how far."],
    fatal: false,
  },
  "disc-unreadable": {
    title: "The disc image could not be read",
    what: "The file is cut short or damaged, or the browser lost access to it while the game was reading.",
    next: () => ["Check the file's size is the one the dump or download should have, and copy it to your computer's own disk if it sits on a removable or network drive.",
      "Then pick it again."],
    fatal: false,
  },
  "startup": {
    title: "The game could not start",
    what: "Something went wrong before the first frame, and the browser gave no clearer reason.",
    next: () => [RELOAD, REPORT],
    fatal: true,
  },
};

const state = {
  // The module cannot be trusted any more: nothing may enter it again.
  dead: false,
  // The first report is the cause; later ones are usually fallout.
  shown: false,
  suppressed: 0,
  adapter: null,
  software: false,
  image: null,
  memory: null,
  // Notices (non-fatal rows) already shown: a repeat is not shown again.
  noticed: new Set(),
};
// `--max-memory` in scripts/build-web.sh, in MiB.
const MEMORY_CEILING = 4096;
const workers = new Set();

const isTrap = (error) => error instanceof WebAssembly.RuntimeError
  || (error instanceof RangeError && /call stack|out of memory/i.test(error.message));

// Wraps the instance's exports so that once the module has trapped, no later
// callback (a frame, a key press, a resize, a promise) can enter it. After a
// trap the stack pointer is wherever the trap left it, and every later call
// traps too: hundreds of errors and a frozen canvas (docs/tools/web.md,
// "The page's stack"). The wrapper is the one place every entry passes.
const realInstantiate = WebAssembly.instantiate.bind(WebAssembly);
const realStreaming = WebAssembly.instantiateStreaming?.bind(WebAssembly);

function guarded(name, fn) {
  return (...args) => {
    if (state.dead) return undefined;
    try {
      return fn(...args);
    } catch (error) {
      if (isTrap(error)) died(error);
      throw error;
    }
  };
}

function guard(result, imports) {
  if (!result?.instance?.exports) return result;
  for (const group of Object.values(imports ?? {})) {
    if (group?.memory instanceof WebAssembly.Memory) {
      state.memory = group.memory;
    }
  }
  const exports = {};
  for (const [name, value] of Object.entries(result.instance.exports)) {
    exports[name] = typeof value === "function" ? guarded(name, value) : value;
  }
  return { module: result.module, instance: { exports } };
}

WebAssembly.instantiate = async (source, imports) => guard(await realInstantiate(source, imports), imports);
if (realStreaming) {
  WebAssembly.instantiateStreaming = async (source, imports) => guard(await realStreaming(source, imports), imports);
}

function memoryMiB() {
  try { return state.memory ? Math.round(state.memory.buffer.byteLength / 1048576) : null; } catch { return null; }
}

// The module is gone: nothing enters it again, its threads end and its sound
// stops.
function stop() {
  state.dead = true;
  for (const worker of workers) worker.terminate();
  workers.clear();
  window.oagAudioClose?.();
}

// A trap the page saw: reports it once, as memory when the module sits at its
// ceiling.
function died(error) {
  if (state.dead) return;
  stop();
  // A stack overflow and a memory limit both look like an out-of-bounds trap
  // or an `unreachable` after an allocation failure: near the ceiling it is
  // memory, whatever the trap said.
  const used = memoryMiB();
  const nearCeiling = used !== null && used >= MEMORY_CEILING * 0.97;
  report(nearCeiling ? "out-of-memory" : "trap", `${error.name}: ${error.message}\n${error.stack ?? ""}`);
}

const el = (tag, text, className) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
};

const STYLE = `
#oag-fault{position:fixed;inset:0;z-index:100;display:flex;align-items:center;justify-content:center;
  background:#000c;padding:1rem;box-sizing:border-box;font:16px/1.5 system-ui,sans-serif;color:#eee}
#oag-fault .card{background:#101418;border:1px solid #4af;border-radius:8px;max-width:38rem;width:100%;
  max-height:100%;overflow:auto;padding:1.4rem 1.6rem;box-sizing:border-box}
#oag-fault h2{margin:0 0 .6rem;font-size:1.3em}
#oag-fault p,#oag-fault ul{margin:.5rem 0}
#oag-fault ul{padding-left:1.2rem}
#oag-fault .buttons{display:flex;flex-wrap:wrap;gap:.5rem;margin:1rem 0 .4rem}
#oag-fault button{min-height:44px;padding:.5rem 1.1rem;border:1px solid #4af;border-radius:4px;
  background:#123;color:#def;font:inherit;cursor:pointer}
#oag-fault button:hover{background:#246}
#oag-fault button:focus-visible,#oag-fault summary:focus-visible,#oag-fault textarea:focus-visible{
  outline:2px solid #fc6;outline-offset:2px}
#oag-fault summary{cursor:pointer;color:#8cf;margin-top:.6rem}
#oag-fault textarea{width:100%;box-sizing:border-box;height:12rem;margin-top:.4rem;background:#000;color:#cde;
  border:1px solid #345;border-radius:4px;font:12px/1.4 ui-monospace,monospace}
#oag-fault .flags{color:#fc6}
`;

function installStyle() {
  if (document.getElementById("oag-fault-style")) return;
  const style = el("style", STYLE);
  style.id = "oag-fault-style";
  document.head.append(style);
}

// The text a bug report wants: no disc path, no key, no storage. The file's
// own name is the only thing from the player's machine.
function detailsText(kind, detail) {
  const used = memoryMiB();
  return [
    "OpenAntiGrav web error report",
    `kind: ${kind}`,
    `build: ${GLUE_HASH ?? "unknown (not a hashed build)"}`,
    `page: ${location.origin}${location.pathname}`,
    `browser: ${navigator.userAgent}`,
    `cross-origin isolated: ${window.crossOriginIsolated}`,
    `webgpu: ${"gpu" in navigator ? "offered" : "absent"}`,
    `adapter: ${state.adapter ?? "none reported"}${state.software ? " (software)" : ""}`,
    `memory: ${used === null ? "unknown" : `${used} MiB of ${MEMORY_CEILING} MiB`}`,
    `image: ${state.image ?? "none picked"}`,
    state.suppressed ? `further errors hidden after this one: ${state.suppressed}` : null,
    "",
    String(detail ?? "").slice(0, 6000),
  ].filter((line) => line !== null).join("\n");
}

async function copyText(text, area) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch { /* no clipboard permission or insecure context: select it instead */ }
  area.select();
  try { return document.execCommand("copy"); } catch { return false; }
}

function removeOverlay() {
  document.getElementById("oag-fault")?.remove();
  for (const id of ["picker", "oag-canvas"]) {
    const node = document.getElementById(id);
    if (node) node.inert = false;
  }
}

// Shows one report. A second call while one is up keeps the first (and counts
// the rest), because everything after the first failure is usually fallout.
function report(kind, detail, fatal) {
  if (state.shown) {
    state.suppressed += 1;
    return;
  }
  const row = { ...(KINDS[kind] ?? KINDS["startup"]) };
  const id = KINDS[kind] ? kind : "startup";
  if (fatal !== undefined) row.fatal = fatal;
  state.shown = true;
  installStyle();
  console.error(`oag: ${id}: ${String(detail ?? "").split("\n")[0]}`);
  const text = detailsText(id, detail);
  window.oagLastFault = { kind: id, fatal: row.fatal, text };

  const root = el("div");
  root.id = "oag-fault";
  root.dataset.kind = id;
  root.dataset.fatal = String(row.fatal);
  const card = el("div", undefined, "card");
  card.setAttribute("role", "alertdialog");
  card.setAttribute("aria-modal", "true");
  const heading = el("h2", row.title);
  heading.id = "oag-fault-title";
  const what = el("p", row.what);
  what.id = "oag-fault-what";
  card.setAttribute("aria-labelledby", heading.id);
  card.setAttribute("aria-describedby", what.id);
  card.append(heading, what);
  const steps = row.next();
  if (steps.length === 1) card.append(el("p", steps[0]));
  else {
    const list = el("ul");
    for (const step of steps) list.append(el("li", step));
    card.append(list);
  }
  if (state.software && row.fatal) {
    card.append(el("p", "The browser gave WebGPU a software (CPU) adapter, which is slow and fails on "
      + "things a graphics card does not. On Linux, start Chrome with "
      + "--enable-unsafe-webgpu --enable-features=Vulkan and check chrome://gpu.", "flags"));
  }

  const buttons = el("div", undefined, "buttons");
  const first = el("button", row.fatal ? "Reload the page" : "Back to the picker");
  first.type = "button";
  first.addEventListener("click", () => { if (row.fatal) location.reload(); else dismiss(); });
  const copy = el("button", "Copy details");
  copy.type = "button";
  buttons.append(first, copy);
  if (!row.fatal && kind === "gpu-validation") first.textContent = "Keep playing";
  card.append(buttons);

  const more = el("details");
  const summary = el("summary", "Technical details");
  const area = el("textarea");
  area.readOnly = true;
  area.value = text;
  area.setAttribute("aria-label", "Technical details for a bug report");
  more.append(summary, area);
  card.append(more);
  copy.addEventListener("click", async () => {
    more.open = true;
    const done = await copyText(text, area);
    copy.textContent = done ? "Copied" : "Select the text and copy it";
    area.select();
  });
  card.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !row.fatal) dismiss();
  });
  root.append(card);

  for (const id of ["picker", "oag-canvas"]) {
    const node = document.getElementById(id);
    if (node) node.inert = true;
  }
  document.getElementById("oag-canvas")?.blur();
  document.body.append(root);
  first.focus();
}

// Takes the report down so the page is usable again (the non-fatal rows).
function dismiss() {
  removeOverlay();
  state.shown = false;
  window.oagLastFault = undefined;
}

window.oagDismissFault = dismiss;

window.oagFatal = (kind, detail) => {
  if (!state.dead) stop();
  report(kind, detail, true);
};
window.oagNotice = (kind, detail) => {
  if (state.noticed.has(kind)) return;
  state.noticed.add(kind);
  report(kind, detail, false);
};
window.oagReport = report;
window.oagAdapter = (info, software) => {
  state.adapter = info;
  state.software = Boolean(software);
};
window.oagImageName = (name) => { state.image = name; };

// A trap that reached the page without passing a wrapped export, and every
// error after the first (hidden, and counted for the details).
window.addEventListener("error", (event) => {
  if (state.dead) {
    event.preventDefault();
    state.suppressed += 1;
    return;
  }
  if (isTrap(event.error)) {
    event.preventDefault();
    died(event.error);
  }
}, true);
window.addEventListener("unhandledrejection", (event) => {
  if (state.dead) {
    event.preventDefault();
    state.suppressed += 1;
  } else if (isTrap(event.reason)) {
    event.preventDefault();
    died(event.reason);
  }
});

// Keeps a worker so the page can end it with the module. A worker's panic or
// trap arrives as `{ fatal: { kind, detail } }`.
export function watchWorker(worker) {
  workers.add(worker);
  worker.addEventListener("message", ({ data }) => {
    if (data?.fatal) window.oagFatal(data.fatal.kind, data.fatal.detail);
  });
  worker.addEventListener("error", (event) => {
    event.preventDefault();
    window.oagFatal("trap", `a worker stopped: ${event.message}`);
  });
}

// What a rejected `init()`, `inspect()` or `start()` means. The module tags its
// own refusals with `kind`; a browser's are told apart by name.
export function kindOfFailure(error) {
  if (error?.kind && KINDS[error.kind]) return error.kind;
  const name = error?.name ?? "";
  const message = String(error?.message ?? error);
  if (error instanceof WebAssembly.CompileError || error instanceof WebAssembly.LinkError) return "wasm-unsupported";
  if (error instanceof RangeError && /memory/i.test(message)) return "wasm-memory";
  if (name === "TypeError" && /fetch|network|load failed/i.test(message)) return "module-load";
  if (/failed to fetch Wasm/i.test(message)) return "module-load";
  return "startup";
}

export const failureText = (error) =>
  `${error?.name ?? "Error"}: ${error?.message ?? error}\n${error?.stack ?? ""}`;

export const isDead = () => state.dead;

// Whether this browser can run the game at all, before anything is fetched.
// Returns the kind that stops it, or null.
export function preflight() {
  if (!("gpu" in navigator) || !navigator.gpu) return "no-webgpu";
  if (!window.crossOriginIsolated || typeof SharedArrayBuffer === "undefined") return "no-isolation";
  return null;
}
