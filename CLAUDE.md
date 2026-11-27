# CLAUDE.md

Guidance for Claude Code in this repository.

## What this is

A clean-room Rust reimplementation of the Studio Liverpool anti-gravity racing
engine: **Wipeout Pure, Pulse, HD/Fury, 2048 and the Omega Collection** on one
shared engine. **Pulse** (PSP/PS2) is the reference: most reverse-engineered
and most verified. The other titles are built on the same architecture as their
milestones open. The original executables are a specification, not source:
behaviour is observed, documented with evidence and a confidence score, then
reimplemented fresh. Nothing is transliterated from the decompiler.

Check [`docs/overview/roadmap.md`](docs/overview/roadmap.md) and
[`docs/overview/status.md`](docs/overview/status.md) (per-title, per-subsystem
matrix) before assuming a subsystem exists.

## Commands

`just` is the gate every commit must pass (`just --list` shows every recipe,
`just --show check` the checks). Each `check-*` recipe's comment says what it
guards.

```sh
just fmt | lint | build | docs
just test         # nextest, quick profile (our crates opt-level 0, examples not built)
just test-data    # also the #[ignore]d ground-truth tests; needs data/images/; skips binaries that passed unchanged, --full runs all
just lead-test-data [-n M]   # the lead's full test-data as M (4) slices, each under its own gate lock; never inside `flock`
just gate-affected / test-data-affected   # same checks in full, only the tests the branch can reach (scripts/affected.py); a drive member's gate
scripts/gate-run.sh RECIPE   # a gate recipe under the shared gate lock, prebuilt while it waits (docs/architecture/gate-wall-time.md); never inside `flock`
just check-docs   # internal links in docs/ (not CLAUDE.md, .claude/ or HANDOVER.md)
just check-deps | check-unused-deps (cargo shear) | check-determinism | check-size | check-comment-history   # the rules below
just check-names | check-handover | check-schemas | check-weapon-lineage | check-guests   # `check-guests --strict` (the gate) fails a stale ext/*.wasm; `just rebuild-guests` regenerates
just check-temp-use # no test, script or recipe writes under /tmp (a tmpfs with a quota)
just check-status # status.md's RE-coverage table matches names.tsv; `just gen-status` regenerates it
just check-title-branching | check-title-reach   # ratchets: per-title behaviour is `Title` data (ADR-0058), and generic crates reach a title only through `oag_title::Title`, the oag-source registry or the composition root
just check-hd-clock-grab   # every HD/Fury grab of the original goes through `rpcs3-drive.py`'s `grab_with_clock`
just bench-render [title...] # whole-race render benchmark; takes the gate lock itself, never inside a gate (docs/tools/render-bench.md)
just audit-leakage           # no tracked game content
just unpack|wad|view|play ...   # asset tools and the game; just build-allegrex (JDK 21); just mine-names <image>; just blender-fixtures (Blender 4.2+, not in the gate)
just sync-saves <host> [-n]     # data/saves/ both ways with another workstation over ssh, newer file wins, nothing deleted
```

Single test: `cargo nextest run -p oag-core some_test_name` (nextest, not
`cargo test`). `just test` builds `target/quick`, `just test-data` builds
`target/debug`; pass `--cargo-profile quick` to a plain test to reuse `just test`'s
build, and omit it for a ground-truth test (`crates/*/tests/*ground_truth*.rs`:
`#[ignore]`d, needs a disc image under `data/images/`, never runs in CI).

### The test-data suite

Ceilings (`scripts/check-test-budget.py`): suite 450 s, single test 300 s,
both *under load*, so they catch gross regressions, not drift; judge speed on
an idle machine or not at all. `just test-data` skips a test binary that passed
unchanged (`scripts/test-data.py`; skips are listed by name, never counted as
passed); `--full` (or `OAG_TEST_DATA_FULL=1`) runs everything and is what the
suite ceiling is judged on. Measurements and the profiling method:
`docs/architecture/workspace-layout.md` ("Test-data suite measurements").

