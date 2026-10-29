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

```sh
just              # the gate every commit must pass: fmt-check lint test check-docs check-deps check-unused-deps check-determinism check-size check-title-branching check-title-reach check-names check-handover check-status check-weapon-lineage
just fmt | lint | build | docs
just test         # nextest, quick profile (our crates opt-level 0, examples not built)
just test-data    # also the #[ignore]d ground-truth tests; needs data/images/; skips binaries that passed unchanged, --full runs all
just gate-affected / test-data-affected   # same checks in full, only the tests the branch can reach (scripts/affected.py); a drive member's gate
just check-docs   # internal links in docs/ (not CLAUDE.md, .claude/ or HANDOVER.md)
just check-deps   # the six dependency rules below
just check-unused-deps   # no unused or test-only-used normal dependency (cargo shear)
just check-determinism   # no platform transcendental in simulation code
just check-size   # ratchet: file length 1,500, inline #[cfg(test)] mod 500
just check-title-branching   # ratchet on `title.name == oag_X::TITLE.name` in generic crates; per-title behaviour is `Title` data (ADR-0058)
just check-title-reach       # ratchet on generic crates naming a title package by path; they go through `oag_title::Title`, the oag-source registry or the composition root
just check-names  # every names.tsv row still matches its evidence page, offline
just check-handover          # HANDOVER.md under 256 KiB (the Read tool's ceiling)
just check-weapon-lineage    # docs/gameplay/weapon-lineage.md's checked axes match the title crates
just check-schemas           # shipped TOML validates against schemas/v1/ (ADR-0064)
just check-status # status.md's RE-coverage table matches names.tsv; `just gen-status` regenerates it
just check-strings           # menu.toml rows have string_id + english.toml text (ratchet)
just check-test-budget       # no test-data suite over 450s, no test over 300s (ratchet)
just blender-fixtures        # Blender add-on fixtures; needs Blender 4.2+, not in the gate
just bench-render [title...] # whole-race render benchmark to data/bench/; takes the gate lock itself (`OAG_BENCH_NO_LOCK=1` for an idle machine), never inside a gate; `just bench-compare A B`; docs/tools/render-bench.md
just audit-leakage           # no tracked game content
just unpack|wad|view|play ...   # asset tools and the game (see the justfile); just build-allegrex (JDK 21); just mine-names <image>
```

Single test: `cargo nextest run -p oag-core some_test_name` (nextest, not
`cargo test`). `just test` builds `target/quick`, `just test-data` builds
`target/debug` (opt-level 2); pass `--cargo-profile quick` to a plain test to
reuse `just test`'s build, and omit it for a ground-truth test.
Ground-truth tests (`crates/*/tests/*ground_truth*.rs`) are `#[ignore]`d, need
a disc image under `data/images/`, and never run in CI.

### The test-data suite

Ceilings (`scripts/check-test-budget.py`): suite 450 s, single test 300 s,
both *under load*, so they catch gross regressions, not drift. Wall clock on
this machine is contended: judge speed on an idle machine or not at all (the
same tree measured 402-456 s at load 2-21). History and measurements:
`docs/architecture/workspace-layout.md` ("Test-data suite measurements").

`just test-data` skips a test binary that passed unchanged
(`scripts/test-data.py`, `target/test-data-pass.json`). Skips are listed by
name and never counted as passed. `--full` (or `OAG_TEST_DATA_FULL=1`) runs
everything and is what the suite ceiling is judged on; a partial run holds each
test to its own ceiling only.

When a ceiling fires, **stop and re-profile the tail before adding tests**:

1. Sort the log: `grep -E '^\s+(PASS|FAIL)\s+\[' target/test-data.log | sed -E 's/^\s+\w+\s+\[\s*([0-9.]+)s\].*\)\s+/\1 /' | sort -rn | head -20`.
2. The slowest test is almost always a matrix (circuits x tiers x seeds) in nested `for` loops in one `#[test]`. Nextest parallelises across tests, so it runs on one core.
3. Make the matrix the test axis, choosing a split the assertion survives: `max` over a partition is `max` over the whole (`stall_rescue_ground_truth.rs`), an ordered `assert_eq!` filters with its list (`spawn_heading_ground_truth.rs`), a chained ordering is its pairwise links (`ai_roll_ground_truth.rs`).
4. Add a `BASELINE` row only when a split would change what is asserted, and say why in the row's comment (`ram_ground_truth.rs`: its bound is a ratio and slicing rebuilds the small-denominator flake).

