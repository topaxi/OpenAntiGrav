# Workspace layout

## Principle

**The simulation must not know that a renderer exists.** It takes an input
snapshot in and emits a state snapshot out. Everything else - drawing, audio,
windowing, file I/O - happens outside it.

This is not architectural taste. It is what makes the simulation testable
without a GPU, deterministic without a scheduler, portable to a host with no
windowing system, and comparable against a trace from the original. Every other
rule here follows from it.

## Vocabulary

Three axes get confused if they share a word, so they do not share one here:

- **title** - Wipeout Pulse, Pure, HD/Fury, 2048. A data axis, per
  [ADR-0022](adr/0022-title-packages.md): tables, never dispatch.
- **console** (also **source**) - PSP, PS2, PS3, Vita. The machine the original
  ran on, and where its assets come from. Deliberately *not* a code axis, per
  [ADR-0004](adr/0004-asset-pipeline.md).
- **host** - Linux, Windows, macOS, and later the handheld and web targets. What
  this reimplementation runs on.

Older pages, and [`goals.md`](../overview/goals.md)'s scope table, use
"platform" for the first two; read them with this distinction in mind.

## Crates that exist

| Crate | Path | Purpose |
| --- | --- | --- |
| `oag-core` | `crates/core` | Deterministic math, fixed-timestep clock, seeded PRNG, state hashing. Depended on by everything. |
| `oag-disc` | `crates/disc` | CHD and raw ISO readers, ISO 9660 walker, console identification. |
| `oag-formats` | `crates/formats` | Asset container identification and parsing. Currently triage only; parsers land as formats are decoded. |
| `oag-tools` | `crates/tools` | Command line tools: `oag-unpack`, `oag-wad`. |
| `oag-render` | `crates/render` | The wgpu renderer: mesh pipeline, track-ribbon builder, cameras. Owns no window, so the viewer and the game can each keep their own. |
| `oag-view` | `crates/view` | wgpu asset viewer. The first crate with a window. |
| `oag-assets` | `crates/assets` | Runtime asset access: a WAD read from a path or straight out of a disc image, by index, name or name hash. |
| `oag-trace` | `crates/trace` | Trace capture and comparison against the original. |
| `oag-input` | `crates/input` | Input mapping and the per-tick input snapshot. |
| `oag-physics` | `crates/physics` | Ship dynamics and collision. |
| `oag-race` | `crates/race` | Race rules, lap timing, track progress. |
| `oag-gameplay` | `crates/gameplay` | The `World` struct and the input snapshot the simulation consumes. |
| `oag-audio` | `crates/audio` | The mixer and playback device; see [ADR-0018](adr/0018-audio-mixer-architecture.md). |
| `oag-title` | `crates/title` | The engine-side *types* a title package fills in. Tables, no data. |
| `oag-pulse` | `crates/pulse` | Wipeout Pulse's tables: what that title ships. |
| `oag-pure` | `crates/pure` | Wipeout Pure's tables, in the same shape. |
| `oag-game` | `crates/game` | The composition root. Boots the front end; see [front-end boot](frontend-boot.md). A thin binary over a library, so the boot sequence can be tested without a GPU. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-weapons` | M5 | Pickups, projectiles, damage. |
| `oag-ai` | M5 | Opponent behaviour. |
| `oag-ui` | M5 | HUD and menus. |
| `oag-replay` | M7 | Input recording and playback. |
| `oag-net` | M8 | Multiplayer. |

## Title packages

[ADR-0022](adr/0022-title-packages.md) splits two questions that used to be one.

**"How does this byte stream decode?" is answered by the file, inside
`oag-formats`.** Class-ID tables, header shapes and schema variants are selected
from the artifact's own version word - `vex::classes::{V6, V4, V3}` and the
equivalents for `collision`, `track` and `handling`. The decoder never learns
which title a file came from, and no caller tells it. This is why `oag-formats`
has no dependency on any title crate, and must not grow one: the reverse edge
exists, so a forward edge is a cycle.

**"What does this title ship?" is answered by a title package.** Archive
candidate lists, entry names, name hashes, team lists, HUD and front-end names:
`oag-pulse` and `oag-pure`. `oag-title` holds the types, and deliberately only
for the axes where two corpora have actually been measured - archive candidates,
entry names, handling schema version. Presentation vocabulary stays as plain
constants inside `oag-pulse` until Pure forces its shape.

Title crates are tables, so they depend on `oag-title` and `oag-assets` and
nothing else. That keeps them readable from both sides of the simulation
boundary, and it is why they are listed among the gameplay crates in
`scripts/check-dependency-rules.py` - rule 1 applies to them unchanged.

**Being readable from both sides means the renderer reads them too.**
`oag-render` depends on `oag-pulse` for the tables that say which of Pulse's
textures animate and what the loading wave's numbers are. That is not a breach:
rule 1 forbids gameplay -> render, and this is the reverse edge, which is the
direction a table is meant to be read in.

The simulation's own Pulse constants - the physics literals, the Zone scoring,
the handling scale factors - have their seam defined but have **not** moved into
`oag-pulse`. [ADR-0009](adr/0009-multi-game-fanout.md) item 2 gates second-title
simulation work on M4's exit criterion, and the force law is that milestone's
live blocker. The move is mechanical once it clears, and
`oag_race::zone`'s four numbers are the ones to move first: Pure ships zone mode
too. `Course::START_LINE_OFFSET` is the one exception and could **not** be
filed - at confidence 65 it stands in for a computation nobody has read, so
calling it title-specific would assert one of the two answers still open.

## Dependency rules

```
                         oag-core
                            |
        +-------------------+--------------------+
        |                   |                    |
    oag-disc            oag-physics          oag-render
        |                oag-ai                  |
   oag-formats          oag-weapons          oag-audio
        |     \           oag-race             oag-input
    oag-assets  oag-title      |                    |
        |         /   \        |                    |
        |   oag-pulse oag-pure |                    |
        |         \   /        |                    |
        +----------> oag-gameplay <---------------+
                            |
                        oag-game
