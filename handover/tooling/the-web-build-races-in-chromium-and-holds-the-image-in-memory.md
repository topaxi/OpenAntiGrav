# The web build races in Chromium and holds the whole image in memory

2026-10-09, lane `web-pages`. `just web` builds `oag-game` for
`wasm32-unknown-unknown` + WebGPU into `target/web/dist`; `pages.yml` deploys it.
Pulse PSP EU boots to the front end, the menus answer the keyboard and the
mouse, and a race on Moa Therma White runs at the 60 fps cap in headless
Chromium 153. Architecture, verification and limits:
[docs/tools/web.md](../../docs/tools/web.md).

## Open

- **`pages.yml` has never run on GitHub.** `act` fails on this podman host
  before any step (`statat var/run/act: path escapes from parent`); the job's
  steps were run in a clean `rust:1.99-bookworm` container instead. The first
  manual run after Pages is enabled is the real check.
- **Not tried in a headed browser or on another machine.** Only headless
  Chromium on the maintainer's Linux box. Safari, Firefox on Windows, Chrome on
  Windows/macOS/Android are unchecked, and so is the frame rate anywhere real.
- **Images over about 2 GB.** The picked file is copied whole into wasm memory
  (4 GiB cap, shared with the game). The fix is the Worker architecture: the
  game in a Web Worker with an `OffscreenCanvas`, reading `File.slice()`
  through `FileReaderSync` behind an `oag_disc::mount::Blob`. It needs `App`
  driven without winit, which cannot run in a worker (its web runner calls
  `web_sys::window()`), so window events would come from the page by
  `postMessage`.
- **No sound, no movies.** Audio is `--no-audio`; the AV1 decoder does not build
  for wasm32 (`re_rav1d` uses `libc` items the target lacks); the movie and
  audio caches are files made by `ffmpeg`. Options: WebCodecs for H.264/AV1,
  the Web Audio API through cpal's wasm backend, and the decode caches in
  memory or OPFS.
- **Persistence.** Settings, records, ghosts and campaign progress are lost on
  reload (`std::fs` is `Unsupported`). IndexedDB or OPFS behind the existing
  load/save seams.
- **Other Tint uniformity errors.** Only the paths a Pulse PSP race reaches
  were compiled by Chrome; HD, 2048 and Omega shaders and the post chain's other
  modes may hold more `textureSample`-in-non-uniform-branch cases that naga
  accepts. A naga-side check would need its uniformity analysis to match Tint's.
- **Gamepad and touch** are compiled in (gilrs Gamepad API backend, winit
  touch) and untried.
- **The audio health warning repeats** on the null mixer in the console once a
  second during a race (`106 voice(s) refused ... of 0 buffer(s)`); worth
  checking whether native `--no-audio` does the same.

## Next Steps

1. Enable Pages (Settings, Pages, source "GitHub Actions"), run `Pages` by
   hand, open the URL in desktop Chrome, pick `pulse-psp-eu.chd`.
2. Plug in a pad and drive one race in that tab.
3. Then the Worker architecture, which lifts the size limit and keeps the page
   responsive during the inline race load.
