# The web build

`oag-game` in a browser tab: WebAssembly (`wasm32-unknown-unknown`) drawing
through WebGPU, with no launcher. The page asks for a disc image, reads it in the
tab, and boots it. It boots Wipeout Pulse (PSP, EU) to the front end, walks the
menus with the keyboard and the mouse, and races, in headless Chromium at 60 fps
(2026-10-09). Wipeout HD (a 2.2 GB decrypted PS3 ISO) boots to its front end and
Pulse PS2 (a 3.7 GB CHD) to Language Selection, the image read a slice at a time.
It has not been tried in a browser on a real desktop, on a phone,
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

The published folder is `index.html`, `main.js`, `style.css`,
`pkg/<hash>/oag_web.js` (the wasm-bindgen glue, 105 KB),
`pkg/<hash>/oag_web_bg.wasm` (9.9 MB; 3.6 MB at `gzip -9`), the licence files,
`_headers` (Cloudflare's, see "Hosting") and `.nojekyll`. `<hash>` is the first
16 hex digits of the SHA-256 of the glue and the module together, and
`build-web.sh` rewrites `main.js`'s one import to name it, so the two files can
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
- **Reading in slices.** The disc readers are synchronous and run on the
  page's thread, which may not wait for a promise, and `File` only reads
  asynchronously there. What reads it synchronously is a synchronous
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

  Measured in headless browsers on the HD ISO, 64 reads of 1 MiB spread over
  the file (2026-10-09): Chromium 153 57 MB/s warm (34 cold), Firefox 155
  350 MB/s, WebKit 26.6 287 MB/s, all byte-identical with the asynchronous
  read. `?log=debug` logs every fetch (`web: image fetch at ...`). Pulse PSP,
  from the pick to a race on Moa Therma White, made 84 fetches, 8 of them in
  the race load and none once it finished; the race ran at the 60 fps cap, and
  its load reached its last stage after 1,328 ms against 989 ms held in memory
  (a debug build). HD to its front end: 131 fetches, 131 MiB.

  The design the earlier version of this page proposed, the image read in a
  Web Worker through `FileReaderSync` and handed over a `SharedArrayBuffer`,
  needs cross-origin isolation, which GitHub Pages cannot give, and a
  main-thread wait: `Atomics.wait` is refused on a page's main thread, so it
  would spin. A synchronous request already blocks the thread exactly as long
  as the read takes, the same way the race load already runs inline (below),
  works on both hosts, and needs no second file.
- **The main thread, not a Web Worker.** The game itself stays on the page's
  thread because winit cannot run in a worker: its web event loop calls
  `web_sys::window()` and panics without one (`only callable from inside the
  Window`, winit 0.30.13), and `WindowAttributesExtWebSys::with_canvas` takes an
  `HtmlCanvasElement`, never an `OffscreenCanvas`. Running in a worker means
  driving `App` without winit. With reads in slices it no longer buys a size
  limit; what it would still buy is a page that stays responsive while a race
  loads.
- **No threads.** `wasm32-unknown-unknown` without the `atomics` target feature
  has none ("Threads" below says what turning it on takes). So every place the desktop build moves
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
  nothing kept between reloads. The storage is per origin, so the GitHub and
  Cloudflare copies keep separate profiles.
- **The page.** `web/` is plain HTML, CSS and one ES module, no bundler. A plain
  `<input type="file">` is the way in, in every browser. Where
  `showOpenFilePicker` exists (Chromium), a second button keeps the file's handle
  in IndexedDB, and the next visit offers "Play <name> again", which asks the
  browser for read permission again rather than re-picking. The page hides
  every element carrying `hidden` with `display: none !important`: an author
  `display` rule otherwise outranks the browser's own, which once drew that
  button empty beside the other two with nothing to replay.

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
counts animation frames. The browser is muted. `--url` drives a page another
server already serves instead of serving `--dist` itself, `--read memory`
forces the in-memory fallback, and the script prints `crossOriginIsolated`.
The walk this page's claims rest
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

The same folder is published to two hosts by one workflow,
`.github/workflows/pages.yml`, which builds once and deploys twice:

| Host | URL | Headers |
| --- | --- | --- |
| GitHub Pages | https://topaxi.github.io/OpenAntiGrav/ (expected; the `deploy` job's `page_url` is authoritative) | none can be set |
| Cloudflare Pages, project `openantigrav` | https://oag.topaxi.com/ | `web/_headers` |

Cloudflare reads `_headers` from the deployed folder: `Cross-Origin-Opener-Policy:
same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on every path, which
make `crossOriginIsolated` true (what `SharedArrayBuffer` and wasm threads
need), and `Cache-Control: public, max-age=31536000, immutable` on `pkg/*`,
whose directory name is the content hash. The page itself keeps Cloudflare's
default, `public, max-age=0, must-revalidate`, so a deploy is seen on the next
load. The page loads nothing from another origin, so `require-corp` blocks
nothing it uses. GitHub Pages serves `_headers` as a plain file and sends none
of it; nothing the page does needs them today, so both copies behave the same.
They are there so a threaded build (below) has somewhere to run.

The workflow: `build` runs `scripts/build-web.sh` on `ubuntu-latest` with the
pinned toolchain, `wasm-bindgen` at the `Cargo.lock` version and binaryen 130
from their release tarballs, then uploads `target/web/dist` twice: as the Pages
artifact (`actions/upload-pages-artifact`, a tarball in a format that action
owns) and as a plain `web-dist` artifact for Cloudflare. `deploy` publishes the
first with `actions/deploy-pages`; `deploy-cloudflare` downloads the second and
runs `cloudflare/wrangler-action@v3` with `pages deploy dist --project-name
openantigrav --branch main`, the token from the repository secret
`CLOUDFLARE_API_TOKEN` and the account from the repository variable
`CLOUDFLARE_ACCOUNT_ID`. That job is skipped when the variable is unset (a
fork). One build means both hosts serve byte-identical files. It runs on a `v*`
tag or by hand. GitHub Pages has to be switched on once in the repository's
settings, with "GitHub Actions" as the source.

Neither deploy has run on GitHub yet. `act` cannot run the workflow on a podman
host (`docker cp` into the image's `/var/run/act` fails with "path escapes from
parent", with or without `--use-new-action-cache`), so the build job's own steps
were run in a clean `rust:1.99-bookworm` container from an export of the
committed tree instead: the tool downloads, the dist build, `wasm-opt` and the
leakage check passed in 2 min 30 s from a cold cache. The Cloudflare side was
checked locally against `npx wrangler pages dev target/web/dist --port 8796`
(wrangler 4.149), which applies `_headers` the way Pages does: `curl -I` shows
both COOP/COEP headers on `/` and the immutable lifetime on the module,
`crossOriginIsolated` reads true in the page, and a Pulse PSP race runs there at
60 fps. `actionlint` passes on the workflow. `wrangler-action` installs its own
default wrangler for the deploy; only `pages dev` from 4.149 was exercised
here.

## Threads

Out of scope so far; what turning them on would take, now that one host sends
the headers:

1. A nightly toolchain for the web build only, `-Z build-std=std,panic_abort`
   with `-C target-feature=+atomics,+bulk-memory,+mutable-globals` (the
   prebuilt `std` is not compiled with atomics), and `wasm-bindgen` run on the
   result, which then emits a shared memory.
2. Threads that are Web Workers: `std::thread::spawn` does not work on this
   target even with atomics. `wasm-bindgen-rayon` or a small spawner that starts
   a worker running the same module on the shared memory would back the
   `LoadWorker`, `BuildWorker` and `MediaWorker` sites that run inline today.
3. The main thread may still not block: `Atomics.wait` is refused there, so a
   `Mutex` the main thread contends spins, and every channel `recv` on it must
   become `try_recv` polled from the frame loop, which those three workers
   already are on native.
4. wgpu's `fragile-send-sync-non-atomic-wasm` feature has to go, since its
   soundness rests on there being one thread, and wgpu's web types are then
   not `Send`: the GPU stays on the main thread and only CPU work moves.
5. A fallback for GitHub Pages, where `crossOriginIsolated` is false and a
   shared memory cannot be created: either a second, single-threaded module
   chosen at load, or a `coi-serviceworker` there.

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
- **Other titles in the browser.** Pulse PSP races; HD and Pulse PS2 were
  booted to their front ends only. Pure should open the same way. HD's race
  shaders are the likeliest to meet another Tint uniformity error.
