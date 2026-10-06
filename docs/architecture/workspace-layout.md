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
| `oag-display` | `crates/display` | The picture's configuration vocabulary, and the grid a source authors in. Extracted from `oag-game` because it was the most-named module in the composition root, and a shared vocabulary living inside one of its own consumers is not shared. |
| `oag-formats` | `crates/formats` | Asset containers, compression, triage, and console byte layout - what every decoder needs before it can start. Split by format family per [ADR-0050](adr/0050-format-crates-split-by-format-family.md). |
| `oag-rcs` | `crates/rcs` | The `RCSMODEL` scene. `rcsmodel` and `rcsmaterial` reference each other, so they are one crate by necessity as well as by subject; `rcsmodel::psp2` stays inside it, because the console is not a code axis. |
| `oag-vex` | `crates/vex` | The `.vex` scene tree and its class-tagged payloads. They are chunks of one tree rather than neighbouring formats, which is why they are one crate. |
| `oag-pob` | `crates/pob` | The `.pob` particle system: the `SYSP` container, emitter tree, channels, modifiers, embedded sprite textures and byte coverage. Split out of `oag-vex` on 2026-10-05: it needs only `oag-formats`, and the `.vex` scene merely names a `.pob` (class `0x3c4`), so neither crate depends on the other. This supersedes the placement ADR-0050 called arguable ("`pob` goes to `oag-vex`"). |
| `oag-texture` | `crates/texture` | Every pixel format the originals ship. Four consoles in one crate deliberately: `gtf` and `gxt` share the same S3TC block maths, so a console cut would run through it. |
| `oag-tables` | `crates/tables` | Every table a title authors as XML, and the tag reader underneath them. The only crate in the workspace with no dependencies at all. |
| `oag-video` | `crates/video` | Video containers - what the originals wrap a bitstream in, plus this project's own movie cache. The first crate carved off `oag-formats`, and the one with no workspace dependencies at all. |
| `oag-tools` | `crates/tools` | Command line tools: `oag-unpack`, `oag-wad`. |
| `oag-render` | `crates/render` | The wgpu renderer's scene: shadows, PVS placement, the track-ribbon builder, the gantry, the ghost and loading overlays and cameras. Owns no window, so the viewer and the game can each keep their own. Composes `oag-fx`, `oag-mesh`, `oag-post` and `oag-gpu`; callers import those directly, there are no re-exports. |
| `oag-fx` | `crates/fx` | The renderer's visual effects: the `.pob` particle player (`psys`) and the collision-spark adapter, the exhaust, mist, clouds, the Leach beam, the screen flash, the Cannon's quads and the hull overlays. Split out of `oag-render` on 2026-10-05; depends on `oag-mesh` for the vertex type and the format crates, never on `oag-render`, so the effects need no camera, shadow or PVS code (the one shared leaf, `hull_overlay::pulse`, moved with them). Nothing gameplay may depend on it. |
| `oag-mesh` | `crates/mesh` | The mesh pipeline: `.vex` and `RCSMODEL` models decoded into one portable vertex and index buffer, the pipeline that draws them offscreen or into a surface, the orbit camera it frames them with, and the headless capture. Split out of `oag-render` on 2026-10-05; depends on `oag-gpu` and the format crates, never on `oag-render`, `oag-post` or a title package (dependency rule 3). Its `shine_pass` module holds the CPU half of the hull's extra pass, which `oag-livery` and `oag_render::shine` share. |
| `oag-post` | `crates/post` | Post-processing between a scene and the surface: bloom (PSP, PS2, HD), the Omega tonemap, motion blur, SMAA, FXAA, FSR 1 and FSR 3, with the FSR 3 jitter sequence. Split out of `oag-render` on 2026-10-05; depends on `oag-gpu` only. |
| `oag-shader-check` | `crates/shader-check` | A build-dependency only: links a crate's WESL, validates every final shader with `naga` (see "Shader validation at build time" below) and holds the WESL modules the render crates share under `shaders/` ([ADR-0060](adr/0060-every-render-crate-shader-is-wesl-with-one-shared-module-set.md)). Depends on `naga` and `wesl`, nothing in the workspace. |
| `oag-gpu` | `crates/gpu` | The little the mesh pipeline and the post chain share and neither may own: the scene and velocity target formats, the `perf-probe` instrumentation and GPU timestamp timing. It is what keeps `oag-mesh` and `oag-post` from depending on each other. |
| `oag-view` | `crates/view` | wgpu asset viewer. The first crate with a window. |
| `oag-assets` | `crates/assets` | Runtime asset access: a WAD read from a path or straight out of a disc image, by index, name or name hash. |
| `oag-trace` | `crates/trace` | Trace capture and comparison against the original. |
| `oag-testdata` | `crates/testdata` | Test-only: locating the user-supplied disc images, and the `OAG_REQUIRE_GAME_DATA` skip contract. |
| `oag-input` | `crates/input` | Input mapping and the per-tick input snapshot. |
| `oag-physics` | `crates/physics` | Ship dynamics and collision. |
| `oag-race` | `crates/race` | Race rules, lap timing, track progress. |
| `oag-ai` | `crates/ai` | Opponent behaviour: a line-following driver that emits ship controls. Depends on `oag-core` and `oag-physics` only. |
| `oag-weapons` | `crates/weapons` | Pickups, projectiles, blasts, beams, disruption and slowdown: what a craft holds, what it fires and what a hit costs. Simulation code, in `GAMEPLAY_CRATES` and the determinism scan. **Below `oag-gameplay`**: `World` and `Ship` embed its state, and it reaches a craft only through the `oag_weapons::Craft` trait, which `oag_gameplay::Ship` implements. Depends on `oag-core`, `oag-physics`, `oag-race` and `oag-tables`. |
| `oag-gameplay` | `crates/gameplay` | The `World` struct and the input snapshot the simulation consumes. |
| `oag-replay` | `crates/replay` | Replays: the per-slot input stream as the truth, a state hash a second to catch a desync, and a ghost lap's pose track. Created when M7 opened, per [ADR-0055](adr/0055-replays-are-inputs-and-a-ghost-is-poses.md). In `GAMEPLAY_CRATES`: it depends on `oag-core` and `oag-gameplay` and nothing that draws. |
| `oag-audio` | `crates/audio` | The mixer and playback device; see [ADR-0018](adr/0018-audio-mixer-architecture.md). |
| `oag-present` | `crates/present` | The frame between the scene and the glass: `upscale` (blit, FSR/FXAA/SMAA/temporal, grade, screen filter, composite), `drs` (dynamic resolution controller) and `perf` (frame meter, cost breakdown, `SceneStats`). Depends on `oag-display`, `oag-gpu`, `oag-post` and `oag-ui`; the host passes plain data in. Its device-and-`Renderer` composite tests stay in `oag-game`. |
| `oag-livery` | `crates/livery` | Per-slot ship assets for a race: hulls, skins (`ship_skin::apply`), shield shells, boost plumes, engine lights, wrecks, plus `entry`, the archive entry names they come from. Depends on `oag-mesh` and `oag-fx`, not `oag-render` (cut 2026-10-05). The catalogue half of skin selection (`resolve`) stays beside the race load in `oag-raceplay` (`load/skin.rs`) because it reads `catalogue`. |
| `oag-music` | `crates/music` | Soundtrack reading and decoding: a title's music listing (declared, sniffed or Omega's Wwise states) and the ATRAC3+, ATRAC9, MP3 and Wwise decoders. `oag-sound` opens the source through its `Library` and calls it. |
| `oag-sound` | `crates/sound` | What the game plays: soundtrack selection and the music playlist, the effect banks and cues, HD's authored mix, over the `oag-audio` mixer. Depends on neither `oag-game` nor a title package; the host supplies a `Library` (opens a source as a title, lists disc images) and, per tick, a plain-data `RaceFrame`. `oag-game` keeps `oag_game::sound`, which implements the first and builds the second from a `Race`. |
| `oag-raceplay` | `crates/raceplay` | A race from load to finish line: `race::load`, the front-to-back tick (`Race`, `RaceSim`), the scene it draws, weapon visuals, scenery effects, the replay and ghost glue, plus what a race load reads with them (`catalogue`, `pilots`, `loader_log`, the `scoreboard` table and the track-panel assets). Renderer-coupled, so it sits above `oag-render`, `oag-fx`, `oag-mesh`, `oag-sound`, `oag-hud`, `oag-livery` and `oag-present` and below `oag-game`; classified in `NOT_TITLE_PACKAGES`, nothing gameplay-side may depend on it. It reaches nothing in `oag-game`: the host builds the loading screen's `Progress` from `LoadProgress`, and the headless capture path (`oag_game::race_capture`) stays behind because it composites the front end's overlays. |
| `oag-source` | `crates/source` | Finding a title's disc image and opening it as whichever title it is: the image search path (`source`), `title::open_source`, downloadable content (`dlc`), a race's track and craft sources (`remix::Remix`) and the derived-cache directories (`cache`). Split out so the boot, `oag-sound`'s host and `oag-raceplay`'s load can all open a source without one depending on another. |
| `oag-hud` | `crates/hud` | The in-race HUD and the sprite sheet: layout model, readouts, draw list. Extracted from `oag-game`; the two wgpu passes (`hud_overlay`, `hud_countdown`) stay behind because they need `oag-game`'s `Renderer`. Classified in `NOT_TITLE_PACKAGES` - it reads titles and depends on `oag-race`, and nothing gameplay-side may depend on it. |
| `oag-ui` | `crates/ui` | The front end: boot movies, menus, the HUD's font, the strings a screen draws. Extracted from `oag-game` the same way `oag-display` was; `oag-game`'s own `render.rs` rasterises the `Draw` list it emits. Classified in `NOT_TITLE_PACKAGES` rather than `GAMEPLAY_CRATES` - menus draw, so it may link a renderer, which is exactly what nothing gameplay-side may ever do. |
| `oag-ui-screens` | `crates/ui-screens` | The front end's individual screens, built on `oag-ui`'s vocabulary: the campaign map and flyer, the end-of-race screens, the garage and track pickers, the name-entry and confirm prompts, the ticker marquee and the track panel. Split out of `oag-ui`, which keeps the lower core (`frontend`, `menu`, `screen`, `language`, `pointer`, `font`) and never depends back on it. Same `NOT_TITLE_PACKAGES` classification. |
| `oag-title` | `crates/title` | The engine-side *types* a title package fills in. Tables, no data. |
| `oag-pulse` | `crates/pulse` | Wipeout Pulse's tables: what that title ships. |
| `oag-pure` | `crates/pure` | Wipeout Pure's tables, in the same shape. |
| `oag-hd` | `crates/hd` | Wipeout HD / Fury's, in the same shape - and the release that forced `ArchiveCandidates::extra`, a *set* of archives all of which mount, because HD ships seven and they do not split by kind. |
| `oag-omega` | `crates/omega` | Wipeout: Omega Collection's, on PS4 - HD's own `PI001` front-end plugin carried forward, off a base package plus a mandatory patch. Racing is out of scope: the patch's `data09.psarc` carries the complete front end and no circuits at all. |
| `oag-game` | `crates/game` | The composition root. Boots the front end; see [front-end boot](frontend-boot.md). A thin binary over a library, so the boot sequence can be tested without a GPU. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-ui` | M5 | HUD and menus. |
| `oag-net` | M8 | Multiplayer. |

**`oag-weapons` was reserved here for M5 and created on 2026-10-05.** Pickups
and projectiles had landed in `oag-gameplay` on 2026-08-11 because it already
held what they needed, and `oag-gameplay` grew to carry about two thirds weapons
code. The split put them below it: `World` and `Ship` hold a `Projectiles`, a
`Held` and a `Disruption`, so the edge has to run from gameplay to weapons.
The weapons code reached into `Ship` for nine fields, which became the
`oag_weapons::Craft` trait, used generically (never `dyn`) so the arithmetic is
the same instructions as before. The three functions that took a `&mut World`
(`projectile::step`, `slowdown::drain`, `disruption::advance`) now take the
pool and the occupied slice of ships; `oag_raceplay::tick` passes
`&mut world.ships[..world.ship_count as usize]`. `MAX_SHIPS` is defined once in
`oag-weapons`, the lowest crate that needs it, and `oag_gameplay::MAX_SHIPS` is
that constant.

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
too. `Course::START_LINE_OFFSET` was the one exception that could **not** be
filed - at confidence 65 it stood in for a computation nobody had read. That
computation is read now (`Course::START_LINE_ADVANCE`, a literal in Pulse's
`RaceManager_Construct`; [race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md)),
so it is the title's; it stays on `Course` because that is the one place able
to apply it, and its doc comment says so.

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

1. **No gameplay crate may depend on `oag-render`, `oag-fx`, `oag-audio`, `oag-input`,
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
| `oag-core`, `oag-physics`, `oag-ai`, `oag-race`, `oag-weapons`, `oag-gameplay` - the sim, driven for thousands of ticks per behavioural test; `oag-raceplay` (the whole `race/` tick, formerly in `oag-game`, which keeps its own entry for the capture loop) and `oag-render` (the particle systems it steps), added 2026-10-01 (`oag-fx`, `oag-mesh` and `oag-post`, split out of it on 2026-10-05, carry the same entry) - see [the gate-speed section](#the-race-itself-was-the-unoptimised-cost-2026-10-01) | `oag-view`, `oag-input`, `oag-audio` |
| `oag-disc`, `oag-formats`, `oag-assets` - LZSS, the GS and GE texture swizzles, the `.vex` node walk, and the sector-at-a-time read under them; `oag-video` - demuxing a whole movie a packet at a time; `oag-tables` - a character-at-a-time XML walk over every row on the disc; `oag-texture` - per-texel palette and block-codec loops; `oag-vex` - the node walk over a whole circuit; `oag-pob` - every particle system on a disc parsed and its bytes claimed; `oag-rcs` - the RCSMODEL geometry and material walk | `oag-title`, `oag-pulse`, `oag-pure`, `oag-hd`, `oag-trace`, `oag-tools` |

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
`oag-ai`, `oag-weapons`, `oag-gameplay`, `oag-physics` - pass unchanged at `opt-level = 2`,
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
It took 117s alone and 135s scheduled first, and **189s** when the suite was
re-measured on 2026-09-09 with no change to the test. Nothing above gets `just
test-data` much below that, which is why it carries the highest priority in that
file: it has to start at t=0 or it *is* the tail.

### Ordering stops paying once one test is the whole tail

Re-measured 2026-09-09, same machine, same 16 cores:

| | 2026-08-17 | 2026-09-09 | after the split | after the shared image |
| --- | --- | --- | --- | --- |
| tests | 2,323 | 4,101 | 4,114 | 4,114 |
| test phase wall | 196s | **587s** | 379s | **344s** |
| test CPU | 1,508s | 3,143s | 3,728s | |
| cores busy | 7.9 of 16 | **5.35 of 16** | 9.8 of 16 | |
| slowest single test | | **525s** | 321s | **184s** |

**CPU went up, deliberately.** Splitting a chained comparison into its pairwise
links measures each interior tier twice, which is where most of that 585s came
from. It is the trade the whole section is about: on a machine running at 5.35
of 16 cores, wall clock is what the gate costs and idle cores are free.

Nothing in `.config/nextest.toml` had rotted - `binary(~ground_truth)` still
caught every expensive test. **The schedule was already near-optimal and it did
not matter**, because only **62s** of that 587s run was not overlapped with a
single test. `ai_roll_ground_truth`'s `a_full_grid_still_rolls_and_a_higher_tier_rolls_no_less`
landed on 2026-09-06 measuring **525s**, which is 89% of the whole wall clock.

That is the shape to recognise: past a point, a scheduling priority has nothing
left to overlap and the only thing that moves is the slowest test itself.

### The unit of parallelism is the test, so a matrix must be tests

Every one of the tail's tests was a matrix - N circuits by M tiers by K seeds -
in nested `for` loops inside one `#[test]`. `cargo nextest` runs each test in its
own process and parallelises across *tests*, so each of those ran on one core
with fifteen idle beside it. Making the matrix the test axis costs nothing and
the assertions come along, given a split chosen to fit them:

