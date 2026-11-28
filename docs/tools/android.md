# Android

A first pass: `oag-game` as an arm64 NativeActivity app, written to open a window,
bring up wgpu on Vulkan and reach the front end, reading the player's own disc
images from the app's files directory. It has been built, linked, packaged, run on Waydroid and driven through a race
with touch. It has run on a phone: a Galaxy S24 (EU) took the install, the
reinstall with kept data and the audio fallback ("Install on a phone" and
"Startup on a phone" below), and the maintainer reports it running well there
(Xclipse 940, 2026-10-14), and a Galaxy S21 (Mali-G78) has been measured through whole races ("Performance on the Galaxy S21" below). Touch layout tuning and rotation are not measured yet. Lifecycle polish is open; "Known gaps" lists it.

## Build

```sh
# One-time, user-local: the rust target and cargo-ndk.
rustup target add aarch64-linux-android
cargo install cargo-ndk

# One-time: an NDK next to the SDK you already have (ANDROID_HOME).
sdkmanager "ndk;27.3.13750724" "build-tools;35.0.0"   # zipalign -P 16 needs 35 or newer

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

From a checkout, `just deploy android` does step 2 (it builds the APK first, arm64 only when the picked device is arm64 - the x86_64 library is for Waydroid alone;
`--skip-build`, `--serial S`, `--dry-run`), and `just push-data android` does
step 3: an fzf multiselect over every disc image, DLC zip, unpacked 2048 or
Omega package and Pure's DLC key table, each row marked `on device`, `partial`
or `-`, copied into `files/data/oag/`, the app's `XDG_DATA_HOME` (both that and
`files/data/images` are searched). Only missing or changed files go over; globs
skip the picker (`just push-data android 'images/pulse-psp-*'`). See
[packaging.md](packaging.md#running-it-on-a-steam-deck) for what is and is not
offered.

`just launch-android` starts the app and follows `adb logcat -s oag` (cleared first, so only this run's lines); `--stop` forces a cold start, `--no-logs` only launches.

**A run without touch: the launch file.** (On main `d5bfef6` `--race` in it crashed at once; fixed, see "The stack".) Android hands a NativeActivity no command
line, so the app reads one from `files/oag-args.txt` at launch and deletes it
(`oag_mobile::android::take_launch_args`). One argument a line; a line
`OAG_NAME=value` sets that environment variable instead (`OAG_SPLIT_FINISH=0`). It is
read in every build, release included, because a measurement needs a release build. The
directory is app-private from Android 11; on Android 8 to 10 an app holding the storage
permission could also write the file. `adb` writes it:

```sh
F=/sdcard/Android/data/org.openantigrav.game/files
printf '%s\n' $F/data/oag/images/hdfury-ps3-eu-dec.iso --race --autopilot \
    --frame-log --script 40:quit > /tmp/args && adb push /tmp/args $F/oag-args.txt
adb shell am start -n org.openantigrav.game/android.app.NativeActivity
```

`scripts/android-race-phases.sh --serial S --out FILE` does that for a scripted HD
Vineta K race and saves the run's logcat with the battery temperature before and
after; `--cool 270` waits for the phone to cool to 27.0 C first, so interleaved runs
start alike. The `frame-phases` and `frame-log` lines reach logcat (tag `oag`) as on
the desktop.

`just logcat-android` follows the same log without launching anything (`--clear`, `--dump` to print and exit, `-- ARGS` for another filter).

All four recipes talk to one device. `--serial S` or `$ANDROID_SERIAL` names it; with
neither, a single authorised device is used as is, and with more than one (the
phone and Waydroid, say) they ask which - an fzf pick, or a numbered prompt without
fzf - and refuse with the list of serials when not run from a terminal
(`scripts/adb-pick-device.sh`). An `unauthorized` device is never offered.

An installed app signed with another key (built elsewhere, or before the key moved
out of `target/`) cannot be updated in place: `adb install -r` fails with
`INSTALL_FAILED_UPDATE_INCOMPATIBLE`. `just deploy android` then offers to
reinstall, keeping the app's data: everything the app writes lives under
`/sdcard/Android/data/org.openantigrav.game`, which it moves aside on the device,
then uninstalls, installs, starts the app once and copies the kept files back in.
Copied, not moved: a directory carried over from the previous install stays
unreadable to the new one (`Permission denied` on `settings.toml`, no image found,
measured on the S24), while files copied into directories the new install created
work. It asks in a terminal; `--reinstall` skips the
question.

## How it is wired

- **Entry.** `oag_game::app::android_main` (`crates/game/src/app.rs`) hands the
  activity to `oag_mobile::android::prepare` (the storage setup below, and the
  `AndroidApp` kept for `oag_mobile::android::event_loop`); the glue lives in
  `crates/mobile` ([ADR-0065](../architecture/adr/0065-one-mobile-crate-for-every-phone-os.md)).
  The `oag_android` example (`crates/game/src/android.rs`, `crate-type = ["cdylib"]`,
  `required-features = ["android"]`, which `build-apk.sh` turns on) is the cdylib
  NativeActivity loads: a bin cannot be a cdylib and `cargo rustc` refuses to mix
  the two crate types, so it is a target of its own. It holds the `#[no_mangle]`
  `android_main` symbol and one call into the library, so the export does not depend on
  a symbol in an upstream rlib surviving the link. The windowed `App` and every stage
  are `oag_game::app`, shared with the desktop binary (`main.rs`) and the browser
  module ([web.md](web.md)); nothing depends on `oag-game`. The Android target is not
  in the gate: `cargo check -p oag-game --example oag_android --target
  aarch64-linux-android --no-default-features --features android,extensions` type-checks it
  (the target's std installed; `just apk` builds it).
- **Official mods.** `build-apk.sh` stages `assets/mods/official` into the APK's `assets/`
  with an index, and `android_main` unpacks it to `<files dir>/assets/mods/official` on the
  first launch of a build, where `oag_mod::mods_search_path` already looks
  ([packaging](packaging.md), "Official mods"). A player's own mods go in `data/mods` there.
- **NativeActivity rather than GameActivity.** No Java: the APK is a manifest and one
  `.so` (`android:hasCode="false"`), buildable with `aapt2`, `zipalign` and
  `apksigner` alone. GameActivity wants the AndroidX games-activity AAR, Gradle and a
  Java class, which buys better IME (the on-screen keyboard) and input; it matters
  once text entry on the phone does, not for a first pass. winit's
  `android-native-activity` feature (enabled by `oag-mobile`) selects the backend.
- **No command line, no working directory.** `oag_mobile::android::prepare` points `HOME` and the
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
- **Missing tools degrade.** The movie and ATRAC3+ paths shell out to `ffmpeg`, which a
  phone does not have. Every other codec decodes in process with no tool: ATRAC9 (2048
  and Omega, all music and effects), MP3 (HD), Wwise media (Omega), and AAC (2048's
  intro sound track); see `docs/formats/atrac9.md`. A desktop run with `PATH` emptied (`--no-audio --screenshot
  --until "Language Selection"`) still reaches the Language Selection screen, which is
  the nearest check available without a device.

## The stack

NativeActivity runs `android_main` on a thread `android-activity` spawns with the
default 2 MiB stack (`std::thread::spawn` in its `native_activity/glue.rs`), and
winit must poll on that thread: the looper that carries the input queue and the
activity's commands is prepared there and `poll_events` reads `ALooper_forThread()`.
So the game cannot choose the stack its front end and its event loop start on, and
its callbacks (`resumed`, every frame) run on whatever is left of 2 MiB.

**What overflowed.** `--race` from the launch file crashed at once with SIGSEGV, on
Pulse and on HD (measured on the S21, Exynos 2100, Android 14, the `dist` build). A
stack overflow in `naga`'s WGSL parser, on `Stage::race` -> `Scene::new` ->
`create_shader_module`: the parser recurses, and the frames above it were
`app::run` 481,936 bytes, `run_race` 545 KB, `App::open_with` 385 KB,
`Stage::race` 315 KB, `Scene::new` 172 KB and the winit closure 160 KB, about 2.07 MB
before the recursion began. The menu route survived because it never entered
`run_race`, and builds its scene on the `race-build` thread. Upstream knows the
recursion: gfx-rs/wgpu#9497, #9456 and #5759.

**Three things stand in its way now.**

- **A big stack for the scene.** `oag_thread::on_big_stack` runs a closure on a
  thread with `oag_thread::BIG_STACK` (16 MiB) and waits for it. `Stage::race` (the
  `--race` route) builds its scene there, and the `race-build` worker the menus use
  is spawned with the same stack. Every platform takes this path, so a desktop
  `--race` is the one that proves it (below). The thread holds a
  `gpu_workers::Guard`, as every GPU-touching thread does. The browser has no
  second thread and builds in place, as it did.
- **Frames that end before the loop starts.** `app::launch` (the old `run`) and
  `headless::race_launch` (the old `run_race`) build the `App` and return it boxed
  (`Launch::Window`); `app::run_window` then runs the loop from a 1.6 KB frame. Before,
  the 470 KB and 532 KB frames of the two functions stayed under every callback of the
  game. `Session` (160 KB) and the race's `Loaded` (62 KB) travel boxed
  (`App::state`, `open_with`'s result, `Stage::race`, `BuildWorker::spawn`), so the
  winit closure's frame fell from 156 KiB to 2.7 KiB.
- **The build reads the frames.** `scripts/check-android-stack.py` reads every
  function's frame off the unstripped library and `scripts/build-apk.sh` runs it:
  `android_main`, `run_window`, the winit callback and `Stage::race` must each stay
  under 64 KiB, and must still exist under that name (a rename cannot leave one
  unchecked, and a resident function with no prologue found fails too). It follows the
  stack pointer through each function's first 300 instructions (probed frames, a
  prologue after an early exit) and refuses a stripped library. It reads prologues, not
  call depth: a margin, not a proof. `--all` checks every function; `--top N` lists the
  largest; `--selftest` (`just check-android-stack-script`, in the gate) runs the reader
  on disassembly snippets.

Frames in the `dist` build (`cargo ndk -t arm64-v8a -P 26 build -p oag-game --example
oag_android --no-default-features --features android,extensions --profile dist`,
read with `scripts/check-android-stack.py`, KiB):

| Function | Before | After | Where it runs |
| --- | --- | --- | --- |
| `app::run` / `app::launch` | 469.8 | 253.8 (`launch`, gone before the loop) | `android_main` thread, transient |
| `headless::run_race` / `race_launch` | 532.2 | 308.2 (gone before the loop) | transient |
| `run_window` (the loop) | inside `run` | 1.6 | `android_main` thread, resident |
| winit `run_app` closure | 156.1 | 2.7 | resident |
| `Stage::race` | 308.1 | 0.8 | `android_main` thread, while it waits |
| `build_race_stage` | inside `Stage::race` | 308.1 | the 16 MiB thread |
| `Scene::new` | 168.2 | 168.2 | the 16 MiB thread |
| `App::open_with` | 376.1 | 144.1 | `android_main` thread (`Session` is built here) |
| `BuildWorker::spawn` | 188.0 | 64.0 | `android_main` thread |

`open_with` went from 356.1 to 144.1 KiB and `Stage::race` from 60.1 to 0.8 KiB (same `dist` build, read
with `scripts/check-android-stack.py`, before and after the change below). `Session` was 160 KB, and five of its fields were 120 KB of
that, held by value: `photo` (39.6 KB), `net` (33.6), `audio` (20.1), `overlay` (14.0) and `watching` (12.3). They are `Box`ed now
(`Option<Box<_>>` where they were an `Option`), so the literal `open_with` builds before it boxes the `Session` is a quarter of its size.
`App::race` holds the race's `Loaded` (62.9 KB) boxed and `Stage::race` takes it boxed, which took its by-value copy out of
both frames. Nothing is `unsafe`. The remaining 144 KiB is the front end's first stage and the shaders compiled under `open_with`; which of
the stage values (`MenuStage` 34.6 KB, `LoadingStage` 24.8 KB, `FrontendStage` 19.9 KB by `-Zprint-type-sizes`) sit in the frame is not read.
`check-android-stack.py` keeps `open_with` under 160 KiB (`RESIDENT_CEILINGS`) and `Stage::race` under 64 KiB; the 356.1 KiB library fails it.

**A type held by value grows every frame above it** (2026-11-28, nightly run
46091145845). `open_with` reached 392.1 KiB and failed the build. Commit `7e3d1c090`
had added `playing: [Option<Playing>; MAX_PROJECTILES]` (128 slots) to
`TravelVoices`, which `SfxVoices` holds five times, so `oag_sound::Audio`, held by
value in `App`, reached 158,640 bytes (`-Zprint-type-sizes`). `TravelVoices` now keeps
its slots in one `Box<[Slot]>`, with a compile-time 4 KiB ceiling on the struct.
Same `dist` build, KiB:

| Function | Last green nightly | Failing | Fixed |
| --- | --- | --- | --- |
| `App::open_with` | under 160 | 392.1 | 120.1 |
| `app::launch` | 256.0 | 529.1 | 229.1 |
| `headless::race_launch` | 308.2 | 540.2 | 296.2 |

`App::net` was also held by value (33,760 bytes, mostly the client's confirmed
`World`); it is boxed from `NetPlay::connect` on, as `Session::net` already was. That
took `open_with` to 96.1, `launch` to 115.2 and `race_launch` to 228.2 KiB.

No gate recipe builds the Android library: the gate runs only the script's
`--selftest`, and the frames of a real library are read by `scripts/build-apk.sh`.

**On the S21** (Galaxy S21, Android 14, the `dist` APK of this change, media volume 0,
`--no-audio`, `--autopilot`, `--frame-log`, default settings): the launch-file
`--race` finishes the race on Pulse (Talon's Junction, 3 laps, 1:49.35, process alive
throughout) and on HD (Vineta K, 3 laps, 1:51.50), where it was a SIGSEGV four
seconds in. The menu route (Racebox, Single Race, the launch file naming the image and
`--autopilot`) also finishes, on Pulse (4 laps) and on HD: each logs
`saved a new ghost: a lap of N ticks` per lap and ends in `race finished`. The
`--race` route has no front end, so it logs the ghost lines (Pulse 2194 and 2167
ticks, HD 2234 and 2211 ticks) and shows the RACE COMPLETE screen, but no
`race finished` line. The phone's speed varies with its temperature, so this proves a lap
finishes and no rate.

**Reproducing it on a desktop.** The main thread's stack is `ulimit -s`. A windowed
`--race` under a 2 MiB limit is the same situation (inside your own display, with
`--no-audio`):

```sh
(ulimit -s 2048; env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 DISPLAY=:91 \
  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
  target/debug/oag-game --no-audio --race --script 20:quit data/images/pulse-psp-eu.chd)
```

Before the change this printed `thread 'main' has overflowed its stack` for Pulse and
for HD in the `dev` build; with it, both run and quit. Not in the gate: it needs a
display.

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

## Startup on a phone

What a first launch does, as opposed to a desktop run (2026-10-07):

- **A movie with no picture is skipped at once, on every platform.** A phone has
  no `ffmpeg`, so the intro would otherwise be a black screen for the forty
  seconds its 1200 frames take. `Frontend::update` presses Start for a movie leg
  whose plan has no picture (`Input::inject_press`), so the leg leaves through
  the same handler a real press uses; a tap skips it too, as a real intro's does
  (the composition root turns an unhandled click into Start and Cross). The
  `INTRO FRAME n / m` counter is now `--overlay` only, everywhere.
- **No ffmpeg also means no ATRAC3+ sound** (Pulse and Pure), so their front end is
  silent as well as pictureless; 2048, HD and Omega music needs no `ffmpeg`. The fix is not to need ffmpeg on the device: convert on the
  computer and push the result. See "Pre-converted caches".
- **No disc image: the chooser's own screen says so.** With nothing under the
  search path a phone cannot print `resolve`'s message anywhere, so
  `launcher::Launcher::not_found` draws it: "NO DISC IMAGE FOUND" and the
  `adb push <image> /sdcard/Android/data/org.openantigrav.game/files/data/images/`
  line, built from the real files directory. Android only; desktop still exits
  with the terminal message.
- **Audio.** cpal's AAudio on the S24 reports `f32` as the default and then
  refuses it (`IllegalArgument`), which used to leave the whole session silent.
  `oag-audio`'s `build_stream` now tries the device's default and then `i16` and
  `f32`, logging which one worked (`audio: the default f32 stream was refused;
  using i16`). The fallback is a pure ordering (`formats_to_try`) with a test; the
  refusal itself only reproduces on the phone.
- **Launcher icon.** `scripts/build-apk.sh` rasterises `assets/icons` through the
  host's `oag-game --write-icon` (the AppImage's own path) at 48, 72, 96, 144 and
  192 px into `res/mipmap-{m,h,xh,xxh,xxxh}dpi/ic_launcher.png`, compiles them with
  `aapt2` and names them in the manifest (`android:icon`, `roundIcon`). The
  leakage audit allows exactly those five paths. **Not an adaptive icon**: that
  wants a foreground layer drawn into the 66% safe zone and a background colour,
  and the legacy icon is what Android 8+ shows in a plain mask.

