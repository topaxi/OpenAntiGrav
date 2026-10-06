# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A clean-room reimplementation, in Rust, of the Studio Liverpool anti-gravity
racing engine - **Wipeout Pure, Pulse, HD/Fury, 2048 and the Omega Collection**
on one shared engine, not a single-title project. **Pulse** (PSP/PS2) is the
reference implementation: the most deeply reverse-engineered and the most
verified against the original. The rest are built on that same architecture
as their own milestones open, not bolted on afterward - see
[`docs/overview/status.md`](docs/overview/status.md) for where each title
actually stands right now. The original executables are a specification, not
source code: behaviour is observed, documented with evidence and a confidence
score, then reimplemented fresh. Nothing is transliterated from the
decompiler.

Status is milestone-driven; check [`docs/overview/roadmap.md`](docs/overview/roadmap.md)
for what is actually done versus planned, and
[`docs/overview/status.md`](docs/overview/status.md) for the per-title,
per-subsystem matrix, before assuming a subsystem exists.

## Commands

```sh
just              # fmt-check + lint + test + check-docs + check-deps + check-unused-deps + check-determinism + check-size + check-title-branching + check-names + check-handover + check-status - the gate every commit must pass
just fmt          # cargo fmt --all
just lint         # cargo clippy --workspace --all-targets -- -D warnings
just test         # cargo nextest run --workspace
just test-data    # also runs #[ignore]d ground-truth tests that need data/images/ populated
just check-docs   # validates internal links in docs/ (scripts/check-doc-links.py)
just check-deps   # asserts the three dependency-boundary rules below (scripts/check-dependency-rules.py)
just check-unused-deps # no crate declares a dependency its code never uses, or a normal one only tests/examples use (`cargo shear`)
just check-determinism # asserts no platform transcendental reaches simulation code (scripts/check-transcendentals.py)
just check-size   # ratchet on file length (1k lines) and on inline #[cfg(test)] modules (200) (scripts/check-file-size.py)
just check-title-branching # ratchet on `title.name == oag_X::TITLE.name`-style comparisons in generic crates; per-title behaviour is `Title` data, ADR-0058 (scripts/check-title-branching.py)
just check-names  # every names.tsv row still matches its evidence page, offline (scripts/check-ghidra-names.py)
just check-handover # HANDOVER.md stays under 256 KiB, the Read tool's own ceiling (scripts/check-handover-size.py)
just check-status # docs/overview/status.md's RE-coverage table matches names.tsv and the evidence pages; `just gen-status` regenerates it (scripts/gen-re-coverage.py)
just check-strings # every menu.toml row/title has a string_id and its english.toml text, ratchet over pre-existing debt (scripts/check-strings.py)
just check-test-budget # no test-data suite over 450s and no single test over 300s, ratchet (scripts/check-test-budget.py)
just build        # cargo build --workspace
just docs         # cargo doc --workspace --no-deps --document-private-items
just audit-leakage # asserts no tracked game content or reproduction (scripts/check-leakage.py)
```

Single test: `cargo nextest run -p oag-core some_test_name` (nextest, not `cargo test`).
Ground-truth tests (in `crates/*/tests/*ground_truth*.rs`) are `#[ignore]`d because they
need a real disc image under `data/images/`; they never run in CI, only via `just test-data`.

**`just test-data` runs 5,831 tests in 230-280s (3,500s of CPU) with other members' builds running beside it - it was 368s and 5,855s of CPU before 2026-10-01's profile change, see `docs/architecture/workspace-layout.md` - tees the run to
`target/test-data.log`, and then checks it against `scripts/check-test-budget.py` - which
fails when the suite exceeds 450s or any single test exceeds 300s.** Both are durations
*under load*: a test competing with 5,800 others for the cores reports well over its
isolated cost, so these gate gross regressions rather than drift.