| test | was | is | split on | why the assertion survives |
| --- | --- | --- | --- | --- |
| `ai_roll` full grid | 525s | **114s** | seed x adjacent tier pair | each seed is separately monotone (its own table says so); a chain is its pairwise links |
| `ram` clearance + outcome | 226s + 206s | **116s** | *merged*, not split | both ran the same 40-race sweep in separate processes; the ratio bound needs the sample whole |
| `stall_rescue` | 158s | **61s** | difficulty | `max` over a partition is `max` over the whole |
| `spawn_heading` HD | 147s | **81s** | forward vs reversed | both constants are grouped by direction, so the ordered `assert_eq!` filters exactly |
| `spawn_heading` sources | 106s | **37s** | source image | each source's count and stale list are its own |
| `hd_trackwall` grid | 155s | **29s** | archive | the claim is per circuit, and the archive is the axis the file is already written around |

`ram` is the one that did not split, and the reason is worth reading before
splitting anything: its bound is a *ratio*, and `RACES`'s own doc comment records
a day spent discovering that six races could not carry it. Slicing the sample
would have rebuilt the flake that was just removed. Merging its duplicate sweep
still halved both its CPU and its wall clock.

### Then the disc opens, which is worth less than it looks

With the tail gone the suite is throughput-bound rather than tail-bound, so the
next thing to cut is CPU. The obvious target is the disc: `Archives::open` used
to walk the same image once per archive - `survey` opened it, walked the whole
ISO 9660 tree, and **dropped** it, then every `Container::open` behind it opened
and walked it again, twice over on a Pulse disc and eight times on a Wipeout HD
one. Each container also carried its own single-hunk CHD cache, so a caller
alternating `Data.wad` and `FE.wad` thrashed two caches that never saw each
other's hunks.

