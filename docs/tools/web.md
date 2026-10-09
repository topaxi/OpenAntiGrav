# The web build

`oag-game` in a browser tab: WebAssembly (`wasm32-unknown-unknown`) drawing
through WebGPU, with no launcher. The page asks for a disc image, reads it in the
tab, and boots it. It boots Wipeout Pulse (PSP, EU) to the front end, walks the
menus with the keyboard and the mouse, and races, in headless Chromium at 60 fps
(2026-10-09). Wipeout HD (a 2.2 GB decrypted PS3 ISO) races too, its circuit
loaded on a Web Worker while the loading screen draws (see "Threads"), in
headless Chromium and Firefox; Pulse PS2 (a 3.7 GB CHD) boots to Language
Selection, the image read a slice at a time. It has not been tried in a browser
on a real desktop, on a phone, or with a gamepad. "What is missing" lists the
rest.

```sh
just web            # target/web/dist: dist profile (fat LTO) + wasm-opt -O
just web --dev      # a debug build, no wasm-opt: quicker to iterate on
just web --serve    # then serve target/web/dist on http://127.0.0.1:8000/ with COOP/COEP
```

One-time setup: the pinned nightly the threaded module is built with
(`rustup toolchain install nightly-2026-10-08 --component rust-src --target
wasm32-unknown-unknown`; `OAG_WEB_TOOLCHAIN` overrides the name, and the
script says which it wants), the `wasm-bindgen` CLI at the exact version
`Cargo.lock` pins (`cargo install wasm-bindgen-cli --version <v>`; the script
says which), and `wasm-opt` from binaryen (any recent release; 130 is what CI
pins). The rest of the workspace stays on the pinned stable toolchain; the web
build has a target directory of its own, `target/web-threads`, because its
flags rebuild `std` and every crate.

**The page needs cross-origin isolation.** The module's memory is shared
between threads, and a browser only creates shared memory on a page whose
server sends `Cross-Origin-Opener-Policy: same-origin` and
`Cross-Origin-Embedder-Policy: require-corp`. A plain `python3 -m http.server`
does not, and the page then says so instead of starting. `just web --serve`
runs `scripts/serve-web.py`, which sends both; `npx wrangler pages dev
target/web/dist` is the closer copy of the real host (it reads `_headers`).

