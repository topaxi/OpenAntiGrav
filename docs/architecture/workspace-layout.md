# Workspace layout

## Principle

**The simulation must not know that a renderer exists.** It takes an input
snapshot in and emits a state snapshot out. Everything else - drawing, audio,
windowing, file I/O - happens outside it.

This is not architectural taste. It is what makes the simulation testable
without a GPU, deterministic without a scheduler, portable to a platform with no
windowing system, and comparable against a trace from the original. Every other
rule here follows from it.

## Crates that exist

| Crate | Path | Purpose |
| --- | --- | --- |
| `oag-core` | `crates/core` | Deterministic math, fixed-timestep clock, seeded PRNG, state hashing. Depended on by everything. |
| `oag-disc` | `crates/disc` | CHD and raw ISO readers, ISO 9660 walker, platform identification. |
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
| `oag-game` | `crates/game` | The composition root. Boots the front end; see [front-end boot](frontend-boot.md). A thin binary over a library, so the boot sequence can be tested without a GPU. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-weapons` | M5 | Pickups, projectiles, damage. |
| `oag-ai` | M5 | Opponent behaviour. |
| `oag-audio` | M5 | Mixing and playback. |
| `oag-ui` | M5 | HUD and menus. |
| `oag-replay` | M7 | Input recording and playback. |
| `oag-net` | M8 | Multiplayer. |

## Dependency rules

```
                         oag-core
                            |
        +-------------------+--------------------+
        |                   |                    |
    oag-disc            oag-physics          oag-render
        |                oag-ai                  |
   oag-formats          oag-weapons          oag-audio
        |                oag-race             oag-input
    oag-assets               |                    |
        |                    |                    |
        +----------> oag-gameplay <---------------+
                            |
                        oag-game
```

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
`oag-assets` is currently only asset *access*, not the platform-normalising
registry it is meant to become, and it does not yet know about PS2 assets.

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