`survey` now hands its image back and every container mounts on it. Measured on
`Archives::open`, five opens, best of:

| image | before | after |
| --- | --- | --- |
| `pulse-ps2-eu.chd`, 3.7 GB | 47.2ms | **17.9ms** |
| `pulse-psp-eu.chd` | 27.5ms | **10.0ms** |

**Measure this sort of thing before spending a refactor on it.** The estimate
that motivated the work was that the disc-open path was a large fraction of
3,728s of CPU. It is not: 889 disc-backed tests at tens of milliseconds an open
is *single-digit percent*, and the suite was at 5.35 of 16 cores when it was
first proposed, so it would have bought nothing at all until the tail was split.
The measurement is the reason it is documented here as a modest win rather than
sold as the fix.

`oag_testdata::image` is the other half and is the larger one: a CHD is
compressed, so every read decompresses a hunk, and `Archives::open` costs
**31ms** against `pulse-psp-usa.chd` against **0.05ms** against the raw `.iso`
that `just extract-isos` writes beside it. `DiscImage::open` sniffs by magic, so
the two are interchangeable; the test helper prefers the extract when it exists
and is no older than the image it came from. A stale extract would feed wrong
bytes to 148 test files in silence, which is why the mtime check is not
optional, and `oag_testdata::exact` exists for the two files where the *image*
rather than its contents is the subject - `oag-disc`'s own ground truth diffs a
CHD against its extract, and `launcher_ground_truth` reads the directory the
images live in.