## Photos and replays

A phone offers photo mode and records replays: both write into the app's files directory,
`/sdcard/Android/data/org.openantigrav.game/files/data/oag/photos` and `.../replays`, the tree the disc
images are copied into. On a recent Android that directory is hidden from the phone's own file manager,
so a copy is also handed to the phone's shared storage (`oag_mobile::export`):

- **A shot** is added to the gallery as `Pictures/OpenAntiGrav/<name>.png` the moment it is saved, and
  the screen says PHOTO SAVED TO GALLERY. **The gallery then holds the only copy**: the files directory
  is hidden from the phone's file manager, so a second copy there could never be deleted. A shot the
  gallery refused stays in the files directory.
- **A replay** is exported by the REPLAYS page's EXPORT row to `Download/OpenAntiGrav/<name>.oagr`, where
  the file manager and the share sheet of any app reach it. The row is on the page on a phone only.

Both go through `MediaStore` with `IS_PENDING` set while the bytes are written, so the gallery never
shows a half-written file. The calls are made over JNI on the JVM the activity started (the APK still
has no Java: `android:hasCode="false"`), need no permission on Android 10 (API 29) and later, and are
refused with a log line on 8 and 9, where the shot or replay stays in the files directory. **Never run on
a device**: the JNI path type-checks for `aarch64-linux-android` and the decisions (folder, MIME type,
name) are unit tests, but the calls themselves have not been made on a phone. The camera of both modes is a multi-touch camera ([photo mode](../ui/photo-mode.md),
"Touch"; [replays](../ui/replays.md), "Touch"), and the on-screen racing controls are not drawn over
either.

## Back, touch controls and a gamepad

Landed 2026-10-07 (android-controls lane). Everything here is **chosen, not
measured**: no original title has a touch scheme or an Android pad story, so there
is no capture to match and no confidence score to give.

### System Back is the front end's Back

The back gesture or key reaches winit as `Key::Named(NamedKey::BrowserBack)`
(`winit-0.30.13/src/platform_impl/android/keycodes.rs`, `Back`), which no binding
claimed, so it used to be dropped. `app.rs` now sends it where Escape goes, through
`Session::back`: exactly `Session::escape` (pops a page, a picker or a campaign
screen, cancels a prompt or a key capture, and from a race parks it and opens the
menus) except that it **never quits**. At the root page, the disc chooser, the boot
movies and `PRESS START` it does nothing. A QUIT row is on the root for a player who
means it, and an accidental swipe closing the game is worse than a swipe that does
nothing; no quit prompt exists in the front end, and inventing one was not worth it.
That is a choice. In a race Back is the pause *menu* (the race is parked, and
backing out of the menus resumes it); the Start button's own pause freezes the race
with no menu, as before. The manifest sets `android:enableOnBackInvokedCallback="false"`
so a newer target SDK's predictive-back cannot swallow the key. A key capture on the
CONTROLS page treats Back as cancel, like Escape.

Verified on Waydroid (`adb shell input keyevent 4`): from the RACE page Back went to
the root (log line `system back`, at `debug`), at the root and at the Language Selection the
process stayed alive, and in a race it opened the root menu over the parked race.

There is no QUIT row on Android (2026-10-07, maintainer): a phone app is left with
the system's home or app switcher, not quit from inside, and the app is suspended
(race paused) rather than ended. `menu::Definition::drop_quit` removes the row at
boot (`app/prepare.rs`, `oag_mobile::MOBILE`); desktop keeps it.

### On-screen racing controls

`oag_input::touch` holds the layout, the finger tracking and the rules, with no
winit type, so every rule is a unit test; `oag_game::touch_controls` draws it as
round translucent buttons with a vector glyph each (see "Design pass" below);
`crates/game/src/app/touch.rs` is the glue. Shown only on Android (or on any
platform with `OAG_TOUCH_CONTROLS` set, which is how to look at it on a desktop with
a touchscreen), only in a running race, and drawn under the pointer in the overlay
pass.

