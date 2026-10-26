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
| `oag-bytes` | `crates/bytes` | A generic byte reading and manipulation library, endian aware |
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
| `oag-unity` | `crates/unity` | A generic Unity asset-bundle reader: UnityFS containers (LZMA, LZ4, LZ4HC), SerializedFiles v17 to v23, a typetree-driven value reader and the built-in classes. Knows no game and no title; `check-dependency-rules.py` rule 4 allowlists `oag-bytes` and nothing else, and a game-specific layer sits on top as a separate crate, [ADR-0062](adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md). Outside the determinism-bound crates. |
| `oag-schema` | `crates/schema` | The `[toml-schema]` header every TOML file of ours carries: kind and major read from its `location`, refused by name when unknown. Depends on `toml`, `serde`, `thiserror` only; its `validate` feature (the reference TOML Schema validator, a pinned git dependency) is switched on by dev-dependencies alone. Read by `oag-ui`, `oag-raceplay` and `oag-trackfile`. [ADR-0064](adr/0064-project-toml-files-declare-a-toml-schema.md). |
| `oag-trackfile` | `crates/trackfile` | This project's own custom-track format (glb plus TOML manifest) and the `Track` model every importer and exporter targets. Depends on `oag-vex` for the types, `gltf` (no `image`), `serde_json`, `serde`, `toml`. In the determinism scan; forbidden to gameplay crates. [ADR-0063](adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md). |
| `oag-tools` | `crates/tools` | Command line tools: `oag-unpack`, `oag-wad`. |
| `oag-render` | `crates/render` | The wgpu renderer's scene: shadows, PVS placement, the track-ribbon builder, the gantry, the ghost and loading overlays and cameras. Owns no window, so the viewer and the game can each keep their own. Composes `oag-fx`, `oag-mesh`, `oag-post` and `oag-gpu`; callers import those directly, there are no re-exports. |
| `oag-fx` | `crates/fx` | The renderer's visual effects: the `.pob` particle player (`psys`) and the collision-spark adapter, the exhaust, mist, clouds, the Leach beam, the screen flash, the cockpit view's hit static, the Cannon's quads and the hull overlays. Split out of `oag-render` on 2026-10-05; depends on `oag-mesh` for the vertex type and the format crates, never on `oag-render`, so the effects need no camera, shadow or PVS code (the one shared leaf, `hull_overlay::pulse`, moved with them). Nothing gameplay may depend on it. |
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
| `oag-thread` | `crates/thread` | A closure on another thread on every target: `std::thread` natively, a Web Worker on the module's shared memory in the browser (`oag_thread::web`). `Task<T>` (polled, never waited on from the page) and `lock` (spins on the page's thread, where a contended lock traps). Below the race load, the music fetch and the audio render loop; depends on nothing of this project's. |
| `oag-mobile` | `crates/mobile` | What a phone needs around the windowed game: the Android app's files directory and event loop (`android`), the iOS container's working directory (`ios`), and `MOBILE`. A library under `oag-game`, which keeps the entry points; no gameplay crate depends on it. [ADR-0065](adr/0065-one-mobile-crate-for-every-phone-os.md). |
| `oag-mod` | `crates/mod` | Mods: a folder, a `.zip` or a `.oagmod` read in place, the `oag-mod.toml` manifest (`schemas/v1/mod.tosd`), the search path, `pack` (writes zstd entries) and the built-in `oag-extras`. Knows no menu and no title; `oag-ui` and `oag-game` read its files. [ADR-0066](adr/0066-the-menu-is-the-disc-plus-mods.md). |
| `oag-log` | `crates/log` | The binaries' log sink: the terminal as `env_logger` always built it, plus one appended, timestamped file with its own filter, pruned to seven days at startup. Plain data in; knows no game. `oag-game`, `oag-view` and `oag-trace` install it, the last two through `oag_log::tool`. |
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
| `oag-omega` | `crates/omega` | Wipeout: Omega Collection's, on PS4 - HD's own `PI001` front-end plugin carried forward, off a base package plus a mandatory patch. Racing is out of scope: the patch's `data09.psarc` carries the newest front end (older layers sit in `data00` and `data08`) and no circuits at all. |
| `oag-net` | `crates/net` | Network play: the wire messages (`wire`), a transport of two channel kinds (`link::Link`, an in-process pair and a latency/loss shim), WebTransport over `web-transport-quinn` (`wt`, native only), one race setup for server and client (`race`), the authoritative host (`host`), the predicting client (`client`), and the `oag-server` binary. No gameplay crate depends on it (rule 1). [ADR-0070](adr/0070-the-network-transport-is-webtransport.md). |
| `oag-game` | `crates/game` | The composition root. Boots the front end; see [front-end boot](frontend-boot.md). A thin binary over a library, so the boot sequence can be tested without a GPU. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.
`oag-net`, reserved here for M8, was created on 2026-10-14 once the transport was
decided ([ADR-0070](adr/0070-the-network-transport-is-webtransport.md)).

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-ui` | M5 | HUD and menus. |

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

## The lean race (2026-10-14)

`oag-raceplay` has a default-on **`view`** feature. Off, the crate is the
simulation half of a race and nothing else: `load` reads what the simulation
reads, `Race::start` builds the `RaceSim` and a `RaceView` of five fields (the
results board, the slot teams, the seat, the wreck's end tick, the run
statistics), and every tick runs the way `Race::tick_sim_only` runs one
(`Race::draws()` is `cfg!(feature = "view") && !quiet`). `oag-render`, `oag-fx`,
`oag-gpu`, `oag-hud`, `oag-input`, `oag-livery`, `oag-mesh`, `oag-post`,
`oag-present`, `oag-sound`, `oag-ui`, `oag-ui-screens`, `wgpu`, `winit` and
`bytemuck` are optional and come only with `view`.

The workspace dependency on `oag-raceplay` is `default-features = false`;
`oag-game` asks for `view`. `oag-net` takes it without, and has a **`client`**
feature (`oag-raceplay/view`) that gates `oag_net::client` and the overlay's
`stats::Collector`: the game turns it on, `oag-server` builds without it, and
`oag-net`'s own tests turn it on through a dev-dependency on the crate itself.
So `cargo build -p oag-net --bin oag-server` links no renderer, window, audio or
UI crate. **Rule 6** of `just check-deps` holds that by running
`cargo tree -p oag-net -e normal` (the package alone: `cargo metadata` unifies
the workspace's features, and `oag-game` turns `view` back on, so a
workspace-wide check cannot see this build), and `just lint` ends with a
`cargo clippy -p oag-net --lib --bins`, the only gate step that compiles
`oag-raceplay` without `view`.

**How the split was made.** One tick body, not two: view-only modules are gated at
their `mod` line, the view half of a mixed function at the statement
(`#[cfg(feature = "view")] if self.draws() { .. }`), and `Setup`'s view fields
and the whole of `Loaded` beyond the setup by field. Where the simulation read a
type that lived on the render side, the type moved down, with no re-export:

| Moved | From | To | Why the simulation reads it |
| --- | --- | --- | --- |
| `Cue`, `CueEvent`, `COLLISION_COOLDOWN` | `oag_sound::sfx`, `oag_fx::sparks` | `oag_gameplay::cue` | `RaceSim::cues` and the contact cue's re-arm; what a cue plays as stays in `oag_sound::sfx::CueTables` |
| `ChaseParams`, `Target`, `anchor`, `look_at_point` and the rig's other rigid points | `oag_render::camera::chase` | `oag_race::chase_rig` | the lock-on projects through the settled chase rig (`RaceSim::lock_rig`) |
| `teams_for_slots`, `ROSTER_SALT` | `oag_livery::draw` | `oag_race::roster` | each slot's handling follows its team |
| `geometry_is_external` | `oag_mesh::mesh` | `oag_vex::vex` | decides `vex_geometry`, which gates Pulse PSP's track stats and so the finished thrust |
| the `WO Track` parse of `oag_render::track::load` | `oag_render::track` | `oag_vex::track::from_vex` | the AI line every craft flies |

One value the simulation read **off the view** moved to `RaceSim`:
`laid_from_rear`, which moves where a laid Mine or Bomb is dropped. `Sighting`'s lock grid is read without the HUD by
`Sighting::without_view(platform, art)`, the same `Space::of(platform)` and
title art `Sighting::of(hud)` reads. `Spline::tube_frame` (`oag_fx::beam`) is a
view method and is gated rather than moved.