Together: 379s to **344s**, and the slowest single test from 321s to 184s -
mostly because less total CPU means less contention, not because that test got
faster.

### The ratchet, because none of this was anybody's bad commit

`ai_roll`'s 525s test budgeted its own cost honestly in a doc comment and passed
review anyway. `scripts/check-test-budget.py` (`just check-test-budget`, run by
`just test-data` itself) fails when the suite exceeds **450s** or a single test
exceeds **300s**, as a ratchet over a `BASELINE` that may shrink and not grow.
The ceiling is deliberately loose - a nextest duration is wall-per-test under
load, not isolated CPU, which is why these durations sum to 4,639s against
3,143s of real CPU - so it gates gross regressions rather than drift.

### Re-measured 2026-09-17, isolated: the split still holds, and a contended run is not evidence

A `lane/test-tail-split` pass was briefed against numbers that turned out to be
a contended `just test-data` run overlapping a concurrent lane's own gate - 6
of the ai_roll tests read 212-350s and `spawn_heading`'s forward test read
243s, which look like ceiling breaches. They were not: `CEILING` is checked
against a nextest duration that is wall-per-test-under-load by design (see
above), and this project has now cost real time twice to the same
misreading - the 2026-09-06/09-09 history this file already records, and this
one.

Measured instead with the machine confirmed idle - CPU busy 6.9-14.1% over a
3s `/proc/stat` sample, zero `nextest`/`rustc`/`rpcs3` processes running, no
other lane active - rather than by load average, which lags a finished run by
several minutes and read 7.0 at the moment CPU busy was 12.6%. **Load average
is a lagging indicator and the wrong signal for "is the machine free to
measure"; an instantaneous CPU-busy sample plus a process check is the right
one.**

