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
- **Race controls.** No on-screen sticks or buttons; no gamepad mapping checked on
  Android. Touch events already reach `Session::pointer`.
- **AArch64 determinism.** `oag-core`'s determinism test builds for the target; run it
  on the phone with the command in the doc and compare to the committed hashes.
- **Cargo warning on every invocation.** The cdylib is a second target rooted at
  `src/main.rs` (an `[[example]]`), so Cargo warns "present in multiple build
  targets". Moving the windowed `App` and its stages into `oag-game`'s lib removes
  both the warning and the example; `android_main` would then live in the lib.
- **`oag-source` has no Android search path.** `android_main` works around it by
  changing the working directory to the files directory. A real
  `cfg(target_os = "android")` root (and a DLC and package-extract root) belongs in
  `oag-source`, owned by the setup-friction lane.
- **Audio and lifecycle.** Pause audio and the race on `Suspended`; check rotation;
  `cpal`'s AAudio device-lost recovery.
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
3. Touch controls for racing (left half steering, right half buttons), through the
   existing pointer layer.
