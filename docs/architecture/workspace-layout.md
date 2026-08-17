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
| `oag-ai` | `crates/ai` | Opponent behaviour: a line-following driver that emits ship controls. Depends on `oag-core` and `oag-physics` only. |
| `oag-gameplay` | `crates/gameplay` | The `World` struct and the input snapshot the simulation consumes. |
| `oag-audio` | `crates/audio` | The mixer and playback device; see [ADR-0018](adr/0018-audio-mixer-architecture.md). |
| `oag-title` | `crates/title` | The engine-side *types* a title package fills in. Tables, no data. |
| `oag-pulse` | `crates/pulse` | Wipeout Pulse's tables: what that title ships. |
| `oag-pure` | `crates/pure` | Wipeout Pure's tables, in the same shape. |
| `oag-hd` | `crates/hd` | Wipeout HD / Fury's, in the same shape - and the release that forced `ArchiveCandidates::extra`, a *set* of archives all of which mount, because HD ships seven and they do not split by kind. |
| `oag-game` | `crates/game` | The composition root. Boots the front end; see [front-end boot](frontend-boot.md). A thin binary over a library, so the boot sequence can be tested without a GPU. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-weapons` | M5 | Pickups, projectiles, damage. **Not created; the work landed elsewhere** - see below. |
| `oag-ui` | M5 | HUD and menus. |
| `oag-replay` | M7 | Input recording and playback. |
| `oag-net` | M8 | Multiplayer. |

**`oag-weapons` is a plan this project did not follow, and the reason is the
warning above it.** Pickups landed on 2026-08-11 and projectiles the same day,
and they went into the crates that already had what they needed:
`oag_formats::weapons` for the table, `oag_gameplay::pickup` for the draw and
the inventory, `oag_gameplay::projectile` for flight and blasts,
`oag_game::race` for the trigger and the fire buttons, and
`oag_physics::damage` for what a hit costs. A new crate would have needed
`oag-formats` (to name a `Weapon`) and `oag-physics` (to move a body), which is
`oag-gameplay`'s dependency set exactly - so it would have been a second name
for the same layer rather than a boundary.

Splitting it out later is still open, and the thing that would justify it is
weapons growing past what one module should hold. Recorded here rather than
silently diverging, because the table above is what a contributor reads first.

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
        | oag-pulse oag-pure  |                    |
        |       oag-hd        |                    |
        |         \   /       |                    |
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

`oag-ai` opened with M5 (2026-08-11) and is the shape the table predicted, with
one thing worth stating because it is what keeps rule 1 cheap: **it depends on
`oag-core` and `oag-physics` and on no asset crate.** A driver takes the line it
follows as a list of world-space points the caller built, not as a decoded track
node, so the controller is tested against a synthetic circle with no disc image,
no `oag-formats` and no GPU. See [ai.md](../gameplay/ai.md).

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

### `package."*"` does not reach a workspace member

The glob matches **dependencies only**. From 62b4c44 (2026-07-26), where the
override was introduced, until this change, that made the paragraph above false
of the crates it was written for: `oag-physics` and `oag-core` carry
the sim loop, `oag-formats` and `oag-disc` carry the decoders, and all four
compiled at `opt-level = 0` in every `cargo build`, `cargo run` and `cargo test`.
Only the third-party crates around them were optimised.

Verified rather than assumed, in a two-member scratch workspace: with
`[profile.dev.package."*"] opt-level = 2` and nothing else, `cargo build -v`
passes **no** `-C opt-level` flag to either member. Adding
`[profile.dev.package.<member>]` does apply it, to the lib and to the `--test`
harness alike.

So the crates carrying the heavy loops are named one by one in `Cargo.toml`. The
split is by what the loop is, not by crate ownership:

| optimised | left at `opt-level = 0` |
| --- | --- |
| `oag-core`, `oag-physics`, `oag-ai`, `oag-race`, `oag-gameplay` - the sim, driven for thousands of ticks per behavioural test | `oag-render`, `oag-game`, `oag-view`, `oag-input`, `oag-audio` |
| `oag-disc`, `oag-formats`, `oag-assets` - LZSS, the GS and GE texture swizzles, the `.vex` node walk, and the sector-at-a-time read under them | `oag-title`, `oag-pulse`, `oag-pure`, `oag-hd`, `oag-trace`, `oag-tools` |

The right-hand column is where a debugger actually gets pointed, so it keeps the
debuggability the paragraph above is about.

Measured on `just test-data`, 16 cores, 2,323 tests, all images present, warm
page cache, three full runs of the identical tree:

| | wall | test phase | CPU | cores busy | `difficulty_ground_truth` |
| --- | --- | --- | --- | --- | --- |
| before | 11:27 | 686s | 3,253s | 4.1 of 16 | 600s |
| `opt-level` only | 6:35 | 393s | 1,586s | 4.1 of 16 | 149s |
| + `nextest.toml` ordering | **3:17** | **196s** | 1,508s | 7.9 of 16 | 69s |

The `opt-level` half is the CPU saving - it halves the work. The ordering half
spends what is left on more cores at once, and buys almost as much wall clock
again without making anything faster. Both are needed; neither is most of it.

The four `matches_the_committed_reference` determinism tests - `oag-core`,
`oag-ai`, `oag-gameplay`, `oag-physics` - pass unchanged at `opt-level = 2`,
which is the check that matters before touching this: see
[determinism](determinism.md).

`just scripted-sim` - the loop the paragraph at the top of this section is
actually about, and a `cargo run` rather than a test - goes from **2.20s to
0.96s** on the whole-lap scenario, prebuilt both times so the figure is the run
and not the compile.

### It costs nothing at build time, which is not what you would guess

The obvious objection is that `just test`'s inner loop pays for this. Measured,
it does not. Touching `crates/core/src/lib.rs` - the worst case, since
everything depends on it - and rebuilding every test binary, three runs each way
after a warm-up run to settle the fingerprints:

| | rebuild after touching `oag-core` |
| --- | --- |
| `opt-level = 0` | 26.3s, 27.2s, 31.2s |
| `opt-level = 2` | 21.6s, 23.1s, 23.8s |

If anything it is *faster*, though the gap is close enough to machine noise that
the honest claim is only "no penalty". Either way there is no build-time reason
to prefer `opt-level = 1` here, which is the compromise this table was measured
to test.

### The suite is tail-bound, so ordering is worth as much as speed

1,508s of work over 16 cores would finish in about 95s. The gap is shape, not
throughput: a dozen ground-truth tests take minutes and the other ~2,300 take
milliseconds, so in the default order the run *ends* with a few multi-minute
tests alone on an idle machine. `.config/nextest.toml` gives the ground-truth
binaries a scheduling priority, so they start first and the cheap tests fill in
around them.

The floor under that is one test: `oag-game::ps2_source_ground_truth`'s
`an_uncapped_transcode_still_reports_a_total_to_divide_by` transcodes 950 frames
of the PS2 intro, and its `refresh: true` is load-bearing - the assertion is
about the progress a *transcode* reports, so a cache hit would report nothing.
It takes 117s alone and 135s scheduled first. Nothing above gets `just test-data`
much below that, which is why it carries the highest priority in that file: it
has to start at t=0 or it *is* the tail.