`ai_roll_ground_truth`'s six full-grid tests, each its own process (`cargo
nextest run -p oag-game --run-ignored all -E 'binary(ai_roll_ground_truth)'`):

| test (tier pair, seed) | 2026-09-13 | 2026-09-17 (idle) |
| --- | --- | --- |
| skilled/novice, held-out | within 158-165s band | **88.8s** |
| skilled/novice, default | within 158-165s band | **91.5s** |
| ace/elite, held-out | within 158-165s band | **96.8s** |
| ace/elite, default | within 158-165s band | **97.9s** |
| elite/skilled, default | within 158-165s band | **99.3s** |
| elite/skilled, held-out | within 158-165s band | **99.7s** |

`spawn_heading_ground_truth` (`-E 'binary(spawn_heading_ground_truth)'`):
forward **43.3s** (was 81s), reversed **37.1s**; the three per-source tests
5.5-31.2s (was 37s max). All comfortably under both the historical split
figures and `CEILING`.

**No cost regression despite the weapons work landed since 2026-09-13**
(rocket, mine, plasma, disruptor, LeachBeam slowdown, wrecked-opponent
respawn, and the multiplayer per-slot `Race::tick` input change) - if
anything these tests got faster, plausibly from unrelated CPU-cost
reductions elsewhere in the same window rather than from anything these
tests touch. **The split from 2026-09-09 needs no further work**: `ai_roll`
has no valid split beyond the one it already has (its own doc comment and
`git show 6f7f7711` already argue why a per-circuit split would change the
assertion rather than merely partition it), and `spawn_heading` is already
split on both its available axes. Every `check-test-budget` red seen this
session was contention, not drift.

### The race itself was the unoptimised cost (2026-10-01)

The table above left `oag-game` and `oag-render` at `opt-level = 0`. That was
right when the sim lived in `oag-race`, and stopped being right when the whole
race tick (`crates/raceplay/src/`) and the particle systems it steps moved into
those two crates: `ai_roll`, `ram`, `eliminator_finish`, `difficulty` and the
rest of the sim-driving ground-truth tests were running the race unoptimised.
Three changes, all in `Cargo.toml`'s profiles, none touching float semantics
(`opt-level` keeps IEEE; the determinism hashes and the all-twelve
`a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` output are
byte-identical before and after, see below):

1. `oag-game` and `oag-render` at `opt-level = 2`.
2. `[profile.dev] incremental = false`. Incremental mode uses 256 codegen units
   and, at `opt-level = 2`, stops small `#[inline]` functions (`Vec3::from_array`,
   `Iterator::next`) inlining across them: `perf` showed `Vec3::from_array` at 11%
   of a sim test as a real call. Measured on the two sim tests in
   `eliminator_finish_ground_truth` + `difficulty_ground_truth`, user CPU
   (machine loaded by other members, so read as ratios): oag-game at O2 with
   incremental 50s + 24s, without 30s + 16s, plus `oag-render` at O2 22s + 10s.
   It also lets `sccache`, the rustc wrapper in `~/.cargo/config.toml`, cache our
   own crates; it refuses incremental ones.
