# The Android APK builds, packages and passes the leak audit, and has never run

`just apk` produces `OpenAntiGrav-<v>-android-arm64.apk` (NativeActivity, arm64,
minimum API 26, about 9 MB, no game content) and `release.yml` has an `android` job
for it. Evidence and the build are in [`docs/tools/android.md`](../../docs/tools/android.md).
Nothing has run on a device or an emulator: no system image on the development
machine, and the maintainer's phone is the first test. Target phone: Galaxy S24, EU
model, Exynos 2400 with an Xclipse 940 GPU (AMD RDNA3-based, Samsung Vulkan driver),
not Adreno.

## Open

- **First on-device run.** Expect the first failures at Vulkan adapter or surface
  creation (`adb logcat -s oag`, and `files/state/oag/logs/oag-game.log`). What to
  check in order: the launcher finds `files/data/images/*.chd`; the surface
  configures; Language Selection draws; a touch moves the menu cursor.
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
- **16 KB pages and an icon.** `-Wl,-z,max-page-size=16384` for 16 KB-page devices;
  an app icon (the `--write-icon` rasteriser exists).
- **GameActivity** if on-screen keyboard text entry matters.
- **Omega and 2048 check.** Nothing title-specific moved; the packaging is title-agnostic
  (checked, applies to every title, no per-title wiring).

## Next Steps

1. `just apk`, `adb install`, push `pulse-psp-eu.chd`, launch, read the log.
2. Fix what the first run shows, in the order above.
3. Touch controls for racing (left half steering, right half buttons), through the
   existing pointer layer.
