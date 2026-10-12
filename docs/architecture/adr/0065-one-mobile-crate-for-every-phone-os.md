# ADR-0065: One mobile crate holds the glue for every phone OS

## Status

Accepted (2026-10-12). Relates to [ADR-0050](0050-format-crates-split-by-format-family.md)
(a crate per concern, not per title) and to the `oag-thread` precedent: a small
crate under `oag-game` that hides a per-target mechanism behind one name.

## Context

Android's glue lived inside `oag-game`: a `#[cfg(target_os = "android")]` entry
(`android_main`), a `OnceLock<AndroidApp>`, the function that points `HOME` and
the `XDG_*` roots at the app's files directory, the event-loop builder and the
winit `android-native-activity` feature. iOS arrived next and needs the same
three things done differently: a working directory in the app container, an
event loop that needs no hand-over (winit's iOS backend runs `UIApplicationMain`
itself), and the Files app as the way a player gets a disc image onto the device.
Four `cfg!(target_os = "android")` tests in the game also stood for "this is a
phone", not for "this is Android" (the touch overlay, the pilot-assist default,
the missing QUIT row, the not-found screen).

## Decision

1. A crate `oag-mobile` (`crates/mobile`) holds what a phone needs around the
   windowed game: `android::{prepare, event_loop}`, `ios::prepare`, and
   `MOBILE`, a constant of the build. One crate for every mobile OS, a module
   per OS behind `cfg(target_os)`, so the next one (a Fire tablet is Android; a
   Switch-class handheld is not on the list) adds a module, not a crate.
2. Lifecycle is not in it. Suspend, resume and surface recreation are winit's
   `Suspended`/`Resumed` events, handled by `App::suspended`/`resumed` the same
   on every platform; there is no per-OS half to hold.
3. **The entry points stay in `oag-game`.** `android_main` and the iOS `main`
   call `run`, and `run` builds the windowed `App` from the binary's own
   modules. Rule 2 of `just check-deps` forbids any crate from depending on
   `oag-game`, so a crate below it cannot reach `run`. The alternative, moving
   the `App` into the lib so that `oag-mobile` could sit above as a second
   composition root, needs an exception to rule 2 and a large move of a binary's
   modules; neither is warranted for one function call. `oag-mobile` is therefore
   a library `oag-game` depends on, and `just check-deps` needed no change.
4. iOS needs no second target: the desktop `[[bin]]` is the iOS executable
   (`cargo build --target aarch64-apple-ios -p oag-game`), because winit's
   `EventLoop::run_app` on iOS is `UIApplicationMain`. `scripts/build-ios-app.sh`
   wraps it into an `.app` and an `.ipa`. Android keeps its `oag_android` cdylib
   example, which still `include!`s `main_body.rs`.

## Alternatives considered

- **A crate per OS** (`oag-android`, `oag-ios`): two crates of under 100 lines
  each, with a shared `MOBILE` constant that would belong in a third.
- **`oag-mobile` as a second composition root** that owns the entries: needs the
  `App` moved into `oag_game`'s lib and a named exception to rule 2. Cleaner on
  paper, a refactor of the whole windowed path for a gain of one `include!`.
- **Leave it in `oag-game` and add iOS beside Android**: a second block of
  `cfg` and a second storage function in a file that is already 750 lines, and
  the phone-versus-Android confusion above stays.

## Consequences

- `oag-game` loses its `ANDROID_APP`, `android_env` and the Android-only winit
  feature; it gains one dependency and no behaviour on desktop. The APK's file
  list and manifest are unchanged (the `.so` differs by 3 KB of code).
- The entry is still two files `include!`-ing one body; moving the `App` into the
  lib (so the example and the `include!` both go) remains open and is now a
  smaller change.
- `MOBILE` is a build constant, so a tablet in desktop mode or a phone with a
  keyboard is still "mobile": the on-screen controls and the missing QUIT row
  follow the OS, not the attached hardware.
- iOS has not run on a device. Everything here type-checks for
  `aarch64-apple-ios` on Linux; see [ios.md](../../tools/ios.md) for what that
  proves and what it does not.