The published folder is `index.html`, `main.js`, `worker.js`, `style.css`,
`pkg/<hash>/oag_web.js` (the wasm-bindgen glue, 105 KB),
`pkg/<hash>/oag_web_bg.wasm` (about 10 MB), the licence files and `_headers`
(Cloudflare's, see "Hosting"). `<hash>` is the first
16 hex digits of the SHA-256 of the glue and the module together, and
`build-web.sh` rewrites `main.js`'s glue path to name it, so the two files can
be cached forever and a deploy never pairs new glue with an old module. A `--dev`
module is 20 MB; Cloudflare Pages refuses any file over 25 MiB, so only the
`dist` build is meant for publishing. `scripts/build-web.sh` ends by running
`scripts/check-leakage.py --dir` over it, the same audit every other release
artifact gets. **It holds no game content.** The image the player picks is read
by their own browser and is never uploaded: the page has no server side, and it
makes no request after loading itself.

## How it is wired

- **Entry.** `crates/game/src/web.rs` is the `oag_web` example
  (`crate-type = ["cdylib"]`, `required-features = ["web"]`) and `include!`s
  `main_body.rs`, the same arrangement as the Android library
  ([android.md](android.md)): the windowed `App` lives in the binary's own
  modules, and nothing may depend on `oag-game`. It exports one function,
  `start(name, image, log)`, which the page calls with the picked file.
- **The image.** `oag-disc` opens an image through `oag_disc::mount`: on
  native that is `std::fs::File` and nothing changed; on wasm it is a cursor
  over a `Blob` the entry point registered under `/web/<file name>`, which is
  the path the run is then told. The CHD and raw-ISO readers, and the container
  sniff, go through it; the rest of the engine keeps its path-based shape. A
  `Blob` is any synchronous random-access reader. The page hands `start` one of
  two: a reader that fetches slices of the picked `File` on demand
  (`web_image::Sliced`, below), or, as a fallback, the whole file as a
  `Uint8Array` copied into the module's memory.
- **Reading in slices.** The disc readers are synchronous, and on the
  page's thread, which may not wait for a promise, `File` only reads
  asynchronously. What reads it synchronously is a synchronous
  `XMLHttpRequest` on a blob URL of `file.slice(offset, offset + length)`
  (`slicedReader` in `web/main.js`), with the response as text in the
  `x-user-defined` charset, one char per byte: the only binary response a
  synchronous request on a page may have. No `Range` header is involved
  (support for it on blob URLs varies), no cross-origin isolation is needed,
  and no byte leaves the tab: a blob URL names memory the tab already holds.
  `web_image::Sliced` in `crates/game` puts a cache in front of it, 64 blocks
  of 1 MiB, least recently used dropped first (chosen, not measured), and a
  read of 4 MiB or more is fetched whole and bypasses it. The page probes it
  once before boot: a 64 KiB slice from the middle of the file, compared byte
  for byte with the same slice read the asynchronous way; on a mismatch or an
  exception it falls back to the whole file in memory, logs that at warn, and
  boots anyway. `?read=memory` on the page's URL takes the fallback on
  purpose. A synchronous request on a page's main thread is a deprecated API
  (browsers warn, none has removed it); if one does, the probe's exception
  drops that browser back to the in-memory path rather than breaking it.

  **A worker reads differently.** A race load runs on a Web Worker ("Threads"),
  where the page's reader object does not exist: `web/worker.js` gets the
  picked `File` with its start message and puts its own reader at
  `oagImageReader`, `FileReaderSync.readAsArrayBuffer` on a slice, which a
  worker may call and which hands the bytes over as they are. Each thread finds
  its realm's reader on its first read; the block cache is shared. The page's
  thread only ever `try_lock`s it and reads past it while a worker holds it
  (a contended lock would trap there, below).

  Measured in headless browsers on the HD ISO, 64 reads of 1 MiB spread over
  the file (2026-10-09): Chromium 153 57 MB/s warm (34 cold), Firefox 155
  350 MB/s, WebKit 26.6 287 MB/s, all byte-identical with the asynchronous
  read. `?log=debug` logs every fetch (`web: image fetch at ...`). Pulse PSP,
  from the pick to a race on Moa Therma White, made 84 fetches, 8 of them in
  the race load and none once it finished; the race ran at the 60 fps cap, and
  its load reached its last stage after 1,328 ms against 989 ms held in memory
  (a debug build). HD to its front end: 131 fetches, 131 MiB.

- **The main thread, not a Web Worker.** The game itself stays on the page's
  thread because winit cannot run in a worker: its web event loop calls
  `web_sys::window()` and panics without one (`only callable from inside the
  Window`, winit 0.30.13), and `WindowAttributesExtWebSys::with_canvas` takes an
  `HtmlCanvasElement`, never an `OffscreenCanvas`. Running in a worker means
  driving `App` without winit. The race load moved to a worker instead
  ("Threads"), which is what keeps the page responsive while a race loads.
- **One worker, for the race load.** The race load
  (`oag_raceplay::LoadWorker`) runs on a Web Worker; see "Threads". The other
  places the desktop build moves work onto a thread still run it inline on
  wasm, behind `cfg(target_arch = "wasm32")`: the race scene build
  (`race_build::BuildWorker`, which is GPU work and wgpu's web types belong to
  the page's thread), dropping a parked race (GPU resources again), and the
  boot's media (`boot::MediaWorker`; no movie decodes on the web, so it has
  nothing slow to move). A CHD read is serial when `available_parallelism`
  fails, which it does here. The circuit-length worker and the race music fetch
  use `std::thread`, which this target refuses even with atomics, so they fail
  to spawn and log it, as they would on any machine that refuses a thread.
- **Time.** `std::time::Instant::now` panics on this target, so the render-side
  crates (`oag-game`, `oag-raceplay`, `oag-present`) use `web_time::Instant`,
  which is `std::time::Instant` itself on native and `performance.now()` here.
  No simulation crate reads a clock at all.
- **The GPU.** WebGPU hands out its adapter and device through promises, and
  the page's thread may not block on one, so `start` awaits
  `gpu::web::prepare` (the same descriptor the desktop asks for) before the event
  loop exists and `Gpu::new` takes the result. The browser picks the adapter, so
  the RENDERER row offers nothing. `EventLoopExtWebSys::spawn_app` hands the loop
  to the browser, which drives every frame from `requestAnimationFrame`; the
  present mode is `Fifo`, the only one a canvas offers.
- **The canvas.** The page owns `<canvas id="oag-canvas">` and its stylesheet
  sizes it to the tab. The window attributes drop the settings' window size and
  fullscreen on wasm: winit's web backend writes a requested size into the
  canvas's CSS, which drew a 1920x1080 canvas cropped to the corner of a smaller
  tab, and browsers refuse a fullscreen request made outside a click.
- **Shaders.** Chrome compiles WGSL with Tint, which enforces WGSL's uniformity
  rules where naga (the desktop's compiler and `oag-shader-check`'s) lets some
  through. The mesh shadow term sampled with `textureSample` inside a
  non-uniform branch, which made every mesh pipeline invalid and every race
  frame black; it is `textureSampleLevel(.., 0.0)` now, the same texel on a
  one-mip map. Other shaders on paths not yet reached in a browser (HD, 2048,
  Omega, the post chain's other modes) may hide more of these: the browser's
  console names the line, and nothing in `just` would.
- **Logging.** The console, through a small `log` sink in `web.rs`: `warn` and
  above from everything, `info` from this project's own crates. `?log=debug` or
  `?log=trace` on the page's URL widens it, which is how the loader reports
  are read here. There is no log file.
- **Settings and records persist; ghosts do not.** `settings.toml` and
  `records.toml` (best laps, last results, campaign medals) are read and
  written through `oag_game::profile`, which is `std::fs` and
  `dirs::config_dir` on native and the page's `localStorage` on wasm: one item
  per file, keyed `oag:/config/oag/<file>`. `localStorage` is synchronous,
  which those callers' shape needs, and holds a few MiB per origin, far above
  the size of the two files. A language picked on one load is the language the
  next load boots in, Language Selection skipped (headless Chromium,
  2026-10-09). Records go through the same seam and were not exercised in a
  browser. Ghosts (binary files in a directory) and pilot files still hit
  `std::fs`, which returns `Unsupported` here, and each degrades as before:
  nothing kept between reloads. The storage is per origin, so a local serve and
  `oag.topaxi.com` keep separate profiles.
- **The page.** `web/` is plain HTML, CSS and one ES module, no bundler. A plain
  `<input type="file">` is the way in, in every browser. Where
  `showOpenFilePicker` exists (Chromium), a second button keeps the file's handle
  in IndexedDB, and the next visit offers "Play <name> again", which asks the
  browser for read permission again rather than re-picking. The page hides
  every element carrying `hidden` with `display: none !important`: an author
  `display` rule otherwise outranks the browser's own, which once drew that
  button empty beside the other two with nothing to replay.

### First-run defaults and the options pages

**Chosen, not measured.** With no `settings.toml` in `localStorage` (a first
visit), `oag_game::settings::web::first_run` starts light: frame limit 60,
anisotropy off, shadows off in every title's render profile (the type's own
default is already `off`; HD's profile does not override it). A saved profile,
the maintainer's included, keeps what it holds: a file written before this has
240 and 16x as canonical values and is never rewritten. Dynamic resolution is
not turned on.

The render target is held to **720 lines** on the web, always, whatever the
render-scale row says (`settings::web::render_target`, used by every
`target_size` caller, width scaled by the same factor): a 1920x1080 surface logs
`render target 1271x720 for a 1920x1080 surface`, and the race held 60.4 fps in
headless Chromium (2026-10-09). The surface is in physical pixels, so
`devicePixelRatio` is already inside the number it clamps.

The options pages drop VSYNC (a canvas only offers `Fifo`) and MONITOR
(`settings::web::HIDDEN_ROWS`, `Definition::drop_settings`). WINDOW MODE stays
as Windowed/Borderless, the second being the browser's Fullscreen API
(`oagFullscreen` in `web/main.js`; a menu press is the user gesture, checked in
Chromium). Escape or F11 leaves fullscreen without the game seeing the key, so
the page records `fullscreenchange` and the game polls it once a frame, sets the
mode back to Windowed, saves it and re-seeds the open row. WINDOW SIZE is
replaced on the web by CANVAS SIZE (`display.canvas_size`: `fit`, the default,
or a size in CSS pixels): a page has no window, and a size value that means
"fit" cannot live in `display.window_size`, which every desktop row shares. It is
dropped on a desktop. winit's own size request is not used (it cropped the
canvas); the page sets the canvas's CSS and the surface follows.

The language picked on first boot is saved the frame it is picked, not when the
menus open (`session/frame.rs`): a quit from the title screen, which reloads the
page, used to skip the save and ask again. Checked across a reload in headless
Chromium with English and with French.

### Quitting

QUIT (or escape at the root of the menus) ends the event loop, and before
this change that left the last frame frozen on the canvas with no way back
short of reloading by hand (headless Chromium, escape on the title screen:
the title still on screen 9.5 s later, the picker hidden). Now the quit path
calls the page's `oagQuit` (wasm only, `gpu::web::quit`, from
`about_to_wait`) and the page reloads itself, which shows the picker, and
with a remembered handle the one-click "Play <name> again".