**Before adding a disc-backed test, find what already walks that data.**
Nextest shares nothing between tests, so a second test over an archive pays the
whole read and decode again.

1. `rg` the archive or file family in `crates/*/tests/`. If a test walks it, add your claim as a named check function over that one pass (`crates/rcs/tests/gxp_ground_truth.rs`).
2. Sniff before you read (`Archive::peek`, a few decompressed bytes) and read in full only what can match.
3. A test that only prints is a scratch sweep: `#[ignore]` with a reason and an early return unless `OAG_SWEEP` is set.
4. Remove redundancy before splitting for the tail: every slice pays its own fixture load.

**A new `tests/*.rs` file is a `mod` line in its crate's `tests/it*.rs`**, not a
binary of its own. `oag-game` and `oag-raceplay` have five binaries
(`tests/it_*.rs`, by first letter). An unlisted file is never compiled
(`every_test_file_is_listed` fails naming it). `oag-physics`, `oag-gameplay` and
`oag-ai` keep a binary per file (CI runs `--test determinism` by name). Test
names carry the module path; filter with `test(...)`, not `binary(...)`.

**A test goes in the lowest crate whose dependencies reach everything it reads.**
`oag-game`'s binaries rerun on nearly every change. A test belongs in `oag-game`
only if it names `oag_game` or spawns the `oag-game` binary; a race test that
needs only `oag_raceplay::race` goes in `oag-raceplay`, a decoder census in the
decoder's crate.

### Gates by change

- **Affected-only gating**: a change to `oag-core`, a simulation crate,
  `Cargo.toml`/`Cargo.lock`, the toolchain, the `justfile`, `.config/` or a
  `build.rs` selects the full suite; a docs-only change selects no tests.
  Drive members use `gate-affected`; the lead runs the full `just` and
  `test-data` once per merge batch (the only full disc-backed run; CI has no
  images). See `.claude/skills/oag-drive/SKILL.md`.
- **Docs-only** (`docs/`, `handover/`, `HANDOVER.md`, `.claude/`, or doc
  comments in a `.rs` file): run `just check-docs`, plus `just check-handover`
  if `HANDOVER.md` changed. If any `.rs` file changed, also `just fmt-check`
  and `just lint`. If the change turns out to touch code logic, run the full
  `just`.

### Data and images

`data/` is gitignored: user-supplied disc images and derived output only
([`data/README.md`](data/README.md)). **No game content of any kind may be
committed** (`.gitignore`, CI's `leakage` job, `just audit-leakage`;
[`docs/overview/legal.md`](docs/overview/legal.md)).

- **`fd` and `rg` skip `.gitignore`d paths and return nothing under `data/` with exit code 0.** Use `--no-ignore`, or `ls`/`find`/`grep`.
- **A spawned agent that sees `data/images/` empty has a sandbox limitation, not missing images**: `data/` does not travel into a worktree or restricted sandbox. Check `ls -la data/images/` first. If empty, **say so in your report** rather than skipping the real-data check, and give the orchestrator the exact command to run.
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

```
                         oag-core
                            |
        +-------------------+--------------------+
        |                   |                    |
    oag-disc            (physics/ai/            oag-render
        |                weapons/race,             |
   oag-formats           M4/M5 crates)         oag-audio/input
        |                     |                    |
    oag-assets                |                    |
        |                     |                    |
        +----------> oag-gameplay <---------------+
                            |
                        oag-game
```

Crates (`crates/<name>`; the full table and reasoning:
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md)).
Add a crate only when its milestone opens; no placeholder crates.