**Measured.** A lean `oag-server` against the full game as a `--net-bot` client:
1,042 confirmed ticks at a 100 ms `--net-shim` with 0 bad hashes (Pulse,
circuit 01), a whole three-lap race at no shim with 6,240 hash checks, 0 bad, no
correction and the board equal to the server's line for line, and 948 ticks of
HD at 100 ms, 0 bad. The tick-0 fingerprint of the previous (full) server and the
lean one agree on Pulse, HD and Omega. A 12-capture `--screenshot` set (Pulse,
HD and Omega, one and two players, ticks 300 and 900) is byte-identical before
and after. Release server, stripped: 14.3 MB before, 9.3 MB after (9.2 MB
static musl); `cargo tree -p oag-net -e normal --prefix none | sort -u`: 454
lines before, 280 after; a clean release build of the server alone (no compiler
cache): 768 CPU-s before, 254 CPU-s after (152 s and 26 s wall at load averages
of 90 and 25-36). The static build is `scripts/build-server-portable.sh musl`
([networking](../networking/README.md), "Running a server on another machine").

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
| `oag-disc`, `oag-formats`, `oag-assets` - LZSS, the GS and GE texture swizzles, the `.vex` node walk, and the sector-at-a-time read under them; `oag-video` - demuxing a whole movie a packet at a time; `oag-tables` - a character-at-a-time XML walk over every row on the disc; `oag-texture` - per-texel palette and block-codec loops; `oag-vex` - the node walk over a whole circuit; `oag-pob` - every particle system on a disc parsed and its bytes claimed; `oag-rcs` - the RCSMODEL geometry and material walk; `oag-unity` - the LZMA and LZ4 block decodes and a typetree walk over every object of a Unity bundle | `oag-title`, `oag-pulse`, `oag-pure`, `oag-hd`, `oag-trace`, `oag-tools` |

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
nextest run -p oag-game --run-ignored all -E 'test(/^ai_roll_ground_truth::/)'`):

| test (tier pair, seed) | 2026-09-13 | 2026-09-17 (idle) |
| --- | --- | --- |
| skilled/novice, held-out | within 158-165s band | **88.8s** |
| skilled/novice, default | within 158-165s band | **91.5s** |
| ace/elite, held-out | within 158-165s band | **96.8s** |
| ace/elite, default | within 158-165s band | **97.9s** |
| elite/skilled, default | within 158-165s band | **99.3s** |
| elite/skilled, held-out | within 158-165s band | **99.7s** |

`spawn_heading_ground_truth` (`-E 'test(/^spawn_heading_ground_truth::/)'`):
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

### A quiet-machine run (2026-10-07)

One `OAG_REQUIRE_GAME_DATA=1 just test-data` with no other member building,
for the record: **7,126 tests, all passed, suite 373 s, slowest test 180 s**
(`ram_ground_truth`, then the `ai_roll` full-grid tests at 143-147 s), wall
376 s including the build. The load average was 34 at the start, the tail of
the `just` gate run straight before it, and 25-28 during the run, which is the
suite's own parallelism on 24 hardware threads rather than anyone else's work;
the desktop was in use. Against the 230-280 s recorded above for 5,831 tests,
the suite has grown by 1,295 tests and its tail is still the simulation
matrices, not the disc readers. The load-time lane that took this run
([Load time](load-time.md)) added one disc-backed test per change; none is in
the slowest twenty.

### Reopened archives, serial sweeps and an affected gate (2026-10-07, later)

With four members gating beside each other, the lead's full `test-data` run
(7,180 tests) measured **465 s**, over the 450 s suite ceiling. Its nextest
durations summed to **10,521 s**, which over 24 hardware threads is 438 s and
looks like a saturated machine. **It is not evidence of one.** A duration is
wall time under contention: two tests measured 8.6 s and 6.3 s alone and 26 s
and 29 s in that run. The CPU that run used was not recorded. The run after
the changes below used 85 min of user CPU over 328 s, about 0.7 of the machine.
Both kinds of fix paid off. Removing work (the HD reopen) shrank the sum, and
shortening the tail (the parallel sweeps) took the 178 s `ai_roll` tests out of
every `oag-game` selection. Three changes, none to an assertion:

- **The HD render ground truths reopened the disc per texture.** Their texture
  closure was `mesh::read_blob(spec, name)`, which opens the image and parses
  the PSARC table of contents on every call: one circuit in
  `hd_water_ground_truth` opened `hdfury-ps3-eu-dec.iso` **153,293 times**
  (`strace -e openat`, 76 s user and 63 s system time). The race never does
  this; its loader reads through `oag_assets::Archives`. A test-side cache,
  `crates/render/tests/archive_cache/`, keeps each `Container` and each entry
  read for the life of the test. The 20 HD render binaries went from **2,389 s
  of test time to 126 s** (`hd_water` alone 932 s to 15 s).
- **`ai_roll`'s six full-grid tests and `ram` raced their races in series.**
  Each grid test is 24 independent 18,000-tick races and `ram` is 40, and
  neither can be split further without changing what it asserts (a two-tier
  total, a ratio). `crates/game/tests/in_parallel/` races them on scoped
  threads and returns results in input order; the printed tables were diffed
  identical to the serial loop's. Isolated, at load 7-10: a grid test **68 s to
  7 s**, `ram` **89 s to 9 s**. This is the tail of every `oag-game` selection.
- **Members stopped running the whole suite.** `just gate-affected` and
  `just test-data-affected` (`scripts/affected.py`) test the packages a branch
  changed plus their reverse dependencies, and run every check in full. The
  selection is exactly nextest's `rdeps()` (2,904 tests for an `oag-mesh`
  change either way). `oag-core`, the simulation crates and the build inputs
  select the full suite. The lead runs the full gate once per merge batch.

| run | tests | suite | slowest | test-time sum | load |
| --- | --- | --- | --- | --- | --- |
| lead's full `test-data`, before | 7,180 | 465 s | 221 s (`ram`) | 10,521 s | members gating |
| full `test-data`, after | 7,271 | 326 s | 133 s (`ai_fork_split`) | 7,167 s | 20-38 |
| `test-data-affected`, `oag-mesh` change, before the `ai_roll`/`ram` change | 1,187 | 271 s | 178 s (`ai_roll`) | 5,365 s | 40-70 |
| `test-data-affected`, `oag-mesh` change, after it | 1,187 | 213 s | 125 s (`ai_fork_split`) | | 25-36 |
| `gate-affected`, same change (clippy + build + 1,717 tests) | 1,717 | 3 s run, 57-109 s total | | | 11-36 |
| member gate for that change, before: full `just` + full `test-data` | 12,783 | about 65 s + 465 s | | | |
| member gate for that change, after: both affected recipes | 2,904 | 57 s + 214 s | | | 25-36 |
| docs-only change, either recipe | 0 | 0 s | | | |

Every number here is under load and is an upper bound; the isolated figures
above are the ones to compare. `.config/nextest.toml` sets no thread count,
retries or slow-timeout, so all three are nextest's defaults. A release-profile
test build was not tried, because every crate on these tests' hot path has
been at `opt-level = 2` in the dev profile since 2026-10-01.

**Why the affected gate helps a render lane less than its test count says.**
Selection is per package, and every `oag-game` test binary links all of
`oag-game`, so a change in `oag-mesh`, `oag-render` or `oag-fx` selects
`oag-game`, which with `oag-render` was 78% of `test-data`'s test time. The
saving is largest for lanes in the format, texture, UI and title crates. For
render lanes it is the HD reopen fix and the parallel sweeps that matter.

**What is left, and whose it is.** The tail is now real decode and simulation
work: ATRAC9 (`omega_wem`, 118-128 s under load), BC7 (`omega_gnf_pixels`,
85-97 s, 87% of its CPU in `oag_texture::bcn::bc7`), and the 2048 race sweeps
(`ai_fork_split`, `magstrip_wire_2048`, `zone_airbrake_flaps`, `fire_law_inherit`).
In every simulation test the brute-force `TriangleSoup::raycast_all` and
`raycast` are still the largest cost (53% of an `ai_roll` grid race); a spatial
index that returns hits in the same order is a physics lane, recommended
again, not done. `in_parallel::map` applies to any of those sweeps whose races
are independent.

## Re-profiling the 2048 and Omega tail (2026-10-12)

Full runs that day measured 357-659 s against the 450 s ceiling, the slowest test
171 s. The tail was 2048 and Omega. What each slow test *was*, measured with
`cargo nextest run -p oag-game --run-ignored only -E 'binary(...)'` (a contended
number is an upper bound; the machine was shared all day, load 13-55 here):

| test | shape | before | after |
| --- | --- | --- | --- |
| `omega_wem` loose census, data00 / data08 | one sweep over 714 / 734 files | 94 s / 100 s | 15-23 s per shard, 4 shards each |
| `fire_law_inherit` 2048 | two race loads in a `for mode` | 141 s (72 s at load 10) | 51 s per mode |
| `zone_airbrake_flaps` 2048 | two modes, each two `oag-game` runs | 157 s | `single_race` 47 s, `zone` 8 s |
| `magstrip_wire_2048` crossfade | two race loads in a `for mode` | 49 s | 48 s per mode |
| `hd_ai_craft_laws` `field_on_*` | 5 seeds in a loop, per circuit | 88 s (sebenco, under load) | 17 s per seed |
| `hd_ai_craft_laws` lone board | 12 circuits in a loop | one test | 12 tests |
| `ai_fork_split` 2048 altima | **one** 12,600-tick race | 118 s | unchanged |
| `magstrip_wire_2048` `every_craft_visits...` | **one** 5,600-tick race | 113 s | unchanged |

What was done, none of it touching an assertion:

- **`omega_wem` loose census** is a sum over files, so each archive's sweep is
  four shards (every fourth file of the archive's path order), each asserting
  its slice's counts. The eight golden `Census` values add up field by field to
  the two the single tests asserted (714 and 734 files, 665 and 685 decoded, 1
  silent, 49 refused multi-channel in each); every file is still decoded.
- **A `for mode` around a race load is a test per mode** in `fire_law_inherit`,
  `zone_airbrake_flaps` (title by mode, six tests) and the 2048 crossfade table.
  The flaps test also runs its two frames (rest and brake) side by side, since
  they are independent processes.
- **`hd_ai_craft_laws`**: the seeds of `field_on_*` print and assert nothing
  across each other, so each of the 20 is a test (`field_on_vineta_k::seed_3`);
  the lone-opponent board is a test per circuit.

**What is left is one simulation, and it is physics.** Profiled with `perf` on
`a_2048_race_fires_on_the_original_law`, which flies no tick: **95% of the
time is `TriangleSoup::raycast_all` and `raycast`** (a linear scan of every
triangle per ray), in a 2048 `SingleRace` load and `Race::start`. A 2048 `Zone`
load takes 8 s and a `SingleRace` 47 s, so the cost is in the race setup. The
two single-race tests above cannot shard: their assertion is over one race's
edges and forks. A spatial index returning hits in the same order is the lever
for every row still over 100 s; it is a determinism-bound `oag-physics` change
and was outside this lane. The same finding is recorded above for `ai_roll`.

**Full-run tail after the change** (`just test-data-affected`, 1,138 tests of
`oag-game`'s dependants, load 20-100, so the numbers are inflated): suite 289 s,
slowest 173 s (`ai_fork_split`), then `magstrip_wire_2048` 160 s and the 2048
loads at 74-92 s. No Omega, flaps or HD field test is in the top ten.

## Trimming redundant walks (2026-10-12)

A full `just test-data` on `main` that day was 8,005 tests, 832 s wall under
load and 12,908 s summed. Earlier passes had split the matrices and the tail,
so this one went after **redundancy**: tests that each open the same archive
and read, decompress or decode the same entries, since `cargo nextest` runs
every test in its own process and shares nothing between them. The rule this
became is in `CLAUDE.md`'s test-budget section.

Measured with the affected binaries alone, before (`main` at `1511e997a`) and
after, back to back under the gate lock at load 10-18 (contended, so read the
ratio rather than the seconds):

| | tests | summed | wall |
| --- | ---: | ---: | ---: |
| before | 159 | 3,079 s | 214 s |
| after | 142 | 1,458 s | 108 s |

| binary | before | after | what changed |
| --- | ---: | ---: | --- |
| `omega_gnf_ground_truth` + `_pixels` + `_mips` | 1,366 s, 27 tests | 988 s, 17 tests | one binary reads each `.gnf` once and holds it to the parse, decode and mip-chain claims; the decode is reused for the chain's base comparison |
| `hd_ai_craft_laws` | 572 s | 127 s | the 20 print-only `field_on_*` runs gated on `OAG_SWEEP` |
| `sblk_cue` + `sblk_child` | 332 s | 61 s | one shared `banks_in` that decompresses four bytes of an entry before reading it whole |
| `gxp` | 132 s, 4 tests | 36 s, 1 test | four claims over one walk of the packages |
| `gtf` | 98 s, 7 tests | 31 s, 3 tests | five whole-disc claims over one walk |
| `pvs`, `collision`, `alpha_test_reference`, `vex_ps2`, `payload_coverage`, `cloud` | 315 s | 152 s | one shared `wad_walk` that sniffs 16 bytes before decompressing |
| `text_display_type` | 78 s | 16 s | a UTF-8 head sniff before the full read |
| `pob_*`, `psys_inventory` | 114 s | 42 s | `Archive::peek` now decodes only the head of an LZSS entry |
| `novice_jump` + `novice_respawn` | 70 s | 0 s | six print-only diagnostics gated on `OAG_SWEEP` |

Two shapes account for nearly all of it:

- **Decompress everything, keep a few.** A walk over the PS2's `WADS2.WAD`
  (7,200 entries, 221 MiB of LZSS unpacking to 516 MiB) to find its 44 sound
  banks or its `.vex` files read and decompressed every entry, and
  `Archive::peek` did the same to return four bytes. A literal costs nine bits
  and `lzss::decompress` stops at the length asked for, so a head of
  `ceil(9 * n / 8) + 1` stored bytes always yields `n`; each walk now rules an
  entry out on that head and keeps its own filter on the full blob, so the set
  found is the same (every count those tests pin is unchanged).
- **N tests, one walk each.** Tests in one binary that each called the same
  `survey()` or `for_every_*` became one test over one walk, each claim kept as
  a named function or type with its doc comment and assertions as they were.

**What did not change, and why.** The `ai_roll` full-grid tests recompute four
of twelve tier-by-seed grids, but sharing them puts four grids in one test,
past the per-test ceiling under load. `replay_ground_truth` records each title
three times, once per property, which is that file's documented tail split.
The `rcsmodel_decl`, `rcsmodel_material` and `psp2_rcsmodel` repeats are about
6 s a walk, and those files keep a claim per test for legibility. A head sniff
in `vex_class` and `lane_band_census` bought nothing (their archives are stored
uncompressed) and was not kept.

## Trimming redundant walks, second pass (2026-10-12)

The first pass's leftover list, worked in order, and two library speedups the
first pass pointed at. Measured the same way: the affected binaries alone,
before (`main` at `8ade56390`) and after, back to back under the gate lock,
load 7-20 before and 15-17 after (contended, so read the ratio).

| | tests | summed | wall |
| --- | ---: | ---: | ---: |
| before | 160 | 2,809 s | 197 s |
| after | 135 | 1,823 s | 129 s |

| binary | before | after | what changed |
| --- | ---: | ---: | --- |
| `omega_gnf_ground_truth` | 1,164 s | 781 s | the BC7 decoder: mode, subset row, anchors, index roles and weight tables chosen once per block, indices from one 64-bit window; about a quarter faster per block, bit-exact |
| `hd_trackwall` + `spawn_heading` | 655 s | 458 s | `hd_trackwall`'s grid claim runs on the race `spawn_heading`'s HD sweep already loads, the same 28 circuits with the same options |
| `omega_psarc` | 136 s | 16 s | a `.gnf`'s magic read from its first block when that block is stored raw; the whole entry is read by the GNF sweep |
| `hd_ambient_role_census`, `hd_amphiseum_wall_material` | 49 s | 4 s | `oag_mesh::mesh::rcs::scene_from` opens the archive once, not once per texture |
| `psp2_sky_fog` | 106 s | 45 s | one load per circuit for the dome and the fog claim |
| `hd_engine_flare` | 85 s | 39 s | seven load-only claims share one load |
| `hd_original_ai` | 112 s | 47 s | the three Easy readings share one run |
| `gxt` + `gxt_coverage` | 123 s | 93 s | the coverage claim rides on the sweep's walk |

**The two library changes are bit-exact, proved on everything the ground truths
read.** BC7: a digest of every texel `decode()` and `decode_bc7_level()` produce
for all 15,677 Omega `.gnf` entries (every mip level, nine archives) is
identical, and the replaced decoder stays in `crates/texture/src/bcn/bc7/tests.rs`
as an oracle held equal on 1.6 million generated blocks. PSARC: one reused
decompressor per entry, inflating into the entry's own buffer, with the old call
as the fallback for any other outcome; a digest of all 130,546 entries of the 40
PSARCs this project reads (HD through both ISOs and the extracted copy, the HD
PSN download, every 2048 package, Omega) is identical. HD archives read 15-25%
faster, hashing included; 2048's barely deflated packages are unchanged.

**A fix the sweep turned up.** `ps2_texture_ground_truth` skipped every blob
`ps2_texture::header` refused, then asserted a total size `header` had already
guaranteed, so a texture whose size does not close vanished unseen. Two exist,
the "two outliers" `docs/formats/ps2-texture.md` already records; they are now
named in the test and any other one fails.

Not changed, with why, is in the open thread for the next pass:
`replay_ground_truth`, the `rcsmodel_*` repeats and the `ai_roll` grids are
the maintainer's call.

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

## The speed plans stopped being learned at every `Race::start` (2026-10-14)

`just test-data` read 968 s on `main` against the 450 s ceiling, twice, on an
idle machine, with summed test time 4,901 s the day before and 7,064 s then.
The growth was in tests that only **start** a race: `Race::start` learned this
project's driver's speed plan, hd-opponent-variant-2's per-hull adapted plans
and the route plans, whether or not anything flew them (`spawn_heading`'s PS2
test never ticks). `Race::start` now learns nothing; the head of
`Race::tick`, `Race::speed_plan` and `Race::plan_of` learn the plans
(`Race::ensure_speed_plans`) when an opponent flies this project's driver, the
player's autopilot is on, or the circuit has a fork. See
[ai.md](../gameplay/ai.md), "Which AI flies".

Measured per test binary, back to back under the gate lock, load 0.7 to 3.6
(sum of test times; slowest test):

| Binary | before | after |
| --- | --- | --- |
| `spawn_heading_ground_truth` (12) | 527 s (122 s) | 174 s (52 s) |
| `speed_plan_ground_truth` (8) | 355 s (51 s) | 100 s (18 s) |
| `stall_rescue_ground_truth` (5) | 173 s (41 s) | 153 s (37 s) |
| `pure_chain_walk_ground_truth` (4) | 118 s (103 s) | 117 s (102 s) |
| `hd_opponent_variant_ground_truth` (4) | 115 s (75 s) | 88 s (63 s) |

A test that ticks our driver still pays for the plans. One race's cost, release
build, `--ai ours`, from the log timestamps: Pulse `16_Track` the reference plan
1.1 s plus the per-hull adapted plans 0.45 s (a quarter), HD's default track
3.4 s plus 1.8 s (a third), the adapted ones on threads. The adapt was not
optimised in this pass. `pure_chain_walk` never reads a plan: its time is the
campaign walk.

## One integration-test binary per crate (2026-10-13)

`just test` spends about 2 s running tests and minutes compiling them, and most
of that compile was linking: every `tests/*.rs` file was its own test binary,
533 of them, each linking its crate's whole dependency graph (269 units for
`oag-game` alone, about 2 s each). Three changes:

1. **One binary per crate.** Every crate with more than one `tests/*.rs` sets
   `autotests = false` and names one `[[test]]`, `tests/it.rs`, which declares
   each file as a module (`mod wad_ground_truth;`). As a crate root in
   `tests/`, it finds every file where it already was, so no file moved and no
   test changed (`cargo shear` follows a plain `mod`; it does not follow a
   `#[path]`, which an earlier attempt used). A test is now
   `<file>::<test>` in binary `it` rather than `<test>` in binary `<file>`, so
   `cargo nextest run -p oag-vex wad_ground_truth::` selects one file's tests.
   A shared helper (`tests/<name>/mod.rs`) is declared once at the root, and a
   file's `mod <name>;` became `use crate::<name>;`. `oag-physics`,
   `oag-gameplay` and `oag-ai` keep a binary per file, because CI runs their
   determinism test by `--test determinism`.
2. **`oag-game` is split five ways by first letter** (`it_a_g`, `it_h_l`,
   `it_m_o`, `it_p_r`, `it_s_z`). As one binary it was 65-80 s of rustc and the
   last unit of a cold build, and an edit to any test file rebuilt all 267
   files. Five ranges cost about 70 CPU-s more in a cold build than one binary
   (shared generic code is compiled once per binary). In return they build in
   parallel, and an edit to one test file rebuilds a fifth of them.
3. **`just test` passes `--tests`**, as do `test-affected`, `check-schemas` and
   CI. Plain `cargo nextest run` also links every example and never runs them:
   283 units, 216-249 CPU-s of a cold build. `just lint`'s `--all-targets`
   still type-checks them.

A new test file is compiled only once it is listed in its crate's
`tests/it*.rs`. Each listing carries `every_test_file_is_listed`, which
fails until the file is listed exactly once.

**Measured.** Every build was cold, with no `kache` and a fresh build
directory. Each one ran under the shared test lock with `jobs = 8`, at load
6-14. A second agent's runs held the same machine between builds.

| | before | one binary per crate | plus five `oag-game` binaries and `--tests` |
| --- | --- | --- | --- |
| cold `cargo test --no-run` wall | 308-323 s | 267 s | **254 s** |
| user + sys CPU | 2,603-2,728 s | 2,161 s | **2,023 s** |
| units built | 1,412 | 918 | 638 |
| build directory | 24 GB | 7.6 GB | **3.6 GB** |
| touch an `oag-game` test file | 2-3 s | 28 s | 8 s |
| touch `oag-game/src/lib.rs` | 89-100 s | 47-48 s | **45 s** |
| touch `oag-vex/src/lib.rs` | 133-158 s | 84-97 s | **88 s** |
| touch `oag-core/src/lib.rs` | 132-152 s | 80-91 s | **83 s** |
| `cargo nextest run --workspace` | 10.3-11.0 s | | 8.9-9.4 s |

For the incremental rows the build was settled first, then one file was
touched and rebuilt, and a rebuild with no change afterwards took under 2 s.
Each range is the default run against the `--tests` run. Test counts were
compared name by name with `cargo nextest list`, both default and
`--run-ignored all`. Nothing went missing, and the only new tests are the 26
listing guards.

The one row that got worse is touching a single `oag-game` test file. Before,
that rebuilt one small binary. Now it rebuilds a fifth of the crate's tests.

**Left alone, with the reason.** The workspace builds `wesl`, `wgsl-parse`,
`wgsl-types` and a `wgsl-in`-only `naga` twice, about 110 CPU-s. One copy is
for the build scripts through `oag-shader-check`. The other is for
`oag-shader-check`'s own unit tests. `cargo build -v` shows the only difference
is `-C lto=off`: `[profile.dev] lto = "off"` reaches target units, and
`build-override` cannot set `lto`. Dropping `lto = "off"` would put thin-local
LTO back on every crate, which costs more than the duplicate. The linker is
already `rust-lld`, the 1.90+ default on x86_64 Linux.

## The collision world's broadphase (2026-10-13)

`just test-data` measured 822 s with one red, 12,535 s summed. Most of the work
this day removed redundant walks. `ai_roll` raced each interior tier twice, and
`replay` recorded each title three times. `pure_chain_walk` read 16 circuit
entries once per event, 160 events per pressing. Nine BallisticNG and Unity
tests each unpacked all 79 Workshop bundles. The PS2 track-texture claim loaded
32 circuits that `spawn_heading` already loads.

Then the profile. No `perf` is installed, and `ptrace_scope=1` stops a sampler
from attaching to a sibling process, so the test ran as a child of gdb, which
stopped it every quarter second.

A de Konstruct race put 58% of its CPU in raycasts, most of that in
`CollisionWorld`'s loop over its colliders. A circuit is about 200 colliders of
at most 32 triangles each, and every ray asked every one.

`CollisionWorld` now holds one `Bvh` over every triangle of every collider. The
items are numbered so that ascending order is ascending collider, then ascending
triangle, which is the scan's own order. A query gets exactly the triangles each
collider's own walk would have produced, and the scan's two nearest-hit
comparisons are kept apart. The answers are therefore the scan's by
construction. The printed output of twelve sim ground truths is identical before
and after.

Isolated CPU-s, load 2-20:

| test | before | collider-level BVH | one tree |
| --- | ---: | ---: | ---: |
| `ai_roll`, one seed's grid | 554 | 293 | 248 |
| `speed_plan` | 93 | 57 | 46 |
| `race_ground_truth`, lone craft | 41 | 24 | 20 |
| `stall_rescue`, Ace | 37 | 21 | 16 |

Full runs under the gate lock, `time` around nextest:

| run | tests | suite | user + sys | summed |
| --- | ---: | ---: | ---: | ---: |
| `main` at `1fcfd1255` | 8,295 | 822 s, 1 red | | 12,535 s |
| redundant walks removed | 8,276 | 738 s | 10,413 s | 11,181 s |
| plus the collider-level BVH | 8,323 | 610 s | 8,380 s | 9,069 s |
| plus one tree | 8,341 | 583 s | 8,063 s | 8,616 s |

**It is still over the 450 s ceiling, and the tail is decode now.** In summed
time: `omega_gnf` 972 s (BC7, plus the PSARC reads), `omega_wem` 429 s and
`omega_music` 194 s (ATRAC9 in `atrac9dec`). In the simulation, a gdb sample of
a 2048 field race puts most of the remaining time in **speed plan learning**,
which every race that flies this project's driver pays again.

A duration under full load runs 3-8x its isolated time on this laptop: a test
that takes 13 s alone took 99-135 s in a full run. Compare isolated CPU, or
`time`'s user + sys, not nextest durations.

## Test-data suite measurements

Moved out of `CLAUDE.md`, which keeps the rules and the ceilings.

- **Size and time.** `just test-data` ran 7,271 tests in about 330 s at load 20-38 on 2026-10-07 (465 s before the HD reopen fix and the parallel `ai_roll`/`ram` sweeps; 368 s for 5,831 tests before 2026-10-01's profile change). The table under "Reopened archives, serial sweeps and an affected gate" has the per-step numbers.
- **Why the ceilings exist.** The suite went from 3:17 to 9:47 between 2026-08-17 and 2026-09-09 with no commit that looked wrong, and one test landed on 2026-09-06 was 525 s of a 587 s wall clock. The ceilings (450 s suite, 300 s per test, `scripts/check-test-budget.py`) are measured under load, so they gate gross regressions, not drift.
- **Contended wall clock is not evidence.** The same tree measured 456 s, 402 s and 415 s at load averages of 21, 17 and 2, with four, two and one tests over the per-test ceiling and a different test each time. Two agents read a contended run as a regression on one afternoon. The two `ai_roll` grid tests that trip it measure 76-79 s in isolation, against 95-114 s when the ceiling was set.
- **Skip cache (2026-10-13).** `scripts/test-data.py` records every test binary whose tests all passed in `target/test-data-pass.json`, hashed over the binary's bytes, its package's executables, its crate's `tests/*.rs`, `data/` (size and mtime), the repository files tests read (`REPO_INPUTS`) and the environment variables crates read. The next run skips a binary whose hash still matches and lists every skip by name before and after the run (`test-data: PARTIAL RUN: SKIPPED ...`). It builds with `OAG_GIT_HASH=test-data` so the embedded commit hash does not change every binary; `oag-game` recompiles once (about 45 s) when a plain dev build such as `just play` alternates with `just test-data`.
- **Skip cache measured, 2026-10-13.** No change: 2.6 s. A string change in `oag-game` reran its 7 binaries (1,936 tests, 387 s at load 15-23). A string change in `oag-texture` relinked 31 crates but only 8 binaries changed bytes (`oag-texture::it` and `oag-game`'s 7), 435 s. A rebuild of unchanged source is byte-identical, so a change the optimiser discards skips everything. `oag-game`'s binaries are most of what any run still pays.
- **Redundant walks (2026-10-12 trim).** Three binaries read and BC7-decoded every Omega `.gnf`, two copies of one helper LZSS-decompressed all of `WADS2.WAD` to find its banks, four `gxp` tests walked the same packages, and `Archive::peek` decompressed whole entries to return four bytes. See "Trimming redundant walks" above.
- **Affected gating (2026-10-07).** `scripts/affected.py` selects the packages a branch changed since its merge base with `main` plus their dependents. Each of `oag-game`'s five test binaries links the whole crate, so a lane in `oag-mesh`, `oag-render` or `oag-fx` still selects most of the suite's time; the saving lands on lanes in the format, texture, UI and title crates.