A reload and not a second `start` in the same page, on purpose: winit's web
backend leaves the first loop's canvas listeners and its `spawn_app` state in
place with no way to tear them down, `oag_gpu::init_buffer`'s registered
queue and the device in `PREPARED` are one-per-page, and wasm linear memory
never shrinks, so a second run would sit on top of the first one's peak. The
reload costs nothing in saved state: settings and records are written to
`localStorage` synchronously as they change (`profile.rs`), so there is
nothing left to flush. Measured on a Pulse PSP image: boot, title, escape,
picker visible, pick the image again, the front end draws (a dev build, the
walk in `web-screenshot.py`'s style with a second pick). Native quit is
unchanged. Errors still go through `oagFatal`, never through a reload.

### Leaving mid-load

Reloading or closing the tab while a race loads used to panic in Firefox:
`winit-0.30.13 .../web/event_loop/runner.rs:683` "RefCell already borrowed".
The load then ran on the page's thread, its reads were synchronous requests,
and Firefox runs a nested event loop inside one that delivered `pagehide` to
winit while a winit handler was still on the stack. Reproduced in headless
Firefox 155 by reloading one second into an HD race load: 2 of 2 runs panicked
(2026-10-09, `web-screenshot.py --browser firefox --reload 51` on the walk in
"Verifying it").

Two things now close it. The race load reads on a worker, so the page's thread
is not inside a synchronous request while it loads (0 of 2 on the threaded
build). And `web/main.js` registers a capturing `pagehide` listener before the
game starts that stops the event reaching winit while a synchronous read is on
the page's stack, which the menus' and the boot's reads still are: with only
that listener and the old inline load, 0 of 2. Nothing is lost by winit missing
that one `pagehide`: the page is going away, and settings and records are
already in `localStorage`. A worker still running when the page goes is
terminated with it. Reloading during the boot's reads did not reproduce the
panic either way (0.3 s and 0.6 s after the pick, Firefox).

## Browser support

WebGPU is required; there is no WebGL fallback (`adapter::BACKENDS` is
`PRIMARY`, which includes the browser's WebGPU and excludes GL, for the reason
that constant's own documentation gives). The picker page lists every disc it knows, with the formats it accepts and how
far each gets; that table is checked against `docs/overview/status.md`,
section 9, by `crates/game/tests/web_picker_table.rs`. WebGPU ships in Chrome and Edge 113+
(Windows, macOS, ChromeOS; Android 121+), Safari 26, and Firefox 141+ on Windows.
Chromium on Linux still needs `--enable-unsafe-webgpu --enable-features=Vulkan`;
Firefox on Linux needs `dom.webgpu.enabled` and `gfx.webgpu.ignore-blocklist`
set in `about:config` (Firefox 155 then hands out a hardware adapter, headless
included). The page says so when `navigator.gpu` is missing rather than
failing later, and when the page is not cross-origin isolated (above), which
every one of these browsers supports.

## Verifying it

`scripts/web-screenshot.py` serves a built `dist` folder, opens it in headless
Chromium with WebGPU on, hands an image to the file input, and writes
screenshots at given times; it presses, holds and releases keys, clicks, and
counts animation frames. The browser is muted. It serves `--dist` through
`scripts/serve-web.py`, COOP/COEP included; `--url` drives a page another
server already serves instead (`npx wrangler pages dev`), `--read memory`
forces the in-memory fallback, and the script prints `crossOriginIsolated`.
`--browser firefox` runs Playwright's Firefox with the two WebGPU prefs above;
`--stalls MS` lists every gap between animation frames longer than `MS` once
the run ends, timed from the pick, which is the responsiveness number (a
screenshot waits for a free page thread, so its timestamp hides a stall);
`--reload SECONDS` reloads the page then. The walk this page's claims rest
on, from Language Selection to a race on Moa Therma White:

```sh
just web
uv run --with playwright python3 scripts/web-screenshot.py \
    --image data/images/pulse-psp-eu.chd --out data/web-shots \
    --at 30,62,70,78 --press Enter@8 --press Space@12 --press Enter@16 \
    --press Enter@20 --press Enter@24 --press Enter@28 \
    --down x@50 --down ArrowLeft@72 --up ArrowLeft@74 --fps 5
```

And Wipeout HD into the campaign's first event (Blitzed, Talon's Junction,
Venom, Assegai), the load "Threads" measures:

```sh
uv run --with playwright python3 scripts/web-screenshot.py \
    --image data/images/hdfury-ps3-eu-dec.iso --out data/web-shots \
    --at 50.3,50.8,51.3,51.8,52.3,53,60 --press Enter@15 --press Enter@25 \
    --press Enter@35 --press Enter@45 --press Enter@50 --log debug --stalls 100
```

Firefox draws slower, so its walks want the presses further apart.

`/usr/bin/chromium` (Chromium 153) is the browser it was run with: Playwright's
own Chromium build has no WebGPU on Linux. Headless Chromium here reports a hardware
adapter (`GPUAdapterInfo`: vendor `amd`, architecture `rdna-3`,
`isFallbackAdapter` false), not SwiftShader; `fps: 60.2` is the
`requestAnimationFrame` rate during the race, which is the cap, not a measure of
headroom. Native and web frames of the Language Selection screen match glyph for
glyph; the native one has the menu's backdrop movie behind it and the web one
does not (see below).

The simulation's determinism on wasm is checked by `just test-wasm`: the
`oag-core` determinism test against its committed reference, and the
`oag-physics`, `oag-gameplay`, `oag-race` and `oag-ai` suites, built for
`wasm32-wasip1` (the browser target's code generation, with a runner) and run
under wasmtime. All pass, debug and release (2026-10-09, wasmtime 49). The same
suites pass built the way the web module is, with the pinned nightly,
`-Z build-std=std,panic_abort -Z panic-abort-tests` and
`+atomics,+bulk-memory,+mutable-globals` (2026-10-09, run with
`CARGO_TARGET_WASM32_WASIP1_RUNNER="wasmtime -W threads=y"`). That run
covers the atomics code generation, which is what could touch a float; its
memory was not shared (no `--shared-memory` link argument), which no
simulation code can observe on one thread.

## Troubleshooting

**The page shows "OpenAntiGrav stopped".** A Rust panic, a lost GPU device or a
GPU out-of-memory/internal error lands on the page instead of a white canvas
(`web/main.js`, `window.oagFatal`, called from `gpu::web::fatal` and the panic
hook). It prints the cause and, when the adapter is a CPU one, the flags below.
The full text, with the stack, is in the browser console.

**The log says `renderer: webgpu:  (cpu)`.** The browser gave WebGPU a software
adapter (the `web: adapter "" (Cpu)` line says the same); frames then take 40 to
90 ms. On Linux Chrome and Chromium this is what you get without the Vulkan
feature. Start the browser with

```sh
google-chrome --enable-unsafe-webgpu --enable-features=Vulkan
```

or switch "Vulkan" and "Unsafe WebGPU Support" on in `chrome://flags`, restart,
and open `chrome://gpu`: its "WebGPU" line should read "Hardware accelerated"
and the adapter list should name your GPU rather than SwiftShader. A hardware
adapter logs `renderer: webgpu:  (setting: default, ...)` with no `(cpu)`.

**A `createBuffer ... mappedAtCreation == true` RangeError on a software
adapter** (2026-10-09: reproduced at the first window resize on
`--use-webgpu-adapter=swiftshader` with no Vulkan feature, 32-byte buffer) was
the small constant buffers built mapped. They are filled through
`oag_gpu::init_buffer` now, which writes through the queue on the web and maps
only on native, so a software adapter no longer panics there. That bare
SwiftShader mode still never presents the canvas (it stays white; the game runs
behind it), which is a reason to use the flags above, not something the game
can repair. `scripts/web-screenshot.py --software` runs the presenting CPU
adapter and `--software bare` that mode; `--resize WxH@SECONDS` resizes the
viewport mid-run.

## Hosting

Cloudflare Pages, project `openantigrav`, at https://oag.topaxi.com/, is the
only host; `.github/workflows/pages.yml` ("Pages (Cloudflare)") builds the
folder and deploys it. GitHub Pages was dropped on 2026-10-09 (maintainer's
decision): it cannot send response headers, and the threaded module needs two.

Cloudflare reads `_headers` from the deployed folder: `Cross-Origin-Opener-Policy:
same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on every path, which
make `crossOriginIsolated` true (what `SharedArrayBuffer` and so the module's
shared memory need), and `Cache-Control: public, max-age=31536000, immutable`
on `pkg/*`, whose directory name is the content hash. The page itself keeps
Cloudflare's default, `public, max-age=0, must-revalidate`, so a deploy is
seen on the next load. The page loads nothing from another origin, so
`require-corp` blocks nothing it uses; `worker.js` and the module are
same-origin. No `coi-serviceworker` and no second, single-threaded module.

The workflow: `build` runs `scripts/build-web.sh` on `ubuntu-latest` with the
pinned nightly (`nightly-2026-10-08`, `rust-src`, the wasm32 target),
`wasm-bindgen` at the `Cargo.lock` version and binaryen 130 from their release
tarballs, then uploads `target/web/dist` as the `web-dist` artifact;
`deploy-cloudflare` downloads it and runs `cloudflare/wrangler-action@v3` with
`pages deploy dist --project-name openantigrav --branch main`, the token from
the repository secret `CLOUDFLARE_API_TOKEN` and the account from the
repository variable `CLOUDFLARE_ACCOUNT_ID`. That job is skipped when the
variable is unset (a fork). It runs on a `v*` tag or by hand.

`nightly.yml` calls this workflow as its `web` job (`workflow_call`, `secrets:
inherit`) after its own `check`, only on a night that builds a new commit; a
manual dispatch of either stays. `actionlint` passes on both; `act -n` plans
the nightly's first job and cannot go further here (podman, below).

The deploy has not run on GitHub yet. `act` cannot run the workflow on a podman
host (`docker cp` into the image's `/var/run/act` fails with "path escapes from
parent", with or without `--use-new-action-cache`), so the single-threaded
build job's steps were run in a clean `rust:1.99-bookworm` container from an
export of the committed tree instead (2 min 30 s from a cold cache); the
threaded build has been run locally only, and the nightly install step is
untried in CI. The Cloudflare side was checked locally against `npx wrangler
pages dev target/web/dist --port 8798` (wrangler 4.149), which applies
`_headers` the way Pages does: `curl -I` shows both COOP/COEP headers on `/`
and on `worker.js`, `crossOriginIsolated` reads true in the page, and an HD and
a Pulse race run there (2026-10-09). `actionlint` passes on the workflow.

## Threads

The browser build is threaded since 2026-10-09, for one job: **the race load.**
Before it, starting a Wipeout HD race froze the page for the whole load: the
loading screen never drew, Firefox offered to stop the page, and a reload
mid-load panicked ("Leaving mid-load", above).

### What the load cost, and where

HD EU, the campaign's first event (Talon's Junction, Venom, Assegai), dist
build, headless Chromium 153 on a hardware adapter, 2026-10-09. The gaps
between animation frames (`--stalls`), and a CPU profile (the Chrome DevTools
protocol's `Profiler` over the dist module before `wasm-opt`, so function names
survive):

| | before (inline) | after (worker) |
| --- | --- | --- |
| page frozen at LAUNCH | 5.3 s, then 4.2 s | none during the load; 0.7 to 0.85 s at its end |
| circuit load (`race::load`) | 5.9 s on the page's thread | 1.5 to 1.8 s on a worker, `Race::start` included |
| scene build (`build_race_stage`) | 2.9 to 4.2 s | 0.7 to 0.83 s, page's thread |
| loading screen | never drew | draws and animates its bar |
| load plus build | about 9.5 s, all frozen | about 2.5 s, 0.8 s of it frozen |

The load's 5.9 s before, inclusive:

- **Disc reads, 3.75 s**: 200 synchronous requests for 200 MiB, of which the
  request itself was 2.5 s and the page's byte loop over the `x-user-defined`
  text 1.0 s; a further 1.7 s of garbage collection over the run came mostly
  from those 1 MiB response strings. On a worker the same reads go through
  `FileReaderSync` and return an `ArrayBuffer`, with no text and no loop.
- **Inflate**, 1.3 s (`miniz_oxide`); **parse and decode** the rest, about
  0.7 s.
- **GPU work**: only the texture sink's uploads, about 0.15 s.

The build's 2.9 s (the profiled run): `Scene::new` 1.8 s, of which
`writeBuffer` 1.6 s (140 MiB of vertices and indices, about 88 MB/s; 80
pipelines created in 4 ms, the browser compiles them elsewhere), and
`Race::start` 1.1 s, CPU only (`SpeedPlan::build_within`'s simulated laps). On
the threaded build the same two measured 0.7 s and 0.65 s, and `Race::start`
then moved onto the worker too (below), which is the after column.

Four HD loads in one tab (launch, escape to the campaign page, launch again,
one of them Mallavol): each reached its race, the module's memory read 876,
908, 908 and 908 MiB after them, and the page made no disc read at all between
LAUNCH and the scene being built, every one of the 185 to 207 reads per load
being the worker's (the `web: image fetch ... on a worker` debug line). So the
locks the page shares with a worker (the disc cache, the mount table) are
never contended during a load. Escaping to the menus still freezes 0.5 to
0.6 s, which this lane did not look at.

### The choice: threads, not resumable steps

The load is CPU work with one exception, the texture sink, so it can move off
the page's thread whole; cutting `race::load` into steps that yield between
frames would mean rewriting one long function into a state machine, and each
step would still freeze the page for as long as its slowest read. Threads also
took the disc reads off the synchronous request, which was most of the cost.
The scene build is GPU work, and wgpu's web types are not `Send` once the
module has atomics (wgpu's `send_sync` cfg drops them whatever
`fragile-send-sync-non-atomic-wasm` says), so it stays on the page's thread.

### How it is built

1. **The module.** A pinned nightly with `-Z build-std=std,panic_abort` (the
   prebuilt `std` has no atomics), `-C
   target-feature=+atomics,+bulk-memory,+mutable-globals`, and linker
   arguments rustc does not add on its own for this target: `--shared-memory
   --import-memory --max-memory=4294967296` and the four TLS exports
   wasm-bindgen sets a thread up with (`scripts/build-web.sh`). wasm-bindgen
   then emits a glue whose `init` takes a memory; the script fails the build if
   it does not (`thread_stack_size` missing from the glue). `wasm-opt` gets
   `--enable-threads`.
2. **A worker is a thread.** `std::thread::spawn` stays unsupported on this
   target even with atomics. `oag_raceplay::web_thread::spawn` parks the
   closure in a table and calls the page's `oagSpawnWorker(module, memory,
   id)` (`web/main.js`), which starts `web/worker.js` as a module worker; that
   imports the same glue, instantiates the module on the same memory, and calls
   `oag_worker_entry(id)`, then `__wbindgen_thread_destroy` to hand its stack
   and TLS block back before it closes. No new third-party crate (`oag-raceplay`
   gains `wasm-bindgen` and `js-sys`, already in the build) and no `unsafe`. A
   panic on a worker reaches the page's `oagFatal` through a message.
3. **The page's thread never waits.** `memory.atomic.wait32`, where a
   contended `std::sync::Mutex` ends up, traps there. `LoadWorker` polls an
   `AtomicBool` the worker sets after it has stored the result and let go of
   its lock; the disc cache is `try_lock`ed (above); `web_thread`'s own table
   is locked by spinning on `try_lock`. The allocator already spins.
4. **No GPU on a worker.** The texture sink is a stub on wasm
   (`mesh_render/texture_sink/web.rs`) and `Texels::Uploaded`, which holds a
   `wgpu::TextureView`, exists only on native, so `race::Loaded` is `Send`. The
   textures then wait on the CPU for the scene build: the module's memory was
   876 to 908 MiB after the HD loads above (`web: module memory ... after the race
   load`, logged once per load), of a 4 GiB ceiling that never shrinks.
   Omega's Tech De Ra, whose BC7 textures were 2.3 GiB at their peak, would not
   fit, but no Vita or PS4 package opens in the browser.
5. **CPU work of the scene build moves too where it can.** `race::Race` is
   `Send`, so the worker also runs `Race::start` on a copy of the setup and
   hands the started race over in `Loaded::started`; `build_race_stage` uses
   it instead of starting one. Native leaves it `None` (its build runs on a
   thread anyway).
6. **The simulation stays single-threaded and deterministic**: nothing in it
   changed, a race started on the worker is the same `Race::start` on an equal
   setup, and the sim suites pass built with the atomics flags ("Verifying
   it").

### What is still on the page's thread

- **The scene build's GPU half**, 0.7 to 0.85 s at the end of the HD load, the
  loading screen's last frame held for that long. The buffer uploads
  (`writeBuffer`, about half of `Scene::new`) could be spread over frames under
  a byte budget; not done.
- **The boot**: about 2 s between the pick and the first frame (Chromium), and
  the menus' own reads (a flyer, a ship), still synchronous requests.
- **Dropping a parked race**, and `boot::MediaWorker`, inline as before.

## What is missing

- **Images over about 2 GB in a browser without synchronous slice reads.**
  The fallback copies the whole image into the module's memory, which wasm32
  caps at 4 GiB. Every browser tried reads slices (Chromium, Firefox, WebKit).
- **Encrypted PS3 images, Vita and PS4 packages.** A disc key or a package's
  sibling files cannot be found beside a picked file; only CHD and plain ISO go
  through `oag_disc::mount`.
- **Sound.** No audio device is opened (`--no-audio`); music and effects would
  also need their decode caches, which are files. Nothing is played rather than
  anything faked.
- **Movies.** The AV1 decoder (`re_rav1d`) does not build for wasm32, and the
  movie cache it reads is made by `ffmpeg` into a directory; the boot movies and
  the menu backdrop are absent, and the loader report says so (`no picture: ...`,
  logged at warn).
- **Ghosts and pilot files** last one tab; settings and records persist.
- **Untested inputs.** gilrs's Gamepad API backend is compiled in and not tried
  with a pad; touch is not tried. The keyboard and the mouse work.
- **Other titles in the browser.** Pulse PSP and HD race (Chromium and
  Firefox, headless); Pulse PS2 was booted to its front end only and Pure not
  at all. HD drew its race with no Tint error on the one circuit tried.
- **Firefox draws slowly**: 9 to 14 fps in a race in headless Firefox 155,
  against Chromium's 60 on the same machine, and the same before threads
  (Pulse, Moa Therma White: 12.2 fps on the single-threaded module, 12.4 on
  the threaded one). Not investigated.