| Crate | Purpose |
| --- | --- |
| `oag-bytes` | Endian-aware byte reading. |
| `oag-core` | Deterministic `f32` math (no SIMD), `TickClock`, seeded `Rng`, state hashing. Everything depends on it. |
| `oag-disc` | CHD and raw ISO readers, ISO 9660, platform identification. |
| `oag-display` | Screen/shape/pixel/brightness vocabulary and a source's layout coordinate space. Depends on `oag-disc` and `serde` only. |
| `oag-formats` | Containers and sound: WAD, PSARC, LZSS, sound banks, magic triage, byte order, GE swizzle. Split by format family ([ADR-0050](docs/architecture/adr/0050-format-crates-split-by-format-family.md)). |
| `oag-rcs` | `RCSMODEL` scene (PS3/Vita): models, materials, visibility sets, Vita shader programs. |
| `oag-vex` | `.vex` scene tree and its payloads: geometry, collision, spline, pads, PVS, fog, lights, emitters, shadow hulls, PS2 VIF, 2048 collision. |
| `oag-pob` | `.pob` particle system. Depends on `oag-formats` only. |
| `oag-texture` | Every pixel format decoded to RGBA8888, plus a PNG writer. |
| `oag-tables` | XML-authored tables (handling, weapons, campaign, load manifests, lighting). Zero dependencies. |
| `oag-video` | `.PMF`, `IPUF`, Bink, and the IVF/AV1 cache. No workspace dependencies. |
| `oag-unity` | Generic Unity asset-bundle reader; knows no game; depends on `oag-bytes` only ([ADR-0062](docs/architecture/adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md)). |
| `oag-ballisticng` | BallisticNG Workshop content on `oag-unity`: `.wtrk`, `.csf`, track data as typed structs. Reads; maps nothing into this engine. |
| `oag-trackfile` | Own custom-track format: glTF 2.0 binary plus TOML manifest; refuses an unknown major by name. The BallisticNG importer is `oag_raceplay::foreign`. Forbidden to gameplay crates ([ADR-0063](docs/architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)). |
| `oag-schema` | The `[toml-schema]` header of every TOML of ours ([ADR-0064](docs/architecture/adr/0064-project-toml-files-declare-a-toml-schema.md)); `validate` feature is tests-only. |
| `oag-assets` | Runtime `Archive`: a WAD by path or out of a disc image, by index/name/hash. |
| `oag-ui-screens` | Front-end screens on `oag-ui` (campaign, end of race, pickers, prompts). `oag-ui` never depends on it. |
| `oag-title` | The `Title` type and its three axes (archive candidates, entry names, foreign-serial deny-list). Types only. |
| `oag-pulse` | What Pulse ships: archive/entry names, hashes, presentation tables. |
| `oag-pure` | Same for Pure, deliberately thin: only what `docs/formats/pure-status.md` measured. |
| `oag-hd` | Same for HD/Fury: seven PSARC archives on a PS3 disc. Front end `Measured` since 2026-09-05 ([ADR-0025](docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md), `docs/formats/hd-frontend.md`). |
| `oag-omega` | Same for Omega (PS4): HD's `PI001` front end off a base package plus a mandatory patch (`data09.psarc` carries the only complete front end). Boot chain `Provenance::Declared` (nobody has watched an Omega boot). shadPS4 0.20 runs Omega (frames via `scripts/shadps4-omega-pair.py`, no memory readout); `eboot.bin` is in Ghidra (`docs/ghidra/functions/ps4-omega-eu/`). Races start on Omega data but are incomplete: Zone, boost plume, speed classes unmeasured (`docs/formats/omega-status.md`). |
| `oag-2048` | What 2048 ships (Vita PSARC packages and paths), deliberately thin. |
| `oag-tools` | CLI: `oag-unpack`, `oag-wad`. |
| `oag-physics` | Ship dynamics and collision queries. Depends on `oag-core` only. |
| `oag-race` | Race rules: modes, lap timing, progress. |
| `oag-ai` | Opponent driver following the authored racing line. Depends on `oag-core`, `oag-physics` only. |
| `oag-weapons` | Pickups, projectiles, blasts, beams; reaches a craft only through the `Craft` trait. |
| `oag-gameplay` | The `World` struct, `InputSnapshot`, spline spawning. |
| `oag-replay` | Replays (per-slot input stream is the truth, state hash for desync) and ghosts (pose track), [ADR-0055](docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md). |
| `oag-mesh` | Mesh pipeline: decodes `.vex`/`RCSMODEL`, draws offscreen or to a surface; orbit camera, capture. |
| `oag-gpu` | Shared GPU vocabulary: perf probe, GPU timestamps, offscreen formats. |
| `oag-post` | Bloom, tonemap, motion blur, SMAA, FXAA, FSR 1/3. |
| `oag-fx` | Visual effects: `.pob` player, exhaust, mist, clouds, beams, flashes, hull overlays. Render-side. |
| `oag-render` | wgpu renderer: track ribbon, shadows, PVS, cameras. Owns no window. Reads `oag-pulse`'s tables (allowed; rule 1 forbids only the other direction). |
| `oag-present` | Scene-to-glass: upscale/grade/filter composite, dynamic resolution, perf meter. |
| `oag-shader-check` | Build-dependency only: validates every WESL shader with `naga`. |
| `oag-livery` | A race's hulls, skins, shields, plumes, wrecks from a title's archive entries. |
| `oag-thread` | A closure on another thread on every target (`std::thread` or a Web Worker); `Task<T>`, `lock`. Depends on nothing of ours. |
| `oag-log` | Binaries' log sink: terminal plus a pruned timestamped file. |
| `oag-mobile` | Android/iOS app glue under `oag-game` ([ADR-0065](docs/architecture/adr/0065-one-mobile-crate-for-every-phone-os.md)). |
| `oag-mod` | Mods (folder, `.zip`, `.oagmod`), `oag-mod.toml`, search path, built-in `oag-extras` ([ADR-0066](docs/architecture/adr/0066-the-menu-is-the-disc-plus-mods.md)). |
| `oag-modhost` | WebAssembly extension host behind the `extensions` feature: wasmtime (Cranelift, fuel, NaN canonicalisation), the ABI word and layout revision checked by name, a fresh pooled instance per tick, the fault path; no engine until a module loads. Knows no game ([`docs/architecture/wasm-extensions/`](docs/architecture/wasm-extensions/README.md)). |
| `oag-ext-abi` | ABI 1 defined once: the I/O buffer's layout, records and shared vocabulary, and its `REVISION`. `no_std`, no dependencies; host and guest SDK compile against it. |
| `oag-ext` | The Rust guest SDK a mod is built on (`Extension`, `Observation`, `Reply`, `blob!`, `export!`, `test::Race`). No engine crate may depend on it. |
| `oag-net` | Network play: wire messages, WebTransport `Link`, authoritative `host`, predicting `client` (feature), the `oag-server` binary ([ADR-0070](docs/architecture/adr/0070-the-network-transport-is-webtransport.md)). |
| `oag-input` | Real devices to the abstract button layer and an `InputSnapshot`. |
| `oag-view` | Asset viewer: CLI, window, texture browser. |
| `oag-trace` | Per-tick trace capture and comparison against the original (`show\|run\|compare\|script\|drive\|track`). |
| `oag-audio` | Voice pool, mixer, `cpal` stream or none, WAV writer. Cues are a per-tick *output*, never `World` state ([ADR-0018](docs/architecture/adr/0018-audio-mixer-architecture.md)). |
| `oag-music` | A title's soundtrack: reading it and decoding ATRAC3+, ATRAC9, MP3, Wwise to PCM. |
| `oag-sound` | Soundtrack selection, effect banks and cues, HD's authored mix, over `oag-audio`. |
| `oag-raceplay` | A race from load to finish: tick, scene, weapon visuals, scenery effects, replay glue, load catalogue. Its drawing half is the default `view` feature; without it, the simulation half that `oag-server` links. In `NOT_TITLE_PACKAGES`. |
| `oag-source` | Finding and opening a title's disc image, DLC packs, a race's track/craft sources, caches. |
| `oag-hud` | In-race HUD: layout model, `Readout`, `Draw` list, `sprite` sheet. No GPU pass (`oag_game::hud_overlay`, `hud_countdown`). In `NOT_TITLE_PACKAGES`. |
| `oag-ui` | Front end: boot movies, menus, HUD font, strings, the `Draw` vocabulary `oag_game::render` rasterises. In `NOT_TITLE_PACKAGES`. |
| `oag-game` | Composition root; a thin `[[bin]]` over `[lib]`. |
| `oag-testdata` | **Dev-dependency only.** Finds disc images for tests and the `OAG_REQUIRE_GAME_DATA` skip-or-fail contract. No game knowledge. |