3. `[profile.dev.package."*"] debug = false`. Third-party DWARF was over half of
   a 91 MB test binary (now 41 MB) and of the sys time spent linking 130 of them.
   Our own crates keep `line-tables-only`, which is what a failing test is read
   from.

And one source fix the profile pointed at: `oag_texture::bcn::bc7` read its
bitstream a bit at a time and built each texel then `swap`ped channels, which
the compiler turned into byte stores reloaded as a word (a store-forwarding
stall at 54% of the function). A `u128` shift-and-mask reader and a rotation
written as a choice of whole arrays halve the Omega GNF pixel sweep
(`data00_gnf_entries_decode_or_refuse_by_name` 152s to 77s). The bc7 unit tests
and the ground-truth sweep still pass unchanged.

A fourth, separate bug: `crates/game/build.rs` named the worktree's own
`refs/heads/<branch>` as a `rerun-if-changed` path. Refs live in the common git
directory, so in every linked worktree that path never existed, cargo treats a
missing path as changed, and every cargo invocation rebuilt `oag-game` and
relinked its ~130 test binaries (a no-op `cargo test --no-run` cost 217 CPU-s,
now 0.7).

**Measured** (`just test-data`, same tree otherwise, all images present; the
machine ran other members' builds and tests throughout, load 15-55 at the start,
so wall clock is an upper bound and CPU is the figure to trust):

| | before | after |
| --- | --- | --- |
| tests | 5,831 | 5,831, all passed |
| user CPU | 5,855s | 3,667s with the profile change, **3,516s** with the bc7 fix |
| wall | 368s | 276s, then **232s** (suite per `check-test-budget`: slowest test 153s then 131s) |
| `a_lone_craft_gets_round...` | 53-57s | 15s, output byte-identical |
| `ai_roll` full-grid tests | 207-220s each | 98-100s each |
| `ram` | 242s | 129s |

What that costs: the edit-and-rebuild loop in `oag-game` pays more CPU, because
a non-incremental `opt-level = 2` rebuild of the 130k-line library and its test
crates replaces an incremental `opt-level = 0` one. A comment-only edit followed
by `just lint` + `cargo nextest run --no-run` measured about 370 CPU-s (7.7s
wall for lint, 47s for the build, load 37-48) against about 280 CPU-s before.
The data run saves roughly ten times what that costs, but a member iterating on
one test with `cargo nextest run -E 'test(...)'` should expect the first build
after an edit in `oag-game` to take longer.

**What is left.** The remaining tail is not scheduling: it is the PS2 intro
transcode (`ffmpeg`/libaom, about 130s, the floor), `omega_gnf_pixels` BC7
decode volume, ATRAC9 decode (`omega_wem_ground_truth`, 100s each, third-party
`atrac9dec` and a non-inlined libm `floor`), and in the sim tests
`oag_physics::collide::TriangleSoup::raycast_all` at 25% plus `raycast` at 14%
of a `ram` run: a brute-force walk of every triangle. A spatial index there
would speed every sim test and the game itself, but it must return hits in the
same order to keep the state hashes, so it belongs to a physics lane and is
recommended, not done.

## Dependency graph clean-up (2026-10-05)

