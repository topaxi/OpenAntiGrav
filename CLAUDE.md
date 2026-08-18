# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A clean-room reimplementation of **Wipeout Pulse** (PSP/PS2) in Rust, and a
foundation for the wider Studio Liverpool anti-gravity racing lineage (Pure, HD/Fury,
2048). The original executables are a specification, not source code: behaviour is
observed, documented with evidence and a confidence score, then reimplemented fresh.
Nothing is transliterated from the decompiler.

Status is milestone-driven; check [`docs/overview/roadmap.md`](docs/overview/roadmap.md)
for what is actually done versus planned before assuming a subsystem exists.

## Commands

```sh
just              # fmt-check + lint + test + check-docs + check-deps + check-determinism + check-size - the gate every commit must pass
just fmt          # cargo fmt --all
just lint         # cargo clippy --workspace --all-targets -- -D warnings
just test         # cargo nextest run --workspace
just test-data    # also runs #[ignore]d ground-truth tests that need data/images/ populated
just check-docs   # validates internal links in docs/ (scripts/check-doc-links.py)
just check-deps   # asserts the two dependency-boundary rules below (scripts/check-dependency-rules.py)
just check-determinism # asserts no platform transcendental reaches simulation code (scripts/check-transcendentals.py)
just check-size   # ratchet on file length (1k lines) and on inline #[cfg(test)] modules (200) (scripts/check-file-size.py)
just build        # cargo build --workspace
just docs         # cargo doc --workspace --no-deps --document-private-items
just audit-leakage # asserts no tracked game content or reproduction (scripts/check-leakage.py)
```

Single test: `cargo nextest run -p oag-core some_test_name` (nextest, not `cargo test`).
Ground-truth tests (in `crates/*/tests/*ground_truth*.rs`) are `#[ignore]`d because they
need a real disc image under `data/images/`; they never run in CI, only via `just test-data`.

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
| `oag-formats` | `crates/formats` | Asset container identification and parsing (WAD, LZSS, textures, `.vex`, track data, front-end XML, PMF, fonts). |
| `oag-assets` | `crates/assets` | Runtime asset access: `Archive` reads a WAD by path or straight out of a disc image, by index/name/hash. |
| `oag-title` | `crates/title` | The `Title` type and its three axes: archive candidates, entry names, foreign-serial deny-list. Types only, no title's data. |
| `oag-pulse` | `crates/pulse` | What Wipeout Pulse ships: archive and entry names, hashes, and its presentation tables (HUD, front end, loading wave, animated textures, race defaults). |
| `oag-pure` | `crates/pure` | The same for Wipeout Pure, deliberately thinner - it holds only what `docs/formats/pure-status.md` measured. |
| `oag-hd` | `crates/hd` | The same for Wipeout HD / Fury, and the one that is shaped differently: seven PSARC archives on a PS3 disc rather than two WADs. Its front end is wired off a **declared** boot chain rather than a measured one - the distinction the type carries per [ADR-0025](docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md); see `docs/formats/hd-frontend.md`. |
| `oag-tools` | `crates/tools` | CLI: `oag-unpack`, `oag-wad`. |
| `oag-physics` | `crates/physics` | Ship dynamics and collision queries. Depends on `oag-core` and nothing else, deliberately. |
| `oag-race` | `crates/race` | Race rules: modes, lap timing, track progress. |
| `oag-ai` | `crates/ai` | Opponent behaviour: a driver that follows the authored racing line and emits ship controls. Depends on `oag-core` and `oag-physics` only. |
| `oag-gameplay` | `crates/gameplay` | The `World` struct, the `InputSnapshot` type the simulation consumes, spline spawning. |
| `oag-render` | `crates/render` | The wgpu renderer: mesh pipeline, track ribbon, cameras. Owns no window. Reads `oag-pulse`'s tables, which runs against the arrows below and is allowed - rule 1 only forbids the other direction. |
| `oag-input` | `crates/input` | Maps real devices onto the abstract button layer and produces an `InputSnapshot`. |
| `oag-view` | `crates/view` | Asset viewer: CLI, window and texture browser over `oag-render`. |
| `oag-trace` | `crates/trace` | Per-tick trace capture and comparison against the original: `oag-trace show\|run\|compare\|script\|drive\|track`. The reading half of the M3 verification harness. |
| `oag-audio` | `crates/audio` | Mixing and playback: a hardware-free voice pool and sample loop, a `cpal` stream or none at all, and a WAV writer so a headless run is checkable. Cues are a per-tick *output* of the simulation, never `World` state. See [ADR-0018](docs/architecture/adr/0018-audio-mixer-architecture.md). |
| `oag-game` | `crates/game` | Composition root; boots the front end. A thin `[[bin]]` over `[lib]` so boot logic is testable headlessly. |

Later crates (`oag-weapons`,
`oag-ui`, `oag-replay`, `oag-net`) are
added only when their milestone opens - see
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md) for the
full table and reasoning. Don't create placeholder crates ahead of that.

Two dependency rules, both enforced by `just check-deps` (part of the `just` gate,
`scripts/check-dependency-rules.py`) so a `cargo add` that breaks one fails CI, not just
review:

1. No gameplay crate depends on `oag-render`, `oag-audio`, `oag-input`, `winit` or `wgpu`.
   The simulation consumes an input *snapshot type* owned by `oag-gameplay`, never the
   input system itself.
2. No crate depends on `oag-game` (the composition root).

Two size rules, both enforced by `just check-size` (`scripts/check-file-size.py`) as
ratchets over a frozen baseline. `BASELINE` holds the 27 files still over rule 1 and each
may shrink but not grow; `TEST_BASELINE` is **empty**, so rule 2 has no exemptions at all:

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
  not at all. A name recovered but not written down is a name the next contributor
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

### Never invent what the assets already author

**If a thing exists in the disc's own data, play the data. Do not author a
stand-in for it, and do not hand-transcribe it into a `const` either.** This
applies to particle effects (`Data\Psys\*.POB`), models, textures, tables and
tuning values alike. Two failures this rule exists to prevent, both of which
already happened here:

1. **A hand-transcribed table.** `oag_render::sparks` carried the collision
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
`crates/render/src/psys.wgsl` on 2026-08-12 was a sprite substitute that had
grown into an override of the emitter's own colour table.

The mechanism for effects is already generic: `oag_render::psys::Library` loads
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
isn't in the docs tree itself: work in flight, what was deliberately deferred, which
findings are independently verified versus single-source, and traps that cost time -
read it before starting non-trivial work, and update it when handing off.

ADRs are immutable; a changed decision gets a new ADR that supersedes the old one, not an edit.
