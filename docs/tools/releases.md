# Releases and CI

Two workflows live in [`.github/workflows/`](../../.github/workflows/):
`ci.yml` gates every push and pull request, `release.yml` builds the downloads.
Neither is verified by pushing: the maintainer's rule is that members never push,
tag or trigger a run, so both were checked with local builds and a clean-container
rerun of the same commands (see "What was verified" below).

## ci.yml

| Job | What it runs | Notes |
| --- | --- | --- |
| `check` | `fmt`, `clippy -D warnings`, `nextest --workspace`, the Python ratchets | No `data/` on the runner: every disc-backed test is `#[ignore]`d or skips. Installs `libpipewire-0.3-dev`, `libasound2-dev`, `libudev-dev`, `libclang-dev`, `pkg-config` - the list `packaging/appimage/Containerfile` already uses. |
| `docs` | `scripts/check-doc-links.py` | |
| `msrv` | `cargo +1.97.1 build --workspace` | The floor is **1.97.1**: `wesl` 0.6 and its siblings (the shader linker) declare it. `Cargo.toml`'s `rust-version` says the same. Lowering it means dropping or downgrading `wesl`; raising it is free, but then `rust-version` and this job move together. |
| `determinism` | the three `*_determinism_report` examples and the four `determinism` test suites, release and debug, on Linux, Windows and macOS | Never edit a reference constant to make it pass. |
| `leakage` | `scripts/check-leakage.py` | |

Pinned toolchain: `rust-toolchain.toml` (1.99.0). `ci.yml` installs the same
version, so rustup never downloads a second toolchain mid-job.

### Why every run on `main` was red (at least since 2026-08-28)

Every run the API returned (the oldest 399, 2026-08-28 to 2026-10-06) failed, for
independent causes, none a real regression in the simulation:

1. `check`: `alsa-sys` could not find `alsa.pc`; the runner image no longer ships
   `libasound2-dev` and `oag-audio`'s `cpal` needs it besides PipeWire.
2. `msrv`: `rust-version = "1.88"` was a promise nothing kept after `wesl` 0.6
   (rustc 1.97.1) joined the build.
3. `determinism` (all three OSes): the workflow ran `--example determinism_report`,
   but commit `a53cddc18` (2026-09-11) renamed the three examples to
   `core_`/`physics_`/`ai_determinism_report`. The job has died at its first step on
   every run since, so **Windows and macOS have not compared a hash against the
   committed reference for about 26 days** (the four `determinism` test suites
   never ran). The first green run of the fixed job is the real cross-platform
   test; a mismatch there is a finding to report, not a constant to update.
4. Hidden behind the `alsa-sys` stop, found by running the `check` job's commands
   in a clean container with no `data/`:
   - `talons-junction-clean-lap.csv` was never tracked, so three `oag-trace` tests
     panicked on a runner ([ADR-0046](../architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md));
     it is tracked now.
   - `oag-render`'s `psp_slope_lod` read a no-GPU sentinel (`usize::MAX`) as a pixel
     count; it now skips without an adapter like the rest of the file. If the
     runner image has lavapipe those GPU tests run there instead of skipping.
   - Raising `rust-version` to 1.97.1 turns on clippy's `manual_isolate_lowest_one`;
     `gxt.rs` and four examples now use `isolate_lowest_one()`.

### What the first repaired runs found (2026-10-07)

Runs 37595443950, 37599197810 and 37599357599, after the fixes above:

- `check` died twenty minutes into `cargo nextest run --workspace` with the
  runner's own `No space left on device` (an annotation on the job, no step log).
  The job now frees the runner's unused toolchains first and builds with
  `CARGO_PROFILE_DEV_DEBUG=0`/`CARGO_PROFILE_TEST_DEBUG=0`. The steps after
  nextest (dependency rules, transcendentals, size, names, captures, handover,
  strings) have still never run on GitHub; they pass locally under `just`.
