# The web build runs in Chromium and deploys to two hosts

2026-10-09, lanes `web-pages` then `web-next`. `just web` builds `oag-game` for
`wasm32-unknown-unknown` + WebGPU into `target/web/dist`; `pages.yml` builds it
once and deploys it to GitHub Pages and to Cloudflare Pages (`oag.topaxi.com`,
with COOP/COEP from `web/_headers`). Pulse PSP EU boots, menus and races at the
60 fps cap in headless Chromium 153; the image is read a slice at a time
(synchronous XHR on a blob URL of `file.slice`, behind a 64 MiB block cache), so
Wipeout HD (2.2 GB ISO) boots to its front end and Pulse PS2 (3.7 GB CHD) to
Language Selection. Settings and records persist in `localStorage`.
Architecture, measurements, hosting and limits:
[docs/tools/web.md](../../docs/tools/web.md).

## Open

- **Neither deploy has run on GitHub.** `act` fails on this podman host
  before any step (`statat var/run/act: path escapes from parent`); the build
  job's steps ran in a clean `rust:1.99-bookworm` container, and the Cloudflare
  side was checked against `npx wrangler pages dev` locally (headers, a race,
  `crossOriginIsolated` true). The first manual run is the real check, for the
  wrangler-action step and the `cloudflare-pages` environment it creates above
  all.
- **Not tried in a headed browser or on another machine.** Only headless
  Chromium on the maintainer's Linux box (Firefox 155 and WebKit 26.6 ran the
  slice reader alone; they have no WebGPU here). Safari, Firefox on Windows,
  Chrome on Windows/macOS/Android are unchecked, and so is the frame rate
  anywhere real.
- **HD, Pure and PS2 beyond the front end.** HD and Pulse PS2 were booted to
  their front ends, not raced; Pure not booted. HD's race shaders are the
  likeliest to meet another Tint uniformity error (`textureSample` in a
  non-uniform branch, which naga accepts).
- **Encrypted PS3 images, Vita and PS4 packages** cannot open: a disc key or a
  package's sibling files cannot be found beside one picked file.
- **No sound, no movies.** Audio is `--no-audio`; the AV1 decoder does not build
  for wasm32 (`re_rav1d` uses `libc` items the target lacks); the movie and
  audio caches are files made by `ffmpeg`. Options: WebCodecs for H.264/AV1,
  the Web Audio API through cpal's wasm backend, and the decode caches in
  memory or OPFS.
- **Ghosts and pilot files are not persisted** (binary, a directory; still
  `std::fs`). OPFS's synchronous access handles exist only in workers, so on
  the page this is IndexedDB with an async write-behind, or base64 in
  `localStorage` for small files. Records persist through the same seam as
  settings but were not exercised in a browser (needs a finished race).
- **Threads** (out of scope so far): docs/tools/web.md's "Threads" lists
  the five steps (nightly `-Zbuild-std` with atomics, workers as threads, no
  main-thread blocking, dropping wgpu's fragile-send-sync feature, a GitHub
  Pages fallback).
- **The game in a Worker** would keep the page responsive while a race loads;
  it needs `App` driven without winit (winit 0.30's web runner calls
  `web_sys::window()`). It no longer buys image size.
- **Gamepad and touch** are compiled in (gilrs Gamepad API backend, winit
  touch) and untried.
- **The audio health warning repeats** on the null mixer in the console once a
  second during a race (`106 voice(s) refused ... of 0 buffer(s)`); worth
  checking whether native `--no-audio` does the same.

## Next Steps

1. Enable GitHub Pages (Settings, Pages, source "GitHub Actions"), run `Pages`
   by hand, and check both jobs deploy; open https://oag.topaxi.com/ and the
   GitHub URL in desktop Chrome, pick `pulse-psp-eu.chd`, and in the Cloudflare
   tab's console check `crossOriginIsolated` is true.
2. Plug in a pad and drive one race in that tab.
3. Race HD in the browser and fix any Tint error the console names.