`NOT_TITLE_PACKAGES` crates (`oag-raceplay`, `oag-hud`, `oag-ui`,
`oag-ui-screens`) draw or read titles, so nothing gameplay-side may ever depend
on them.

Six dependency rules, enforced by `just check-deps`
(`scripts/check-dependency-rules.py`):

1. No gameplay crate depends on `oag-render`, `oag-fx`, `oag-audio`, `oag-input`, `oag-net`, `winit` or `wgpu`. The simulation consumes an input *snapshot type* owned by `oag-gameplay`.
2. No crate depends on `oag-game`, and no engine crate on the guest SDK `oag-ext`.
3. No generic render-side crate (`oag-fx`, `oag-gpu`, `oag-mesh`, `oag-post`) reaches a title package (`oag-pulse`, `oag-pure`, `oag-hd`, `oag-omega`, `oag-2048`); tables are injected by the caller. `oag-render` still reads `oag-pulse`'s presentation tables and is not listed.
4. `oag-unity` depends on no workspace crate but `oag-bytes` (allowlist). A content layer is a separate crate on top, never the reverse.
5. `oag-ballisticng` reaches `oag-unity`, `oag-texture`, `oag-formats`, `oag-bytes` only; no gameplay crate may depend on it or on `oag-unity`. How a foreign track maps into this engine is the consumer's choice.
6. `oag-server` (`oag-net` built with default features) links no renderer, window, audio or UI crate: `cargo tree -p oag-net -e normal` reaches none of `wgpu`, `winit`, `cpal`, `oag-render`, `oag-fx`, `oag-audio`, `oag-sound`, `oag-mesh`, `oag-ui`, `oag-hud`, `oag-input` and the other drawing crates. Checked per package, because the workspace's unified features hide it.

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
translation may fill a gap, as `ID = { text = "...", human = false }`; `human`
is `true` only when a human wrote or approved it (convention in
`english.toml`'s doc comment).

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
the algebraic float ops (`f32::algebraic_add`, Rust 1.98) are fair game in a
proven-hot per-frame loop.

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
- **Every recovered name also goes in `docs/ghidra/functions/psp-pulse-usa/names.tsv` in the same change**: `address<TAB>kind<TAB>name<TAB>confidence<TAB>page`, `kind` is `function` or `data`, name column is the bare name (the `_q` is derived from confidence). `scripts/apply-ghidra-names.py` refuses a row whose address and name are not both on the cited page; `just check-names` makes the same checks offline. `just apply-names` replays the file into a fresh Ghidra import.
- Loop: observe, hypothesise, verify, document, implement ([methodology](docs/reverse-engineering/methodology.md)). Do not implement from a plausible reading.
- The PSP `BOOT.BIN` and PS2 main executable are unencrypted ELFs. Ghidra setup: [`docs/reverse-engineering/toolchain.md`](docs/reverse-engineering/toolchain.md). The Allegrex/VFPU module (`just build-allegrex`) is required for PSP ([`docs/psp/allegrex-vfpu.md`](docs/psp/allegrex-vfpu.md)). The `ghidra-mcp` server (`.mcp.json`) drives Ghidra when a project is open.

### A 2048 finding is checked against Omega, and the reverse

2048 (Vita) and Omega (PS4) share an asset lineage: Omega reads through 2048's
`.rcsmodel` readers (8-byte pointers aside), keeps 2048's ship directories and
three-way plugin split, and re-ships 2048's circuits at the same layouts
([`docs/formats/omega-status.md`](docs/formats/omega-status.md)). Re-shipped is
not identical: XML tables are byte-identical, but models are re-cooked (same
hull triangle and material counts, re-welded vertices; altima's collision
re-encodes its node array around identical leaf data), textures are `.gnf`
against `.gxt`, shaders are recompiled. HD's circuits ship in 2048's container
with HD's geometry. Omega's front end is HD's. So a law read off one title's
shader or model is checked against the other's own file, never assumed; a
front-end finding is checked against HD.

**Anything recovered or wired for one of the two (format field, law, material
rule, table, asset path) gets a quick check against the other in the same
change**: does it ship the same file or code, and does the reader or wiring
hold? Record the answer on the finding's doc page as one of:

- **ported**: the other title uses it too, with its own test or ground truth;
- **checked, applies, not wired**: same data or code, wiring left open, named in a `handover/` thread;
- **checked, differs**: what differs, with evidence;
- **not checkable**: why, never silence. "No PS4 emulator" is not a reason: Omega's executable is in Ghidra and shadPS4 runs it (no memory readout, so a live Omega value is the usual reason).

The check is a census and a reader run, not a second lane; if it becomes real
work, write it down as open. Omega's incomplete racing does not exempt a format
or asset finding.

### Never invent what the assets already author

**If a thing exists in the disc's own data, play the data. Do not author a
stand-in, and do not hand-transcribe it into a `const`.** This covers particle
effects (`Data\Psys\*.POB`), models, textures, tables and tuning values. Two
failures this rule exists for:

1. A hand-transcribed table (`oag_fx::sparks` carried four emitters as a `const`): correct, but not re-derivable and no use for the other 34 effects.
2. A plausible stand-in (the rocket drew invented smoke puffs and blast billboards because the `.pob` payload was believed undecoded). A legible stand-in is how a wrong picture survives review.

Parse it and play it; if it will not parse, **draw nothing and say so** in the
loader report. Where a stand-in cannot be avoided (a texture with no located
WAD entry), keep it a substitute for the missing asset alone, never an override
of data you have (`crates/fx/src/psys.wgsl` lost a whitening term for this on
2026-08-12).

Effects load generically: `oag_fx::psys::Library` loads any
`Data\Psys\<name>.POB` and `psys::Stage` plays any number. Adding an effect is
adding its name and its **trigger**, and the trigger needs reverse-engineering;
an effect with no recovered trigger stays unwired rather than fired on a guess
(`HANDOVER.md` lists them).

## Documentation tree

Start at [`docs/README.md`](docs/README.md). Entry points:
[goals](docs/overview/goals.md), [roadmap](docs/overview/roadmap.md),
[glossary](docs/overview/glossary.md), [ADR index](docs/architecture/adr/README.md),
[format status](docs/formats/README.md). [`HANDOVER.md`](HANDOVER.md) holds what
is not in the docs tree (deferred work, single-source versus verified findings,
traps); read it before non-trivial work and update it when handing off.

Work in flight is one file per thread under [`handover/`](handover/), each with
`## Open` and `## Next Steps`, in `rendering/`, `gameplay/`, `frontend/`,
`tooling/` or `audio/` (a thread crossing categories carries a `categories:`
frontmatter block naming the extras). Every thread also carries `status`,
`titles`, `updated` and `summary` frontmatter; `just threads [--status S]
[--title T] [--category C]` is the index (`HANDOVER.md`, "Open threads"). Delete
a thread's file once the work lands; `just check-handover` gates the frontmatter.

**Nothing outside `handover/` and `HANDOVER.md` may link into `handover/`.**
Threads are deleted when landed, and `check-docs` only proves links resolve
today. Cite the doc page, test or address instead. The reverse direction is
normal.

ADRs are immutable; a changed decision gets a new ADR that supersedes the old.
