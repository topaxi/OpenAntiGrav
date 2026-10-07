# Android

A first pass: `oag-game` as an arm64 NativeActivity app, written to open a window,
bring up wgpu on Vulkan and reach the front end, reading the player's own disc
images from the app's files directory. **It has been built, linked, packaged and checked
as an APK; it has not been run on a device or an emulator**. Touch controls for a race, a gamepad story and lifecycle polish are
open; "Known gaps" lists them.

## Build

```sh
# One-time, user-local: the rust target and cargo-ndk.
rustup target add aarch64-linux-android
cargo install cargo-ndk

# One-time: an NDK next to the SDK you already have (ANDROID_HOME).
sdkmanager "ndk;27.3.13750724"

just apk                      # or: scripts/build-apk.sh [--version V] [--debug]
```

Output: `target/apk/OpenAntiGrav-<version>-android-arm64.apk` (about 9 MB; the
stripped library is 22 MB, deflated). The script needs `ANDROID_HOME` with
build-tools and one platform (it takes the newest of each), and finds the newest NDK
under `$ANDROID_HOME/ndk` unless `ANDROID_NDK_HOME` names one. It signs with a
sideload key it generates once at `~/.android/oag-debug.keystore` (or
`$OAG_APK_KEYSTORE`; never committed): a sideload build, not a store build. The key
sits outside the checkout so every checkout and worktree on one machine signs alike;
until 2026-10-07 it lived under `target/apk/`, so an APK built in a worktree could
not be updated from the main checkout once the worktree was gone. Every build
machine's key still differs (a GitHub release's APK included), so an APK from another
machine will not install over this one without uninstalling first - and
uninstalling deletes the app's files directory, game data included.

The script ends by unpacking the APK and running `scripts/check-leakage.py --dir`
over it, the same audit every other release artifact gets. The APK holds a
manifest, one library and the signature, nothing else.

## Install on a phone

1. Enable developer options, then USB debugging, and connect the phone.
2. `adb install -r target/apk/OpenAntiGrav-*-android-arm64.apk`
   (or copy the file over and open it with "install unknown apps" allowed for the
   file manager).
3. Copy a disc image the player owns into the app's files directory. The app creates
   it on first launch; `adb shell mkdir -p` makes it earlier:

   ```sh
   D=/sdcard/Android/data/org.openantigrav.game/files/data/images
   adb shell mkdir -p $D
   adb push pulse-psp-eu.chd $D/
   ```

   Image names are the same ones the desktop search path recognises
   (`pulse-psp-usa.chd`, `pulse-psp-eu.chd`, `pure-psp-eu.chd`, and so on; see
   `data/README.md`). A multi-gigabyte `adb push` takes minutes; the files directory
   is app-private on Android 11 and later, so a file manager cannot see it, but
   `adb` can.
4. Launch "OpenAntiGrav". Logs: `adb logcat -s oag`, and the same lines with
   timestamps in `files/state/oag/logs/oag-game.log` (`adb pull`).

