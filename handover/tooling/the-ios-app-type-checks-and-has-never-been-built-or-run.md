---
status: blocked-hardware
titles: [all]
updated: 2026-10-12
summary: "iOS app: run the release.yml ios job and sideload the .ipa; no device run, signing and gamepad unchecked"
---

# The iOS app type-checks for aarch64-apple-ios and has never been built or run

2026-10-12. `release.yml`'s `ios` job (`continue-on-error`, not published) builds
`oag-game` for `aarch64-apple-ios` and `scripts/build-ios-app.sh` wraps it into an
ad-hoc signed `.app` inside an `.ipa`. The phone glue lives in `oag-mobile`
([ADR-0065](../../docs/architecture/adr/0065-one-mobile-crate-for-every-phone-os.md)).
Reference: [`docs/tools/ios.md`](../../docs/tools/ios.md). `cargo check --target
aarch64-apple-ios -p oag-game` passes on Linux; the job has never run (checked with
`actionlint` only).

## Open

- **First run of the job.** Push, run `release.yml` by hand, read the `ios` job's log.
  Expect link-time surprises (frameworks, `cc`-built `-sys` crates, `re_rav1d`).
- **First launch on a device**: Metal adapter and features, `UIApplicationMain` via winit,
  CoreAudio session, the Files-app `Documents/data/images` path, the empty launch screen.
- **Signing**: only the free-Apple-ID/AltStore route is documented; TestFlight needs a
  paid account, a profile and an upload step, none started.
- **Memory**: no BC on Apple GPUs, so textures take the CPU-decode to RGBA8 path; see
  `rendering/texture-compression-only-covers-bc-astc-is-the.md`. A phone may not fit a race.
- **Input**: no gamepad (`gilrs` has no iOS backend), no on-screen keyboard, no
  safe-area insets (notch).
- **Publishing**: the nightly does not include the `.ipa`; add `ios` to
  `nightly.yml`'s download pattern and `release`'s `needs` once a run is green.
- `cargo clippy --target aarch64-apple-ios` trips `missing_const_for_thread_local` in
  `oag-gpu`; the desktop gate does not.

## Next Steps

1. Run the job once and read its log.
2. Sideload the `.ipa` (ios.md, "Signing and installing") and report the first screen.
