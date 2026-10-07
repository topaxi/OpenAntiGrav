# The Android APK builds, runs in Waydroid, and awaits its first phone run

`just apk` produces `OpenAntiGrav-<v>-android-arm64.apk` (NativeActivity, arm64,
minimum API 26, about 9 MB, no game content) and `release.yml` has an `android` job
for it. Evidence and the build are in [`docs/tools/android.md`](../../docs/tools/android.md).
Nothing has run on a device or an emulator: no system image on the development
machine, and the maintainer's phone is the first test. Target phone: Galaxy S24, EU
model, Exynos 2400 with an Xclipse 940 GPU (AMD RDNA3-based, Samsung Vulkan driver),
not Adreno.

## Open

Landed 2026-10-07 (android-startup lane, see `docs/tools/android.md`, "Startup on a
phone"): a pictureless movie leg is skipped on every platform and the counter is
`--overlay` only; AAudio falls back from a refused `f32` to `i16`; no disc image
shows the chooser's not-found screen with the exact `adb push` path; launcher icon
mipmaps; an x86_64 library in the APK for Waydroid; `--dry-run --prefetch`, the cache
manifest and `cache/<image>` rows in `just push-data`.

- **First on-device run.** The maintainer's phone is still the first real test.
  Expect failures at Vulkan adapter or surface creation on the Xclipse 940
  (`adb logcat -s oag`, and `files/state/oag/logs/oag-game.log`). Check in order:
  the launcher finds `files/data/images/*.chd`; the surface configures; the intro
  is skipped (or plays, with the pushed cache); Language Selection draws; a touch
  moves the menu cursor; the audio line reads `using i16` or no refusal at all.
- **Touch controls and a pad, landed 2026-10-07** (android-controls lane; see
  `docs/tools/android.md`, "Back, touch controls and a gamepad"): system Back is the
  front end's Back and never quits; an on-screen stick and buttons for a running race,
  hidden when a pad speaks; Android pad buttons and the left stick through the desktop
  pad table. All chosen, not measured, driven on Waydroid with one finger only.
  **Touch-go lane, 2026-10-07:** GO carries airbrake corners (bottom-left L,
  bottom-right R, dead centre column; `[controls] touch_go_zones`, default on, menu row
  GO BRAKE ZONES), the overlay was re-laid out clear of Pulse's and HD's HUD
  readouts with safe margins and height-scaled targets. Driven on Waydroid with one
  finger. Still open: **tune the layout on the S24** (real thumb reach, whether
  0.05h is enough for its cutout - the real cutout is not queried, left-handed
  mirror). **Touch-design lane, 2026-10-07:** round translucent buttons with vector
  glyphs, edge/corner layout, `touch_opacity` setting, `--touch-overlay` capture pose,
  overlay gated on the race drawing; checked in captures at 2340x1080, not on the S24.
  **Pad triggers,
  right stick and hat** are not delivered by winit 0.30 (read `MotionEvent` axes via
  `android-activity` or move to GameActivity); no haptics.
- **AArch64 determinism.** `oag-core`'s determinism test builds for the target; run it
  on the phone with the command in the doc and compare to the committed hashes.
- **`android_main` still lives in the `oag_android` example.** Moving the windowed
  `App` and every `main/` module into `oag-game`'s lib (so `android_main` lives in
  the lib and the example goes) was the optional last step of the controls lane and
  was **not started**: it touches about 13,000 lines of the binary's modules
  (`main_body.rs`, the `#[path]` tree, `main.rs`'s rustdoc header) and wants its own
  lane with a behaviour-identical check. The new `touch` module is one more
  `#[path]` module there.
- **Cargo warning on every invocation.** The cdylib is a second target rooted at
  `src/main.rs` (an `[[example]]`), so Cargo warns "present in multiple build
  targets". Moving the windowed `App` and its stages into `oag-game`'s lib removes
  both the warning and the example; `android_main` would then live in the lib.
- **`oag-source` has no Android search path.** `android_main` works around it by
  changing the working directory to the files directory. A real
  `cfg(target_os = "android")` root (and a DLC and package-extract root) belongs in
  `oag-source`, owned by the setup-friction lane.
- **Lifecycle.** ~~Pause audio and the race on `Suspended`~~ (done 2026-10-07, auto-pause
  lane; see `docs/tools/android.md`, "Pausing when the window goes away"). Still open: rotation;
  `cpal`'s AAudio device-lost recovery; whether `Stream::pause` works on the S24's AAudio (only
  Waydroid ran it, and it logged no error).
- **Xclipse 940.** Present modes, the temporal-upscaler capability probe and the
  adapter list (`BACKENDS` is Vulkan-primary) are unmeasured on Samsung's driver.
- **Storage.** Images must be `adb push`ed; a storage-access-framework picker or
  `MANAGE_EXTERNAL_STORAGE` flow would let a player load from the phone.
- **16 KB pages and an adaptive icon.** `-Wl,-z,max-page-size=16384` for 16 KB-page
  devices; the launcher icon is the legacy mipmap, not an adaptive one.
- **Caches for the other titles.** Prefetch (and so the manifest and the `cache/` row)
  knows Pulse's archives only; Pure, HD, 2048 and Omega movies still need `ffmpeg` on
  the device, and a PS2 image's four loose movies are not in the manifest.
- **GameActivity** if on-screen keyboard text entry matters.
- **Omega and 2048 check.** Nothing title-specific moved; the packaging is title-agnostic
  (checked, applies to every title, no per-title wiring).

## Next Steps

1. `just deploy-android`, then `just push-data android 'images/pulse-psp-eu.chd' 'cache/pulse-psp-eu.chd'` (the cache row converts first if it has to), launch, read the log.
2. Fix what the first run shows, in the order above.
3. Install on the S24 and drive a race by touch: `just deploy-android` (pick
   R3CX707JA9T), launch, Racebox, Start. Note thumb reach and which HUD readouts the
   buttons hide, then tune `oag_input::touch::layout`.
4. Pair a Bluetooth pad and check A/B/X/Y/shoulders/Start/Select and the left stick.
