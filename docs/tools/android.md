# Android

A first pass: `oag-game` as an arm64 NativeActivity app, written to open a window,
bring up wgpu on Vulkan and reach the front end, reading the player's own disc
images from the app's files directory. It has been built, linked, packaged, run on Waydroid and driven through a race
with touch. It has **not** run on a phone yet. Lifecycle polish is open;
"Known gaps" lists it.

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

`just launch-android` starts the app and follows `adb logcat -s oag` (cleared first, so only this run's lines); `--stop` forces a cold start, `--no-logs` only launches.

`just logcat-android` follows the same log without launching anything (`--clear`, `--dump` to print and exit, `-- ARGS` for another filter).

All four recipes talk to one device. `--serial S` or `$ANDROID_SERIAL` names it; with
neither, a single authorised device is used as is, and with more than one (the
phone and Waydroid, say) they ask which - an fzf pick, or a numbered prompt without
fzf - and refuse with the list of serials when not run from a terminal
(`scripts/adb-pick-device.sh`). An `unauthorized` device is never offered.

An installed app signed with another key (built elsewhere, or before the key moved
out of `target/`) cannot be updated in place: `adb install -r` fails with
`INSTALL_FAILED_UPDATE_INCOMPATIBLE`. `just deploy-android` then offers to
reinstall, keeping the app's data: everything the app writes lives under
`/sdcard/Android/data/org.openantigrav.game`, which it moves aside on the device,
then uninstalls, installs, starts the app once and copies the kept files back in.
Copied, not moved: a directory carried over from the previous install stays
unreadable to the new one (`Permission denied` on `settings.toml`, no image found,
measured on the S24), while files copied into directories the new install created
work. It asks in a terminal; `--reinstall` skips the
question.

## How it is wired

- **Entry.** `crates/game/src/main_body.rs` carries `android_main`. Two crate roots
  `include!` it: `main.rs` (the desktop binary, which also keeps the crate's rustdoc
  header - an included file may not carry inner attributes) and `android.rs`, the
  `oag_android` example with `crate-type = ["cdylib"]` and
  `required-features = ["android"]`, which `build-apk.sh` turns on. That is the cost
  of the windowed `App` living in the binary's own modules: a bin cannot be a cdylib,
  `cargo rustc` refuses to mix the two crate types, and nothing may depend on
  `oag-game`. Both roots sit in `src/`, so the body's `#[path = "main/..."]` module
  paths resolve the same from either. Until 2026-10-07 both targets pointed at
  `main.rs` itself and Cargo warned "found to be present in multiple build targets"
  on every invocation. Moving the `App` into the lib would remove the example
  altogether; that is still open (handover thread).
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

## Startup on a phone

What a first launch does, as opposed to a desktop run (2026-10-07):

- **A movie with no picture is skipped at once, on every platform.** A phone has
  no `ffmpeg`, so the intro would otherwise be a black screen for the forty
  seconds its 1200 frames take. `Frontend::update` presses Start for a movie leg
  whose plan has no picture (`Input::inject_press`), so the leg leaves through
  the same handler a real press uses; a tap skips it too, as a real intro's does
  (the composition root turns an unhandled click into Start and Cross). The
  `INTRO FRAME n / m` counter is now `--overlay` only, everywhere.
- **No ffmpeg also means no ATRAC3+ sound**, so the front end is silent as well
  as pictureless. The fix is not to need ffmpeg on the device: convert on the
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
the root (log line `system back`), at the root and at the Language Selection the
process stayed alive, and in a race it opened the root menu over the parked race.

### On-screen racing controls

`oag_input::touch` holds the layout, the finger tracking and the rules, with no
winit type, so every rule is a unit test; `oag_game::touch_controls` draws it as
plain translucent rectangles with a word on each (no disc art, no invented icons);
`crates/game/src/main/touch.rs` is the glue. Shown only on Android (or on any
platform with `OAG_TOUCH_CONTROLS` set, which is how to look at it on a desktop with
a touchscreen), only in a running race, and drawn under the pointer in the overlay
pass.

| Control | Where | Button |
| --- | --- | --- |
| steering stick | a finger landing in the left 40% below the top strip is the stick; where it landed is centre, 16% of the window height is full lock; X steers, Y pitches | stick axes |
| GO | right edge, mid-height, largest (0.36 of the height) | `Cross` (thrust), plus `L`/`R` in its brake corners |
| FIRE | left of GO, in the thumb's arc | `Square` |
| ABSORB | upper right, left of BRAKE R | `Circle` |
| BRAKE L, BRAKE R | upper corners, below the lap and record readouts | `L`, `R` (airbrakes) |
| PAUSE, VIEW | right of BRAKE L | `Start` (pause), `Select` (camera cycle) |

**Dynamic GO (2026-10-07, touch-go lane, chosen, not measured).** The finger's
position *within* GO, tracked while it is held, adds an airbrake: the bottom 34% of
GO is split into a left corner (34% of its width, `L`), a dead centre column and a
right corner (34%, `R`); the centre and everything above is thrust alone. Sliding
between zones needs no lift, and a thumb in the slop below GO still reads the corner.
It is on by default behind `[controls] touch_go_zones` (CONTROLS page, GO BRAKE ZONES,
`on`/`off`); off makes GO thrust alone. The separate BRAKE L/R buttons stay. GO draws
the two corners with `L` and `R` and thin dividers, and the corner under a finger lights
amber. Zones add to the reading only: they latch no tap. A stick-zone finger that lands
on a control is that control's finger (controls win over the floating stick, which
the upper-left brake buttons need).

**Design pass (chosen, not measured).** Sizes are fractions of the window's height
(GO 0.36, about 24 mm on a 6.2 in phone; the small buttons 0.16 to 0.22, 11 to 15 mm),
so a button is one physical size on every phone. Everything stays 0.05h in from the
left and right and 0.03h from the top and bottom for rounded corners and a camera
cutout; the cutout itself is **not queried** (winit 0.30 has no insets API). Idle
fills are 7% white with a 45% outline; a held control turns cyan with a near-opaque
outline. The first version's BRAKE buttons sat on Pulse's lap and record readouts
and GO on speed and shield; checked against Pulse and HD (Waydroid and a desktop
window, 16:9 to 2.17:1) the controls now keep to the free mid-height of each side.
PAUSE and VIEW moved off the top centre because HD draws its shield meter there.
No controls-opacity setting was added.

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
- Verified on Waydroid: race running (timer 0:13), `input keyevent 3` (HOME) logged
  `window away (Suspended): Menu`, relaunch showed the root menu over the parked race, Back
  resumed it from 0:16 with the timer running. **Not exercised:** the audio actually going
  quiet (no listening allowed; `--no-audio` does not apply on Android, and the log showed no
  `could not pause` warning), `Occluded` on a desktop (Xvfb has no compositor to send it),
  the S24.

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
