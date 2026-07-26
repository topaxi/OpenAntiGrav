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
| `oag-view` | `crates/view` | wgpu asset viewer. The first crate with a window. |

## Crates that do not exist yet

Added when their milestone opens. Empty placeholder crates are noise, and a
crate created before its shape is understood tends to get the wrong shape.

| Crate | Milestone | Purpose |
| --- | --- | --- |
| `oag-assets` | M1 | Runtime asset registry. Normalises PSP and PS2 assets into shared runtime types. |
| `oag-trace` | M3 | Trace capture and comparison against the original. |
| `oag-render` | M4 | wgpu renderer. |
| `oag-input` | M4 | Input mapping and the per-tick input snapshot. |
| `oag-physics` | M4 | Ship dynamics and collision. |
| `oag-gameplay` | M4 | The `World` struct and the tick function. |
| `oag-race` | M5 | Race rules, lap timing, positions. |
| `oag-weapons` | M5 | Pickups, projectiles, damage. |
| `oag-ai` | M5 | Opponent behaviour. |
| `oag-audio` | M5 | Mixing and playback. |
| `oag-ui` | M5 | HUD and menus. |
| `oag-replay` | M6 | Input recording and playback. |
| `oag-game` | M6 | The binary that wires it all together. |
| `oag-net` | M7 | Multiplayer. |

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
2. **No crate may depend on `oag-game`.** It is the composition root.

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
