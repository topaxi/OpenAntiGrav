# The web build

`oag-game` in a browser tab: WebAssembly (`wasm32-unknown-unknown`) drawing
through WebGPU, with no launcher. The page asks for a disc image, reads it in the
tab, and boots it. It boots Wipeout Pulse (PSP, EU) to the front end, walks the
menus with the keyboard and the mouse, and races, in headless Chromium at 60 fps
(2026-10-09). Wipeout HD (a 2.2 GB PS3 ISO, decrypted or encrypted with its key) races too, its circuit
loaded on a Web Worker while the loading screen draws (see "Threads"), in
headless Chromium and Firefox; Pulse PS2 (a 3.7 GB CHD) boots to Language
Selection, the image read a slice at a time. It plays sound: the effects on
every title, and Wipeout HD's MP3 music, through an `AudioWorklet` fed from a
Web Worker (see "Sound"); Pulse's and Pure's ATRAC3+ music is absent. The PSP
movies play, the boot intro and the menu backdrop, decoded by the browser's
WebCodecs (see "Movies"); the PS2's and HD's do not. It has
not been tried in a browser on a real desktop, on a phone, or with a gamepad.
"What is missing" lists the rest.

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
- **Workers for the race load, the race music and the mix.** The race load
  (`oag_raceplay::LoadWorker`) and the soundtrack fetch
  (`oag_sound::MusicFetchWorker`) are `oag_thread::Task`s, each a Web Worker
  here; the mixer's render-ahead loop is a third, for as long as the page
  lives ("Sound", "Threads"). The other
  places the desktop build moves work onto a thread still run it inline on
  wasm, behind `cfg(target_arch = "wasm32")`: the race scene build
  (`race_build::BuildWorker`, which is GPU work and wgpu's web types belong to
  the page's thread), dropping a parked race (GPU resources again), and the
  boot's media (`boot::MediaWorker`; a movie's open is a demux, and its decode
  is the browser's own and asynchronous, so it has nothing slow to move). A CHD read is serial when `available_parallelism`
  fails, which it does here. The circuit-length worker uses `std::thread`, which
  this target refuses even with atomics, so it fails to spawn and logs it, as it
  would on any machine that refuses a thread.
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
- **A disc key.** An encrypted PS3 `.iso` needs its key, which the page cannot
  find beside the picked file (see "Disc keys"). It takes the key as a `.dkey`
  or `.key` file (dropped with the image, dropped alone, or picked in the key
  box that appears when an image needs one) or as pasted hex digits.
- **The page.** `web/` is plain HTML, CSS and one ES module, no bundler. A plain
  `<input type="file">` is the way in, in every browser. Where
  `showOpenFilePicker` exists (Chromium), a second button keeps the file's handle
  in IndexedDB, and the next visit offers "Play <name> again", which asks the
  browser for read permission again rather than re-picking. The page hides
  every element carrying `hidden` with `display: none !important`: an author
  `display` rule otherwise outranks the browser's own, which once drew that
  button empty beside the other two with nothing to replay.

### Disc keys

The page asks the module, before booting, whether the image needs a key
(`oag_web::inspect`, which calls `oag_disc::ps3_probe::probe`, the one check the
desktop launcher's drop path shares). A plain image, a CHD and a decrypted PS3
dump are `ready` and ignore any key given; an encrypted PS3 image is `missing`,
`malformed` or `wrong` until a key opens it, and the page says which in plain
words under the picker. "Opens" is `ps3_crypt::unlock`'s own oracle (every
target file reads as its magic), not a format check alone.

- **Three ways in** (headless Chromium 153, the encrypted HD image, dev build,
  2026-10-09; each reached the front end): both files dropped on the page at once
  (a small file, 256 bytes or fewer or named `.dkey`/`.key`, is the key, the
  larger one the image; dropped alone either is held until the other comes), the
  key file input shown when an encrypted image is picked without a key, and the
  hex typed or pasted into the field beside it (Enter or "Use this key"). The key
  box, the saved-keys list and the file inputs are real controls: Tab reaches
  them, the file inputs are visually hidden and not `display: none`, and the page
  has no horizontal overflow at 390 px.
- **The key reaches the engine as a file.** `start` mounts it at
  `<stem>.dkey` beside the image in `oag_disc::mount`, which is the first place
  `ps3_crypt::find_keys` looks, so `DiscImage::open` finds it with no web branch
  and the race-load worker finds it through the shared mount table with no
  message of its own. `find_keys` reads a sibling through
  `oag_disc::mount::read_all` (a file natively, the blob here); the keys
  directory is still `std::fs`, which is nothing on the web.
- **Remembered per disc, never by name.** On a good key the page writes the key's
  hex to `localStorage` under `oag:diskey:<serial>` (the serial reads through the
  encryption: `BCES-00664` for HD EU), unless the "Remember" box is cleared. Any
  later pick of an image with that serial, whatever its file name, finds the key
  itself (checked: boot, reload, pick again with no key, front end drawn), and a
  stored key that no longer opens its disc is dropped. The picker lists every
  saved key by serial with a Forget button. The key stays in this browser's
  storage for this origin: it is never sent, logged or put in a URL.
- **Cost.** Decryption is software AES, wherever the read happens. The race
  load reads on a worker, so the page stays responsive: HD's first event reached
  `CraftsBuilt` after 5,988 ms on the encrypted image and 2,575 ms on the
  decrypted one in the same dev build (one run each; the desktop's release build
  measures no difference, see [installing](../overview/installing.md)). The
  boot's and the menus' reads still run on the page's thread and decrypt there:
  the longest gap between animation frames after the pick was 2.0 to 4.0 s on
  the encrypted image (five runs) and 0.8 to 3.0 s on the decrypted one (two
  runs) in dev builds on a loaded machine, so the difference is not separated
  from the noise. Not measured on the `dist` build.
- **One mount per file.** `inspect` registers the picked image once; `start`
  and a retry after a wrong key pass nothing and reuse it, because an in-memory
  image (`?read=memory`) is a copy in the module's memory, which never shrinks,
  and a second registration would hold the image twice.
- **Also dropped alone, in either order.** The decrypted ISO dropped alone boots
  to the front end; the encrypted one dropped alone shows the key box; a key
  dropped first is held and the image dropped 1.5 s later boots (all headless
  Chromium, dev build). Tab reaches the file inputs, "Choose, and remember it"
  and each Forget button; Enter on a Forget removes the stored key (checked,
  `localStorage` read back) and the next pick of that image asks again; Enter in
  the hex field submits it (a wrong key gives the "does not open" line). At
  390 px `#picker.scrollWidth` equals its `clientWidth`.

### First-run defaults and the options pages

**Chosen, not measured.** With no `settings.toml` in `localStorage` (a first
visit), `oag_game::settings::web::first_run` starts light: frame limit 60,
anisotropy off, shadows off in every title's render profile (the type's own
default is already `off`; HD's profile does not override it). A saved profile,
the maintainer's included, keeps what it holds: a file written before this has
240 and 16x as canonical values and is never rewritten. Dynamic resolution is
not turned on.

The render target is held to **720 lines** on the web **by default**: only while
render scale is at its default, 100 (a player cannot tell 100 picked from 100
untouched, so 100 is the capped value; 125 and up go past 720, 50 and 75 below it, as on a desktop). The options row has no help line to say so, so this page does (`settings::web::render_target`, used by every
`target_size` caller, width scaled by the same factor): a 1920x1080 surface logs
`render target 1271x720 for a 1920x1080 surface`, and the race held 60.4 fps in
headless Chromium (2026-10-09). The surface is in CSS pixels, not physical ones: at
`devicePixelRatio` 2 on a 960x544 page the canvas is 960x544 and the target is
960x544 (measured, `web-screenshot.py --dpr 2`), so the cap binds only on a page
taller than 720 CSS pixels and a phone never reaches it. Choosing a
render scale other than 100 lifts the cap. At the
60 limit the game's own overlay (`graphics.perf_overlay = "fps"`) read `60 FPS
16.7 MS` in a race, and the same at a limit of 120 (the page's animation frames
cap it), so 60 does not halve the rate.

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
dropped on a desktop. (A desktop `settings.toml` is not given the key: it is skipped when
serialising off the web.) On the web the WINDOW MODE row reads WINDOWED and
FULLSCREEN (`relabel_choice`; the stored value stays `borderless`). The
fullscreen row is a browser request: Chromium and Firefox honour it from a
key or a click on the menu (headless, 2026-10-09), a gamepad button is not on
the user-activation list so a pad alone cannot enter fullscreen, and a click
on a row while fullscreen lands where that layout puts it. winit's own size request is not used (it cropped the
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
`--key FILE --key-mode pick|drop|hex|drop-image` hands an encrypted image's key
to the page the way the mode says ("Disc keys"); `--stalls MS` lists every gap between animation frames longer than `MS` once
the run ends, timed from the pick, which is the responsiveness number (a
screenshot waits for a free page thread, so its timestamp hides a stall);
`--swipe X0,Y0,X1,Y1,MS@T` drags one finger over `MS` ms and lifts it, and
`--tap X,Y@T` taps, both as real touches through Chrome DevTools
(`Input.dispatchTouchEvent`), which winit reports as `WindowEvent::Touch`;
`--profile FROM-TO` (Chromium, repeatable) records a CPU profile of the page's thread between those seconds after the pick, prints the 25 heaviest functions by self time and writes `<out>/profile-FROM.cpuprofile` (a module with function names wants `OAG_WEB_NO_OPT=1 just web`, which skips `wasm-opt`; the sampling perturbs frame times, so a stall table comes from a run without it);
`--reload SECONDS` reloads the page then; `--audio-wav FILE` records
`--audio-seconds` of what the audio worklet outputs ("Sound"). Chromium runs
with `--mute-audio --disable-audio-output`: the second renders to a fake sink,
which still drives the worklet, so no stream reaches the host's sound server at
all (checked with `pactl list sink-inputs` during a run). Firefox has no such
sink, so `--browser firefox` loads the page with `?audio=off` and refuses
`--audio-wav`. The walk this page's
claims rest on, from Language Selection to a race on Moa Therma White:

```sh
just web
uv run --with playwright python3 scripts/web-screenshot.py \
    --image data/images/pulse-psp-eu.chd --out data/web-shots \
    --at 32,64,72,80 --press Enter@8 --press Enter@11 --press Space@14 \
    --press Enter@18 --press Enter@22 --press Enter@26 --press Enter@30 \
    --down x@52 --down ArrowLeft@74 --up ArrowLeft@76 --fps 5
```

The first Enter skips the intro movie, which plays since 2026-10-09
("Movies"); before that the walk was one press shorter.

The encrypted HD image takes its key from the same command with
`--key data/images/hdfury-ps3-eu.dkey --key-mode drop` (or `pick`, `hex`,
`key-first`); `--key-mode drop-image` drops the image alone.

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
glyph, each with the menu's backdrop movie behind it ("Movies").

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
never contended during a load. Escaping to the menus froze 0.5 to
0.6 s then; "Freezes after the threads" below has its cause and its fix.

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
   target even with atomics. `oag_thread::web::spawn` (crate `oag-thread`,
   below every crate that moves work off the frame loop) parks the
   closure in a table and calls the page's `oagSpawnWorker(module, memory,
   id)` (`web/main.js`), which starts `web/worker.js` as a module worker; that
   imports the same glue, instantiates the module on the same memory, and calls
   `oag_worker_entry(id)`, then `__wbindgen_thread_destroy` to hand its stack
   and TLS block back before it closes. No new third-party crate (`oag-raceplay`
   gains `wasm-bindgen` and `js-sys`, already in the build) and no `unsafe`. A
   panic on a worker reaches the page's `oagFatal` through a message.
   `oag_thread::Task<T>` is the cross-target shape on top: `std::thread`
   natively, a worker plus a done flag here.
3. **The page's thread never waits.** `memory.atomic.wait32`, where a
   contended `std::sync::Mutex` ends up, traps there. A `Task` is polled
   through an `AtomicBool` the worker sets after it has stored the result and
   let go of its lock; the disc cache is `try_lock`ed (above); the worker table
   and the audio mixer are locked by spinning on `try_lock`
   (`oag_thread::lock`). The allocator already spins.
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

- **The scene build's GPU half** no longer holds the page for its uploads
  (see below); what is left is the browser compiling the 80 pipelines, which
  shows as a 0.4 to 0.55 s gap between animation frames with the page's own
  thread idle.
- **The boot**: about 1 s between the pick and the first frame (Chromium), of
  which about 0.5 s is synchronous disc reads, 0.16 to 0.24 s the front-end
  music's MP3 decode (`Audio::start_music`, inline), and the rest inflate and
  sprite sheets.
- **Dropping a parked race**, and `boot::MediaWorker`, inline as before.

### Freezes after the threads

2026-10-10, lane `web-freezes`. HD EU, the campaign's first event, headless
Chromium 153 on a hardware adapter, the shipped `dist` build, the walk above
plus an Escape from the race (`--press Escape@62`). Every stall of 300 ms or
more, by run, before and after, interleaved (a1-a3 before, b1-b3 after); the
machine was shared, load average 12 to 73, so only the direction is a result,
not a decimal:

| freeze | before, per run | after, per run |
| --- | --- | --- |
| boot, pick to first frame | 3333, 2217, 3400 ms | 1033, 2017, 2483 ms |
| first menu step after the boot | 967, 850, 850 ms | 383, 650, 733 ms |
| end of the race load (scene build) | 817, 1317, 1400 ms | 550, 917, 433 ms |
| Escape from a race | 750, 1367, 1417 ms | under 300, 400, 250 ms |

At a load average near 8 the same builds measured boot 1.6 to 1.8 s before and
1.0 to 1.2 s after, and Escape 0.6 s before and 0.23 s after. Firefox 155
(presses further apart, stalls of 300 ms or more, one run each, load 10 and
72): Escape 1745 ms before and 583 ms after, the end of the race load 733 and
617 ms before and 250 ms after, boot 1116 ms before and 950 ms after.

**The scene build.** Profiled with `--profile`, the 0.7 to 1.5 s was
`writeTexture` (0.84 s, the disc-authored BC levels) and `writeBuffer` (0.54 s)
and the cause of the second was not the 140 MiB: 0.5 s of it was thousands of
calls of a few hundred bytes to a few KiB, the three animation buffers of each
of 128 instances in each of the weapon pools (`Drawable::own_animation`), at
about 0.13 ms a call whatever the size. `oag_gpu::deferred_upload` parks every
buffer and texture write the build makes when its scope is open and
`BuildWorker::take` writes them 5 ms a frame (256 KiB a step, a texture level
in row bands), then draws the warmup frame once they have all landed. A parked
write holds its source, not a copy: vertices and indices read from the
drawable's `Arc<Model>`, texture levels from the texture's `Arc`; the module's
memory after the load read 953 MiB before and 969 to 971 MiB after (not
investigated). The scope opens on the
web only, so native stays on the immediate `queue.write_*` it always was and
its load time is untouched. The scene build itself measured 0.8 to 1.4 s
before and 0.1 to 0.45 s after; the uploads then take 21 to 29 frames, so the
loading screen is up 0.35 to 0.5 s longer (at 60 Hz) and animates through it.
What is left of the stall is the browser compiling the pipelines: the CDP
profile of that stretch has no busy stretch of the page's thread over 150 ms
while the animation frames are 400 to 550 ms apart. wgpu 30 has no
asynchronous pipeline creation to hand that to.

**Escape from a race.** Not GPU teardown. The worker's 200 MiB of race-load
reads went through the same 64-block LRU the page's thread uses, so the front
end's blocks were gone and reopening Cell Selection made 41 synchronous
requests on the page's thread (`send` 0.54 s, GC 0.37 s, the byte loop 0.21 s
in the profile). The page's thread and a worker now each have a cache
(`web_image::Shape`): after the change Escape makes no request at all.

**Boot.** The chain is `prepare::Pending::windowed`, `boot::load_shell`,
`sprites::read_front_end_first` and `Archives::read_every_name`: 119
synchronous 1 MiB requests (119 MiB) for what is a table of contents, a few
names and a sprite sheet, 0.68 s in `send`, 0.28 s in the byte loop, 0.51 s of
GC, in our code and not the browser compiling the module. The page's cache now
cuts the image into 128 KiB blocks (reads of 512 KiB and over fetched exactly)
and the worker's keeps 1 MiB blocks for the race load's long reads: the boot
makes 307 requests for 38 MiB. Moving the boot onto a worker is not done: the
front end's loads touch the GPU (sprite sheets, textures), which wgpu's web
types keep on the page's thread, so its result is not `Send` as it stands.

## Sound

Sound plays in the browser since 2026-10-09. Three threads, as on the desktop:

1. **The frame loop** starts, stops and retunes voices on the page's thread
   under the mixer lock (`Output::with_mixer`), which spins there rather than
   wait (`oag_thread::lock`).
2. **The render-ahead loop** (`oag_audio::output::render::Ahead`, the same code
   the desktop runs on its `oag-audio-render` thread) mixes 512 frames at a time
   into a ring, held at the same depth as natively (`MIN_BUFFER`, 60 ms, or two
   and a half frames of the frame cap). Here it runs on a Web Worker.
3. **The audio thread** is an `AudioWorkletProcessor` (`web/audio-worklet.js`)
   that copies out of the ring. It runs no wasm at all: the ring
   (`oag_audio::output::shared_ring`) lives in the module's shared memory with
   a fixed header (read index, write index, under-run count, quanta played),
   and the processor reads it by address through `Atomics`. It never blocks;
   whatever the ring lacks plays as silence, ramped down over 64 frames as the
   desktop callback does, and the quantum is counted. `Output::report_health`
   reads the counters back into the same report the desktop logs.

**Why a worker for the mix, not the page.** The page's thread stalls far longer
than any ring a player would accept: 1.7 to 1.9 s at the boot, 0.55 s on the
first menu, 0.72 to 0.78 s at the end of an HD race load (the scene build), all
measured in the runs below with `--stalls 100`. A mixer on the page's thread
would starve through every one of them; on the worker the HD walk below counted
**0 under-runs**, the front-end music playing on through the loading screen and
the scene-build stall.

**Why not cpal's `audioworklet` host.** It runs the stream's callback, which is
our Rust, inside the worklet: it instantiates the whole module (10 MB) on the
audio thread, our callback calls `Instant::now`, which panics on this target,
and it builds its own `AudioContext` inside `build_output_stream`, which here
runs after an `await`, outside any gesture. The JavaScript processor above is
85 lines and cannot panic.

**The context.** `web/audio.js` owns the `AudioContext`. The module asks for it
(`oagAudioOpen`) when the game opens its audio, after the pick, and builds its
mixer at the context's own sample rate (48 kHz on a real output, 44.1 kHz on
Chromium's fake one; the mixer resamples every source, as natively). A page
that already had a click (the picker's) starts it at once; otherwise the first
key press, click or touch resumes it. Headless Chromium counts the page as
already activated (`navigator.userActivation.hasBeenActive` is true before any
input, with `--autoplay-policy=user-gesture-required` too), so the browser's
own refusal was not reproduced; the resume itself was: a context suspended
through `oagAudioSetPaused(true)` stopped advancing and ran again on the next
key press. A hidden tab suspends the context: the
ring stays full and the worker idles. `?audio=off` opens no context and runs
the null mixer, as `--no-audio` does natively. The audio settings page works
unchanged: SFX VOLUME at 75 scaled the menu's navigation sounds from a 0.297 to
a 0.223 peak in the tap, exactly 0.75.

### What plays, per title

| Title | Effects | Music |
| --- | --- | --- |
| Pulse PSP | menus, race | **absent**: ATRAC3+, decoded by `ffmpeg` into a file cache natively |
| Pure | not booted in a browser | absent, the same codec |
| Pulse PS2 | front end (race not tried) | front end: soundtrack track 0, in process as natively (45 s tap, -18.5 dBFS, 0 under-runs) |
| HD / Fury | menus, race | front end and race, MP3 through `symphonia` in process |
| 2048, Omega | no package opens in a browser | `atrac9dec` builds into the web module; unreachable |

The absent music logs once, at warn, and every later track goes to debug:
`audio: no music (decoding Data\Music\FEMusic\frontend1.at3: ATRAC3+ and
RIFF-wrapped ATRAC9 are decoded by ffmpeg, which a browser cannot run); this
music stays absent` (`oag_music::at3::NoDecoderHere`). Nothing stands in for
it. HD's MP3 and Omega's ATRAC9 have no decode cache natively either, so
nothing had to move into memory or OPFS. HD's pre-race fly-over is silent by
the disc's own mix (`PreRace` has music at 0, and the fly-over fires no cues),
so a walk that never presses on from it records 60 s of silence, which is not
a fault.

### Verifying it without speakers

`scripts/web-screenshot.py --audio-wav FILE --audio-seconds S` sets the page's
`?audiotap=S`: the worklet copies exactly what it hands the browser, from its
first quantum, to the page in chunks, and the script writes a 32-bit float WAV
and prints the under-run count. Headless Chromium 153, dev build, 2026-10-09,
`--disable-audio-output` (no stream reached PipeWire):

| Run | Length | RMS | Peak | Under-runs |
| --- | --- | --- | --- | --- |
| Pulse PSP, the walk above to Moa Therma White | 75 s | -15.6 dBFS | 0.0 dBFS | 0 of 25,840 quanta |
| HD, the walk above plus Enter at 62 s and thrust from 68 s | 83.2 s | -21.0 dBFS | -3.0 dBFS | 0 of 28,672 quanta |
| The same HD walk on the `dist` build (`just web`) | 80 s | -21.1 dBFS | -3.5 dBFS | 0 of 27,563 quanta |
| Pulse PS2, boot to its front end | 45 s | -18.5 dBFS | -8.2 dBFS | 0 of 15,504 quanta |

Against the desktop's own WAV (`oag-game --dump-audio`, the null backend, so a
tick-exact render): Pulse's race without its music (`ffmpeg` off `PATH`,
holding thrust) reads -10.5 dBFS against -12 to -17 dBFS a 3 s window in the
browser walk's race, whose driving differs; HD's race after the countdown reads
-17 dBFS in both, and HD's front-end music -20.4 dBFS natively against -21 to
-23 dBFS in the browser. The two cannot be compared sample for sample: the
browser's walk is real time and its presses land on different ticks. The
desktop's dump stayed byte-identical through this change (the four sequences
of `--dump-audio` on Pulse and HD, front end and race, hash for hash).

## Movies

The PSP's movies play in the browser since 2026-10-09: on Pulse PSP EU the
boot intro (`Intro.PMF`, capped at 260 frames as natively) and the menu
backdrop (`Backdrop.PMF`, 270 frames, looping), in headless Chromium 153 and
Firefox 155. Natively a movie is transcoded by `ffmpeg` into a lossless AV1
cache and decoded by `re_rav1d` (ADR-0008); a page can run neither, but every
browser it runs in decodes H.264, which is what a `.PMF` holds.

**How.** `movie::open` demuxes the `.PMF` as natively; on wasm, where the
native path would transcode, `movie/webcodecs.rs` builds a `FrameStore` whose
`VideoDecoder` is the browser's WebCodecs `VideoDecoder`, driven through
`web/movie.js`:

1. The H.264 is cut into access units (`oag_video::pmf::access_units`, where
   `frame_count` counts) and each goes in as one `EncodedVideoChunk`, Annex B,
   no `description`: the first unit carries the SPS and PPS in band and is an
   IDR picture, which `movie_access_units_ground_truth.rs` checks on all three
   Pulse PSP movies (Intro 1,200 units, 71 IDR; Backdrop 270, 5; the dev/pub
   reel 260, 9; all `avc1.4d4015`, Main profile, level 2.1).
2. Frames come out in display order and are **counted, not timed**: the n-th
   frame out since a reset is frame n. Up to 8 pictures are in flight (chosen,
   not measured); after the last unit the decoder is flushed so it lets go of
   any it holds back. A loop's wrap or a reopened menu resets it: a new
   decoder, from the first unit.
3. Each `VideoFrame` is copied out as planes (`copyTo`) and closed, and the
   module takes them as I420, the format `upload_frame` and `video.wesl` read
   natively. Chromium hands out `I420`; `NV12` is split into two planes.
   **Firefox 155 hands out `BGRX`** (with `hardwareAcceleration:
   "prefer-software"` too), converted to RGB with **BT.709** limited range
   (its `colorSpace` says `matrix: "bt709", fullRange: false`; the PSP's
   streams signal no colour description, and Chromium's I420 frames report
   `smpte170m`). The module inverts the matrix the frame names
   (`movie/planes.rs`) to recover the stream's own samples, which `video.wesl`
   draws as BT.601 as natively: close to the native frame, not equal to it.
   A frame with no format at all is copied out as RGBA the same way.
4. **Nothing waits.** The decode runs on the browser's threads and its output
   arrives on the page's event loop between frames. The web's `movie::Feed`
   has no worker (natively a thread decodes into its ring): `take_upto` polls
   the store once a frame, and a frame not out yet is `movie::Pending`, retried
   on the next poll. The ring, the epoch and the pacing are the native ones;
   the movie clock is the same `Player` position.

**Why planes and not `copyExternalImageToTexture`.** wgpu 30's web backend has
it, for a `VideoFrame` (`ExternalImageSource::VideoFrame`), but it writes RGBA
the browser colour-converted into an RGBA texture, which `video.wesl` does not
read. The copy keeps one format and one shader, and frames comparable byte for
byte with the native decode; a 480x272 frame is 196 KB, at 30 frames a second.

**Checked, headless, 2026-10-09** (`?log=debug` logs an FNV-1a hash of frame
30's luma plane and the mean of each plane, `movie: <key> frame 30 luma fnv1a
...`; native's from the AV1 cache, and the same from `ffmpeg` on the H.264):

| Frame 30 | Chromium 153 | Firefox 155 | Native |
| --- | --- | --- | --- |
| Intro, luma hash | `8b671cb032854bb6` | `05d5306bfa75b6d8` | `8b671cb032854bb6` |
| Intro, mean Y / Cb / Cr | 98.73 / 142.41 / 95.04 | 98.52 / 141.81 / 95.15 | 98.73 / 142.41 / 95.04 |
| Backdrop, luma hash | `b64ab88564c708a8` | `91fd55cb81c01a29` | `b64ab88564c708a8` |
| Backdrop, mean Y / Cb / Cr | 20.84 / 132.38 / 114.41 | 26.72 / 130.39 / 121.10 | 20.84 / 132.38 / 114.41 |

So Chromium's frames are the native ones, byte for byte, and the index is
right. Firefox's are its RGB round trip: the intro within 0.6 of native on
every plane. The backdrop is darker than video black in places (luma below 16),
which RGB clips at 0 and nothing can recover; converting the native frame to
RGB and back by the same path gives Y 25.7 and Cr 121.0 at best, which is what
Firefox shows. Before the matrix was read off the frame (BT.601 assumed), the
intro's mean luma was 5 low in Firefox. Screenshots over the intro show
its successive screens (SYSTEM STARTUP, CONTROL SYSTEM BIOS, WEAPONS SYSTEMS
ANALYSIS, ENGINE CORE ENABLED), and the backdrop's rays and ship pass move
behind Language Selection and the title, as natively. **Enter skips the intro**
to Language Selection, as natively, in both browsers. The backdrop logged
`rewound after 270 frame(s)` at each wrap, every frame of the loop decoded, in
both. The boot's one stall after the pick (1.1 to 1.4 s, Chromium) is the one
"What is still on the page's thread" names; no new one, and the race in the
walk above still runs at the 60 fps cap. Dropped frames at the display (the
ring's skip-ahead) are not counted.

**A browser that cannot decode it.** A decoder error (a browser built
without H.264, a frame format nothing here reads) is an absence, not a stop:
natively a failed decode is a broken cache file and ends the frame loop, but a
browser may lack the codec altogether. The movie logs `movie: <key> has no
picture in this browser: <why>` once at warn and answers `Pending` from then
on, so its screen draws no picture and the sequence runs on the player's clock
as it would with none. Checked by forcing `avc1.ffffff` in a copy of the
built page: both movies warned once (`NotSupportedError: Unknown or ambiguous
codec name`), the intro screen stayed black, Enter skipped it, and Language
Selection drew with no backdrop.

**Sound.** A `.PMF`'s audio is ATRAC3+, which the web build cannot decode
("Sound"): the picture plays silent, and the log says so once (`audio:
LogoFMV plays silently`, warn, and `audio: not decoded (...)` in the loader
report). Nothing stands in for it.

| Title | Movies in a browser |
| --- | --- |
| Pulse PSP | **play**: intro, backdrop (Chromium byte-equal, Firefox through its BT.709 RGB) |
| Pure PSP EU | **plays**: the dev/pub reel `IntroMovieP1_EU.PMF` after Language Selection, frame 30 `a6112c4ced558bc4` in Chromium, the native hash; its other reels are `.PMF` too and were not reached |
| Pulse PS2 | **absent**: `INTRO*.PSS` is MPEG-2, `BG*.IPF` MPEG-2 IPU |
| HD / Fury | **absent**: Bink |
| 2048, Omega | no package opens in a browser. 2048's `.mp4`s are H.264 (`avc1`, `docs/formats/mp4.md`), which WebCodecs decodes: checked, applies, not wired (they would need their samples fed instead of a `.PMF`'s access units); Omega's front end is HD's, whose boot reel is Bink (and `StudioLiverpool.bik` is not in its archives, `omega-status.md`): absent as HD's |

**Why the PS2 and HD stay absent.** The WebCodecs codec registry
(https://www.w3.org/TR/webcodecs-codec-registry/) names AV1, AVC (H.264), HEVC,
VP8 and VP9 and nothing else: no MPEG-1/2 video and no Bink.
`VideoDecoder.isConfigSupported` agrees in both browsers: `avc1.4d4015`,
`av01.0.04M.08` and `vp09.00.10.08` true, `mpeg2video`, `mp2v`, `mpeg1video` and
`bink` false. Such a movie opens for its shape (frame count, size, rate, so the
sequencing holds) and its sound where it has one, and the loader report says
`no picture: the web build has no decoder for this movie: WebCodecs decodes no
MPEG-2 (PS2) or Bink (HD), and a browser cannot run the ffmpeg transcode`, at
warn. A Rust MPEG-2 decoder would be the way to the PS2's; Bink has no other.
**An AV1 path would not help**: AV1 is only what the native cache holds, and
the cache is made by `ffmpeg`, which a page cannot run.

## What is missing

- **Images over about 2 GB in a browser without synchronous slice reads.**
  The fallback copies the whole image into the module's memory, which wasm32
  caps at 4 GiB. Every browser tried reads slices (Chromium, Firefox, WebKit).
- **The key path beyond Chromium.** Firefox and WebKit were not tried with an
  encrypted image, and the decrypt cost is measured on a dev build only.
- **Vita and PS4 packages.** A package's sibling files cannot be found beside
  a picked file; only CHD and ISO (an encrypted PS3 one with its key, "Disc
  keys") go through `oag_disc::mount`.
- **Pulse's and Pure's music.** ATRAC3+ (and RIFF-wrapped ATRAC9) decodes
  through `ffmpeg` into a file cache, and a page can run neither; no Rust
  ATRAC3+ decoder exists and that decision is deferred. Logged once, never
  faked ("Sound").
- **Sound in Firefox and WebKit, and on a real output device.** The worklet ran
  in headless Chromium on its fake sink only.
- **The PS2's and HD's movies**, and every movie's sound ("Movies"). The
  PSP's play.
- **Ghosts and pilot files** last one tab; settings and records persist.
- **Untested inputs.** gilrs's Gamepad API backend is compiled in and not tried
  with a pad; touch is not tried. The keyboard and the mouse work.
- **Other titles in the browser.** Pulse PSP and HD race (Chromium and
  Firefox, headless); Pulse USA PSP races (Chromium, 2026-10-09); Pulse PS2
  and Pure PSP play through the front end and race (the maintainer's own play,
  2026-10-10; browser not recorded). HD drew its race with no Tint error on the one circuit tried.
- **Firefox draws slowly**: 9 to 14 fps in a race in headless Firefox 155,
  against Chromium's 60 on the same machine, and the same before threads
  (Pulse, Moa Therma White: 12.2 fps on the single-threaded module, 12.4 on
  the threaded one). Not investigated.
