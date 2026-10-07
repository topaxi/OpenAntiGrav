# CI is repaired and release.yml is written, but neither has run on GitHub

2026-10-07. Every `ci.yml` run on `main` from 2026-08-28 to 2026-10-06 was red,
for the reasons recorded in [releases.md](../../docs/tools/releases.md). The fixes
and a new `release.yml` were checked locally only (maintainer rule: members never
push, tag or trigger a run; `act` is not installed): the `check` job's commands in
a clean Ubuntu 24.04 container with no `data/` (clippy `-D warnings` clean, nextest
5,322 passed), the MSRV build with `cargo +1.97.1` and `-D warnings`, the four
determinism reports and tests on Linux, a MinGW cross-build of `oag-game`, and the
Linux and Steam Deck AppImages plus the tarball through `scripts/build-appimage.sh`
and `check-leakage.py --dir`.

## Open

- **The first green `ci.yml` run is the first cross-platform determinism check
  since 2026-09-11.** The `determinism` job died at its first step on all three
  OSes once the examples were renamed (`a53cddc18`), so Windows and macOS have not
  compared a hash with the committed reference for about 26 days. A mismatch there
  is a finding to report, never a constant to update.
- Not provable locally: the MSVC link and `7z` on `windows-latest`, `upload-artifact`
  and `gh release create` on GitHub, and `libpipewire-0.3-dev` plus the other apt
  packages on the real runner image (the Ubuntu 24.04 container matches it closely,
  not exactly).
- `data/traces/talons-junction-clean-lap.csv` (817 KB) is now tracked, per
  [ADR-0046](../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md):
  three non-`#[ignore]`d `oag-trace` tests panic without it, which is the third
  cause of a red `check` job. It is a measured-state trace like the seven already
  tracked; the maintainer should confirm the commit.
- A Windows-only warning shows in the MinGW build: `APP_ID` and one import in
  `crates/game/src/main.rs` / `main/window.rs` are unused off Linux. `ci.yml` runs
  clippy on Ubuntu only, so nothing gates it; a `cfg` fix is cheap.
- Release builds are unsigned: no Windows code signing, no AppImage signature, no
  notarisation question (macOS is not built at all).

## Next Steps

1. Maintainer pushes the branch and watches `ci.yml`; read any red job with
   `gh run view <id> --log-failed`.
2. Run `Release` by hand from the Actions tab on a branch (a dry run creates no
   Release), then tag `v*` when happy.
3. A later lane adds `OpenAntiGrav-<v>-android-arm64.apk` as another job feeding
   the `release` job's `needs:`, and an AUR `-bin` PKGBUILD consuming the stable
   names in [releases.md](../../docs/tools/releases.md).
4. Optional: an `macos` artifact once macOS is a supported play target.
