# The web build races in Chromium and Firefox

2026-10-09, lanes `web-pages`, `web-next`, `web-load`, `web-audio` then `web-video`. `just web` builds
`oag-game` for `wasm32-unknown-unknown` + WebGPU into `target/web/dist`,
threaded (pinned nightly, `-Z build-std`, `+atomics`, shared memory);
`pages.yml` builds it and deploys it to Cloudflare Pages (`oag.topaxi.com`,
COOP/COEP from `web/_headers`), the only host since GitHub Pages was dropped.
Pulse PSP EU and HD EU race in headless Chromium 153 and Firefox 155; the race
load runs on a Web Worker reading the disc through `FileReaderSync`, so the
loading screen draws during it (HD Talon's Junction: about 9.5 s frozen
before; after, 1.5 to 1.8 s on the worker, `Race::start` included, then a 0.7
to 0.85 s scene-build stall, since 2026-10-10 spread over frames).
The image is read a slice at a time on the page's thread (synchronous XHR
behind a 64 MiB block cache). Settings and records persist in `localStorage`.
Sound plays through an `AudioWorklet` reading a ring in shared memory that a
worker renders into (0 under-runs over an HD walk to a race, through the
scene-build stall); HD's MP3 music plays, Pulse's/Pure's ATRAC3+ is absent.
Architecture, measurements, hosting and limits:
[docs/tools/web.md](../../docs/tools/web.md).

## Open

- **Defaults and options (2026-10-09, web-defaults lane)**: first-run values
  (60 fps, anisotropy off, shadows off) and the 720-line cap are chosen, not
  measured; dynamic resolution is not on. Untried: the fullscreen row in a
  headed browser (Chromium headless entered and left it), the CANVAS SIZE row
  on a phone, the picker's table on Safari. The nightly's `web` job has not run
  on GitHub (`actionlint` clean, `act -n` plans only its first job).
- **The deploy has not run on GitHub**, and the threaded build has never run
  in CI: the nightly install step (`dtolnay/rust-toolchain@master` with
  `nightly-2026-10-08` and `rust-src`) is untried. `act` fails on this podman
  host before any step.
- **The page's thread after the freeze fixes (2026-10-10, web-freezes lane)**:
  the end of an HD load, Escape from a race and the boot were profiled and
  fixed where the cause was ours (docs/tools/web.md, "Freezes after the
  threads"). ~~The scene build's upload stall~~ and ~~Escape's 0.5 to 0.6 s~~
  closed; what is left: (1) 0.4 to 0.55 s between frames at the end of the
  load while the browser compiles 80 pipelines, the page's thread idle;
  (2) the boot's about 1 s: 307 synchronous requests (38 MiB), the front-end
  music's MP3 decode inline in `Audio::start_music` (0.16 to 0.24 s), inflate;
  moving the music decode onto the music worker is the cheapest next piece, the
  boot's loads touching the GPU keep the rest on the page's thread;
  (3) the menus' own flyer previews rebuild on every return to Cell Selection
  (`oag_game::preview`, another lane's file).
- **No texture sink on the web**: textures wait on the CPU until the scene
  build. 876 to 908 MiB of module memory over four HD loads in one tab; Omega's Tech De Ra (2.3
  GiB of BC7 at peak) would not fit in 4 GiB, though no PS4 package opens in
  the browser. A channel sink (worker decodes, page uploads under a budget)
  would bound it.
- **Firefox raced at 9 to 14 fps** headless against Chromium's 60, the same
  before threads (Pulse 12.2 fps single-threaded, 12.4 threaded). The web loop
  now waits on `requestAnimationFrame` alone instead of winit's `Poll` and
  `WaitUntil` (a `postTask` plus `AbortController` per wake, about 25% of the
  page's thread in a Chromium race; idle 38.9% to 93.1%), and the maintainer
  reports Firefox improved a lot headed (2026-10-10). Not re-measured
  headless; the 60 cap on a display that does not divide 60 (144 Hz runs about
  72) is unverified.
- **Not tried in a headed browser or on another machine.** Safari, Firefox on
  Windows, Chrome on Windows/macOS/Android are unchecked.
- **Pure and PS2 beyond the front end.** Pulse PS2 was booted to Language
  Selection only; Pure reaches its Press Start title and no further.
- **Vita and PS4 packages** cannot open: a package's sibling files cannot be
  found beside one picked file. (An encrypted PS3 image opens with its key,
  which the page takes as a dropped or picked file or as hex; the key path is
  untried in Firefox and WebKit.)
- **Movies (2026-10-09, web-video lane)**: the PSP's `.PMF`s play through
  WebCodecs (Pulse intro and backdrop, Pure's dev/pub reel; Chromium's frames
  byte-equal to native). Open: the PS2's MPEG-2 (`.PSS`, `.IPF`) and HD's Bink
  have no WebCodecs codec and stay absent (a Rust MPEG-2 decoder is the only
  way to the PS2's); every movie is silent (ATRAC3+; HD's Bink audio needs the
  file cache a page lacks); Firefox's frames go through its BT.709 `BGRX` and
  back, not byte-equal (the backdrop's below-black samples clip); display-side dropped frames are not counted; Pure's other
  reels and a headed browser are untried.
- **Pulse/Pure music (ATRAC3+) is absent in the browser**: `ffmpeg` only, and
  the Rust-decoder decision is the maintainer's, deferred. Pulse PS2's front
  end plays its soundtrack in the browser; its race and Pure were not tried.
- **Locks the page shares with a worker**, audited for the music fetch now on
  a worker: the disc block cache (`try_lock`), the mount table and the mixer
  (`oag_thread::lock`, spinning); each fetch opens its own `DiscImage`. Not
  audited: `OnceLock`/`LazyLock` initialisers a worker and the page could hit
  together (a wait there traps the page too).
- **Sound is checked in headless Chromium's fake sink only** (`--audio-wav`):
  not on a real output device, not in Firefox or WebKit. The context starts on
  the first key/click if the browser wants a gesture; not tried by hand.
- **Ghosts and pilot files are not persisted** (binary, a directory; still
  `std::fs`). Records persist through the same seam as settings but were not
  exercised in a browser (needs a finished race).
- **Gamepad and touch** are compiled in and untried.

## Next Steps

1. Run `Pages (Cloudflare)` by hand; open https://oag.topaxi.com/ in desktop
   Chrome, check `crossOriginIsolated` is true in the console, pick
   `hdfury-ps3-eu-dec.iso` and start a race: the loading screen's bar should
   fill while it loads.
2. Disable GitHub Pages in the repository settings (Settings, Pages) if it was
   ever switched on.
3. Plug in a pad and drive one race in that tab, with sound on: the HD front
   end's music should start once the page has had a click.
