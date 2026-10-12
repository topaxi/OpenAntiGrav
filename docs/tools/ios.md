# iOS

`oag-game` as an `OpenAntiGrav.app` for iPhone and iPad (arm64, iOS 16+), built by
the `ios` job of `release.yml` on a GitHub-hosted macOS runner. **It has been built
on paper only: no iPhone has run it, and no Mac has built it.** On Linux the code
type-checks for `aarch64-apple-ios` (see "Compile check"); the bundle layout is
assembled and audited by a script that also runs off macOS. Landed 2026-10-12.

## Get it

Nothing is published yet. The `ios` job is `continue-on-error`, is not a `needs` of
the release job and is outside the nightly's download pattern, so a red `ios` job
cannot hold a nightly back and a green one attaches nothing. To get the file:

1. Push the branch and run the **Release** workflow by hand (Actions, Release, Run
   workflow) - or let a nightly run, which calls it.
2. Open the run, download the artifact named `ios`: `OpenAntiGrav-<v>-ios-arm64.ipa`.
3. Sign and install it, below. Or on a Mac: `just ios-app` builds the same thing
   (`rustup target add aarch64-apple-ios` first).

## Signing and installing

An iPhone runs only code signed by an Apple identity it trusts. The `.ipa` is
ad-hoc signed (`codesign -s -`), which no stock iPhone accepts, so it is meant to
be re-signed. No secret is stored in this repository and none is needed to build.

| Route | Needs | Lasts |
| --- | --- | --- |
| Free Apple ID through Xcode ("Personal Team"), or AltStore / SideStore | a Mac (Xcode) or a computer running AltServer / SideStore's pairing; any Apple ID | 7 days, then re-sign; 3 sideloaded apps at once; one device |
| Paid Apple Developer Program account (99 USD a year), TestFlight | the account; App Store Connect upload of a properly signed build | 90 days per TestFlight build |
| Developer-account ad-hoc distribution | the account and the device's UDID in the profile | 1 year |

The app's bundle id is `org.openantigrav.game`. A free Apple ID cannot use an id
another account holds, so a sideloading tool rewrites it (AltStore appends the
team); nothing in the game reads the id.

TestFlight and the App Store need more than this job makes: a distribution
certificate, a provisioning profile and an upload (`xcrun altool` or Transporter).
That, and whether a game that ships no content passes review, is open and not
started.

## Where the game files go

The app is file-sharing enabled (`UIFileSharingEnabled`,
`LSSupportsOpeningDocumentsInPlace`), so its `Documents` folder shows in the Files
app under **On My iPhone > OpenAntiGrav**, and in Finder or iTunes' file sharing.
Put the disc image in `Documents/data/images` (the app creates it on first launch).
The working directory is `Documents`, so the checkout-relative `data/images` lookup
finds it with no change to `oag-source`. Settings, records, ghosts and caches go to
`Library/Application Support` and `Library/Caches` of the app's container:
`dirs` maps iOS to those and ignores `$XDG_*`, so (unlike Android) no environment is
set. The not-found screen names the Files path. See `oag_mobile::ios::prepare`.

## How it is built

- **Entry.** The desktop `[[bin]]` is the iOS executable: winit's iOS backend runs
  `UIApplicationMain` inside `EventLoop::run_app`, so there is no second target.
  `main` calls `oag_mobile::ios::prepare()` before anything reads the working
  directory and ignores the process arguments (Xcode passes its own). Suspend and
  resume are winit's `Suspended`/`Resumed`, the same handler as Android
  (`App::suspended`, a surface recreated on `Resumed`). [ADR-0065](../architecture/adr/0065-one-mobile-crate-for-every-phone-os.md).
- **Bundle.** `scripts/build-ios-app.sh` (`just ios-app`): `OpenAntiGrav.app` with
  the binary, both licences, `licences/`, five icon sizes from the engine's own
  `--write-icon` rasteriser (run from a host build), and an `Info.plist` written
  with `plistlib`: landscape only, full screen, an empty `UILaunchScreen` (without
  one iOS letterboxes the app), `UIRequiredDeviceCapabilities` arm64 and metal,
  `MinimumOSVersion` 16.0. It runs `check-leakage.py --dir`, ad-hoc signs when
  `codesign` exists, and zips `Payload/OpenAntiGrav.app` as the `.ipa`.
- **Phone behaviour shared with Android** is `oag_mobile::MOBILE`: the touch
  overlay on by default, pilot assist at Extreme, no QUIT row (iOS has no quit).
  The iOS back gesture and a Bluetooth gamepad are not wired; a pad that iOS
  exposes through GameController is not read by `oag-input` (`gilrs` has no iOS
  backend) - open.
- **No `arboard`** (no iOS backend): the disc-key prompt uses its keypad, as on
  Android.

## Compile check

On Linux, with `rustup target add aarch64-apple-ios`:

```sh
cargo check --target aarch64-apple-ios -p oag-mobile
cargo check --target aarch64-apple-ios -p oag-game
```

Both succeed with no Apple SDK: `cargo check` runs build scripts on the host and
never links. They prove the Rust type-checks for the target, including `wgpu`'s
Metal backend, `cpal`'s CoreAudio backend and `winit`'s UIKit backend. **They do
not prove** that it links (`cargo build` needs the iOS SDK's frameworks and
`clang`), that any `-sys` crate with a C build step (`cc`) compiles for the
device, that the shaders and `re_rav1d` (the AV1 movie decoder) build, or that it
runs. Found on the way: `arboard`'s default Linux features pulled
`wayland-backend` into an iOS build, which does not compile; iOS joined its target
exclusion. A `cargo clippy --target aarch64-apple-ios` run fails on a lint in
`oag-gpu` that the desktop gate does not see (`missing_const_for_thread_local`);
not touched here.

## Known gap: texture compression

The disc textures are BC (DXT) blocks and Apple GPUs sample ASTC, not BC. wgpu on
Metal reports no `TEXTURE_COMPRESSION_BC` on an iPhone, so `texture::upload`
takes the fallback it has for any adapter without the feature: it decodes the
blocks on the CPU to `Rgba8Unorm` and uploads those. The picture is the same;
the cost is load time and GPU memory (RGBA8 is 4 to 8 times a BC block texture),
which on a phone with a per-app memory ceiling is the likeliest way a race fails
to load. Open since 2026-09-03 as the ASTC thread (no encoder, no
cache location); nothing was changed for iOS.

## Not done

- No device run, no launch, no frame. Everything above the compile check is
  unverified.
- No sound check (`cpal` CoreAudio, the audio session category).
- No gamepad, no on-screen keyboard for text entry, no safe-area insets for the
  notch (the touch overlay and HUD may sit under it).
- The nightly does not publish the `.ipa`; it is an artifact only.