When a ceiling fires, **stop and re-profile the tail before adding tests**: sort
`target/test-data.log` by time. The slowest test is almost always a matrix
(circuits x tiers x seeds) in nested `for` loops in one `#[test]`, which runs on
one core: make the matrix the test axis with a split the assertion survives
(`workspace-layout.md`, "The unit of parallelism is the test, so a matrix must
be tests"). Add a `BASELINE` row only when a split would change what is
asserted, and say why in the row's comment (`ram_ground_truth.rs`).

**Before adding a disc-backed test, find what already walks that data.**
Nextest shares nothing between tests, so a second test over an archive pays the
whole read and decode again.

1. `rg` the archive or file family in `crates/*/tests/`; if a test walks it, add your claim as a named check function over that one pass (`crates/rcs/tests/gxp_ground_truth.rs`).
2. Sniff before you read (`Archive::peek`); read only what the assertion needs; stop a simulation at the tick its claim is decided.
3. A test that only prints is a scratch sweep: `#[ignore]` with a reason and an early return unless `OAG_SWEEP` is set.
4. Remove redundancy before splitting for the tail: every slice pays its own fixture load.

**A test writes through `oag_testdata::scratch`, never `/tmp` or `std::env::temp_dir()`**: `dir(label)`
for files removed with the test (bind the guard for as long as they are read; `OAG_KEEP_SCRATCH=1`
keeps them), `cache(name)` for a transcode worth reusing. Scripts use `scripts/oag_tmp.py` or
`scripts/oag-tmp.sh`.

**A new `tests/*.rs` file is a `mod` line in its crate's `tests/it*.rs`**, not a
binary of its own. `oag-game` and `oag-raceplay` have five binaries
(`tests/it_*.rs`, by first letter); an unlisted file is never compiled
(`every_test_file_is_listed` fails naming it). `oag-physics`, `oag-gameplay` and
`oag-ai` keep a binary per file (CI runs `--test determinism` by name). Test
names carry the module path; filter with `test(...)`, not `binary(...)`.

**A test module that spawns `oag-game` (or calls `capture_pixels_from`/`capture_from`)
is one name in the `gpu-vram` group's regex in `.config/nextest.toml`**: it caps
those tests at 4 at once so an 8 GiB GPU does not die with `wgpu error: Out of
Memory` (`docs/architecture/workspace-layout.md`, "Test groups").

**A test goes in the lowest crate whose dependencies reach everything it reads.**
`oag-game`'s binaries rerun on nearly every change: a test belongs there only if
it names `oag_game` or spawns the binary; a race test that needs only
`oag_raceplay::race` goes in `oag-raceplay`, a decoder census in the decoder's crate.

### Gates by change

- **Affected-only gating**: `scripts/affected.py` selects the tests that link the change. A docs-only or comments-only change selects no tests; the root `Cargo.toml`, the toolchain or `.cargo/` selects the full suite. Rules and misses: [`docs/architecture/affected-gate.md`](docs/architecture/affected-gate.md). Drive members use `gate-affected`; the lead runs the full `just` and `test-data` once per merge batch (CI has no images). See `.claude/skills/oag-drive/SKILL.md`.
- **Docs-only** (`docs/`, `handover/`, `HANDOVER.md`, `.claude/`, or doc comments in a `.rs` file): run `just check-docs`, plus `just check-handover` if `HANDOVER.md` changed. If any `.rs` file changed, also `just fmt-check` and `just lint`. If the change turns out to touch code logic, run the full `just`.

### Data and images

`data/` is gitignored: user-supplied disc images and derived output only
([`data/README.md`](data/README.md)). **No game content of any kind may be
committed** (`.gitignore`, CI's `leakage` job, `just audit-leakage`;
[`docs/overview/legal.md`](docs/overview/legal.md)).

- **`fd` and `rg` skip `.gitignore`d paths and return nothing under `data/` with exit code 0.** Use `--no-ignore`, or `ls`/`find`/`grep`.
- **A spawned agent that sees `data/images/` empty has a sandbox limitation, not missing images**: `data/` does not travel into a worktree or restricted sandbox. Check `/bin/ls -la data/images/` first. If empty, **say so in your report** rather than skipping the real-data check, and give the orchestrator the exact command to run.
- **Save states never go in a repository**, the private one included (memory dumps, [ADR-0085](docs/architecture/adr/0085-comparison-originals-live-in-a-private-repository.md) rule 4); they travel with `just sync-saves <host>`.
- **In a worktree run `just link-data`** right after `git worktree add` (symlinks `data/` subdirectories from the main checkout; no-op in the main checkout). Without it, tests skip silently, or fail in a heap under `OAG_REQUIRE_GAME_DATA=1`.

## Commit messages

End a commit with the `Co-Authored-By:` trailer naming the model, and nothing
else of that kind: **no `Claude-Session:` line or other session link**,
whatever an attribution template suggests (maintainer's rule).

## Architecture

**The simulation must not know a renderer exists.** It takes an input snapshot
in and emits a state snapshot out; drawing, audio, windowing and file I/O are
outside it. That keeps it testable without a GPU and comparable against a trace
from the original.

Layering: everything depends on `oag-core`; the simulation crates (physics, ai, weapons, race) with `oag-vex` and `oag-tables` meet in `oag-gameplay`, which depends on no render, audio, input or title crate (rule 1); the disc/format/asset chain, the title packages and the render, audio and input crates meet in `oag-game`, the composition root (diagram: `docs/architecture/workspace-layout.md`, "Dependency rules").

Crates (`crates/<name>`; full table and reasoning:
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md)).
Add a crate only when its milestone opens; no placeholder crates.
**Every crate has a short `README.md` in its root, and a new crate lands with
one**: what it is for in a few lines, plus the relations that matter (its layer,
what it must not depend on, the crate it complements). No dependency lists, API
tables or status: `Cargo.toml`, rustdoc and `docs/` carry those.

ADRs are `docs/architecture/adr/NNNN-*.md`.

| Crate | Purpose |
| --- | --- |
| `oag-bytes` | Endian-aware byte reading, the one `ByteOrder`. |
| `oag-core` | Deterministic `f32` math (no SIMD), `TickClock`, seeded `Rng`, state hashing. Everything depends on it. |
| `oag-disc` | CHD and raw ISO readers, ISO 9660, platform identification. |
| `oag-display` | Screen/shape/pixel/brightness vocabulary, a source's layout space. Depends on `oag-disc`, `serde` only. |
| `oag-formats` | Containers and sound: WAD, PSARC, LZSS, sound banks, magic triage, GE swizzle (ADR-0050). |
| `oag-rcs` | `RCSMODEL` scene (PS3/Vita): models, materials, visibility sets, Vita shaders. |
| `oag-vex` | `.vex` scene tree and payloads: geometry, collision, spline, pads, PVS, fog, lights, emitters, PS2 VIF, 2048 collision. |
| `oag-pob` | `.pob` particle system. Depends on `oag-formats`, `oag-bytes` only. |
| `oag-texture` | Every pixel format decoded to RGBA8888, plus a PNG writer. |
| `oag-tables` | XML-authored tables (handling, weapons, campaign, load manifests, lighting). Zero dependencies. |
| `oag-video` | `.PMF`, `IPUF`, Bink, the IVF/AV1 cache. Depends on `oag-bytes` only. |
| `oag-unity` | Generic Unity asset-bundle reader; depends on `oag-bytes` only (ADR-0062). |
| `oag-ballisticng` | BallisticNG Workshop content on `oag-unity` as typed structs. Reads; maps nothing into this engine. |
| `oag-bng-import` | Maps a Workshop track into `oag_trackfile::Track`; `oag-raceplay` reaches the two readers only through it (`ballisticng` feature). Forbidden to gameplay crates. |
| `oag-trackfile` | Own custom-track format: glTF 2.0 binary plus TOML manifest. Forbidden to gameplay crates (ADR-0063). |
| `oag-schema` | The `[toml-schema]` header of every TOML of ours (ADR-0064); `validate` is tests-only. |
| `oag-assets` | Runtime `Archive`: a WAD by path or out of a disc image. |
| `oag-ui-screens` | Front-end screens on `oag-ui`. `oag-ui` never depends on it. |
| `oag-title` | The `Title` type and its three axes (archive candidates, entry names, foreign-serial deny-list). Types only. |
| `oag-pulse` | What Pulse ships: archive/entry names, hashes, presentation tables. |
| `oag-pure` | Same for Pure, deliberately thin: only what `docs/formats/pure-status.md` measured. |
| `oag-hd` | Same for HD/Fury; front end `Measured` (ADR-0025, `docs/formats/hd-frontend.md`). |
| `oag-omega` | Same for Omega (PS4): HD's front end off a base package plus a mandatory patch; boot chain `Provenance::Declared`. Races start but are incomplete: `docs/formats/omega-status.md`. |
| `oag-2048` | What 2048 ships (Vita PSARC packages and paths), deliberately thin. |
| `oag-tools` | CLI: `oag-unpack`, `oag-wad`. |
| `oag-physics` | Ship dynamics and collision queries. Depends on `oag-core` only. |
| `oag-race` | Race rules: modes, lap timing, progress. |
| `oag-ai` | Opponent driver on the authored racing line. Depends on `oag-core`, `oag-physics` only. |
| `oag-weapons` | Pickups, projectiles, blasts, beams; reaches a craft only through the `Craft` trait. |
| `oag-gameplay` | The `World` struct, `InputSnapshot`, spline spawning. |
| `oag-replay` | Replays (input stream is the truth) and ghosts (pose track), ADR-0055. |
| `oag-mesh` | Mesh pipeline: decodes `.vex`/`RCSMODEL`, draws offscreen or to a surface; orbit camera, capture. |
| `oag-gpu` | Shared GPU vocabulary: perf probe, timestamps, offscreen formats. |
| `oag-post` | Bloom, tonemap, motion blur, SMAA, FXAA, FSR 1/3. |
| `oag-fx` | Render-side effects: `.pob` player, exhaust, mist, clouds, beams, flashes, hull overlays. |
| `oag-render` | wgpu renderer: track ribbon, shadows, PVS, cameras; owns no window. Reads `oag-pulse`'s tables (allowed, see rule 3). |
| `oag-present` | Scene-to-glass: upscale/grade/filter composite, dynamic resolution, perf meter. |
| `oag-shader-check` | Build-dependency only: validates every WESL shader with `naga`. |
| `oag-livery` | A race's hulls, skins, shields, plumes, wrecks from archive entries. |
| `oag-thread` | A closure on another thread on every target. Depends on nothing of ours. |
| `oag-ffmpeg` | The one `ffmpeg`/`ffprobe` resolver and verified download (`docs/architecture/ffmpeg.md`). Leaf. |
| `oag-io` | File I/O helpers: the atomic write (ADR-0095). Std only; leaf. |
| `oag-log` | Binaries' log sink: terminal plus a pruned timestamped file. |
| `oag-mobile` | Android/iOS app glue under `oag-game` (ADR-0065). |
| `oag-mod` | Mods (folder, `.zip`, `.oagmod`), `oag-mod.toml`, search path, built-in `oag-extras` (ADR-0066). |
| `oag-modhost` | WebAssembly extension host behind the `extensions` feature; knows no game (`docs/architecture/wasm-extensions/`). |
| `oag-ext-abi` | ABI 1 defined once: buffer layout, records, `REVISION`. `no_std`, no dependencies. |
| `oag-ext` | The Rust guest SDK a mod is built on. No engine crate may depend on it. |
| `oag-net` | Network play: wire messages, WebTransport `Link`, `host`, `client` (feature), `oag-server` (ADR-0070). |
| `oag-input` | Real devices to the abstract button layer and an `InputSnapshot`. |
| `oag-view` | Asset viewer: CLI, window, texture browser. |
| `oag-trace` | Per-tick trace capture and comparison against the original. |
| `oag-audio` | Voice pool, mixer, `cpal` stream or none. Cues are a per-tick *output*, never `World` state (ADR-0018). |
| `oag-music` | A title's soundtrack: reading it, decoding ATRAC3+, ATRAC9, MP3, Wwise to PCM. |
| `oag-sound` | Soundtrack selection, effect banks and cues, HD's authored mix, over `oag-audio`. |
| `oag-raceplay` | A race from load to finish. Drawing half is the default `view` feature; without it, the simulation half `oag-server` links. In `NOT_TITLE_PACKAGES`. |
| `oag-source` | Finding and opening a title's disc image, DLC packs, a race's sources, caches. |
| `oag-hud` | In-race HUD: layout model, `Readout`, `Draw` list, `sprite` sheet. No GPU pass. In `NOT_TITLE_PACKAGES`. |
| `oag-ui` | Front end: boot movies, menus, HUD font, strings, the `Draw` vocabulary. In `NOT_TITLE_PACKAGES`. |
| `oag-game` | Composition root; a thin `[[bin]]` over `[lib]`. |
| `oag-testdata` | **Dev-dependency only.** Finds disc images for tests; the `OAG_REQUIRE_GAME_DATA` skip-or-fail contract. |

`NOT_TITLE_PACKAGES` crates (`oag-raceplay`, `oag-hud`, `oag-ui`,
`oag-ui-screens`) draw or read titles, so nothing gameplay-side may ever depend
on them.

Six dependency rules, enforced by `just check-deps`
(`scripts/check-dependency-rules.py`):

1. No gameplay crate depends on `oag-render`, `oag-fx`, `oag-audio`, `oag-input`, `oag-net`, `winit` or `wgpu`. The simulation consumes an input *snapshot type* owned by `oag-gameplay`.
2. No crate depends on `oag-game`, and no engine crate on the guest SDK `oag-ext`.
3. No generic render-side crate (`oag-fx`, `oag-gpu`, `oag-mesh`, `oag-post`) reaches a title package (`oag-pulse`, `oag-pure`, `oag-hd`, `oag-omega`, `oag-2048`); tables are injected by the caller. `oag-render` still reads `oag-pulse`'s presentation tables and is not listed.
4. `oag-unity` depends on no workspace crate but `oag-bytes` (allowlist). A content layer is a separate crate on top, never the reverse.
5. `oag-ballisticng` reaches `oag-unity`, `oag-texture`, `oag-formats`, `oag-bytes` only; no gameplay crate may depend on it or on `oag-unity`. `oag-bng-import` is the one crate that depends on both.
6. `oag-server` (`oag-net` built with default features) links no renderer, window, audio or UI crate: `cargo tree -p oag-net -e normal` reaches none of `wgpu`, `winit`, `cpal` or the drawing crates (`oag-render`, `oag-fx`, `oag-audio`, `oag-sound`, `oag-mesh`, `oag-ui`, `oag-hud`, `oag-input`). Checked per package, because the workspace's unified features hide it.

Two size rules, `just check-size` (`scripts/check-file-size.py`), ratchets over
a frozen `BASELINE` (files may shrink, not grow; `TEST_BASELINE` is **empty**):

1. A Rust file may not exceed **1,500 lines**.
2. An inline `#[cfg(test)] mod` body may not exceed **500 lines**. Past that: `#[cfg(test)] mod tests;` plus `<module>/tests.rs` (`crates/physics/src/airbrake.rs`; `use super::*` still works). A mod of shared helpers is exempt; a dedicated test file is bound by rule 1 alone. Check a move with `cargo nextest list -p <crate>` before and after: a `mod tests;` that never landed looks like a green run with fewer tests.

Gameplay state is one `World` of plain data with fixed-size arrays, no ECS
([ADR-0003](docs/architecture/adr/0003-no-ecs.md)), so it snapshots in one
`memcpy`-shaped operation.

**A new `assets/ui/menu.toml` row or page needs a `string_id`/`title_string_id`
and its English text in `assets/ui/strings/english.toml`, in the same change**
(`just check-strings`; pre-existing debt is frozen in the script's
`BASELINE_LABELS`/`BASELINE_TITLES`). Another language may lag only by naming
the gap in its `[untranslated]` table, never by omitting the id. Machine
translation may fill a gap as `ID = { text = "...", human = false }`; `human`
is `true` only when a human wrote or approved it.

**GPU debugging**: wgpu debug labels are off in every build; set `OAG_WGPU_DEBUG=1` on `oag-game` for RenderDoc or to read a validation message by label (`docs/architecture/load-time.md`).

### Logging

Level table and how to get lines back: [`docs/architecture/logging.md`](docs/architecture/logging.md).
The default filter shows `info` from our crates, so **every `info!` is a line each
player reads on every launch**. `error`: the player loses something. `warn`:
degraded but working, including every absence. `info`: a lifecycle line a user
wants (the disc, the renderer, a race starting, an acknowledged flag, a capture's
result). `debug`: what a load or subsystem did (counts, timings, loader reports,
"armed" lines, anything that repeats during play). `trace`: one line per item.
**Default a new line to `debug`**; prefer no line over a line nothing reads. The
simulation crates log nothing. Before moving an existing line, grep
`crates/*/tests` and `scripts/` for its text: some tests parse them.

### Determinism

Bit-identical simulation across Linux/Windows/macOS, x86-64/AArch64, debug and
release (not with the PSP/PS2 itself). Full rules:
[`docs/architecture/determinism.md`](docs/architecture/determinism.md).

- `f32` only, strict IEEE-754, no `mul_add`, no reassociating arithmetic. No SIMD in sim code (`glam` is built with `scalar-math`).
- No `HashMap`/`HashSet` iteration feeding simulation state.
- No wall clock; use `TickClock` (`crates/core/src/tick.rs`). Fixed 60 Hz timestep, though the original is variable ([ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)).
- All randomness through the seeded `Rng` (`crates/core/src/rng.rs`).
- Single-threaded unless proven otherwise.

These bind `oag-core`, `oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race` and
`oag-formats` (what `just check-determinism` scans). `oag-render` is exempt (a
pixel is not compared across machines): `mul_add`, SIMD glam, reassociation and
the algebraic float ops (`f32::algebraic_add`) are fair game in a proven-hot
per-frame loop.

CI's `determinism` job asserts hashes match a committed reference
(`crates/core/src/hash.rs`, `crates/core/tests/determinism.rs`). **When it
fails, find the bug. Never update the reference constants.**

## Reverse-engineering workflow

`docs/` is a deliverable: a contributor should understand the engine from it
without reopening Ghidra. Everywhere in `docs/`:

1. Record the evidence, not just the conclusion.
2. Every RE claim carries a 0-100 confidence score
   ([rubric](docs/reverse-engineering/confidence-rubric.md)).

- Names are `Subsystem_VerbNoun`. **Below 70 confidence add `_q`; below 50 do not rename** (leave `FUN_08831af0`, write the hypothesis down).
- Every rename needs a page under `docs/ghidra/functions/<binary>/` (address, purpose, args, confidence, evidence). Docs are authoritative over the Ghidra database ([ADR-0005](docs/architecture/adr/0005-ghidra-conventions.md)).
- **Every recovered name also goes in `docs/ghidra/functions/psp-pulse-usa/names.tsv` in the same change**: `address<TAB>kind<TAB>name<TAB>confidence<TAB>page`, `kind` is `function` or `data`, name column is the bare name (the `_q` is derived from confidence). `just check-names` refuses a row whose address and name are not both on the cited page; `just apply-names` replays the file into a fresh Ghidra import.
- Loop: observe, hypothesise, verify, document, implement ([methodology](docs/reverse-engineering/methodology.md)). Do not implement from a plausible reading.
- Ghidra setup: [`docs/reverse-engineering/toolchain.md`](docs/reverse-engineering/toolchain.md); PSP needs the Allegrex/VFPU module (`just build-allegrex`, [`docs/psp/allegrex-vfpu.md`](docs/psp/allegrex-vfpu.md)). The `ghidra-mcp` server (`.mcp.json`) drives Ghidra when a project is open.

### A 2048 finding is checked against Omega, and the reverse

2048 (Vita) and Omega (PS4) share an asset lineage: Omega reads through 2048's
`.rcsmodel` readers, keeps 2048's ship directories and plugin split, and
re-ships 2048's circuits at the same layouts
([`docs/formats/omega-status.md`](docs/formats/omega-status.md)). Re-shipped is
not identical: XML tables are byte-identical, but models are re-cooked,
textures are `.gnf` against `.gxt`, shaders are recompiled. HD's circuits ship
in 2048's container with HD's geometry; Omega's front end is HD's. So a law
read off one title's shader or model is checked against the other's own file,
never assumed; a front-end finding is checked against HD.

**Anything recovered or wired for one of the two (format field, law, material
rule, table, asset path) gets a quick check against the other in the same
change**: does it ship the same file or code, and does the reader or wiring
hold? Record the answer on the finding's doc page as one of:

- **ported**: the other title uses it too, with its own test or ground truth;
- **checked, applies, not wired**: same data or code, wiring left open, named in a `handover/` thread;
- **checked, differs**: what differs, with evidence;
- **not checkable**: why, never silence. "No PS4 emulator" is not a reason (Omega's executable is in Ghidra, shadPS4 runs it); no memory readout, so a live Omega value, is.

The check is a census and a reader run, not a second lane; if it becomes real
work, write it down as open. Omega's incomplete racing does not exempt a format
or asset finding.

### Never invent what the assets already author

**If a thing exists in the disc's own data, play the data. Do not author a
stand-in, and do not hand-transcribe it into a `const`.** This covers particle
effects (`Data\Psys\*.POB`), models, textures, tables and tuning values. A
hand-transcribed table is correct but not re-derivable; a plausible stand-in (the
rocket's invented smoke puffs) is how a wrong picture survives review.

Parse it and play it; if it will not parse, **draw nothing and say so** in the
loader report. Where a stand-in cannot be avoided (a texture with no located
WAD entry), keep it a substitute for the missing asset alone, never an override
of data you have. Effects load generically (`oag_fx::psys::Library`,
`psys::Stage`): adding one is adding its name and its **trigger**, and the
trigger needs reverse-engineering; an effect with no recovered trigger stays
unwired rather than fired on a guess (`docs/formats/pob.md` lists them).

**Label provenance only where it is in doubt.** A value standing in for
something the original has is marked **chosen, not measured** (no confidence
score; a score means measured). Where it is self-evident the thing is ours
(state-hash tag bytes, our file formats, settings, tooling, a module only this
engine has), write no "chosen, not measured" or "this is ours" comment: it is
redundant (maintainer, 2026-11-02).

**Code comments say what and why, never when or who.** No dates, no
"maintainer said", no lane names, no history ("moved out of `oag-game`",
"was X before", "superseded"). The evidence (date, emulator, run count,
address) lives on the doc page; a comment cites the page and section, e.g.
`` (`docs/physics/<page>.md`, "<section>") ``. Git holds the history. This
binds `.rs`, `.wgsl`/`.wesl` and scripts; docs pages keep their dates.

### A mod API change updates the mod author skill

A change to the wasm extension ABI (`oag-ext-abi`), the guest SDK (`oag-ext`),
`oag-mod.toml`, or an effect, track or string format a mod writes updates
`assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md` in the same change
(commit it on the submodule branch, then the pointer).

## Documentation tree

Start at [`docs/README.md`](docs/README.md). Entry points:
[goals](docs/overview/goals.md), [roadmap](docs/overview/roadmap.md),
[glossary](docs/overview/glossary.md), [ADR index](docs/architecture/adr/README.md),
[format status](docs/formats/README.md). [`HANDOVER.md`](HANDOVER.md) holds what
is not in the docs tree (deferred work, single-source versus verified findings,
traps); read it before non-trivial work and update it when handing off.

Work in flight is one file per thread under [`handover/`](handover/) (`rendering/`,
`gameplay/`, `frontend/`, `tooling/` or `audio/`), each with `## Open`, `## Next
Steps` and `status`, `titles`, `updated`, `summary` frontmatter
(`HANDOVER.md`, "Open threads"; `just threads` is the index). Delete a thread's
file once the work lands; `just check-handover` gates the frontmatter.

**Nothing outside `handover/` and `HANDOVER.md` may link into `handover/`.**
Threads are deleted when landed, and `check-docs` only proves links resolve
today. Cite the doc page, test or address instead. The reverse direction is
normal.

ADRs are immutable; a changed decision gets a new ADR that supersedes the old.