`cargo shear` found 29 unused or misplaced dependencies; they were removed and
`just check-unused-deps` now fails the gate on any new one. Three layering cuts
shortened the longest normal-dependency chain from 10 crates
(`oag-game -> oag-raceplay -> oag-livery -> oag-render -> oag-fx -> oag-mesh ->
oag-pulse -> oag-assets -> oag-texture -> oag-formats`) to 8
(`oag-game -> oag-raceplay -> oag-hud -> oag-ui -> oag-2048 -> oag-assets ->
oag-texture -> oag-formats`):

- `oag-mesh` no longer depends on `oag-pulse`. The PS2 texture-name rule
  (`ps2_texture_name`, `ps2_strip_build_prefix`) lives in `oag_formats::wad`
  beside `hash_name` and `oag-pulse` re-exports it; Pulse's animated-texture
  table and its two consumers (`animated_v_cycles`, `is_blink_light_texture`)
  moved into `oag_pulse::textures`.
- `oag-livery` no longer depends on `oag-render`. `build`/`plain` of the hull's
  extra pass moved to `oag_mesh::mesh::shine_pass`; `oag_render::shine` keeps
  the blend state and the per-frame coordinates.
- `oag-ui` no longer depends on `oag-gameplay`. `Button`, `Input` and
  `button_from_name` moved to `oag_core::buttons` (plain bit vocabulary, no
  floats; `oag-core` is already under both crates, so no new crate was worth
  its weight) and `oag_gameplay::input` re-exports them.

`scripts/check-dependency-rules.py` gained rule 3: `oag-fx`, `oag-gpu`,
`oag-mesh` and `oag-post` may not reach a title data package. `oag-render` is
not covered yet: it still reads `oag-pulse`'s presentation tables (`loading`,
HUD). No ADR changes; none of the cuts alters a recorded decision.

## Title-identity ratchet (2026-10-06)

`scripts/check-title-branching.py` (`just check-title-branching`, in the `just`
gate beside `check-size`) is a ratchet over a per-file `BASELINE` of
title-identity comparisons - `title.name == oag_hd::TITLE.name` and its kin - in
every crate that is not a title package. 37 sites in 20 files at landing, none
migrated. A file may drop, never rise; a new file may have none. **Complete
2026-10-06: the last site (HD's English preselect, now
`Looks::skips_language_picker`) migrated and `BASELINE` is empty.** The script
stays in the gate as the guard: any new comparison in a generic crate fails it.
The decision
behind it, and the migration order, are
[ADR-0058](adr/0058-per-title-behaviour-is-title-data-with-provenance.md): the
workspace's dependency arrows do not change.

## Title-reach ratchet (2026-10-06)

`check-title-branching` guards a generic crate *deciding by title*; the same coupling
also shows up as a generic crate *naming a title package by path* -
`oag_pulse::hud::ART`, `oag_hd::campaign::SCREEN_ENTRY` - which keeps the Cargo
edge to that title alive and makes the crate unbuildable without it.
`scripts/check-title-reach.py` (`just check-title-reach`, in the `just` gate) is a
ratchet over a per-file `BASELINE` of `oag_(pulse|pure|hd|omega|2048)::` references in
non-comment, non-test code of every crate that is not a title package. Same rules as its
sibling: a file may drop, never rise, a new file may have none, a lowered file prints a
hint. 190 references in 51 files at landing (`--by-crate` prints the per-crate sums).

The **end state** is that generic crates depend on `oag-title` only and take a
`&'static Title` (or a value off it, each per-title choice carrying its `Origin`),
and the title packages are reached from the composition root alone: the registry in
`crates/source/src/title.rs` and `oag-game`'s entry (`main.rs`, `bin/`), which the script
names as its allowed reach points. Once a crate reaches zero its title dependencies
can leave `Cargo.toml` (`check-unused-deps` then holds them out) and
`check-dependency-rules.py` can forbid the edge.

`oag-hud` was first: its one non-test reference was `pub use oag_pulse::hud::layouts`
(only a game ground-truth test read it, so the test now names `oag_pulse` itself); its
`oag-pulse` and `oag-hd` dependencies moved to `[dev-dependencies]`, used by its tests
only. Rendered output is unchanged (byte-identical captures, six titles by three modes).

## Effect handles (2026-10-06)