From a checkout, `just deploy-android` does step 2 (it builds the APK first;
`--skip-build`, `--serial S`, `--dry-run`), and `just push-data android` does
step 3: an fzf multiselect over every disc image, DLC zip, unpacked 2048 or
Omega package and Pure's DLC key table, each row marked `on device`, `partial`
or `-`, copied into `files/data/oag/`, the app's `XDG_DATA_HOME` (both that and
`files/data/images` are searched). Only missing or changed files go over; globs
skip the picker (`just push-data android 'images/pulse-psp-*'`). See
[packaging.md](packaging.md#running-it-on-a-steam-deck) for what is and is not
offered.

## How it is wired

- **Entry.** `crates/game/src/main.rs` carries `android_main`, and Cargo builds the
  same file a second time as the `oag_android` example with `crate-type = ["cdylib"]`
  and `required-features = ["android"]`, which `build-apk.sh` turns on. That is the
  cost of the windowed `App` living in the binary's own modules: a bin cannot be a
  cdylib, `cargo rustc` refuses to mix the two crate types, and nothing may depend on
  `oag-game`, so the second crate root is `main.rs` itself. **Cargo prints "file
  found to be present in multiple build targets" on every invocation in the
  workspace because of it.** Moving the `App` into the lib would remove the warning
  and the example; that is the real fix and is open (handover thread).
- **NativeActivity rather than GameActivity.** No Java: the APK is a manifest and one
  `.so` (`android:hasCode="false"`), buildable with `aapt2`, `zipalign` and
  `apksigner` alone. GameActivity wants the AndroidX games-activity AAR, Gradle and a
  Java class, which buys better IME (the on-screen keyboard) and input; it matters
  once text entry on the phone does, not for a first pass. winit's
  `android-native-activity` feature selects the backend.
- **No command line, no working directory.** `android_main` points `HOME` and the
  `XDG_*` roots at subdirectories of the app's external files directory and makes that
  directory the current one, so `data/images`, `data/cache`, `data/dlc`, settings,
  records, ghosts and the log all land under
  `.../org.openantigrav.game/files/` with no change to `oag-source`. (`dirs` reads
  `$HOME` and `$XDG_*` on Android as on Linux.) `oag-source` should grow a proper
  Android search path so this does not depend on a changed working directory.
- **Surface lifetime.** Android destroys the native window on every `Suspended`.
  `App::suspended` stops drawing; the next `Resumed` calls `Gpu::recreate_surface`,
  which opens a new window and a surface from the *same* wgpu instance and reconfigures
  it. Desktop never receives either event, so its behaviour is untouched.
- **Logging.** `oag-log`'s tee also feeds logcat (tag `oag`) on Android, behind the
  terminal's filter, since stderr goes nowhere there.
- **Audio.** `cpal` selects its AAudio backend on Android (the library links
  `libaaudio`, which is why the minimum API level is 26, Android 8). No Oboe, no
  `libc++_shared.so`: the library's only dependencies are `libandroid`, `libaaudio`,
  `liblog`, `libdl`, `libm` and `libc`.
- **Missing tools degrade.** The movie and ATRAC paths shell out to `ffmpeg`, which a
  phone does not have. A desktop run with `PATH` emptied (`--no-audio --screenshot
  --until "Language Selection"`) still reaches the Language Selection screen, which is
  the nearest check available without a device.

## What was verified

| Step | How | Result |
| --- | --- | --- |
| Compiles for `aarch64-linux-android` | `cargo ndk -t arm64-v8a -P 26 build -p oag-game --example oag_android --features android --release` | builds and links; `android_main` and `ANativeActivity_onCreate` are exported |
| Package is valid | `aapt2 dump badging`, `apksigner verify` | package `org.openantigrav.game`, min SDK 26, arm64-v8a, launchable `NativeActivity`, signature verifies |
| No game content | `check-leakage.py --dir` over the unpacked APK | clean |
| Front end without `ffmpeg` | desktop run, empty `PATH` | Language Selection reached |
| On device or AVD | **not done** | this machine has no system image, and an arm64 library cannot run on an x86 AVD |
| AArch64 determinism | `cargo ndk -t arm64-v8a -P 26 test -p oag-core --test determinism --no-run` builds | **not run**: no `qemu-aarch64`, no device |

On a phone, with `adb` connected, the determinism check is:

```sh
CARGO_TARGET_AARCH64_LINUX_ANDROID_RUNNER=cargo-ndk-runner \
  cargo ndk -t arm64-v8a -P 26 test -p oag-core --test determinism
```

(the runner pushes each test binary to `/data/local/tmp` and runs it over `adb`). It
must match the committed hashes; if it does not, find the bug and never update the
constants (CLAUDE.md, "Determinism").

## Known gaps

- **A race cannot be driven.** The menus answer touch through the pointer layer, but
  there are no on-screen race controls, and `Controls` has no gamepad mapping checked
  on Android (`gilrs` builds but is not exercised).
- **Xclipse 940 (the S24's EU GPU, Samsung's Vulkan driver) is untested.** Expect
  surprises in present modes, `PRIMARY` backend adapter selection and the temporal
  upscaler's capability probe.
- **Lifecycle polish.** Audio is not paused on `Suspended`; the race is not paused;
  a rotation or a multi-window resize is untested.
- **ELF alignment.** NDK 27 links with 4 KB pages; a 16 KB-page device wants
  `-Wl,-z,max-page-size=16384`. `zipalign -P 16` is already applied to the archive.
- **No app icon, no `MANAGE_EXTERNAL_STORAGE` flow**, so images must be pushed with
  `adb`; a launcher-side picker is future work.
- **The front end draws at a desktop window size** (`settings.window_size`); the
  phone's aspect ratio goes through the same fit as a desktop window resize.