- `determinism` failed `oag-core`'s reference on macOS and Windows: the probe's
  deliberate platform `sin`, see
  [determinism.md](../architecture/determinism.md#2026-10-07-the-core-probes-own-sin-is-the-first-cross-platform-failure).
  Every later determinism step now runs even after a red one (`if: !cancelled()`),
  so one push shows every stage on every OS.

`ci.yml` also pinned 1.98.0 against `rust-toolchain.toml`'s 1.99.0; rustup's override
meant it only cost a second download, but both say 1.99.0 now.

## release.yml

Triggered by a pushed tag `v*` or a manual run. A manual run uploads workflow
artifacts only. The `release` job runs for a tag only and creates a **draft**
GitHub Release; the maintainer publishes it by hand.

### Artifacts

Names are stable: `OpenAntiGrav-<version>-<platform>.<ext>`, where `<version>` is
the tag without its `v` (a manual run uses `dev-<sha7>`). A consumer (an AUR
`-bin` PKGBUILD) builds its download URL from these.

| File | Contents | For |
| --- | --- | --- |
| `OpenAntiGrav-<v>-linux-x86_64.AppImage` | `oag-game`, baseline x86-64, built in Debian bookworm (glibc 2.36) | any 64-bit Linux PC |
| `OpenAntiGrav-<v>-linux-x86_64.tar.gz` | the same binary, both licences, `licences/`, desktop entry, icon | distribution packages, machines without FUSE |
| `OpenAntiGrav-<v>-steamdeck-x86_64.AppImage` | `oag-game` compiled for Zen 2 (`-C target-cpu=znver2`) | Steam Deck; refuses to start on a CPU without AVX2/FMA/BMI2 |
| `OpenAntiGrav-<v>-windows-x86_64.zip` | `oag-game.exe`, both licences, `licences/` | Windows 10+ x64 (MSVC target) |
| `OpenAntiGrav-<v>-android-arm64.apk` | `oag-game` as a NativeActivity cdylib, stripped, debug-signed, no game content | arm64 phones, Android 8+ with a Vulkan driver ([android.md](android.md)) |
| `SHA256SUMS` | checksums of every file above | verification, AUR `sha256sums` |

Only `oag-game` ships. `oag-unpack`, `oag-wad`, `oag-trace`, `oag-view` and
`oag-psarc-diff` are reverse-engineering tools for contributors, run from a
checkout; a player never needs them.

The Steam Deck build is the existing `just appimage-deck` recipe
(`scripts/build-appimage.sh --container --target-cpu znver2`), not a second
packaging path. Its glibc floor is already settled by
[packaging.md](packaging.md#glibc): a Deck on any SteamOS 3.5 or later branch
clears it, so no Steam Linux Runtime container is needed. Gamepad and button-glyph
detection under Steam Input is the engine's own (`crates/input/src/prompt.rs`),
nothing in the package.

**Adding an artifact later.** Another job that uploads into the same
`dist/`-style artifact name pattern and is added to the `release` job's `needs:`
is enough: `release` downloads every artifact and checksums `OpenAntiGrav-*`. The `android` job is the worked example: it runs `scripts/build-apk.sh`.

### Zero game content

Every artifact is unpacked in the job (`--appimage-extract`, `tar -x`, `7z x`)
and run through `python3 scripts/check-leakage.py --dir <unpacked>`, the same
extension list `just audit-leakage` applies to the git tree, so the repository and
the packages are held to one definition. `scripts/build-appimage.sh` calls the
same mode on its AppDir (it used to keep a shorter hand-copied list that lacked
`psarc` among others). The one allowance is the generated window icon
`oag-game.png`, by path rather than by extension.

### Cutting a release

1. Merge to `main` and wait for `ci.yml` to be green.
2. Set `version` in `Cargo.toml`'s `[workspace.package]` if it should change.
3. `git tag v0.2.0 && git push origin v0.2.0`.
4. Watch the `Release` run. It ends with a draft release holding the six files.
5. Read the draft, edit the notes, publish.

To try the build without a tag: Actions tab, `Release`, "Run workflow" on a
branch. Download the artifacts from the run page.

### What was verified, and what was not

Local only (see the lane report for the logs): the same `cargo` and
`build-appimage.sh` commands the workflow uses, in a clean Ubuntu 24.04 container
for the `ci.yml` steps, and a MinGW cross-build of `oag-game` for Windows. A
cross-build proves the code compiles for Windows; it does not prove the MSVC
link on `windows-latest`, `7z` availability there, or the GitHub-hosted steps
(`upload-artifact`, `gh release create`), which only a real run exercises. The
container had no Vulkan driver, so GPU tests skipped there. `act`
is not installed on the development machine.