When it fires, **stop and re-profile the tail before adding more tests** - that is what
the ceiling is for, and it is not a formality to baseline past:

1. Sort the log: `grep -E '^\s+(PASS|FAIL)\s+\[' target/test-data.log | sed -E 's/^\s+\w+\s+\[\s*([0-9.]+)s\].*\)\s+/\1 /' | sort -rn | head -20`.
2. Look at what the slowest test *is*. It is almost always a matrix - N circuits by M
   tiers by K seeds - in nested `for` loops inside one `#[test]`. `cargo nextest` runs
   every test in its own process and parallelises across **tests**, so that matrix runs on
   one core while the other fifteen idle.
3. Make the matrix the test axis rather than the loop body, choosing a split the assertion
   survives: `max` over a partition is `max` over the whole (`stall_rescue_ground_truth.rs`),
   an ordered `assert_eq!` filters with its list (`spawn_heading_ground_truth.rs`), and a
   chained ordering is its own pairwise links (`ai_roll_ground_truth.rs`).
4. Add a `BASELINE` row **only** when the split would change what is asserted, and say why
   in the row's comment. `ram_ground_truth.rs` is the worked example of a genuine one: its
   bound is a *ratio*, and slicing the sample rebuilds the small-denominator flake that
   `RACES`'s own doc comment records a day being spent on.

The history this exists for: the suite went from 3:17 to 9:47 between 2026-08-17 and
2026-09-09 with no commit that looked wrong, and one test that landed on 2026-09-06 was
525 s of a 587 s wall clock on its own. See `docs/architecture/workspace-layout.md` for
the measurements.

**Wall clock here is contended, so measure it on an idle machine or not at all.** The
same tree measured 456 s, 402 s and 415 s at load averages of 21, 17 and 2, with four,
two and one tests over the per-test ceiling and a *different* test each time. Two agents
independently read a contended run as a regression on the same afternoon. The two `ai_roll`
grid tests that trip it measure 76-79 s in isolation, against the 95-114 s recorded when
the ceiling was set - they got faster, and the full-run number is telling you about the
machine.

**The full `just` gate is not required for a change that touches only `docs/`, `handover/`,
`HANDOVER.md`, `.claude/`, or doc comments inside a `.rs` file (no code logic changed).**
Run `just check-docs` (internal links) and, if `HANDOVER.md` changed, `just check-handover`
(size ratchet) instead - both take well under a second, against minutes for the full suite.
If any `.rs` file changed at all, still run `just fmt-check` and `just lint`: a doc comment
can still fail `rustfmt`'s wrapping or clippy's intra-doc-link lints, and those two are cheap.
`just test`/`test-data` and the rest of the gate exist to catch a *behavior* regression, and a
docs-only diff has no behavior to regress - re-run the full `just` yourself if a change you
believed was docs-only turns out to touch code once you're in it.

Asset/RE tools, normally run through `just` (see `justfile` for the full list):

```sh
just unpack info|list|extract|hexdump|sniff <image>   # oag-unpack
just wad list|tags <image>:<path/to.wad>               # oag-wad
just view <image>:<path/to.wad> [--mesh ...|--track] [--screenshot out.png]
just play [--screenshot out.png] [--until "text"] [--press ...]  # oag-game
just build-allegrex     # builds the Allegrex Ghidra processor module (needs JDK 21)
just mine-names <image> # recovers WAD entry-name candidates
```

`data/` is gitignored and holds only user-supplied disc images and derived output; see
[`data/README.md`](data/README.md) for expected image names. No game content of any kind
(assets, executables, extracted data) may ever be committed - enforced by `.gitignore`
patterns, CI's `leakage` job, and `just audit-leakage`. See
[`docs/overview/legal.md`](docs/overview/legal.md).