A race plays the disc's own `Data\Psys` effects by [`oag_title::Trigger`], not by
string. `oag_title::Effects` is an array indexed by `Trigger` (one effect per
trigger, so a wreck's three effects are three triggers), built with the
`const fn` `Effects::with` from `Effects::engine(origin)`: the engine's own
names, which Pulse measured and every other title tries by inheritance
(`Origin::InheritedFrom("Wipeout Pulse")`), plus the triggers a title answers
for itself (weapon-hit sparks, the weapon spark, the absorb burst, the wreck).
The names, and the RE evidence that sat on `oag-raceplay`'s `effect_names.rs`
constants, live in `oag_title::engine_effects`.

The loader loads `craft_title.effects.names()` (each once, in `Trigger::ALL`
order, then the scenery names a circuit's own data may name) and resolves them
once into `oag_raceplay::EffectHandles`, `[Option<Arc<Effect>>; Trigger::COUNT]`.
A firing site indexes it, so a misspelt effect is a compile error and the
per-frame sites (engine flare, LeachBeam energy) no longer scan a string
list. `RACE_EFFECTS` is now derived from the titles' tables, with the same
39 names. `Trigger::ALL`/`COUNT` is a plain enum plus a length test: a macro
waits for a second enum with the same shape.

What changed on screen: nothing (72 plus 66 headless frames, six sources,
byte-identical). What changed in the load report: a title no longer loads the
four per-title triggers' effects it does not answer (HD's wreck set and
LeachBeam spark, 2048's and Omega's weapon spark, absorb, LeachBeam spark and
wreck set), because nothing fired them there; Pulse's report is unchanged
bar the HD-only weapon spark it never had.

## Shader validation at build time (2026-10-06)

`naga` otherwise runs only inside `wgpu::Device::create_shader_module`, so an
undeclared name or a type mismatch used to surface at race start.
[ADR-0059](adr/0059-render-shaders-are-wesl-compiled-to-wgsl-at-build-time.md)
moved `oag-mesh` and `oag-post` to WESL; `oag-shader-check` (a build-dependency,
never a normal one) now makes the build the first place a shader is checked.

- **A crate that owns shaders has a `build.rs` that calls it.** `link` links a
  WESL module (names unmangled, so overrides, entry points and bindings reach
  `wgpu` as written), validates the text the crate will `include_str!`, then
  writes the artifact. `check_dir("src", &[..])` walks a directory for `*.wgsl`,
  so a new shader is covered without a list to forget. These crates call it,
  and keep every shader but the screen-filter presets as WESL under `shaders/`
  (`link_each` for one artifact per module): `oag-mesh`, `oag-post`, `oag-fx`,
  `oag-render`, `oag-present`, `oag-view` and `oag-game`.
- **Shared modules** live in `crates/shader-check/shaders/` and are imported as
  the package `oag_shaders` (`import oag_shaders::fullscreen::fullscreen_triangle;`):
  `fullscreen`, `colour`, `bytes`, `target`, `scene_vertex`, `quad`. `link`
  mounts that directory with a `wesl` `Router` beside the crate's own
  `shaders/`, and emits `rerun-if-changed` for it, since `wesl` reports only the
  crate's own files (an edit to a shared module was checked to rebuild
  `oag-post`). Items are `public`. Only bodies equal in meaning are shared; the
  list of what was kept apart, and why, is in ADR-0060.
- **Stable output.** `link` stable-sorts the declarations before writing,
  because `wesl` printed them in a different order each build; two builds of an
  unchanged shader are now `cmp`-equal.
- **The check is what `wgpu` does**: `naga`'s WGSL front end, every
  `ValidationFlags` on, and no optional capability. The renderer requests none
  (its `required_features` are texture compression and timestamps), so a shader
  that needs one fails the build as it would fail the device. Revisit if a
  shader ever needs `enable f16` or push constants.
- **Not every `.wgsl` is a module.** `oag-post`'s `screen.wgsl` is a prelude and
  `assets/shaders/screen/*.wgsl` are preset bodies; neither parses alone (and
  stay plain WGSL, ADR-0060, because they are concatenated and loaded at runtime). They
  are validated by `Preset::validate` at runtime and by `oag-game`'s screen
  tests, and `oag-post`'s `build.rs` names `screen.wgsl` in its skip list.
  A skipped file is named with its reason there, never silently.
- **A failure names where it is.** The message is `naga`'s own, then the
  generated line with three lines of context. For a WESL artifact it adds the
  declaration the line sits in, the module that declaration came from (via
  `wesl`'s source map, which stays populated under `ManglerKind::None`), the
  declaration's line in that module and the first line of it that spells the
  text `naga` pointed at. The artifact is printed from the syntax tree, so a
  line inside a declaration does not carry over exactly; the last of those is a
  search, not a mapping. WESL syntax errors still come from `wesl` itself, with
  module and line.
- **Cost**: a clean `cargo build -p oag-mesh -p oag-post` 62 s before and 41 s
  after on a loaded machine (the difference is noise; `naga` is already in the
  graph), and a build-script re-run is about 2 s either way.