```

One edge the sketch leaves out, because drawing it would cross half the tree:
`oag-render` also depends on `oag-pulse`, for the tables described above. It runs
against the arrows, which is the direction a table is read in.

Two rules, both enforceable:

1. **No gameplay crate may depend on `oag-render`, `oag-audio`, `oag-input`,
   `winit` or `wgpu`.** The arrow from `oag-input` goes to `oag-gameplay`, which
   owns the input *snapshot type*; the simulation consumes the snapshot, not the
   input system.
2. **No crate may depend on `oag-game`.** It is the composition root. Its own
   `[[bin]]` depends on its own `[lib]`, which is inside the crate and so does
   not breach this; the library exists purely so the boot sequence is reachable
   from tests.

`oag-render` arrived early too, for the same kind of reason: the only 3D renderer
in the repository was inside `oag-view`, and the game needs the same pipeline to
draw a track and a ship. The mesh loader, the wgpu mesh pipeline and its shader,
the track-ribbon builder, the offscreen render-and-readback capture and the
camera maths all moved out of `crates/view` into `crates/render` unchanged; what
stayed behind is the viewer itself - the CLI, the `winit` event loop, the texture
browser and its screenshot path. `oag-render` has **no `winit` dependency** and
never will: a crate that owns a window cannot be shared with a game that owns its
own. Its seam is `mesh_render::build(&device, &queue, &model, format)`, the same
shape `oag-game`'s own front-end renderer already has, so each caller keeps its
surface to itself.

`oag-assets` and `oag-game` both arrived earlier than the table above expected:
the front end needed asset access and a binary before any of M4 existed.
`oag-assets` has since become the console-normalising registry it was meant to
be - it reads both the PSP and the PS2 asset paths, and per
[ADR-0004](adr/0004-asset-pipeline.md) nothing downstream branches on which. Its
`Layout` candidate resolution is the *mechanism*; the candidate lists themselves
are title data and live in the title crates.

The three older copies of "open a disc image, find a `.wad`, read its
directory, decompress a blob" that used to live in `oag-tools`'s `oag-wad`,
`oag-view`'s asset loader and its mesh loader are gone: all three now go
through `oag_assets::Archive`, which grew a `len()` and a `read_raw()` to cover
what they still needed beyond decoded reads (`oag-wad`'s size summary and its
LZSS-leftover diagnostic). Before/after snapshots of `oag-wad list`, `tags` and
`verify` against both the PSP and PS2 discs came out byte-identical.

Rule 1 is the one that will be under pressure. It is easier to reach for a
texture handle inside physics than to plumb it out. Resisting that is what keeps
headless verification possible.

## Gameplay state

A single `World` struct of plain data, with fixed-size arrays. No ECS. See
[ADR-0003](adr/0003-no-ecs.md).

```rust
// Illustrative, not yet implemented.
struct World {
    tick: u64,
    rng: Rng,
    ships: [Ship; MAX_SHIPS],
    ship_count: u8,
    projectiles: [Projectile; MAX_PROJECTILES],
    race: RaceState,
}
```

A race has at most eight ships and a bounded number of projectiles. An ECS would
add a scheduler whose ordering is a determinism hazard, and archetype storage
that is harder to snapshot, in exchange for flexibility this problem does not
need.

Snapshotting the whole world is one `memcpy`-shaped operation, which is what
makes replays and golden tests straightforward.

## Naming

- Crate names are `oag-*`; directories under `crates/` drop the prefix.
- Nothing is published to crates.io, so the prefix exists for readability rather
  than to claim names.
- Binaries are `oag-*` too, so they are recognisable in a `$PATH`.

## Build profiles

`[profile.dev.package."*"]` sets `opt-level = 2` for dependencies. Behavioural
verification runs the simulation for many thousands of ticks; a fully
unoptimised build makes that loop too slow to iterate on, while keeping our own
crates unoptimised preserves debuggability.