**If you are a spawned agent and `data/images/` looks empty or holds only
`README.md`, that is a sandbox limitation, not a sign the images don't
exist.** Being gitignored, `data/` does not travel into an isolated worktree
or a restricted execution sandbox the way tracked files do, even though it is
fully populated in the orchestrating session. Check with `ls -la
data/images/` before assuming any real-data path (a ground-truth test, a
disc-backed CLI run, `just test-data`) is exercisable. If it is empty:
**say so explicitly in your report rather than silently skipping the
real-data check or reporting untested code as verified.** Give the
orchestrating session the exact command to run - it has the images and can
verify directly, the way a human maintainer would from their own checkout.

**In a worktree, that is fixable in one command: `just link-data`.** It
symlinks every `data/` subdirectory from the main checkout into this worktree
(`scripts/link-worktree-data.sh`), so the disc images, the DLC packs and the
reference traces are all reachable and `just test-data` works here. It is a
no-op in the main checkout and never replaces a real directory. Run it right
after `git worktree add` - the failure it prevents is not an obvious one:
under `OAG_REQUIRE_GAME_DATA=1` a missing `data/` reads as a wall of "is
missing" failures naming files that are sitting in the main checkout the whole
time, and without that variable the same tests skip silently and a run looks
greener than it is. A restricted sandbox with no access to the main checkout
is the case this cannot fix.

**Separately, and even when `data/` genuinely is populated: `fd` and `rg`
respect `.gitignore` by default, so both silently return nothing under
`data/` with exit code 0 - no error, no warning, just an empty result that
reads exactly like "there's nothing here."** Confirmed directly: `rg -l
"chd" data/` and `fd . data/images` both come back empty against a fully
populated `data/images/`, while `rg --no-ignore` and `fd --no-ignore` find
everything. Use `--no-ignore` with either tool inside `data/`, or just use
`ls`/`find`/`grep` directly, which don't filter by `.gitignore` at all. This
is a second, independent way real data can look absent when it isn't -
distinct from the sandbox issue above, and it can bite even the
orchestrating session, not only a spawned agent.

## Architecture

**Core principle: the simulation must not know a renderer exists.** It takes an input
snapshot in and emits a state snapshot out; drawing, audio, windowing and file I/O all
happen outside it. This is what makes the simulation testable without a GPU and
comparable against a trace from the original.

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

Existing crates:

| Crate | Path | Purpose |
| --- | --- | --- |
| `oag-core` | `crates/core` | Deterministic math (`f32`, no SIMD), `TickClock`, seeded `Rng`, state hashing. Depended on by everything. |
| `oag-disc` | `crates/disc` | CHD and raw ISO readers, ISO 9660 walker, platform identification. |
| `oag-display` | `crates/display` | The vocabulary the picture is configured in - which screen, what shape, how many pixels, how bright - plus the coordinate space a source authors its layouts in. Depends on `oag-disc` and `serde` and nothing else. |
| `oag-formats` | `crates/formats` | Asset containers and what sits under them: WAD, PSARC, LZSS, sound banks, magic-number triage, byte order and the GE swizzle. Split by format family per [ADR-0050](docs/architecture/adr/0050-format-crates-split-by-format-family.md); the decoders now live in `oag-vex`, `oag-rcs`, `oag-texture`, `oag-tables` and `oag-video`. |
| `oag-rcs` | `crates/rcs` | The `RCSMODEL` scene the PS3 and Vita ship geometry in: models, materials, visibility sets, and the Vita's compiled shader programs. |
| `oag-vex` | `crates/vex` | The `.vex` scene tree and every payload authored in it: geometry, collision, track spline, pads, PVS, fog, lights, sound emitters, shadow hulls, PS2 VIF packets and 2048's collision. |
| `oag-pob` | `crates/pob` | The `.pob` particle system: `SYSP` container, emitter tree, channels, modifiers, embedded sprites. Depends on `oag-formats` only; the `.vex` scene just names a `.pob`. |
| `oag-texture` | `crates/texture` | Every pixel format the originals ship, decoded to RGBA8888 - PSP `.mip`, PS2 GS packets, PS3 `.gtf`, Vita `.gxt`, fonts, liveries, and a PNG writer. |
| `oag-tables` | `crates/tables` | The tables titles author as XML: handling, weapons, campaign grids, load manifests, lighting rigs. Zero dependencies - the crate the simulation reads its numbers from has no path to a texture decoder. |
| `oag-video` | `crates/video` | Video containers: `.PMF` (PSP), `IPUF` (PS2), Bink (HD), plus the IVF/AV1 movie cache this project writes. Depends on nothing in the workspace. |
| `oag-assets` | `crates/assets` | Runtime asset access: `Archive` reads a WAD by path or straight out of a disc image, by index/name/hash. |
| `oag-ui-screens` | `crates/ui-screens` | The front end's screens built on `oag-ui`: campaign, end of race, pickers, prompts, the ticker marquee, the track panel. `oag-ui` stays the lower core and never depends on it; same `NOT_TITLE_PACKAGES` classification. |
| `oag-title` | `crates/title` | The `Title` type and its three axes: archive candidates, entry names, foreign-serial deny-list. Types only, no title's data. |
| `oag-pulse` | `crates/pulse` | What Wipeout Pulse ships: archive and entry names, hashes, and its presentation tables (HUD, front end, loading wave, animated textures, race defaults). |
| `oag-pure` | `crates/pure` | The same for Wipeout Pure, deliberately thinner - it holds only what `docs/formats/pure-status.md` measured. |
| `oag-hd` | `crates/hd` | The same for Wipeout HD / Fury, and the one that is shaped differently: seven PSARC archives on a PS3 disc rather than two WADs. Its front end was wired off a **declared** boot chain until 2026-09-05, when three cold boots on RPCS3 made it `Measured` - the distinction the type carries per [ADR-0025](docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md); see `docs/formats/hd-frontend.md`. |
| `oag-omega` | `crates/omega` | The same for Wipeout: Omega Collection on PS4 - HD's own `PI001` front-end plugin carried forward, off a base package plus a mandatory day-one patch whose `data09.psarc` is the only archive carrying a complete front end. Boot chain `Provenance::Declared`: no PS4 emulator exists in this project's toolchain to upgrade it. A race starts on Omega's own data (2026-09-29) but is incomplete: Zone, the boost plume and the speed classes are unmeasured - see `docs/formats/omega-status.md`. |
| `oag-tools` | `crates/tools` | CLI: `oag-unpack`, `oag-wad`. |
| `oag-physics` | `crates/physics` | Ship dynamics and collision queries. Depends on `oag-core` and nothing else, deliberately. |
| `oag-race` | `crates/race` | Race rules: modes, lap timing, track progress. |
| `oag-ai` | `crates/ai` | Opponent behaviour: a driver that follows the authored racing line and emits ship controls. Depends on `oag-core` and `oag-physics` only. |
| `oag-weapons` | `crates/weapons` | Pickups, projectiles, blasts, beams, disruption and slowdown. Below `oag-gameplay`, which embeds its state; it reaches a craft only through the `Craft` trait. |
| `oag-gameplay` | `crates/gameplay` | The `World` struct, the `InputSnapshot` type the simulation consumes, spline spawning. |
| `oag-replay` | `crates/replay` | Replays and ghosts: the per-slot input stream as the truth, a state hash a second so playback reports a desync instead of diverging, and a ghost lap's pose track. See [ADR-0055](docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md). |
| `oag-fx` | `crates/fx` | The renderer's visual effects: the `.pob` particle player, exhaust, mist, clouds, beams, flashes, weapon quads, hull overlays. Above `oag-mesh`, below `oag-render`; render-side, so no gameplay crate depends on it. |
| `oag-render` | `crates/render` | The wgpu renderer: track ribbon, shadows, PVS, cameras. Owns no window. Reads `oag-pulse`'s tables, which runs against the arrows below and is allowed - rule 1 only forbids the other direction. |
| `oag-present` | `crates/present` | What happens to a frame between the scene and the glass: the upscale/grade/screen-filter composite, dynamic resolution scaling, and the performance meter. Plain data in; knows no race, menu or settings file. |
| `oag-livery` | `crates/livery` | A race's ship hulls, skins, shields, plumes and wrecks built from a title's own archive entries, plus the entry-name rules for them. |
| `oag-input` | `crates/input` | Maps real devices onto the abstract button layer and produces an `InputSnapshot`. |
| `oag-view` | `crates/view` | Asset viewer: CLI, window and texture browser over `oag-render`. |
| `oag-trace` | `crates/trace` | Per-tick trace capture and comparison against the original: `oag-trace show\|run\|compare\|script\|drive\|track`. The reading half of the M3 verification harness. |
| `oag-audio` | `crates/audio` | Mixing and playback: a hardware-free voice pool and sample loop, a `cpal` stream or none at all, and a WAV writer so a headless run is checkable. Cues are a per-tick *output* of the simulation, never `World` state. See [ADR-0018](docs/architecture/adr/0018-audio-mixer-architecture.md). |
| `oag-raceplay` | `crates/raceplay` | A race from load to finish line: the front-to-back tick, the scene it draws, weapon visuals, scenery effects, replay glue, and what a load reads with it (`catalogue`, `pilots`, the scoreboard table). Above `oag-render`/`oag-fx`/`oag-sound`, below `oag-game`; classified in `NOT_TITLE_PACKAGES`, nothing gameplay-side may depend on it. |
| `oag-source` | `crates/source` | Finding a title's disc image and opening it as whichever title it is, with its DLC packs, a race's track and craft sources, and the derived-cache directories. |
| `oag-hud` | `crates/hud` | The in-race HUD: the layout model, the `Readout` a race hands it, the `Draw` list `draw_list` builds, and the `sprite` sheet every screen packs its textures into. No GPU pass of its own: `oag_game::hud_overlay` and `oag_game::hud_countdown` hold the wgpu halves. Reads titles, so it is classified in `NOT_TITLE_PACKAGES` rather than `GAMEPLAY_CRATES`; nothing gameplay-side may ever depend on it. |
| `oag-ui` | `crates/ui` | The front end: boot movies, menus, the HUD's font, and the strings a screen draws - the `Draw` vocabulary `oag_game::render` rasterises. Menus draw, so it is classified in `NOT_TITLE_PACKAGES` rather than `GAMEPLAY_CRATES`; nothing gameplay-side may ever depend on it. |
| `oag-game` | `crates/game` | Composition root; boots the front end. A thin `[[bin]]` over `[lib]` so boot logic is testable headlessly. |
| `oag-testdata` | `crates/testdata` | **A dev-dependency, never a runtime one.** Where a test finds the disc images this project does not ship, and the `OAG_REQUIRE_GAME_DATA` skip-or-fail contract, in one place instead of 130 copies. Holds no game knowledge at all - no image names, no archive names, no title. |

