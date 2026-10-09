# The web build

`oag-game` in a browser tab: WebAssembly (`wasm32-unknown-unknown`) drawing
through WebGPU, with no launcher. The page asks for a disc image, reads it in the
tab, and boots it. It boots Wipeout Pulse (PSP, EU) to the front end, walks the
menus with the keyboard and the mouse, and races, in headless Chromium at 60 fps
(2026-10-09). It has not been tried in a browser on a real desktop, on a phone,
or with a gamepad. "What is missing" lists the rest.

```sh
just web            # target/web/dist: dist profile (fat LTO) + wasm-opt -O
just web --dev      # a debug build, no wasm-opt: quicker to iterate on
just web --serve    # then serve target/web/dist on http://127.0.0.1:8000/
```

One-time setup: `rustup target add wasm32-unknown-unknown`, the
`wasm-bindgen` CLI at the exact version `Cargo.lock` pins
(`cargo install wasm-bindgen-cli --version <v>`; the script says which), and
`wasm-opt` from binaryen (any recent release; 130 is what CI pins).

The published folder is `index.html`, `main.js`, `style.css`, `pkg/oag_web.js`
(the wasm-bindgen glue, 105 KB), `pkg/oag_web_bg.wasm` (9.9 MB; 3.6 MB at
`gzip -9`), the licence files
and `.nojekyll`. `scripts/build-web.sh` ends by running
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
  `start(name, bytes, log)`, which the page calls with the picked file.
- **The image.** `oag-disc` opens an image through `oag_disc::mount` now: on
  native that is `std::fs::File` and nothing changed; on wasm it is a cursor
  over a blob the entry point registered under `/web/<file name>`, which is the
  path the run is then told. The CHD and raw-ISO readers, and the container
  sniff, go through it; the rest of the engine keeps its path-based shape. The
  blob today is the whole file copied into the module's memory, which is where
  the size limit below comes from. A `Blob` is any synchronous random-access
  reader, so a different backing plugs in without the readers noticing.
- **The main thread, not a Web Worker.** A worker with an `OffscreenCanvas`
  could read the image a slice at a time through `FileReaderSync`, with no size
  limit. It is not what this build does, because winit cannot run in a worker:
  its web event loop calls `web_sys::window()` and panics without one
  (`only callable from inside the Window`, winit 0.30.13), and
  `WindowAttributesExtWebSys::with_canvas` takes an `HtmlCanvasElement`, never an
  `OffscreenCanvas`. Running in a worker means driving `App` without winit,
  which is the open thread's first item, not a build flag.
- **No threads.** `wasm32-unknown-unknown` without the `atomics` target feature
  has none, and turning it on needs a nightly `-Zbuild-std` and cross-origin
  isolation, which GitHub Pages cannot serve (no COOP/COEP headers; a
  `coi-serviceworker` would fake them). So every place the desktop build moves
  work onto a thread runs it inline on wasm, behind
  `cfg(target_arch = "wasm32")`: the race load (`oag_raceplay::LoadWorker`), the
  race scene build (`race_build::BuildWorker`), dropping a parked race, and the
  boot's media (`boot::MediaWorker`). The frame that starts one of these stalls
  for it: Moa Therma White's load reached its last stage after 674 ms in the
  dist build. A CHD read is
  already serial when `available_parallelism` fails, which it does here. The
  circuit-length worker and the race music fetch fail to spawn and log it, as
  they would on any machine that refuses a thread. wgpu's
  `fragile-send-sync-non-atomic-wasm` feature keeps the `Send` bounds those
  native threads need compiling; it is sound only because nothing here runs on
  a second thread.
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
- **Settings, records, ghosts.** `std::fs` returns `Unsupported` here, and each
  of these already degrades on a failed read or write: a run starts on default
  settings and keeps nothing between reloads. Persisting them (IndexedDB or the
  Origin Private File System) is open.
- **The page.** `web/` is plain HTML, CSS and one ES module, no bundler. A plain
  `<input type="file">` is the way in, in every browser. Where
  `showOpenFilePicker` exists (Chromium), a second button keeps the file's handle
  in IndexedDB, and the next visit offers "Play <name> again", which asks the
  browser for read permission again rather than re-picking.

## Browser support

WebGPU is required; there is no WebGL fallback (`adapter::BACKENDS` is
`PRIMARY`, which includes the browser's WebGPU and excludes GL, for the reason
that constant's own documentation gives). WebGPU ships in Chrome and Edge 113+
(Windows, macOS, ChromeOS; Android 121+), Safari 26, and Firefox 141+ on Windows.
Chromium on Linux still needs `--enable-unsafe-webgpu --enable-features=Vulkan`.
The page says so when `navigator.gpu` is missing rather than failing later.

## Verifying it

`scripts/web-screenshot.py` serves a built `dist` folder, opens it in headless
Chromium with WebGPU on, hands an image to the file input, and writes
screenshots at given times; it presses, holds and releases keys, clicks, and
counts animation frames. The browser is muted. The walk this page's claims rest
on, from Language Selection to a race on Moa Therma White:

```sh
just web
uv run --with playwright python3 scripts/web-screenshot.py \
    --image data/images/pulse-psp-eu.chd --out data/web-shots \
    --at 30,62,70,78 --press Enter@8 --press Space@12 --press Enter@16 \
    --press Enter@20 --press Enter@24 --press Enter@28 \
    --down x@50 --down ArrowLeft@72 --up ArrowLeft@74 --fps 5
```

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
under wasmtime. All pass, debug and release (2026-10-09, wasmtime 49).

## Deploying

`.github/workflows/pages.yml` runs `scripts/build-web.sh` on `ubuntu-latest`
with the pinned toolchain, `wasm-bindgen` at the `Cargo.lock` version and
binaryen 130 from their release tarballs, then publishes `target/web/dist`
with `actions/upload-pages-artifact` and `actions/deploy-pages`. It runs on a
`v*` tag or by hand. Pages has to be switched on once in the repository's
settings, with "GitHub Actions" as the source. Single-threaded, so it needs no
COOP/COEP headers and ships no `coi-serviceworker`.

It has not run on GitHub yet. `act` cannot run it on a podman host (`docker cp`
into the image's `/var/run/act` fails with "path escapes from parent", with or
without `--use-new-action-cache`), so the job's own steps were run in a clean
`rust:1.99-bookworm` container from an export of the committed tree instead:
the tool downloads, the dist build, `wasm-opt` and the leakage check passed in
2 min 30 s from a cold cache.

## What is missing

- **Images over about 2 GB.** The whole image is copied into the module's
  memory, which wasm32 caps at 4 GiB and which the game shares. Pulse and Pure
  PSP images (140-210 MB) open; Wipeout HD (2.2 GB) is doubtful; Pulse PS2
  (3.7 GB) and Omega (24 GB) cannot. The page also holds a second copy while it
  hands the bytes over. Lifting this is the Worker architecture above.
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
- **Persistence.** Settings, records, ghosts and campaign progress last one tab.
- **Untested inputs.** gilrs's Gamepad API backend is compiled in and not tried
  with a pad; touch is not tried. The keyboard and the mouse work.
- **Other titles in the browser.** Only Pulse PSP was booted; Pure should open
  the same way, and HD's shaders are the likeliest to meet another Tint
  uniformity error.
