# The web build races in Chromium and Firefox

2026-10-09, lanes `web-pages`, `web-next`, `web-load` then `web-audio`. `just web` builds
`oag-game` for `wasm32-unknown-unknown` + WebGPU into `target/web/dist`,
threaded (pinned nightly, `-Z build-std`, `+atomics`, shared memory);
`pages.yml` builds it and deploys it to Cloudflare Pages (`oag.topaxi.com`,
COOP/COEP from `web/_headers`), the only host since GitHub Pages was dropped.
Pulse PSP EU and HD EU race in headless Chromium 153 and Firefox 155; the race
load runs on a Web Worker reading the disc through `FileReaderSync`, so the
loading screen draws during it (HD Talon's Junction: about 9.5 s frozen
before; after, 1.5 to 1.8 s on the worker, `Race::start` included, then a 0.7
to 0.85 s scene-build stall).
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
- **The scene build still stalls the page** 0.7 to 0.85 s at the end of an HD
  load (GPU work; `Race::start` already moved to the worker): `Scene::new`'s
  `writeBuffer` uploads (about half of it) could go up under a per-frame
  budget. Escaping a race to the menus freezes 0.5 to 0.6 s, not looked at.
- **The boot** still freezes about 2 s after the pick (Chromium), and the menus'
  reads are synchronous requests on the page's thread. Moving the boot onto a
  worker is the same pattern as the race load, if its result is `Send`.
- **No texture sink on the web**: textures wait on the CPU until the scene
  build. 876 to 908 MiB of module memory over four HD loads in one tab; Omega's Tech De Ra (2.3
  GiB of BC7 at peak) would not fit in 4 GiB, though no PS4 package opens in
  the browser. A channel sink (worker decodes, page uploads under a budget)
  would bound it.
- **Firefox races at 9 to 14 fps** headless against Chromium's 60, the same
  before threads (Pulse 12.2 fps single-threaded, 12.4 threaded). Not
  investigated: Firefox's WebGPU or the headless compositor.
- **Not tried in a headed browser or on another machine.** Safari, Firefox on
  Windows, Chrome on Windows/macOS/Android are unchecked.
- **Pure and PS2 beyond the front end.** Pulse PS2 was booted to Language
  Selection only; Pure reaches its Press Start title and no further.
- **Vita and PS4 packages** cannot open: a package's sibling files cannot be
  found beside one picked file. (An encrypted PS3 image opens with its key,
  which the page takes as a dropped or picked file or as hex; the key path is
  untried in Firefox and WebKit.)
- **No movies.** The AV1 decoder does not build for wasm32 (`re_rav1d` uses
  `libc` items the target lacks) and the movie cache is files made by
  `ffmpeg`; HD's Bink movies' audio goes through the same cache. Options:
  WebCodecs for H.264/AV1, the cache in memory or OPFS.
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