Later crates (`oag-net`) are
added only when their milestone opens - see
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md) for the
full table and reasoning. Don't create placeholder crates ahead of that.

Three dependency rules, all enforced by `just check-deps` (part of the `just` gate,
`scripts/check-dependency-rules.py`) so a `cargo add` that breaks one fails CI, not just
review:

1. No gameplay crate depends on `oag-render`, `oag-fx`, `oag-audio`, `oag-input`, `winit` or `wgpu`.
   The simulation consumes an input *snapshot type* owned by `oag-gameplay`, never the
   input system itself.
2. No crate depends on `oag-game` (the composition root).
3. No generic render-side crate (`oag-fx`, `oag-gpu`, `oag-mesh`, `oag-post`) reaches a title
   package (`oag-pulse`, `oag-pure`, `oag-hd`, `oag-omega`, `oag-2048`) - a title's tables are
   injected by the caller, as the PS2 texture-name rule and Pulse's animation table were moved
   out of `oag-mesh`. `oag-render` still reads `oag-pulse`'s presentation tables and is not listed.

Two size rules, both enforced by `just check-size` (`scripts/check-file-size.py`) as
ratchets over a frozen baseline. `BASELINE` holds the files still over rule 1 - 20 as of
2026-08-27, down from 27 - and each may shrink but not grow; `TEST_BASELINE` is **empty**, so rule 2 has no exemptions at all:

1. A Rust file may not exceed **1,000 lines**.
2. An inline `#[cfg(test)] mod` body may not exceed **200 lines**. Past that it moves to a
   file of its own - `#[cfg(test)] mod tests;` where the block was and `<module>/tests.rs`
   beside it, as `crates/physics/src/airbrake.rs` does; `use super::*` still reaches every
   private item. The rule keys on `#[cfg(test)]` and nothing else, so a `mod` of shared
   helpers colocates freely, and a dedicated test file is bound by rule 1 alone. Check a
   move with `cargo nextest list -p <crate>` either side of it: a `mod tests;` that never
   landed looks exactly like a green run with fewer tests in it.

Gameplay state will be one `World` struct of plain data with fixed-size arrays, no ECS
(see [ADR-0003](docs/architecture/adr/0003-no-ecs.md)) - deliberately, so the whole world
snapshots in one `memcpy`-shaped operation for replays and golden tests.

**A new row or page in `assets/ui/menu.toml` needs a `string_id`/`title_string_id` and
its English text in `assets/ui/strings/english.toml`, in the same change** -
`just check-strings` (`scripts/check-strings.py`) fails the gate otherwise, the same
ratchet shape `check-size` above already is: pre-existing debt is frozen in that
script's own `BASELINE_LABELS`/`BASELINE_TITLES`, but nothing *new* may join it. A
second language's file may lag a translation, but only by naming the gap under its own
`[untranslated]` table - never by omitting the id. A machine translation may fill a gap
(2026-10-06), but every entry is `ID = { text = "...", human = false }` and `human` is
`true` only when a human wrote or approved the text. See `assets/ui/strings/english.toml`'s own doc comment for the convention.

### Determinism

