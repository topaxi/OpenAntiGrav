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
just              # fmt-check + lint + test + check-docs - the gate every commit must pass
just fmt          # cargo fmt --all
just lint         # cargo clippy --workspace --all-targets -- -D warnings
just test         # cargo nextest run --workspace
just test-data    # also runs #[ignore]d ground-truth tests that need data/images/ populated
just check-docs   # validates internal links in docs/ (scripts/check-doc-links.py)
just build        # cargo build --workspace
just docs         # cargo doc --workspace --no-deps --document-private-items
just audit-leakage # asserts no game content (.chd/.iso/.wad/.elf/... ) is tracked by git
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
| `oag-tools` | `crates/tools` | CLI: `oag-unpack`, `oag-wad`. |
| `oag-view` | `crates/view` | wgpu asset viewer, standalone. |
| `oag-game` | `crates/game` | Composition root; boots the front end. A thin `[[bin]]` over `[lib]` so boot logic is testable headlessly. |

Later crates (`oag-trace`, `oag-render`, `oag-input`, `oag-physics`, `oag-gameplay`,
`oag-race`, `oag-weapons`, `oag-ai`, `oag-audio`, `oag-ui`, `oag-replay`, `oag-net`) are
added only when their milestone opens - see
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md) for the
full table and reasoning. Don't create placeholder crates ahead of that.

Two dependency rules, both enforceable and worth checking before adding an import:

1. No gameplay crate depends on `oag-render`, `oag-audio`, `oag-input`, `winit` or `wgpu`.
   The simulation consumes an input *snapshot type* owned by `oag-gameplay`, never the
   input system itself.
2. No crate depends on `oag-game` (the composition root).

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

## Documentation tree

Start at [`docs/README.md`](docs/README.md) for the full reading order and tree map.
Key entry points: [goals](docs/overview/goals.md), [roadmap](docs/overview/roadmap.md),
[glossary](docs/overview/glossary.md), [ADR index](docs/architecture/adr/README.md),
[format status table](docs/formats/README.md). [`HANDOVER.md`](HANDOVER.md) tracks what
isn't in the docs tree itself: work in flight, what was deliberately deferred, which
findings are independently verified versus single-source, and traps that cost time -
read it before starting non-trivial work, and update it when handing off.

ADRs are immutable; a changed decision gets a new ADR that supersedes the old one, not an edit.