| Control | Where | Button |
| --- | --- | --- |
| steering stick | a finger landing in the left 40% below the top strip is the stick; where it landed is centre, 16% of the window height is full lock (20% in Easy); X steers, Y pitches | stick axes |
| FIRE | top of the right column, a pill 0.25 wide (over GO's core) and 0.18 of the height tall, a crosshair | `Square` |
| GO | the column's middle, centred at 0.52 of the height, largest (a core 0.25 wide and 0.305 tall; in Standard with the zones on a 0.14 brake wing either side, 0.53 wide), a rounded rectangle | `Cross` (thrust), plus `L`/`R` in its side wings |
| ABSORB | below GO with a 0.03 gap, 0.25 wide (under GO's core) and 0.11 tall (the thinnest), a shield; in Easy with the zones off it is the right half of the bar (0.18 wide, labelled `ABSORB`) | `Circle` |
| BRAKE L, BRAKE R | **only Standard with GO's brake zones off**: BRAKE L upper left, BRAKE R in the thumb's arc left of GO (0.17), a side arrow and `L`/`R`; otherwise neither is drawn nor hit-tested | `L`, `R` (airbrakes) |
| BRAKE (bar half) | **only Easy with the zones off**: the left half of the bar under GO | the airbrake axes, no button |
| PAUSE, VIEW | top-left corner (0.13), beside BRAKE L in Standard with the zones off; pause bars (a play triangle while paused) and a camera | `Start` (pause), `Select` (camera cycle) |

**TOUCH CONTROLS page (2026-10-07, touch-submenu lane, chosen, not measured).** The three touch
rows (TOUCH SCHEME, GO BRAKE ZONES, CONTROLS OPACITY) sit on their own page, `touch_controls` in
`assets/ui/menu.toml`, reached from a TOUCH CONTROLS row at the end of CONTROLS (they were loose
rows on GRAPHICS). Under the rows a **live preview** draws the real overlay: `touch_controls::draw`
for a 2340x1080 phone, in the grid a race draws it in, then `Draw::zoom`ed into a phone-shaped
panel (`touch_controls::preview`). It reads `[controls]` every frame, so a row change shows at
once (Standard/Easy, zones on/off, opacity), and it cycles the same `--touch-overlay` poses for
the chosen setup, 1.6 s each, idle first. The picture behind the panel is the front end's own
backdrop; no art is added. It draws in the overlay pass (the cursor sheet carries the touch art,
the menu's sheet does not). **Visibility:** the row leading to the page is dropped
(`Definition::drop_touch_controls`) unless `touch_controls::available()`: Android, or
`OAG_TOUCH_CONTROLS` set. Chosen: a desktop with no touchscreen has nothing to set, and the env var
is the existing switch. `--menu-page touch_controls` reaches the page on any platform and draws the
idle pose. The rows are ordinary rows, so mouse and touch work as on every page.

**Schemes (2026-10-07, touch-schemes lane, chosen, not measured).** `[controls] touch_scheme`
(CONTROLS > TOUCH CONTROLS, TOUCH SCHEME, `standard`/`easy`, default `standard`; an unrecognised token is
standard) picks how the overlay reads. The simulation is untouched: both feed the existing
analogue `airbrake_left`/`right` axes.

- **GO latches (both schemes).** The finger that has been on GO keeps thrust until it lifts, wherever it
  slides. Sliding it onto FIRE or ABSORB (or Easy's bar BRAKE) also presses that while it is over it. A
  finger that began on FIRE or ABSORB does not latch until it reaches GO. The L/R wings and the
  Easy band read only while the finger is over GO itself. A finger inside a rectangle beats a rectangle
  whose slop it is only in (the 0.03 gap and the nearest-edge rule keep a thumb at GO's bottom from
  being read as ABSORB).
- **The brake carries on past GO's edge (chosen, not measured; the maintainer: the drag "should behave a
  little like the stick").** For a finger latched on GO the pull is read off its position clamped to GO's
  rectangle, not gated on being inside it: past GO's left or right side the pull holds the value it had at the edge
  at that height, above or below GO it holds the value of that column, and thrust stays on. Over the
  core there is none, and coming back inside returns to the analogue value. The same
  holds for Easy's band, whose side still comes from `novice_airbrakes(pull, stick_x)`. A finger dragged
  from a wing onto ABSORB or FIRE (both sit over the core) presses it and loses the brake.
- **Standard's brake wings** (since 2026-10-25, see below) are full height, each 0.14 of the window
  height beside GO's 0.25 core (`GO_WING`, `GO_ZONE_SIDE` = 26% of GO's width, always 0.14 of the window height); still analogue, and the
  pull reads **full at 85% of the way out** (`GO_FULL_DEPTH`).
- **Easy** keeps GO and the right column, and mimics the original's novice scheme
  (`oag_gameplay::controls::novice_airbrakes`, measured, `input-bindings.md`) without being
  measured itself. The stick is a ring of 0.20 of the height with a guide ring at 60%.
  `u` is the drag along X over the radius: steering is linear to full lock at `|u|` = 0.6 and stays 1 beyond;
  the same-side airbrake ramps 0 to 1 from 0.6 to the rim (`touch::easy_curve`):

  | u | stick_x | airbrake (dragged side) |
  | --- | --- | --- |
  | 0.0 | 0.00 | 0.00 |
  | 0.3 | 0.50 | 0.00 |
  | 0.6 | 1.00 | 0.00 |
  | 0.8 | 1.00 | 0.50 |
  | 1.0 | 1.00 | 1.00 |

  GO carries one **BRAKE band**, its bottom 40%, with analogue depth (full at 85%). Its pull goes through
  `novice_airbrakes(pull, stick_x)` unchanged, so it brakes the side the other thumb is steering
  (stick below -0.1 left, above +0.1 right) and **both when centred**. Each side is the larger of the
  stick's rim and the band. With GO BRAKE ZONES **off** there is no band: the bar under GO is split,
  one row `EASY_BAR_W` (0.36) wide, right-aligned with GO, in two equal halves with a divider, BRAKE (left) and ABSORB (right), each labelled with its word, and BRAKE pulls 1.0 the same way (the maintainer's
  decision: separate BRAKE L/R buttons are hard on a phone). A pull is amber-lit inside GO's own outline.
- **Under the simulation's own Novice scheme** (`[controls] scheme`) the sim reads only `airbrake_right`
  and sides it off the steering, ignoring `airbrake_left`; Easy then puts `max(left, right)` on
  `airbrake_right` alone (`touch::fold_for_novice_sim`) so the two agree. Standard is not folded,
  as before: its left corner does nothing under sim Novice.
- Capture poses: `--touch-overlay go-fire|go-left-past|easy-idle|easy-zones-off|easy-brake|easy-bar-brake` beside
  the earlier ones.


**Side wings (2026-10-25, chosen, not measured; the maintainer: L/R "on the left and right side of
the GO button, so that fire and absorb are not accidentally triggered").** The brake corners in GO's
bottom half made a full pull a slide *down*, toward ABSORB, and a thumb resting low on GO braked.
GO is now a 0.36 core (narrowed since, see "GO's size" below) with a 0.14 wing on each side, the left wing `L` and the right `R` at any
height, the pull rising with the slide *outward* and the height ignored. FIRE and ABSORB keep the
core's width and sit over it, and nothing is beside GO (its right edge is the safe margin), so no
braking slide crosses either: `a_full_brake_on_either_side_never_reaches_fire_or_absorb` holds it.
GO widens only in Standard with the zones on; Easy and the zones-off layout keep the bare core.
Each wing draws an outward arrow over its letter, and the lit patch widens from the outer edge. The
paragraph below is the corner design this replaced.

**GO's size (2026-10-26, touch-go-size lane, chosen, not measured; the maintainer: "reduce the width
of the GO button by maybe 25%-33% and the height by 10% - 20%", then "the brake zones should not keep the
proportions, only the height should be reduced there").** GO's core went from a 0.36 square to
`GO_CORE_W` 0.25 wide (-31%) by `GO_CORE_H` 0.305 tall (-15%), about 17 by 20 mm on a 6.2 in phone;
the former `GO_CORE` is split in two because the core is no longer square. The brake wings keep
their 0.14 (`GO_WING`, unchanged), so with the zones on GO is 0.53 wide against 0.64 and `GO_ZONE_SIDE`,
the wings' share of GO's width, rose from 22% to 26% while the wing is the same 0.14 of the height and
the pull ramps over the same distance (full at 85% of it). All of GO, wings included, is 15% lower.
GO's centre stays at 0.52 of the height (it moved from `go_y` 0.34 to a derived one), so the thumb's
resting place is the same; FIRE and ABSORB are the core's width and follow it, 0.25 wide, FIRE sitting
0.0275 lower and ABSORB 0.0275 higher than before. Easy's BRAKE band keeps its 40% share of GO's height
(0.122 tall now, full pull over 0.104). Easy with the zones off is the one place a narrower GO cannot
set the width of what sits under it: the bar's halves carry the words `BRAKE` and `ABSORB` (about 0.13
wide each), so the bar keeps the 0.36 it had (`EASY_BAR_W`), right-aligned with GO and reaching 0.11
left of it. The double chevron is sized off the core (`GO_CHEVRON`, 0.28 of its width, so it is one
size in every layout: 76 px at 2340x1080 against 117) and kept; in Easy with the zones on the word
`GO` moved up (0.44 of GO's height against 0.51) to sit between the chevrons and the band's divider.
Still checked at 2340x1080 for Standard and Easy, zones on and off: nothing overlaps, and the bar
that used to cross the HUD's shield meter at the bottom right now ends above it (0.8125 of the height
against 0.84).

**Dynamic GO (2026-10-07, touch-go lane, chosen, not measured; analogue since the touch-design lane).** The finger's
position *within* GO, tracked while it is held, adds an airbrake: the bottom half of
GO is split into a left corner (42% of its width, `L`), a dead centre column and a
right corner (42%, `R`; see Schemes below for the 2026-10-07 sizes); the centre and everything above is thrust alone. Sliding
between zones needs no lift, and a thumb in the slop below GO still reads the corner.
It is on by default behind `[controls] touch_go_zones` (CONTROLS > TOUCH CONTROLS, GO BRAKE ZONES,
`on`/`off`); off makes GO thrust alone. The separate BRAKE L/R buttons are dropped while the zones are on (the maintainer: "L/R are not necessary when they are embedded in the go button itself") and come back with the setting off, as does pause/view's old place beside BRAKE L. GO draws
the two corners with `L` and `R` and thin dividers, and the corner under a finger lights
amber. **The brake is analogue** (`GoZone::strength`, chosen, not measured): 0 on the zone's inner edge
(the dead centre column) and on its top, 1 in the outer bottom corner, the geometric mean of
the depth on each axis, so sliding down and sliding out both strengthen it; thrust stays
full throughout. It reaches the simulation as `InputSnapshot::airbrake_left`/`right` axes
(`Reading::airbrake_left/right`, merged by `pad::resolve`), not as the `L`/`R` button bits;
the pad and keyboard paths are unchanged. The lit segment grows from 40% of the corner to
all of it and brightens from 22% to 60% as the pull rises. Zones add to the reading only:
they latch no tap. A stick-zone finger that lands
on a control is that control's finger (controls win over the floating stick, which
the upper-left brake buttons need).

**Design pass (2026-10-07, touch-design lane, chosen, not measured).** Sizes are
fractions of the window's height (GO 0.36 then, 0.25 by 0.305 since 2026-10-26, see below; FIRE 0.20,
ABSORB and the brakes 0.17, PAUSE and VIEW 0.13, each with a 0.025h touch slop on every
side), so a button is one physical size on every phone. Everything stays 0.05h in from
the left and right and 0.03h from the top and bottom; the cutout itself is **not
queried** (winit 0.30 has no insets API). The look is the common phone overlay: round
buttons with a thin outline (60% white), a dark 28% body so the white glyph reads on
HD's bright track, and a held state that is a 40% cyan body with a brighter outline,
never opaque. GO is a rounded rectangle, not a circle, so the L and R brake wings
the player presses are inside what is drawn; the lit wing is a translucent amber
clipped to GO's own outline. The stick is a ring with a knob where the thumb landed.
There is **no round `Draw` primitive**, so the shapes are four antialiased textures
generated in code at startup (`touch_controls/art.rs`: disc, ring, triangle, shield, white on
transparent, supersampled) that ride in the overlay renderer's sprite sheet beside the
pointer (`cursor::sheet`). A rounded rectangle is nine cells (four texture quarters for the
corners, five fills); a button is the ring and a disc inset by one ring, which tile the shape,
so nothing is drawn twice - the first design stacked a 45% outline fill under the body, which
read as flat grey and, held, as opaque cyan. A whole overlay is a few dozen draws. (A first
version built the shapes from rows of fills: hundreds of rects a frame and stair-stepped
edges; the lead asked for textures and that is what ships.)
Glyphs are plain shapes from the same textures (double
chevron, crosshair, shield, side arrows, pause bars, a camera body with a lens); the
disc-style prompt glyphs (`oag_ui::prompt`) are a pad's face buttons and none of them
says thrust, shield or pause. The only words are GO and the L and R of the airbrakes.
`[controls] touch_opacity` (CONTROLS > TOUCH CONTROLS, CONTROLS OPACITY, 40/60/80/100) scales every
alpha. The overlay draws only while the race is on the glass (`hud_shown`, so not
through the intro flyby; the loading screen is a different stage and never draws it).
`oag-game --race --screenshot out.png --size 2340x1080 --touch-overlay
idle|idle-buttons|go-left|go-right|stick-go` (and the poses above) draws the overlay in a pose for a
capture (`idle-buttons` is zones off). Pulse's HUD
is 4:3 and HD's about 16:9, so on a 2.17:1 phone most of the controls sit on the
pillarbox bars beside the picture, clear of the readouts.

Multi-touch is the point: every finger is tracked, a stick finger is sticky and a
button finger re-reads what is under it as it slides (a thumb can roll from GO onto
FIRE). Presses are latched through `Controls::tap` so a tap shorter than one tick is
still a press; PAUSE and VIEW act on the edge, so they reach the game only as taps.
While the race is paused only PAUSE (labelled RESUME) gets through. The overlay's
contribution is merged into the keyboard's slot in `Controls::merge_players` after
`pad_spoke` is read, so a finger is never mistaken for a pad. `Triangle` has no
touch control because nothing in a race reads it.

**Hidden when a pad is in use, back on the next touch**: `Controls::pad_spoke` (a
real gamepad, or an Android one) hides it; a touch starting shows it. Verified on
Waydroid: a `KEYCODE_BUTTON_A` event hid it, a tap brought it back.

Waydroid's `adb shell input` injects one pointer, so **two-finger play is covered by
unit tests only** (`oag_input::touch::tests`), not driven end to end. What was
driven: a held tap on GO accelerated the ship (speed readout, plume), a drag in the
stick zone steered it, a PAUSE tap froze the timer and a second resumed it. The
real feel (thumb sizes, how far a thumb slides, whether the buttons cover the HUD
on a 6.2 in phone) needs the S24.

The overlay reads raw per-finger touches in `Session::touch`, not the menus' tap
model (`pointer::Window` reports a tap on lift and a drag past 12 px with no click).
One bug found on the way: any joystick `MotionEvent` arrives as a `Touch` (below).
(A tap that lifted before the next tick also lost its position on Waydroid; main's
tap-on-lift pointer model replaced my fix for that.)

### Bluetooth and USB gamepads

`gilrs` has **no Android backend** (`gilrs-core-0.6.8/src/platform` is Linux,
macOS, Windows and wasm; on Android `Gilrs::new` reports "does not support current
platform" and the pad is keyboard-only, as the log line says). What winit delivers
instead:

- **Buttons** arrive as `KeyboardInput` with `PhysicalKey::Unidentified(NativeKeyCode::Android(code))`,
  the code being `KEYCODE_BUTTON_*`. `oag_input::android::key_of` maps A, B, X, Y,
  L1, R1, START and SELECT through the same `pad::map_button` desktop uses (so A is
  `Cross` whatever the pad prints, per the original's naming), and L2/R2 as a full
  trigger pull. The d-pad arrives as arrow keys and is the keyboard's. An Android
  pad press also latches a tap, so one shorter than a tick still reaches a menu.
- **The left stick** arrives, by accident, as `Touch::Moved`: winit turns every
  `MotionEvent` with action `Move` into a touch at `pointer.x()`/`y()`, which for a
  joystick are `AXIS_X`/`AXIS_Y` in -1 to 1. A device that never began a touch is
  therefore a pad (`crate::touch::Overlay::route`) and its moves go to
  `Controls::android_stick`, not to the menus' pointer (without this a stick push
  would teleport the finger to the screen's corner).
- **Not reachable**: the analog triggers, the right stick and the hat are never
  delivered by winit 0.30. A pad that also sends `BUTTON_L2`/`R2` as keys (most do)
  still works as a digital pull; one that is axis-only has no triggers. Fixing it
  means reading `MotionEvent` axes through `android-activity` directly, which winit
  owns the input queue for, or GameActivity.

Waydroid has no pad. Exercised: the keycode mapping, the stick sign (Android Y is
down-positive), the merge and the hide-on-pad rule, all as unit tests, plus
`adb shell input keyevent 96` (a `KEYCODE_BUTTON_A` from a fake source) driving the
real event path and hiding the overlay. Not exercised: a physical pad, its axes.

## Pausing when the window goes away

Landed 2026-10-07 (auto-pause lane). All of it is **chosen, not measured**: no original
title was observed on a phone, so there is nothing to match and no confidence score.

- **Android `Suspended`** (home, app switch, screen off) parks a running race in the pause
  menu through `Session::back`, the path the system Back key uses, and stops the audio
  device (`Output::set_paused`, cpal's `Stream::pause`). `Resumed` restarts the device and
  recreates the surface; **the race never resumes itself**, the player backs out of the pause
  menu. Movies and menu music need no code: the loop does not tick while suspended, so a
  tick-driven movie holds its frame and the mixer's ring fills and waits.
- **Desktop** gets the same on a minimised window (`Occluded(true)`, or a zero-sized resize,
  which is what Windows sends; X11 reports neither as an event, so it is covered only where
  the platform sends one). Focus loss (`Focused(false)`: alt-tab, the Steam overlay on the
  Deck, a shade pulled down on Android) pauses the race **only if `[display]
  pause_on_focus_loss` is on**, a row on the DISPLAY page. Default **on**: an unseen race is
  a crash and a lost lap, a paused one costs a press; a second-monitor player turns it off.
  Focus loss leaves the audio running.
- A run with no menus (`--race` windowed) freezes the tick the way Start does instead.
  Nothing outside the input side changes, so a replay, a ghost or a state hash cannot tell.
  Decision table: `session/lifecycle.rs` (unit tested).
- Verified on Waydroid: race running (timer 0:13), `input keyevent 3` (HOME) logged (at `debug`)
  `window away (Suspended): Menu`, relaunch showed the root menu over the parked race, Back
  resumed it from 0:16 with the timer running. **Not exercised:** the audio actually going
  quiet (no listening allowed; `--no-audio` does not apply on Android, and the log showed no
  `could not pause` warning), `Occluded` on a desktop (Xvfb has no compositor to send it),
  the S24.

## The display's refresh rate

winit 0.30's Android backend answers `None` for `refresh_rate_millihertz`, so
`oag_mobile::refresh` asks the activity itself:
`activity.getWindowManager().getDefaultDisplay().getRefreshRate()` over JNI, once a second on a
thread of its own (`android-refresh`, the poller the X11 reader also uses, `oag_thread::poller`).
`getRefreshRate` answers the rate the app's frames are shown at: the active mode's, or the
system's override for this app. No `DisplayListener` is registered, because it needs a Java
class and the APK carries none; a change is followed at the next round. Each new surface starts a
new reader and drops the old one. A failed call keeps the last answer, and before any answer
there is none, so the game falls back to 60 as it does on a platform that reports nothing.

The value reaches `Host::refresh_millihertz`, so everything the desktop's report feeds reads it
([dynamic-resolution.md](../rendering/dynamic-resolution.md), "The target is held to the frame
limiter and to the display"): FRAME LIMIT `refresh_rate`, TARGET FPS `default`, the display
clamp on dynamic resolution and the CPU governor's period, the overlay's graph scale, the
labels `REFRESH RATE (N)` and `DEFAULT (N)`, and the benchmark's verdict and `refresh_hz`.
Nothing is a per-platform default: a fresh profile's frame limit (240) and TARGET FPS (off) are
unchanged.

**Measured 2026-11-26** on the Galaxy S21 (SM-G991B, Android 14, panel modes 48, 60, 96 and
120 Hz, the system's adaptive "motion smoothness"), `dist` APKs before (`5893b60d5`) and after
(`616b21cd3`), Pulse's benchmark on Talon's Junction White, `--no-audio --ticks 1500
--benchmark-frame-limit refresh_rate` (a capped run, so the FPS compares with nothing but
itself):

| Run | Present mode | Result's refresh | Frame limit in force | Budget | Over budget | Average FPS |
| --- | --- | --- | --- | --- | --- | --- |
| before, VSYNC on | `Fifo` | 60, `assumed` | none | 16.7 ms | 317 of 1330 | 65.0 |
| after, VSYNC on | `Fifo` | 120, `display` | none | 8.3 ms | 1276 of 1276 | 62.3 |
| before, VSYNC off | `Mailbox` | 60, `assumed` | 60 | 16.7 ms | 536 | 55.1 |
| after, VSYNC off | `Mailbox` | 120, `display` | 120 | 8.3 ms | 1161 of 1161 | 56.7 |

`dumpsys display` read 120 Hz (`mActiveSfDisplayMode` and `mActiveRenderFrameRate`) in every
sample of those runs. The VSYNC-on runs agree on their own: `Fifo` presents at most one frame a
refresh, so an average above 60 frames a second cannot come from a 60 Hz output, including in the
before run that called itself 60. The race itself is bound by the frame's own cost, which this
does not change.

**60 Hz** was not tried as a display setting (that is a system setting of the owner's). It was
observed from the phone's own switching on returning to the app from the home screen. In one
return sampled about every half second, the first `dumpsys` sample read 60 and the next, begun
0.5 s later, 120; the game's reader answered 60 between the starts of those two samples and 120
at its next round a second later. Other returns logged the same 60 then 120, and the frame limit
in force under `refresh_rate` went to 60 and back with it. Each such change also drops the
measured half of `oag_present::refresh`, as any change of report does.

## Pre-converted caches

`oag-game --no-audio --dry-run --prefetch <image>` is the headless
convert-everything-and-exit run (the intro is about 57 s of it, every PSP movie
about 10 minutes, the sounds 5.5 s and 0.7 GB of PCM). It writes
`data/cache/manifests/<image file>.txt`: one cache-root-relative path per line,
`movies/...` and `audio/...`, for exactly the files that image owns (the caches are
content-keyed and shared by every disc). `just push-data android` (and `deck`)
shows a `cache/<image>` row beside each Pulse PSP image: `on device`, `partial`,
`-`, or `no cache`. Selecting a `no cache` row runs the prefetch first, then copies
into the device's cache directory (`files/cache/oag/{movies,audio}` on Android,
`$XDG_CACHE_HOME/oag` on the Deck). Prefetch only knows Pulse's archives today, so
Pure, HD, 2048 and Omega have no cache row; a PS2 image's four loose movies are not
in the manifest.

## Testing without a phone (Waydroid)

The APK also carries an `x86_64` library whenever the `x86_64-linux-android` rust
target is installed (`rustup target add x86_64-linux-android`; `--arm64-only` skips
it), which is what lets it run in Waydroid on the build machine. The phone ignores
that library. One container exists system-wide, so use your own compositor and stop
it afterwards:

```sh
weston --backend=headless --renderer=gl --width=1280 --height=720 \
    --socket=oag-android-startup --idle-time=0 --debug &       # note the PID
WAYLAND_DISPLAY=oag-android-startup waydroid session start &   # note the PID
WAYLAND_DISPLAY=oag-android-startup waydroid app install target/apk/*.apk
WAYLAND_DISPLAY=oag-android-startup waydroid app launch org.openantigrav.game
WAYLAND_DISPLAY=oag-android-startup weston-screenshooter       # writes a PNG in $PWD
waydroid session stop; kill <weston PID>
```

`--debug` is what authorises `weston-screenshooter`; `grim` does not work on weston.
A container that has no visible window freezes itself, so `waydroid app launch`
before looking for a device. `adb connect <IP from waydroid status>:5555` needs the
RSA prompt accepted inside Android (or the host key in the container's `adb_keys`:
append `~/.android/adbkey.pub` to `~/.local/share/waydroid/data/misc/adb/adb_keys`,
which the user owns). Waydroid's Play Protect "App scan recommended" dialog covers
the app on a fresh install; `adb -s <ip>:5555 shell settings put global
package_verifier_enable 0` and Back dismisses it. Pass `-s <serial>` to adb (it has no
`--serial`), because the phone is usually attached too.

Verified here (2026-10-07, Waydroid, the PSP EU image and cache pushed with
`scripts/push-game-data.sh android --serial <ip>:5555`): the not-found screen with the
adb push path, the intro playing with its picture and a tap skipping it to the language
menu, front-end sound mixing, the launcher icon (visible in the Play Protect dialog),
and the Vulkan backend (the host's RADV; the GLES fallback was not exercised). A
`settings.toml` the app cannot read (mode 000, as a reinstall that restores files under
another owner leaves it) logs one warning and runs on defaults.

## Performance on the Galaxy S21

Measured 2026-11-24 on a Galaxy S21 (SM-G991B: Exynos 2100 with one X1, three A78 and four A55
cores; Mali-G78 MP14, driver r38p0; 7 GiB; Android 14; 1080x2400 panel, which the game draws at
2320x1080 in landscape), on USB with the battery at 96 to 100 %, with a release (`dist`) build of main
`d5bfef6` made by `OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk` (`--no-audio`, a fresh profile). The race
is the **Racebox single race, 8 ships, weapons on, 4 laps, `--autopilot --seed 1`**, reached through the
menus (see "Reaching a race" below); Pulse is the PSP EU image on Talon's Junction (default circuit), HD/Fury is the
decrypted PS3 EU image on Vineta K. **Defaults** are what a fresh profile writes (render scale 100, which on
this panel is a 1906x1080 scene under the pillarbox; vsync off, frame limit 240, no dynamic resolution,
motion blur and MSAA off, shadows `original`). **Low** is one lower preset chosen here: `render_scale = 50` in the
title's `[render_profiles]` table and nothing else (953x540 scene). The game has no quality tiers to name.
Runs began after waiting for the AP sensor to read 38 degrees C or less (40 for the HD A/B runs), **except
"HD defaults, cold start"**, which began after the phone had idled to AP 29.5 and skin 29.1 C. The start state matters (below).

| | Pulse defaults | Pulse low | HD defaults, cold start (skin 29 C) | HD defaults, warm start (skin 37 C) | HD low, warm start (skin 36 C) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Frame time over about two laps (75 s), median / p95 / p99 / worst, ms | 14.7 / 20.1 / 24.4 / 36.9 | 14.2 / 19.5 / 23.2 / 41.3 | 118.1 / 163.9 / 181.9 / 195.8 | 175.8 / 236.5 / 292.6 / 399.3 | 85.9 / 122.8 / 134.4 / 219.3 |
| fps in that window | 66.4 | 69.0 | 8.4 | 5.6 | 11.4 |
| fps over the first 60 s of racing (median of the per-second log) | 68.0 | 68.3 | 7.5 | 5.5 | 10.4 |
| **Settled rate**: median fps over the racing seconds of the last 5 of a 10 minute run | **59.9** | **65.5** | **7.5** | 5.9 (one race only) | **11.1** |
| ... its mean frame / mean p99, ms | 16.6 / 23.2 | 15.4 / 21.8 | 131 / 228 | 164 / 300 | 90 / 180 |
| Scene pass on the GPU (`gpu_ms` of `--frame-log`), racing mean | 4.8 ms | 2.1 | 97 | 122 | 41 |
| Race load, confirm to first race frame (`--measure-race-load`, run 1 / run 2) | 12.8 / 13.4 s at first, 24.2 / 31.3 s warm | 28.1 / 30.0 s (warm) | 71.6 s | 72.7 s | 72.4 s |
| Peak PSS (`dumpsys meminfo`) | 309 MB | 293 MB | 1,287 MB | 1,228 MB | 1,314 MB |
| GPU memory of the process (Perfetto `android.gpu.memory`) | 468 MB | 447 MB | 2,963 MB | 2,961 MB | 2,919 MB |
| App CPU, share of one core (frame thread; Mali's `mali-cmar-backe` thread), 75 s trace | 59 %; 28 % | 69 %; 34 % | 19 %; 13 % | 19 %; 14 % | 37 %; 17 % |
| GPU busy, last 5 min (`/sys/kernel/gpu/gpu_busy`), at a mean clock of | 77 % at 345 MHz | 73 % at 318 MHz | 84 % at 656 MHz | 95 % at 398 MHz | 79 % at 371 MHz |

How the frame time is taken: the app's `BufferTX` counter in a Perfetto trace gives every queued buffer, so
the percentiles are over every frame (4,972, 5,158, 624, 418 and 853), not over `--frame-log`'s once-a-second
means. The window is 75 s long and starts 4 to 13 s after the first race frame, which on Pulse is inside the race (its HUD's best lap was
31.6 s, so about two laps); on HD it starts at the end of the countdown and is two laps only if the simulation keeps its 60 Hz pace,
which a race of 118 to 131 s against a 4-lap length suggests it does. The settled rate is the median of the per-second
`fps` in `--frame-log`, **racing seconds only**: a run restarts the race when it ends, and the seconds after `race finished` (the finish camera,
30 to 70 s a cycle) are left out. Pulse: three races of 134 s in 10 minutes (6.6 racing minutes, the rest menus, 13 to 30 s loads and the finish camera); HD: two races of 118 to 131 s
and two loads of 72 s. The flyby and countdown (about 20 s) at the head of each race are in the racing seconds.
The HD warm start has one race only (it was run before the restart loop; its 142 racing seconds are all of it).

**Run to run, the Pulse defaults read 68.0, 60.6 and 61.0 fps over their first minute of racing** (the first is
the 10 minute run above; the other two are the two attempts at pinning the thread, under "Reaching a race"), so the 5.6 fps between Pulse defaults and low is about the spread: the low preset halves the scene
pass on the GPU (4.8 to 2.1 ms) and moves the frame by 1 to 2 ms at most. **The start state moves HD by about 50 %** (8.4 fps cold, 5.6 warm, defaults): see the thermal section.

### The shape of the frame

- **Pulse is bound by the frame thread, not the pixels.** `frame-phases` (ms per frame, defaults, settled): ticks 1.1,
  scene record 4.4, submit 3.9, present 6.5 (the wait for a buffer; the GPU's scene pass is 4.6 ms of it).
  The low preset cuts the pixels by 4 and leaves `scene` 4.3 and `submit` 4.1 where they were.
  The thread ran 46 to 54 % of the trace on the A78 cores and 4 to 22 % on the X1; the other 28 to 34 % of a core is Mali's
  command-stream backend thread.
- **HD is bound by the GPU, then by a CPU floor.** Defaults, warm start: scene pass 122 ms at 400 MHz and 95 % busy, and the
  frame thread waits 130 ms a frame in present (cold start: 97 ms at 656 MHz). A/B on the same race, each from a start at AP 40 C or less (race seconds 20 to 80, mean of the
  per-second lines; the "defaults" row is the warm-start run):

  | HD variant | fps | frame ms | scene GPU ms |
  | --- | ---: | ---: | ---: |
  | defaults | 5.5 | 183 | 114 |
  | anisotropy `off` | 6.0 | 176 | 103 |
  | shadows `off` | 6.1 | 176 | 125 |
  | render scale 50 | 10.6 | 97 | 41 |
  | render scale 25 (476x270) | 16.1 | 64 | 14 |

  Anisotropy and the shadow passes cost nothing worth the picture (both inside the spread). Render scale 25 leaves a
  frame of 64 ms of which the frame thread's own phases are 28 ms (ticks 6.6, scene 9.1, submit 11.5) and the
  rest is the GPU passes outside the scene pass. HD's tick is 7 to 13 ms of a slow frame because the fixed-step loop catches
  up with several ticks per frame.
- **Memory.** HD holds 2.96 GB of GPU memory against Pulse's 0.47 GB, on a 7 GiB phone, while `dumpsys meminfo` shows
  1.2 GB PSS (the GPU's share is in neither the PSS nor the `Graphics` row, which read 21.5 to 51.6 MB). Mali has no
  `TEXTURE_COMPRESSION_BC`, so HD's DXT textures are decoded back to RGBA8 on upload
  (`oag_mesh::mesh_render::texture::upload`, whose own comment prices block-compressed binding at about 250 MiB in a race on a
  desktop). That is the likely source and was not isolated.

### Thermal throttling (Pulse defaults, the run that began at thermal status 0: AP 37.5 C, skin 35.6 C)

Times are wall seconds from the first race frame (the sampler's loop is about 1.1 s an iteration, scaled to the run's wall time).

| Race seconds | A78 cap (MHz) | GPU cap (MHz) | AP (C) | skin (C) | fps (median of the log) |
| --- | ---: | ---: | ---: | ---: | ---: |
| before GO (menus and the 13 s load) | 2808, down to 1872 at 23 s before | 858 | 37.5, then 45 | 36 | |
| 0 to 60 | 1872 | 858 | 45 to 46.5 | 37 to 38 | 68 |
| 90 to 120 | 1560 | 767 | 47 | 38.4 | 62 |
| 120 to 180 | 1352 | 676 to 585 | 47 | 38.7 | 59 to 61 |
| 300 on | 1352 | 403 | 46.5 to 47 | 39 to 39.6 | 56 to 67 (the track's stretches differ) |

The phone throttled its CPU during the load, before the first race frame (the A78 cap fell to 1872 MHz 23 s earlier); the GPU's cap
held at 858 MHz until 91 s of racing and then stepped down to 403 MHz over about 3 minutes. The system's thermal status first read 1
(light) at 84 s and 2 (moderate) at 395 s. The rate falls from 68 to 59 fps between the first minute and the third and then sits at
a median of 59.9 over the last five minutes; the AP sensor holds 46 to 47 C from the start, so the controller is holding a temperature and
the rate is what that temperature allows. The other warm-start runs began with the skin sensor at status 1 already (the phone charges over USB,
battery 35 to 36 C) and show their caps from the first second, so they carry no onset.

**The throttle follows the skin sensor, and the phone starts warm.** On USB the battery and skin sensors sit at 35 to 37 C, so every "cooled to AP 38" start already read thermal status 1 and capped the AP at about 47 C
(HD's GPU held at 403 MHz). The HD cold start (AP 29.5, skin 29.1, battery 28 C) let the GPU run at 858 MHz and the AP climb to 52 to 57 C: 7.3 to 9.1 fps through the first race, then, as the skin passed 40 C (thermal status 2 at about 2 min and 3, severe, at about 4 min),
the GPU clock fell to 750 to 624 MHz and the second race held 7.2 to 7.8 fps; the run ended at AP 56 and skin 44.3 C. The Pulse race load doubled from 12.8 s on a cool phone to 24 to 31 s on a warm one. A comparison of two presets is only fair from the same start state;
the HD columns above say which they had.

### Top three costs

**CPU** (`simpleperf record --app org.openantigrav.game -e cpu-clock -f 1500 -g --duration 20`, Pulse defaults, 30 s after
GO; 28,662 samples; the frame thread is 63 % of all app samples and Mali's backend thread 32 %; the shares below are inclusive and
overlap each other):

1. **`Scene::prepare_view` 36 %** (6.6 % itself): node animation sampling 6.1 % (`write_node_anims_started`, `AnimTransform::sample` 4.9 %),
   `wgpu::Queue::write_buffer` 9.9 %, collision raycasts 4.0 %.
2. **wgpu command encoding at finish, 22 %** (`CommandEncoder::finish` to `encode_render_pass`): wgpu-core replays and validates every pass's
   commands on the frame thread.
3. **`wgpu::Queue::submit` 15 %**, of which `vkQueueSubmit` into Mali's driver is 8.5 %; the Mali backend thread runs
   `draw_build_command` and `cmarp_*` for the draws (the 32 %).

Next: `tick` 8.4 %, `present` 5.9 %, `memcpy` 6 % (4 % itself).

**GPU**: Perfetto on this phone offers `android.gpu.memory`, `android.power`, `android.surfaceflinger.frametimeline`,
`linux.sysfs_power` and `linux.ftrace`, **no `gpu.counters` and no `gpu.renderstages`**, so the only GPU timing is the game's own
scene-pass timestamp. The costs of HD's GPU are the A/B table above: the pixels (scene pass 114 ms at 1906x1080 to 14 ms at 476x270),
and a floor that does not fall with them.

### Driver messages (full `logcat`, 10 minute runs)

- `gralloc4: Format allocation info not found for format: 38` and `3b`, `AHardwareBuffer: GraphicBuffer(w=4, h=4) failed`, and `Gralloc4: isSupported(1, 1, 56 / 59) failed`:
  12 lines a launch, the Mali driver probing two 4x4 formats while the adapter opens. Harmless, no effect seen.
- `wgpu_core::instance: Missing downlevel flags: DownlevelFlags(SURFACE_VIEW_FORMATS)` once a launch.
- No Mali, Vulkan or `ANR` line in any race, no crash in the menu route. `OAG_WGPU_DEBUG=1` added only debug labels: the APK carries no
  Vulkan validation layer, so a run with it printing nothing does not show validation clean.
- `audio: ... voice(s) refused` under `--no-audio` (a count, not a fault).

### Reaching a race, and what does not work

- **The launch file's `--race` crashed on `d5bfef6`; fixed** ("The stack"). `android_main` runs on a 2 MiB thread (`android-activity` spawns it with `std::thread::spawn`),
  and the `--race` path stacked `app::run` (481 KB frame), `headless::run_race` (545 KB), `App::open_with` (385 KB), `Stage::race` (315 KB),
  `Scene::new` (172 KB) and winit's closure (160 KB): 2.07 MB, and naga's WGSL parser overflowed the stack in `create_shader_module` (SIGSEGV, `SEGV_MAPERR` in `android_main`;
  Pulse and HD alike). `scripts/android-race-phases.sh` uses `--race` and failed the same way (it passes no `--no-audio`). The menu route had no `run_race` frame and ran.
  A `llvm-addr2line` over the unstripped library names the frames; the sizes are the `sub sp` of each prologue.
- **The menu route, scripted.** The launch file takes the image and `--no-audio --frame-log --autopilot --seed 1 --no-intro`; `files/config/oag/settings.toml` skips the
  prompts (top-level `language = "English"`, `[player] name` and `tag`) and holds `[race] mode = "single_race"`; then `adb shell input tap` on the 2400x1080 frame:
  Pulse after 12 s: tap (1200, 540) for PRESS START, (660, 292) RACEBOX, (576, 634) START, (1704, 1032) twice for the track and the ship;
  HD after 16 s: (942, 194) twice for RACEBOX, (598, 508) START, (828, 360) to land on Vineta K, then `input keyevent 96` (pad A) twice. A tap on an HD track select hex
  moves the cursor and does not confirm. The in-game name entry also opens the phone's keyboard over it.
- Pinning the frame thread with `adb shell taskset -p` is refused (`Permission denied`) for a release app's thread, so the X1 placement below is not measured.
- `simpleperf` attaches to a release build only with `--app` and the build marked `<profileable android:shell="true"/>`: `OAG_APK_PROFILEABLE=1 just apk` (`scripts/build-apk.sh`).

### Reproducing it

```sh
adb push scripts/android-perf-poll.sh /data/local/tmp/       # GPU busy and clock, CPU clocks, thermal HAL, meminfo, per second
adb shell "nohup sh /data/local/tmp/android-perf-poll.sh /data/local/tmp/poll.txt 700 >/dev/null 2>&1 &"
adb shell 'perfetto -c - --txt -o /data/misc/perfetto-traces/oag.pftrace' < scripts/android-perf-trace.cfg   # edit duration_ms
uv run --with perfetto python3 -I scripts/android-perf-trace.py oag.pftrace
adb shell /data/local/tmp/simpleperf record --app org.openantigrav.game -e cpu-clock -f 1500 -g --duration 20 -o /data/local/tmp/perf.data
```

`simpleperf` is `$ANDROID_HOME/ndk/<ver>/simpleperf/bin/android/arm64/simpleperf`; `report --symfs` over a directory holding the unstripped
`target/aarch64-linux-android/dist/examples/liboag_android.so` at the on-device path names the Rust frames.
The `--frame-log` lines reach logcat as tag `oag`; `--measure-race-load`'s lines come out under `RustStdoutStderr`.

### Where an HD frame goes on the S21

Measured 2026-11-24/25 on the same phone, release builds of main `f928359` plus `OAG_SKIP_PASSES` (`oag_gpu::skip`), HD/Fury at
**render scale 25** (476x270 scene, 2320x1080 surface), Vineta K, `--autopilot --seed 1`, `--no-audio`. Two findings decide how to measure here:

- **The one-tick GPU bench misleads on Mali.** `OAG_RENDER_GPU_BENCH` (an `OAG_APK_FEATURES=perf-probe` build) submits a frame and waits
  for it, so the GPU idles between runs and its clock falls; its marks inside a pass read nearly zero on a tiler (the binning, not the shading,
  lands between them), and it put the advert cards and the glass target at nothing. Its whole-frame figure did show a floor that does not fall
  with the pixels (238x135 to 1906x1080: 49, 49 and 35 ms outside the race pass and the bloom), and three switches it did settle: the depth
  prepass **pays** here too (off: race pass 14 to 27.5 ms), binding arrays help (`OAG_BINDLESS=0`: 58.5 to 67 ms), and the shadow passes cost nothing (`--shadows off`: 67.6 against 67.6 ms).
- **What does work is leaving one pass out every other 10 s of one live run** (`OAG_SKIP_PASSES=<name>` and `OAG_SKIP_PASSES_TOGGLE=10` as launch-file
  lines; `scripts/android-skip-windows.py` splits the `--frame-log` by arm). Both arms see the same temperature, which moves a whole run by half.
- **`--race` on HD stood in the pre-race flyby, and now races.** HD's flyby repeats until a press (`Ending::OnlyBySkip`, measured on RPCS3,
  `docs/gameplay/race-intro.md`, "Other titles"), and a launch-file run has nobody to press: a 20 s `simpleperf` of it had no `Race::tick` sample, the screen showed the flyby
  camera with no HUD, and `frame-phases` read ticks 0.2 ms (the "30 fps at scale 25" was a flyby's). It is not Android: a windowed desktop `--race` on HD waits the same way.
  Pulse's flyby ends on its own after its `AnimEnd`, so Pulse's `--race` was always racing (a ticks reading of 0.2 to 0.7 ms does not tell its flyby from racing: read the
  log's `pre-race flyby: ends after N tick(s)` with `OAG_LOG_FILE_FILTER=warn,oag=debug` as a launch-file line, which writes `files/state/oag/logs/oag-game.log`, and a screencap).
  `--autopilot` now presses the skip of a flyby only a press ends, as a held Cross would; `--no-intro` drops every flyby. Measured on the S21 (the `dist` APK of this change, `--no-audio`,
  `--autopilot --seed 1`): HD Vineta K `pre-race flyby: ends after 1 tick(s)` and the HUD with lap and position on screen; Pulse (Talon's Junction) `ends after 1529 tick(s)` and the
  HUD with the lap clock. The menu route (Racebox, Single Race, taps as above) also reaches `stage=race` with the HUD on both. Tests: `an_hd_race_under_autopilot_leaves_the_flyby`
  (`drive_ground_truth`) fails when the skip is dropped. The menu route, which loads `track_reversed.vex`, is the route the tables above this section were measured on (Vineta K **reversed**): ticks 5 to 11 ms, `Race::tick` 25 % of the frame thread.

Per pass, menu route (racing), one run each, about 120 frame-log seconds per arm, drawn against left out:

| Pass left out | Frame ms, drawn / left out | fps median | What it is |
| --- | ---: | ---: | --- |
| `glass` | 50.4 / 39.8 | 19.8 / 25.6 | Vineta K's behind-the-glass target: 640x360 whatever the render scale (1.8 times the 476x270 scene), the glass chunks drawn again with no prepass, every frame |
| `adverts` | 55.5 / 54.2 | 18.6 / 19.1 | eight advert cards at 512x256, redrawn every frame: inside the noise |
| `bloom` | 41.8 / 43.8 | 23.9 / 22.5 | HD's bloom chain: no gain |

- **The audio mixer under `--no-audio` no longer mixes.** The 6.8 % of the racing frame thread was `Mixer::render` summing the voices of a mixer nobody hears: a null
  backend renders each tick and drops it so that voices end, free their slots and the race song advances. `Audio::open` now makes a `--no-audio` run's mixer silent when
  there is no `--dump-audio` (`Output::headless`, `Mixer::set_silent`): `Mixer::advance_tick` moves every voice on through the same `voice_gate` the mixer uses (delay,
  release, loop, end) and writes no samples and no buffer; a voice that cannot end or wrap this tick only adds its step. With a convolver loaded, with a dump, or with a
  device it renders in full, and a machine with no device and no `--no-audio` renders and drops as before. Measured with a 20 s `simpleperf` of the racing `android_main`
  thread (default settings, `--autopilot --seed 1`, share of all samples, the A-only build against this change's):

  | Race | `Audio::tick` before | after | `Mixer::render` before | `advance_voices` after | `android_main` share before / after |
  | --- | ---: | ---: | ---: | ---: | ---: |
  | HD Vineta K | 6.37 % | 0.44 % | 6.28 % | 0.40 % | 61.4 % / 56.6 % |
  | Pulse Talon's Junction | 0.53 % | 0.09 % | 0.49 % | 0.06 % | not read |

  So the mixer does not mix, but its voice bookkeeping still runs (0.4 % on HD, 0.06 % on Pulse): "no work per frame" is false as worded and 94 % of it is gone on HD.
  Desktop, HD `--race --no-audio --screenshot --ticks 3600` (dev profile, `perf stat -e instructions:u`, three runs each, spread under 0.1 %): 45.65 G user instructions on main's mixer, 38.41 G on this one (-15.9 %).
  Desktop, `--dump-audio` of 600 ticks of a race on Pulse and on HD: the WAV is byte-identical between main's mixer and this one (the audible path went through the same `voice_gate`),
  and `--race --screenshot` with and without `--no-audio` is byte-identical between the two builds.
On the flyby (`--race`), the same switches read glass 42.5 / 26.8, adverts 42.4 / 35.5, bloom 36.4 / 29.7, `hud` 36.8 / 32.0, `composite` 39.9 / 34.6
and `resolve` 45.2 / 41.8 ms: the flyby's frame is lighter, so the same passes are a larger share of it. Only the glass target holds its cost while racing.
The glass target's size is now a setting, `graphics.reflection_resolution` (`original`, the default, `half` or `quarter` of the original's size per axis, whatever the render scale:
Vineta K's 640x360 becomes 320x180 or 160x90); the default is `original` on every platform, and a player picks `half` or `quarter` for their hardware
([water.md](../rendering/water.md), "The reflection target's resolution"). The phone numbers are under "What `quarter` buys on Vineta K" below.
The racing frame thread is about 23 ms (ticks 5.6, scene recording 8.3, `submit` 9.1, the 10 minute run's racing seconds 20 to 110).
A 20 s `simpleperf` of a racing frame thread: `Race::tick` 25 % (of it the audio mixer 6.8 % under `--no-audio`, mixing nothing since the bullet above, `RaceView::capture_poses` 6 %,
the particle systems about 10 %, opponents 4.6 %), `CommandEncoder::finish` 23 %, `Scene::prepare_view` 23 %; Mali's backend thread is 37 % of the app's samples.

**What `quarter` buys on Vineta K** (`graphics.reflection_resolution`, `--reflection-resolution`). Release build of main `da2268e`
(`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk`), HD/Fury on the S21, **render scale 25** (476x270 scene), `--no-audio`, default settings otherwise, media volume 0, the
launch file's `--race --track /data/environments/01_vineta_k/track.vex --autopilot --seed 1 --no-intro --frame-log`: Vineta K **forward**, three laps, the race ends 1:37 game time after the start
(the earlier tables are the menu route's reversed circuit). The launch file's `--race` takes the circuit from `[race] track`, which on a profile written for Pulse names Talon's
Junction; `--benchmark` also takes it from there on the phone, so pass `--track`. Each arm starts after the phone cooled to AP 32.0 C and skin 31.5 to 31.7 C (AP climbs to 45 to 52 C within
20 s of any arm, so skin is the temperature that carries over), in the order `original`, `quarter`, `quarter`, `original`. The window is racing seconds 10 to 100 of the `--frame-log` (90 lines,
one per second: `frame_ms` is that second's mean, so the median and p95 are over the 90 per-second means, and `p99` is the median of each second's own p99). The loader report confirms the arm
(`the glass and sea target is 160x90 at reflection resolution quarter (the original's is 640x360)`).

| Arm (start AP / skin C) | Frame ms median / p95 | fps (median / of the mean) | p99 ms (median of seconds) | seconds over 33.3 ms | scene `gpu_ms` / `present_ms` (median) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `original` 1 (32.0 / 31.5) | 39.4 / 48.3 | 25.4 / 25.6 | 54.3 | 75 of 90 | 9.5 / 24.2 |
| `quarter` 1 (32.1 / 31.5) | 25.1 / 36.6 | 39.8 / 37.9 | 34.6 | 7 of 90 | 9.4 / 13.4 |
| `quarter` 2 (32.0 / 31.7) | 24.9 / 35.8 | 40.2 / 37.8 | 35.5 | 7 of 90 | 9.4 / 14.5 |
| `original` 2 (32.0 / 31.6) | 45.4 / 60.2 | 22.0 / 21.8 | 63.5 | 82 of 90 | 11.6 / 27.3 |
| **`original`, the two pooled** | **41.9 / 55.5** | **23.5** (mean) | | | |
| **`quarter`, the two pooled** | **24.9 / 35.8** | **37.9** (mean) | | | |

`quarter` takes the median frame from 41.9 to 24.9 ms (-17.0 ms, against 10.6 ms the one-pass toggle read, "Per pass" above) and the mean rate from 23.5 to 37.9 fps. The two `quarter`
arms agree within 0.2 ms; the two `original` arms differ by 6 ms (39.4 and 45.4) at the same start temperature, so the delta is -14.3 to -20.5 ms depending on which `original` arm it is read
against, and either way larger than the glass pass's own 10.6 ms: the scene `gpu_ms` moves little (9.5 to 11.6 against 9.4), the rest shows in the frame thread's wait on the present (24 to 27 against 13 to 14 ms; why the toggle read 10.6 ms and this reads 17 is not settled). **`quarter` alone reaches 30 fps on average (37.9) and in 83 of 90 seconds; it does not hold it in every second**:
the seven seconds over 33.3 ms (peak 38.9 ms, 25.7 fps) fall in one slow stretch of each lap, where the frame time climbs from 24 to 37 ms (not tied to the glass: `original` has the same stretch, higher), and p95 is 35.8 ms. `original` is over 33.3 ms in 75 to 82 seconds of 90. One more run of each arm
(60 s each, no figures taken from them, a `screencap` every 12 s) shows the same ship positions in both: the glass in the tunnel is visibly coarser under `quarter` (a 160x90 target stretched over the
view) and the rest of the picture is unchanged. Moa Therma (the sea), Ubermall, Chenghou, Modesto and `half` are not measured.
REFLECTION RESOLUTION `auto`, the default since 2026-11-25, settles at `quarter` here within about 10 s of the
race starting and matches the fixed `quarter` within the run-to-run spread (34.6 against 33.9 fps pooled, same
protocol, two arms each); `half` reads 30.3 ms, and leaving the pass out entirely gains nothing over `quarter`
(so there is no `off`). The table and the policy are in [water.md](../rendering/water.md),
"REFLECTION RESOLUTION `auto`".

**Measured and dropped** (same method, one run each unless said):

- **Drawing the resolve and the HUD straight onto the surface** when the composite would only copy (brightness and gamma 100, no screen filter):
  byte-identical by construction and by a readback test, and **slower** on the flyby, 38.7 against 34.6 ms. The surface image is then needed from the first pass of the frame on rather than the last.
- **Skipping the glass target on frames that show no glass draw** (`slots::REFRACTION`): byte-identical (26 of 26 desktop frames), but the glass draws pass the PVS and the
  frustum on most of the lap, so it fired on 2 of 26 sampled frames and moved nothing on the phone (measured on the flyby).
- **A depth prepass for the glass target's chunks**: 31.4 against 33.7 ms mean frame (30.4 against 30.2 fps, two runs from skin 29 to 30 C), inside the noise, and not exact: each of the
  target's two fog groups runs its own prepass, so a depth tie between them shades the other group (1 pixel, 1 level, in 2 of 26 target dumps).
- **A portrait surface** (`screenOrientation="portrait"`, a measuring build): SurfaceFlinger composites the landscape app with the GPU
  (`dumpsys SurfaceFlinger`: `composition=CLIENT`, `bufferTransform=ROT_90`, because `wgpu-hal` sets `preTransform` to `IDENTITY`); a portrait buffer
  goes to the display hardware (`DEVICE`), and the frame did not move (30.0 against 29.1 fps median). No wgpu issue or PR asks for pre-rotation.

**Against the 30 fps claim** (menu route, render scale 25, `frame_limit = "30"`, 10 minutes from AP 34 C, three races): racing seconds median 17.0 fps
(first minute 21.1, last five minutes 16.3); the Perfetto window (75 s, 1,509 frames) has a median frame of 48.2 ms, p99 85.7 ms, and 95.8 % of frames over 33.3 ms.

### What the frame thread waits for

Measured 2026-11-25 on the same phone. Release builds (`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk`) of main `2d42134` (the `simpleperf` and Perfetto runs) and of
main plus the frame log's chain fields and a local, uncommitted `OAG_DIAG_FRAME_LATENCY` override of `desired_maximum_frame_latency` (every other run; the override is inert unless set),
HD/Fury Vineta K forward, the launch file's `--race --track /data/environments/01_vineta_k/track.vex --autopilot --seed 1 --no-intro --no-audio --frame-log`,
**render scale 25** (476x270 scene), REFLECTION RESOLUTION `auto` (it settles at `quarter`), every other setting a fresh profile's. Each run starts once the AP sensor reads
33 C or less; figures are medians over racing seconds 20 to 80. The start state still moves a run by about a third: from AP 27 C the GPU held 858 MHz and the frame read 26.3 ms;
from AP 33 C it was capped at 403 MHz within a minute and the frame read 34.1 to 34.8 ms.

**The frame thread's phases** (`frame-phases`, the AP 27 C run, ms): ticks 0.62, scene recording 2.34, compose 0.22, overlay 0.09, `submit` 8.11 (wgpu's command encoding at
`finish` plus `Queue::submit`), readback 0.08, **present 13.98**, total 25.4. The CPU's share of a frame (`cpu_ms - present_ms`) is 11.5 to 12.9 ms across the runs; HD's bloom chain is
2.8 to 3.1 ms of GPU time a frame (`bloom_ms`, which the frame log now carries; [logging.md](../architecture/logging.md), "The frame-log line").

**`simpleperf` with off-CPU time.** `--trace-offcpu` works on this phone and gives every sample its wall time, so a blocked thread shows where it blocks:

```sh
adb shell /data/local/tmp/simpleperf record --app org.openantigrav.game -e cpu-clock -f 1000 -g --trace-offcpu --duration 60 -o /data/local/tmp/perf.data
python3 $ANDROID_HOME/ndk/<ver>/simpleperf/binary_cache_builder.py -i perf.data -lib <dir holding the unstripped liboag_android.so>
python3 -I scripts/android-simpleperf-phases.py perf.data binary_cache android_main
```

A 60 s record slows the race (frame 44.4 against 26 to 35 ms), so its shares say where the time goes, not how much there is. The frame thread's wall time:

| Phase | Share of wall time | of it on the CPU |
| --- | ---: | ---: |
| `Queue::present` | 65.4 % | 1.5 % |
| `CommandEncoder::finish` (wgpu-core replays and encodes every pass) | 14.2 % | 13.2 % |
| draw-list building (`Scene::prepare_view`, `Recorder::record`, `draw_lists`) | 8.1 % | 7.2 % |
| `Queue::submit` (of it `vkQueueSubmit` into the Mali driver 4.3 %) | 7.3 % | 6.8 % |
| `Race::tick` | 2.0 % | 1.8 % |
| HUD, pass recording, everything else | 2.9 % | 2.6 % |

The thread is on the CPU 33 % of the time. Its top functions by own CPU time are `memset` 5.5 % (all of it inside the Mali driver: per-draw command memory, job enqueue,
command-list reset), `AnimTransform::sample` 2.9 % and `AnimChannel::at` 2.5 % (node animation), and wgpu-core's `register_init_action` 2.9 %, `TextureUsageScope::merge_single` 2.6 % and
`encode_render_pass` 2.6 %.

**Where the present blocks.** All of the 65 % is one stack: `Queue::present` → `vulkan::driver::QueuePresentKHR` (Android's loader) → `vkQueueSignalReleaseImageANDROID` →
the Mali driver's `semaphores_to_platform_fence` → `enqueue_base_fence_signal` → `sem_wait`. `get_current_texture` waits 0.07 ms a frame, and no `device.poll(Wait)` or map is on
the frame thread. Mali's backend thread (`mali-cmar-backe`) spends 82 % of its time off the CPU, mostly in `poll` on the kernel driver: it waits on the GPU, it does not build
commands for the frame thread to wait on.

**What the wait is for: the previous frame, so the CPU and GPU already overlap, with a gap between frames.** A Perfetto trace with the `gfx` category (15 s, 384 frames;
`scripts/android-fence-timeline.py`) pairs each frame's `QueuePresentKHR` with the GPU completion fence `queueBuffer` hands over, which the app's own FenceMonitor thread
records when it signals. Medians: the present returns **5.3 ms after the previous frame's fence signals**, and the frame's own fence signals **30.3 ms after the present has
returned** (in 99.7 % of frames it signals after). Fences signal every 35.3 ms and frames come every 36.7 ms. So frame N's GPU work runs while the CPU builds frame N+1, and the frame
rate is set by the GPU's timeline, not by the CPU's work plus the GPU's. That timeline is not full: the GPU is 81 to 86 % busy over the racing window (`/sys/kernel/gpu/gpu_busy`,
which counts every user of the GPU, SurfaceFlinger's composition of this landscape surface included), about 5 to 7 ms idle a frame, and in the 5.3 ms between the previous
fence and the present's return Mali's backend thread is on the CPU for a median 4.5 ms (86 % of that gap; its `thread_state` in the same trace). That reads as the driver
submitting the next frame's jobs after the previous frame completes, with the GPU waiting on it; the GPU's idle time inside that gap is not measured directly. Its timestamps add up to about 15 ms (scene 8.9 to 10.5, glass, adverts, bloom and the rest), half of what the
fence period says the GPU spends a frame.

**A third swapchain image changes nothing**: `desired_maximum_frame_latency` 3 against the default 2 (Mailbox, fullscreen), from the same start (AP 32.9 C), read 35.0 against
34.8 ms. Mailbox and 2 frames queued are what the surface runs with by default (`surface: present mode Mailbox (offered [Mailbox, Fifo]), 2 frame(s) queued`).

**The advert cards cost the phone more than their row says.** One run with `OAG_SKIP_PASSES=adverts` and `OAG_SKIP_PASSES_TOGGLE=10` (`scripts/android-skip-windows.py`,
racing seconds 15 to 100): drawn against left out, frame 28.4 against 25.8 ms (mean of the per-second means), median 35.7 against 40.8 fps, the `frame-phases` total 29.4 against
25.3 ms (`submit` 9.1 against 8.2, present 16.2 against 13.3). The `ADVERT CARDS` row reads 1.3 to 1.6 ms. The eight cards are 512x256 each whatever the render scale: 1.05 million
pixels, about eight times the 476x270 scene's, drawn with the circuit's material shaders in eight render passes (95 draws, 7,300 triangles, 1.38 million fragments on a desktop).
An earlier run of the same toggle with the window edges recovered from the row rather than the switch's own line read 30.9 against 22.8 ms; the edges are less sure there.

**The same row on a desktop is the integrated GPU's.** On this workstation `ADVERT CARDS` reads 1.60 to 1.68 ms on the Ryzen 9 7900's Radeon 610M (2 compute units), where a
compositor on that GPU puts a default-adapter window, and 0.16 ms (0.02 ms a card) on the Radeon RX 7800 XT at the same 1440x816. On the 610M every card costs 0.16 to 0.30 ms
whatever its draw count (26 draws 0.16 ms, 3 draws 0.30 ms), the cards with their draws left out and only the clears kept read 0.15 ms for all eight, and the frame goes from about 80
to about 99 fps without them; anisotropy `off`, `RADV_DEBUG=nodcc` or `nohiz`, and `OAG_BINDLESS=0` move nothing. The single-frame bench on the 7800 XT at 1920x1080 reads 91 us
for the eight cards' draws and 2.3 us with the clears only. That is consistent with the cost being the pixels the cards shade (the clears are cheap, and the 7800 XT with
30 times the 610M's compute units is about 16 times faster on them); not checked are the cards' texture sizes, formats and mips (on Mali HD's DXT textures are decoded to RGBA8).

### What a render pass costs

Measured 2026-11-25 (`s21-passes`) on the same phone with the skip-toggle protocol above: release `dist` APK
(`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk --arm64-only`), HD/Fury Vineta K forward, render scale 25, REFLECTION
RESOLUTION `auto`, `--autopilot --seed 1 --no-audio --no-intro --frame-log --race --track /data/environments/01_vineta_k/track.vex`,
each run started at AP 33 C or less, `OAG_SKIP_PASSES=<name>` and `OAG_SKIP_PASSES_TOGGLE=10` as launch-file lines, split with
`python3 -I scripts/android-skip-windows.py LOGCAT 10` (racing seconds 15 on). Two names exist only to price a pass
(`oag_gpu::skip`): `extra-passes` adds 16 one-pixel clear passes to the frame, `advert-draws` keeps the eight advert
card passes and their clears and drops their draws.

| Arm (on in the odd windows) | Frame ms, off / on | What it says |
| --- | ---: | --- |
| `extra-passes`, run 1 | 33.9 / 36.1 | 16 empty passes cost 2.2 ms: 0.14 ms a pass |
| `extra-passes`, run 2 | 31.3 / 37.9 | 6.6 ms: 0.41 ms a pass. `submit` grows 1.3 to 1.5 ms both times, the rest is present wait |
| `adverts` (all eight cards out, every card drawn) | 35.8 / 30.7 | the cards cost 5.1 ms |
| `advert-draws` | 34.7 / 36.3 | dropping only their draws saves nothing (inside the noise) |
| `adverts`, with the cards culled to the view | 28.9 / 28.3 | what is left of the cards after culling: inside the noise |

So **a render pass costs 0.14 to 0.41 ms on the S21**, and the advert cards cost their eight 512x256 targets' clears
and stores, not their shading: on Mali the pixels a pass shades are not the price, the pass and its attachments are.
Two runs of the same arm differ by up to 3x, so read one toggle run to a factor of two. The cards are now drawn
only when a view shows them ([frame-thread-cpu.md](../rendering/frame-thread-cpu.md), "Advert cards: drawn only
when shown"), measured there at -4.2 and -5.6 ms on this phone.

The same arms elsewhere (offscreen race bench, 3000 ticks of Vineta K, frame p50):

| Machine | 16 extra passes | Per pass |
| --- | ---: | ---: |
| Steam Deck, 1280x800 | +0.22 to +0.24 ms frame, +0.28 to +0.35 ms CPU | about 0.015 ms |
| Steam Deck, 1920x1080 and 3840x2160 | -0.1 to +1.0 ms frame (noise), +0.49 to +0.55 ms CPU | about 0.03 ms CPU |
| Desktop, RX 7800 XT (RADV) | not resolved: the whole frame is 1.4 to 1.6 ms and the machine was at load 10 to 51 | |

The cards on the Deck (same bench): drawing all eight against leaving them out costs 0.84 ms at 1280x800, 0.65 ms at
1920x1080 and 0.60 ms at 3840x2160 (three runs of every card against two of none). The Deck's `advert-draws` arm ran on a
build that already culls the cards to the view, so it does not price "the draws of all eight".

**One presentation pass, measured and dropped.** Drawing the resolve, the HUD and the blooms straight onto the
surface and leaving the composite out (a diagnostic arm, only where that is the same picture: grade neutral, no
screen filter, no FSR 3, one view, no photo) read **slower** in both toggle runs: 26.0 against 26.8 ms and 26.9
against 28.2 ms (three passes against two). The earlier single-run reading (38.7 against 34.6 ms, slower) holds.
The grade and the screen filter apply to the HUD too (`oag_present::upscale`, module doc), so a merged pass could
only ever be the same picture at a neutral grade with no filter.

### Freezing the frame's writes

Measured 2026-11-25 (`s21-shading`) with the protocol of "What a render pass costs" (release `dist` APK, HD/Fury Vineta K
forward, render scale 25, REFLECTION RESOLUTION `auto`, 10 s in-run toggles from AP 33 C or less), plus a 60 s Perfetto trace
from racing second 30 whose `android.log` data source carries the switch's own `OAG_SKIP_PASSES: window N on|off` lines, so the
fence timeline (`scripts/android-fence-timeline.py`'s pairing of each `QueuePresentKHR` with its GPU completion fence) splits by
arm. Every per-frame write of an HD race goes through `oag_gpu::writes::Writes` on the queue (a gdb census of
`wgpu::Queue::write_buffer` callers over 100 against 300 ticks: about 95 writes a frame, all through the trait), so one switch
covers them. Three arms, each left out in the odd windows once the race has drawn its first frame (`oag_gpu::skip`):

- `writes`: every write dropped. The picture freezes at the window's first frame, so the GPU draws a stale camera.
- `writes-same`: every write made, with the bytes each buffer range held when the window began. Same frozen picture, every
  copy and barrier kept.
- `writes-dummy`: every write copied into a scratch buffer of its own that nothing reads. Same frozen picture, the copies
  and their barriers kept, no write into a buffer the previous frame's passes read.

| Arm, run | Frame ms, made / arm | Fence period ms, made / arm | Previous fence to present return, ms |
| --- | ---: | ---: | ---: |
| `writes` 1 | 28.4 / 19.1 | 29.8 / 18.7 | 2.6 / 0.06 |
| `writes` 2 | 32.3 / 23.6 | 31.6 / 22.6 | 3.0 / 0.07 |
| `writes` 3 | 31.6 / 23.3 | 30.8 / 21.8 | 2.7 / 0.07 |
| `writes-same` 1 | 31.1 / 26.5 | 29.1 / 24.6 | 2.4 / 2.5 |
| `writes-same` 2 | 33.1 / 27.3 | 28.6 / 22.9 | 2.6 / 2.5 |
| `writes-dummy` 1 | 30.9 / 26.3 | 27.8 / 24.0 | 2.5 / 1.9 |
| `writes-dummy` 2 | 30.2 / 25.2 | 27.0 / 22.0 | 2.4 / 1.6 |

Reproduce: `OAG_SKIP_PASSES=<arm>` and `OAG_SKIP_PASSES_TOGGLE=10` as launch-file lines, a Perfetto trace from
`scripts/android-perf-trace.cfg` (which carries the `android.log` source), then
`uv run --with perfetto --with pandas python3 -I scripts/android-fence-timeline.py TRACE --by-window`.

**Verdict.** Freezing the writes takes 8.3 to 9.3 ms off the frame and as much off the GPU's own fence period, and the present
then returns as the previous fence signals. About half of that, 4.6 to 5.8 ms, is the frozen picture itself (`writes-same`
and `writes-dummy` freeze the same picture and keep it). The other half, about 3.5 to 4.7 ms, comes with the copies, and it
comes as much with copies into scratch buffers nobody reads as with copies into the buffers the previous frame still reads.
So **frame N's upload copies do not wait on frame N-1's reads; the cost is the copies and their barriers**: wgpu-core 30.0.2
records, for every `Queue::write_buffer`, a barrier from the buffer's last use to `COPY_DST` and a copy, at the head of the
submission (`wgpu-core/src/device/queue.rs`, the write's `transition_buffers` then `copy_buffer_to_buffer`). A ring of destination
buffers changes neither, so it is not built. Upstream reported the pattern as gfx-rs/wgpu#6673 and fixed it in gfx-rs/wgpu#10319
("Record 1 barrier for N Queue::write_buffer/texture calls by deferring to submit time", merged 2026-10-25, after 30.0.2's
branch; a first write per resource per submission only, copies unchanged). Its 13 hunks do not apply to 30.0.2 as they stand
(3 fail by this count; `git apply --reject` with the paths rewritten rejects 6, see the next section). What would cut the copies is fewer writes: one per-frame buffer that the passes read at offsets (the S21 perf plan's item 4).

### gfx-rs/wgpu#10319 on 30.0.2: what one barrier buys

Measured 2026-11-25 (`wgpu-write-barriers`) with a measuring build only: #10319 backported by hand onto a local copy of
wgpu-core 30.0.2 through a `[patch.crates-io]` that was never committed. With `git apply --reject`, 6 of the PR's hunks fail
there (the import list, the new `PendingWrites` methods, a `PendingWrites::clear_buffer` that 30.0.2 does not have, and the
two in the unmap path), all written by hand; the unmap path keeps 30.0.2's two inline barriers, since it is not per frame. The backport takes a
runtime switch, so `OAG_SKIP_PASSES=hoist` with `OAG_SKIP_PASSES_TOGGLE=10` turns it on in the odd windows of one run (a batch,
the writes of one submission, reads the switch when it starts, so none mixes the two ways). Same protocol as above: release
`dist` APK, HD/Fury Vineta K forward, render scale 25, REFLECTION RESOLUTION `auto`, each run from AP 33 C or less, racing
seconds 15 on, a 60 s Perfetto trace from racing second 30.

What it changes, counted inside wgpu-core per batch (S21, 120-batch averages): about 28 `write_buffer` calls into about 25
distinct buffers; 30.0.2 records one `vkCmdPipelineBarrier` before each copy (28 a batch), the backport records 3 inline (the
writes into a buffer already written in that batch, whose `COPY_DST` to `COPY_DST` barrier must stay between the two copies) and
one barrier of about 53 entries (every staging buffer and every first destination) in a command buffer ahead of the copies. The
copies themselves stay. The census prints every 120 batches, about once a second against 35 to 45 frames a second, so a frame
is about three batches (submissions): about 85 writes a frame, in line with the gdb census's 95. On the desktop's 1280x720 race bench the same count is 65 to 89 writes into 48 to 60 buffers, inline
barriers 65 to 89 against 17 to 29.

| Run (start AP) | Frame ms, 30.0.2 / #10319 | `submit` phase ms | Present ms | Previous fence to present return, ms |
| --- | ---: | ---: | ---: | ---: |
| 1 (29.4) | 23.9 / 21.5 | 6.85 / 5.77 | 13.7 / 11.8 | 2.05 / 0.05 |
| 2 (33.0) | 28.4 / 25.7 | 7.39 / 6.09 | 17.0 / 14.9 | 2.46 / 0.06 |
| 3 (33.0) | 29.0 / 25.9 | 7.90 / 6.73 | 16.8 / 14.2 | 2.25 / 0.06 |

So **the backport takes 2.4 to 3.1 ms off the S21's HD frame** (three runs, the same sign every time), 1.1 to 1.3 ms of it on
the frame thread's `submit` and the rest in the present wait: the present now returns as the previous frame's fence signals,
as it did with every write frozen (`writes`, above), where with the 30.0.2 barriers kept (`writes-same`, `writes-dummy`) it
returned 1.6 to 2.5 ms later.

**What remains: nothing measurable.** The same subtraction as above, on the measuring build with the backport on throughout
(`OAG_WGPU_HOIST=1` plus the `writes` or `writes-same` toggle, one run each), and an A/A control run (the toggle naming no
arm) for the noise:

| Run, backport on throughout (start AP) | Frame ms, made / arm | Present ms | Previous fence to present return, ms |
| --- | ---: | ---: | ---: |
| A/A control (no arm toggled) (27.3) | 20.4 / 19.9 | 9.9 / 9.3 | |
| `writes` (32.9) | 27.9 / 23.5 | 15.7 / 13.2 | 0.07 / 0.08 |
| `writes-same` (33.0) | 29.4 / 24.5 | 17.0 / 13.3 | 0.08 / 0.07 |

Dropping every write saves 4.4 ms, keeping every copy with the same frozen picture saves 4.9 ms: what the copies still cost is
the difference, -0.5 ms, inside the control's 0.5 ms. On 30.0.2 the same pair differed by 3.5 to 4.7 ms. So with #10319 the
remaining ~85 copies and ~9 inline barriers a frame cost **about 0 ms (within 0.5 ms, one run each)**, and the 4.4 to 4.9 ms
is the frozen picture itself; fewer writes would buy nothing more on this phone. The fence period in the trace moved both ways (19.6 to 17.5, 21.3 to 21.1, 20.0 to 23.4 ms), with six
windows a trace; read it as unresolved.

Desktop (RX 7800 XT, RADV, `OAG_WGPU_HOIST=1` turning the backport on for a whole run), no regression:

| Measure | 30.0.2 | #10319 | Command, load |
| --- | ---: | ---: | --- |
| Race bench CPU / GPU / frame p50, us | 1123 / 567 / 1302, 1154 / 565 / 1336 | 1180 / 564 / 1369 (a first run read 3671 / 558 / 4337 at load 28: an outlier) | `scripts/bench-render.py run hd --binary <measuring build>`, 7,005 frames, load 22 to 44 |
| User instructions, G | 83.196, 83.195 | 82.920, 83.068 (-0.2 to -0.3 %) | `perf stat -e instructions:u` over a 3,000-tick `OAG_RENDER_BENCH_RACE` run of Vineta K at 1280x720, load 46 to 50 (cycles at that load are noise) |

The GPU's p50 does not move on RDNA, as the PR's own Bevy test found on a desktop. The picture is byte-identical: a 960x540
`--screenshot` of HD Vineta K at tick 900 with every batch hoisted and with none, `cmp` equal. That is one frame on RDNA only:
no Khronos validation layer is installed on the workstation, so no synchronisation validation ran, a missing barrier would not
necessarily show on RDNA, and the S21's picture during the hoisted windows was not inspected. Upstream reviewed and merged the
reordering; a build that ships it should take it from a wgpu release.

Reproduce: the backport is not in the tree. Copy wgpu-core 30.0.2 out of the cargo registry, apply the PR's diff
(`gh pr diff 10319 -R gfx-rs/wgpu`, paths without the `wgpu-core/` prefix) with `git apply --reject`, write the 6 failing
hunks by hand, add the switch, point a root `[patch.crates-io] wgpu-core` at the copy, and build the APK as above.

### wgpu trunk (the git pin) against 30.0.2

Measured 2026-11-26 (`wgpu-trunk`): the tree on gfx-rs/wgpu trunk at `60cef2be828e` (which carries #10319 and everything
else merged since 30.0.2; `docs/architecture/dependency-upgrades.md`, "Git pins") against the 30.0.2 tree it replaced, two
dist APKs (`OAG_APK_DEV_CLI=1 OAG_APK_PROFILEABLE=1 just apk --arm64-only`), installed in turn with `adb install -r`,
interleaved main, trunk, main, trunk, main, trunk. The protocol of the section above with no toggle and no trace: HD
Vineta K, render scale 25, REFLECTION RESOLUTION auto, `--autopilot --seed 1 --no-audio --no-intro --frame-log`, each start
at AP <= 33.0 C, 200 s from launch. Medians of the `frame-log` lines:

| Round (host load main / trunk) | Frame ms, racing s 20-80: 30.0.2 / trunk | Whole race (s 10-170) | Present phase ms, s 20-80 |
| --- | ---: | ---: | ---: |
| 1 (52 / 70) | 31.4 / 19.5 | 31.0 / 19.0 | 20.4 / 10.0 |
| 2 (38 / 64) | 30.7 / 19.0 | 30.5 / 25.8 | 20.1 / 8.2 |
| 3 (50 / 37) | 32.3 / 29.2 | 29.9 / 27.5 | 20.2 / 17.3 |

Trunk is faster in every round, by 2.4 to 12.0 ms over the whole race, and the gain is all in the present phase (the wait
for the previous frame's fence); CPU work before present is unchanged (`cpu_ms - present_ms` 10.5 to 11.6 ms on both). 30.0.2
holds 30 to 33 ms through each race; trunk runs in two modes, about 18 ms for a stretch and then about 27 ms (round 1
stayed fast to the end, round 2 switched at racing second 70, round 3 at second 20). The slow mode alone is the 2.4 to 3.1 ms
the #10319 backport bought; the fast mode is more than #10319 explains, and which trunk change (or which GPU clock
behaviour) gives it is not known. GPU clock median 403 MHz in five runs, 858 MHz in trunk round 1. The surface is set up the same on both (`surface: present mode Mailbox (offered [Mailbox, Fifo]), 2 frame(s) queued`), so the fast mode is not a present-mode change. Both builds finish the race
in the same time (1:37.43) and draw the same race mid-way (phone screencaps 40 s into racing, read by eye).

One reading looks wrong and is open: the `hd_zoom` pass timestamp reads 1.6 to 2.9 ms on trunk against 0.1 to 0.2 ms on
30.0.2, while the frame got faster. The likely cause is where the hoisted write barrier now lands between timestamp pairs,
not a slower pass; it is not proven.

### Inside the scene pass

Same protocol, no trace. `track-plain` and `track-rich` leave out the shading half of the circuit's draws that name no
emissive row (Vineta K: 90 % of its 2.47 M indices) and of those that name one (10 %); the depth prepass stays. `mips` binds
every material texture two mip levels down (a measure of texture bandwidth against arithmetic).

| Arm | Frame ms, drawn / arm | Scene GPU ms, drawn / arm |
| --- | ---: | ---: |
| `track-plain` | 34.7 / 31.2 | 12.0 / 11.2 |
| `track-rich` | 33.5 / 29.2 | 11.1 / 5.6 |
| `mips` | 33.1 / 31.5 | 11.3 / 10.5 |

One run each. By the scene pass's own timestamps the rich tenth of the circuit costs half of the pass and the plain nine tenths
0.8 ms; by the frame the two read 4.3 and 3.5 ms, and on this GPU a pass's own timestamps read well under what leaving it out
saves (every skip toggle so far reads 2 to 6 times its row), so the plain half is not cheap on the frame's reading. Texture
bandwidth (`mips`) reads 1.6 ms of frame and 0.8 ms of pass. Arm's offline compiler (`malioc` 2026.5, Mali-G78 r1p1, a model of driver r51p0 where the phone runs r38p0) on
the circuit's own specialised pipelines (naga 30.0.2 to SPIR-V with wgpu-hal's Vulkan options) says why: the rich shader uses
all 64 work registers at 50 % occupancy and spills 512 bytes of stack, load/store bound (155 load/store cycles summed over every
path, against 72 arithmetic), where the plain one spills 64 bytes and is arithmetic bound at about 15 cycles. Stubbing one
function at a time moves the stack by 16 to 32 bytes each; forcing every row-kind test false takes it to 208 bytes. The fix,
which keeps the picture byte-identical, is in [hires-gpu-levers.md](../rendering/hires-gpu-levers.md), "A circuit's row kinds
fold where its table has none". With it, the same toggle (`row-codes`, a diagnostic build whose odd windows shade the rich half
unmasked):

| Run (start AP C) | Frame ms, masked / unmasked | Scene GPU ms | Fence period ms |
| --- | ---: | ---: | ---: |
| 1 (32.1) | 28.7 / 30.3 | 7.8 / 10.8 | 26.7 / 30.4 |
| 2 (33.0) | 32.1 / 32.7 | 9.0 / 11.4 | 27.9 / 31.1 |

The GPU's frame shortens by 3.2 to 3.7 ms; the frame on the CPU's side moves 0.6 to 1.6 ms, because the frame thread's own
work (about 11.5 ms) and the copies above keep it waiting.

**HD's textures as ASTC.** The S21 has no `TEXTURE_COMPRESSION_BC`, so HD's DXT textures are decoded to RGBA8. A diagnostic arm
(`OAG_DIAG_ASTC=1`, not in the tree) uploaded each decoded texture as ASTC 4x4 void-extent blocks, one per 4x4 tile at its average
colour: the size and bandwidth of real ASTC 4x4 at 8 bits a texel, not its picture, and a constant block may decode cheaper
than a general one. Separate runs (no in-run toggle: the textures are uploaded once):

| Run (start AP C) | Frame ms | p99 ms (median of seconds) | Scene GPU ms | GPU memory of the process / device |
| --- | ---: | ---: | ---: | ---: |
| RGBA8 1 (33.0) | 29.9 | 47.3 | 8.4 | 2.66 / 2.96 GB |
| ASTC 1 (32.9) | 28.8 | 41.5 | 7.0 | 1.50 / 1.80 GB |
| RGBA8 2 (33.0) | 30.8 | 48.9 | 8.4 | 2.62 / 2.91 GB |
| ASTC 2 (33.0) | 29.0 | 42.3 | 6.9 | 1.59 / 1.90 GB |

ASTC saves about 1.1 GB of GPU memory, 1.4 ms of scene pass and 1.1 to 1.8 ms of frame here (racing seconds 15 to 140,
medians of the per-second `--frame-log` lines; memory from Perfetto's `android.gpu.memory`).

The picture of a real encode, offline: Arm's `astcenc` 5.7.1 (`-medium`, 4x4) round trip of the 341 textures Vineta K's circuit
uses (`just export-host-textures`, 61.3 M texels) against our RGBA decode: median 56.2 dB (RGB PSNR), texel-weighted mean 53.4 dB,
worst 39.6 dB, 12.0 % of texels off by more than 2 levels in some channel, largest error 54 levels. 15 s of encoding on 24 host
threads for one circuit. ASTC is therefore a lossy opt-in at best, with its encode cached off the render thread:
that is the TEXTURE COMPRESSION row, measured with the real encode in
[texture-compression.md](../rendering/texture-compression.md).

### Against the S24

This page held no S24 frame-time table before this one. What exists is [`frame-thread-cpu.md`](../rendering/frame-thread-cpu.md)
("The reported phone regression"), read off a different settings profile and an older build, so the rows are not like for like:

| | S24 (Xclipse 940, 2026-10-27 builds) | S21 (Mali-G78, `d5bfef6`) |
| --- | --- | --- |
| Circuit and race | HD Vineta K, flyby and 40 s of racing (and the campaign's first event, 45 s) | HD Vineta K, Racebox single race, full races |
| Profile | maintainer's: FSR1, motion blur medium at half resolution, render scale 200, `target_fps` 60, `frame_limit` 120, CPU governor on | fresh profile: render scale 100 (and 50), the rest off or `original` |
| fps | GPU-bound, 50 to 57 | 5.6 to 8.4 at render scale 100; 10.5 to 11.4 at 50 |
| Per-second `p99_ms`, median | 18.5 to 19.4 ms | 228 to 300 ms mean at scale 100; 180 at scale 50 |
| CPU row (`cpu_ms - present_ms`) | 8 to 10 ms | the frame thread's phases: 28 ms at render scale 25 |

The S21's HD scene pass is about 30 times a desktop iGPU's 1080p race pass ([`hd-gpu-cost.md`](../rendering/hd-gpu-cost.md): 3.3 ms).

## Known gaps

- **Touch controls are a second version.** Sizes and positions are chosen and untested
  on a real phone (the S24 is the check); the cutout inset is a constant; the overlay
  also draws over the race's loading screen; no haptics, no
  user-adjustable layout, no per-player scheme (a stick or zones, left- or
  right-handed). A gamepad's analog triggers, right stick and hat are not reachable
  through winit (see "Bluetooth and USB gamepads").
- **Xclipse 940 (the S24's EU GPU, Samsung's Vulkan driver) is untested.** Expect
  surprises in present modes, `PRIMARY` backend adapter selection and the temporal
  upscaler's capability probe.
- **Lifecycle polish.** A rotation or a multi-window resize is untested, and
  so is `Stream::pause` on the S24 (see the next section).
- **ELF alignment.** NDK 27 links with 4 KB pages; a 16 KB-page device wants
  `-Wl,-z,max-page-size=16384`. `zipalign -P 16` is already applied to the archive.
- **No `MANAGE_EXTERNAL_STORAGE` flow**, so images must be pushed with `adb`; a
  launcher-side picker is future work. The icon is the legacy one, not adaptive.
- **The front end draws at a desktop window size** (`settings.window_size`); the
  phone's aspect ratio goes through the same fit as a desktop window resize.