The simulation must produce bit-identical state across Linux/Windows/macOS and
x86-64/AArch64, in debug and release (not bit-identical with the PSP/PS2 itself - VFPU
isn't IEEE-conformant for rsqrt/recip). Full rules in
[`docs/architecture/determinism.md`](docs/architecture/determinism.md); the ones that bite:

- `f32` only, strict IEEE-754, no `mul_add`, no reassociating arithmetic for readability.
- No SIMD in sim code - `glam` is built with `scalar-math` for this reason.
- No `HashMap`/`HashSet` iteration feeding simulation state (unstable per-process hasher).
- Never read the wall clock in the simulation; use `TickClock` (`crates/core/src/tick.rs`).
- Fixed timestep at 60 Hz always, even though the original game itself uses a *variable*
  timestep (see [ADR-0007](docs/architecture/adr/0007-fixed-timestep-vs-original.md)).
- All randomness goes through the seeded `Rng` in `crates/core/src/rng.rs`, never OS entropy.
- Simulation is single-threaded unless proven otherwise.

These rules bind `oag-core`, `oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race` and
`oag-formats` - the crates `just check-determinism` actually scans, because their
arithmetic reaches a committed state hash. `oag-render` is deliberately exempt: a pixel
is not compared across machines, so it may use `mul_add`, SIMD glam, reassociated
arithmetic, or the algebraic float ops stabilized in Rust 1.98 (`f32::algebraic_add` and
friends) freely - reach for them in a proven-hot per-frame CPU loop rather than leaving
performance on the table out of sim-code caution.

CI's `determinism` job runs on all three OSes and asserts hashes match a committed
reference (`crates/core/src/hash.rs`, `crates/core/tests/determinism.rs`). **When that
test fails, find the bug - never update the reference constants to make it pass.**

## Reverse-engineering workflow

This project's other main deliverable is `docs/` itself - a contributor should be able to
understand the engine from the docs tree without reopening Ghidra. Two rules apply
everywhere in `docs/`:

1. Record the evidence, not just the conclusion.
2. Every RE claim carries a 0-100 confidence score from the
   [confidence rubric](docs/reverse-engineering/confidence-rubric.md).

Consequences that affect how you name and touch things in Ghidra / decompiled code:

- Functions/data are named `Subsystem_VerbNoun` (PascalCase), e.g. `Ship_UpdateSteering`.
- **Below 70 confidence**, the name gets a `_q` suffix (`Ship_ApplyAirbrakeDrag_q`).
- **Below 50 confidence, do not rename at all** - leave `FUN_08831af0` and write the
  hypothesis down instead. A guess dressed as a name stops other people from looking.
- Every rename needs a doc page under `docs/ghidra/functions/<binary>/` (address, purpose,
  args, confidence, evidence) - the docs are authoritative if they and the Ghidra database
  ever disagree. See [ADR-0005](docs/architecture/adr/0005-ghidra-conventions.md).
- **Every name you recover also goes in `docs/ghidra/functions/psp-pulse-usa/names.tsv`, in the
  same change that recovers it.** The Ghidra project is not committed, so a fresh import
  starts at `FUN_08940d0c` again; that file is the only thing that makes the database
  reproducible from the repository, and `just apply-names` replays it. Rows are
  `address<TAB>kind<TAB>name<TAB>confidence<TAB>page`, `kind` being `function` or `data`.
  `scripts/apply-ghidra-names.py` **refuses** a row whose address and name do not both
  still appear on the page it cites, so the row and its evidence page land together or
  not at all. It only gets to refuse when a Ghidra bridge is up, so `just check-names`
  makes the same checks offline and is part of the `just` gate. The name column holds
  the **bare** name - the `_q` is derived from the confidence column, never written
  there. A name recovered but not written down is a name the next contributor
  re-derives from scratch.
- The loop is observe -> hypothesise -> verify -> document -> implement; don't skip
  straight to implementing from a plausible reading. See
  [methodology](docs/reverse-engineering/methodology.md).
- The PSP `BOOT.BIN` is an unencrypted ELF and loads straight into Ghidra with no
  decryption step; same for the PS2 main executable. Ghidra bridging setup is in
  [`docs/reverse-engineering/toolchain.md`](docs/reverse-engineering/toolchain.md); the
  Allegrex/VFPU processor module (`just build-allegrex`) is required, not optional, for
  correct PSP vector-instruction decoding - see
  [`docs/psp/allegrex-vfpu.md`](docs/psp/allegrex-vfpu.md).
- The `ghidra-mcp` MCP server (configured in `.mcp.json`) drives Ghidra directly when
  a Ghidra project is open.

### A 2048 finding is checked against Omega, and the reverse

**Wipeout 2048 (Vita) and the Omega Collection (PS4) share an asset lineage**:
Omega reads through 2048's `.rcsmodel` readers (8-byte pointers aside), keeps its
ships in 2048's directory scheme, splits its plugin definitions three ways like
2048, and re-ships 2048's own circuits byte-for-byte at the same layouts (see
[`docs/formats/omega-status.md`](docs/formats/omega-status.md)). Omega's front end
is HD's instead, so a front-end finding is checked against HD.

So **anything recovered or wired for one of the two - a format field, a law, a
material rule, a table, an asset path - gets a quick check against the other in the
same change**: does the other title ship the same file or code, and does the reader
or the wiring hold there? Record the answer in the finding's doc page either way,
as one of:

- **ported**: the other title now uses it too, with its own test or ground truth;
- **checked, applies, not wired**: same data or code, wiring left open, named in a
  `handover/` thread;
- **checked, differs**: what differs, with evidence;
- **not checkable**: why (no capture path for Omega's executable, say), never silence.

The check is quick by design - a census and a reader run, not a second lane. When it
turns into real work, write it down as open rather than widening the current lane.
Omega racing being incomplete does not exempt a format or asset finding from the
check.

### Never invent what the assets already author

**If a thing exists in the disc's own data, play the data. Do not author a
stand-in for it, and do not hand-transcribe it into a `const` either.** This
applies to particle effects (`Data\Psys\*.POB`), models, textures, tables and
tuning values alike. Two failures this rule exists to prevent, both of which
already happened here:

1. **A hand-transcribed table.** `oag_fx::sparks` carried the collision
   effect's four emitters as a `const` read off the file by hand. Every value
   was *correct* - the parser reproduces them - and it was still wrong: not
   re-derivable, and no use at all for the other 34 effects on the disc.
2. **A plausible-looking stand-in.** The rocket drew invented smoke puffs and
   expanding blast billboards because the `.pob` payload was believed
   undecoded. A stand-in that reads as *legible* is exactly how a wrong
   picture survives review, and it silently removed the pressure to decode the
   real thing.

So: parse it, play it, and if it will not parse, **draw nothing and say so** in
the loader report. Nothing is an honest, visible absence; an invention is not.
Where a stand-in genuinely cannot be avoided (a texture that has no located WAD
entry yet), keep it to a *substitute for the missing asset alone* and never let
it override data you do have - the whitening term removed from
`crates/fx/src/psys.wgsl` on 2026-08-12 was a sprite substitute that had
grown into an override of the emitter's own colour table.

The mechanism for effects is already generic: `oag_fx::psys::Library` loads
any `Data\Psys\<name>.POB` by name and `psys::Stage` plays any number of them at
once. Adding an effect is adding its name and its **trigger** - and the trigger
is the part that needs reverse-engineering, so an effect with no recovered
trigger stays unwired rather than fired on a guess. `HANDOVER.md` lists the ones
still waiting.

## Documentation tree

Start at [`docs/README.md`](docs/README.md) for the full reading order and tree map.
Key entry points: [goals](docs/overview/goals.md), [roadmap](docs/overview/roadmap.md),
[glossary](docs/overview/glossary.md), [ADR index](docs/architecture/adr/README.md),
[format status table](docs/formats/README.md). [`HANDOVER.md`](HANDOVER.md) tracks what
isn't in the docs tree itself: what was deliberately deferred, which findings are
independently verified versus single-source, and traps that cost time - read it before
starting non-trivial work, and update it when handing off. Work in flight is one file per
thread under [`handover/`](handover/), each with `## Open` and `## Next Steps`, sorted
into five subdirectories by category (`rendering/`, `gameplay/`, `frontend/`, `tooling/`,
`audio/`) - pick whichever fits the thread's primary subject; a thread that genuinely
crosses categories carries a `categories:` YAML frontmatter block naming the extras.
`HANDOVER.md`'s own "Open threads" section is just the index - delete a thread's file and
its index line together once the work lands, the same rule this file always applied to a
row.

**Nothing outside `handover/` and `HANDOVER.md` may link into `handover/`.** A thread
file gets deleted the moment its work lands, so a link from a permanent doc, a code
comment, a script or a test into one is a dead link waiting to happen with nothing to
catch it - `just check-docs` only proves links resolve today, and only for `docs/**/*.md`.
The reverse direction is normal and expected: a `handover/` thread citing a `docs/` page
as its evidence is how every thread is written. When a finding grew out of a thread, cite
the evidence (the doc page, the test, the address) instead of the thread file.

ADRs are immutable; a changed decision gets a new ADR that supersedes the old one, not an edit.
