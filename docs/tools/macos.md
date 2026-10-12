# macOS

`oag-game` as an `OpenAntiGrav.app` bundle for Apple Silicon, built by the
nightly workflow on a GitHub-hosted macOS runner. **It has been built on paper
only: no Mac has run it.** The Rust code type-checks for `aarch64-apple-darwin`
(see "Compile check"), and the bundle layout is assembled and audited on Linux,
but the first launch on real hardware is open.

## Get it

The `nightly` pre-release carries `OpenAntiGrav-nightly-<date>-<sha7>-macos-arm64.zip`
(a tagged release carries `OpenAntiGrav-<v>-macos-arm64.zip`). Apple Silicon only;
macOS 11 or newer. Unzip it and move `OpenAntiGrav.app` to Applications.

## First launch (unsigned)

The app is ad-hoc signed (Apple Silicon refuses unsigned arm64 code) but not
signed with a Developer ID and not notarized, so Gatekeeper blocks a downloaded
copy. Do one of these once:

- macOS 14 and older: right-click the app, choose Open, then Open again.
- macOS 15 and newer: Apple removed the right-click shortcut. Try to open the app,
  dismiss the warning, then System Settings > Privacy & Security > scroll to
  "OpenAntiGrav was blocked" > Open Anyway.
- Any version, Terminal: `xattr -dr com.apple.quarantine /Applications/OpenAntiGrav.app`

These steps come from Apple's published Gatekeeper behaviour and are **not yet
checked on a Mac**.

## Where the game files go

Put the disc images in `~/Library/Application Support/oag/images` (Finder: Go,
Go to Folder...). That is `dirs::data_dir()` joined with `oag/images`, read in
`crates/source/src/source.rs`. Settings, records and logs sit in the same
per-user tree. Note: a Finder-launched app starts with its working directory at
`/`, so the checkout-relative `data/images` lookup finds nothing, and the
"next to the executable" lookup resolves to `OpenAntiGrav.app/Contents/MacOS`,
where a player should not put files (it would break the signature). Searching
next to the `.app` instead is not done. `ffmpeg` is looked up on `PATH`, which a
Finder launch does not inherit from a shell, so the optional intro movies need
`oag-game` started from a terminal until that is checked. See the
[installing guide](../overview/installing.md).

## How it is built

- `.github/workflows/release.yml`, job `macos`, on `macos-latest`:
  `cargo build --profile dist -p oag-game` with the same pinned toolchain and
  `OAG_VERSION_SUFFIX` the Windows job uses, `MACOSX_DEPLOYMENT_TARGET=11.0`.
- `scripts/build-mac-app.sh` (`just mac-app` runs the build and the script):
  lays out `Contents/MacOS/oag-game` and `Contents/Resources/` (both licences,
  `licences/`, an `.icns` made with `iconutil` from the engine's own
  `--write-icon` rasteriser), writes `Info.plist` with `plistlib`, runs
  `scripts/check-leakage.py --dir` over the bundle, ad-hoc signs it with
  `codesign --force --deep -s -`, and zips it with `ditto -c -k --keepParent`.
  Bundle id `org.openantigrav.game`. `CFBundleVersion` is the Cargo version (it
  must be dotted integers); the nightly name is in `CFBundleGetInfoString`.
- Off macOS the script skips `iconutil`, `codesign`, `ditto` and `plutil` and
  zips with `zip`: enough to check the layout, not a shippable bundle.
- `nightly.yml` downloads the `macos` artifact with the others, checksums it in
  `SHA256SUMS`, and says in the release notes how to open it. A failing `macos`
  job fails `build`, and so holds the nightly and a tag release back, as the
  Android job does.
- One architecture only. A universal binary would need an `x86_64-apple-darwin`
  build as well, a second fat-LTO compile, plus `lipo`; not done.

## Signing hook (disabled by default)

The step "Developer ID signing and notarization (opt-in)" in the `macos` job
re-signs the bundle with `rcodesign` (apple-codesign) using the hardened runtime,
submits it to the notary service, staples it and re-zips. It runs only when the
repository variable `OAG_MAC_SIGN` is `true` and these secrets exist; nothing
requires them:

| Secret | Holds |
| --- | --- |
| `OAG_APPLE_CERT_P12_B64` | base64 of a Developer ID Application `.p12` |
| `OAG_APPLE_CERT_PASSWORD` | its password |
| `OAG_APPLE_API_KEY_JSON_B64` | base64 of the App Store Connect API key JSON that `rcodesign encode-app-store-connect-api-key` writes |

The hook has never run. It needs a paid Apple Developer account.

## Compile check

`cargo check --target aarch64-apple-darwin -p oag-game` passes on Linux with no SDK
(2026-10-12), covering wgpu's Metal backend, winit, cpal, `arboard` and `dirs`:
`cargo check` does not link, and no dependency here has a build script that needs
the SDK. Linking, the Metal driver, CoreAudio and the window are untested. The `ci.yml` `determinism`
job already runs on `macos-latest`. A known gap: the texture-compression support
covers BC and ASTC only; see the handover thread on it.
