# The macOS app is built by the nightly and has never run on a Mac

`release.yml`'s `macos` job builds `oag-game` on `macos-latest` and
`scripts/build-mac-app.sh` wraps it into `OpenAntiGrav.app`. Reference:
[`docs/tools/macos.md`](../../docs/tools/macos.md). The code type-checks for
`aarch64-apple-darwin` on Linux; the workflow was checked with `actionlint` and
`act -l` only, and the job itself has never run.

## Open

- **First run of the job.** Push, then `gh workflow run nightly.yml -f force=true`.
  Expect surprises at link time (frameworks), `iconutil`, or `codesign --deep`.
- **First launch on a Mac** (the maintainer has none): Metal adapter, window, CoreAudio,
  Gatekeeper wording on macOS 15 (taken from Apple's published behaviour, unchecked).
- **Finder launch environment.** cwd is `/`, `PATH` has no ffmpeg; the portable lookup
  resolves inside `Contents/MacOS`. Searching next to the `.app` is not implemented.
- **Universal binary.** `x86_64-apple-darwin` plus `lipo` is a second fat-LTO build;
  not done.
- **Signing hook** (`OAG_MAC_SIGN`, rcodesign) is unused and unexercised.
- **iOS** has its own thread: `tooling/the-ios-app-type-checks-and-has-never-been-built-or-run.md`.
- **Texture compression** covers BC and ASTC only; see
  `rendering/texture-compression-only-covers-bc-astc-is-the.md`.

## Next Steps

1. Run the job once and read its log.
2. Have someone with a Mac open the zip and report the first screen.
