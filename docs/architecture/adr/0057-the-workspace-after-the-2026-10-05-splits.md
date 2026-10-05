# ADR-0057: the workspace after the 2026-10-05 splits

## Status

Accepted; extends [ADR-0051](0051-the-composition-root-splits-below-itself.md)
and supersedes one placement in
[ADR-0050](0050-format-crates-split-by-format-family.md): `.pob` particles live
in `oag-pob`, not `oag-vex`.

## Context

`oag-game` was 125k non-test lines and `oag-render` 63k. Both held several
unrelated concerns behind one name, and a contributor looking for "the HUD", "the
sound effects" or "the weapons" had to know which 100k-line crate to open.
[ADR-0051](0051-the-composition-root-splits-below-itself.md) already settled that
the reason to split is legibility and dependency honesty, **not build time**
(the floor is relinking the big crates, which a split does not remove), so this
ADR makes no build-time claim either.

## Decision

Fourteen crates were extracted in one day, each by moving code with its tests
and updating every caller to import the new crate directly - **no re-exports, no
compatibility paths**. Test counts were identical before and after (5,109 default,
6,559 with data), and `just test-data` passes on the merged tree.

| Crate | From | Owns |
| --- | --- | --- |
| `oag-music` | `oag-game` | Soundtrack listing and the ATRAC3+/ATRAC9/MP3/Wwise decoders |
| `oag-sound` | `oag-game` | `Audio`, sfx, hd_mix, race music. Reads sources through a `Library` trait the host implements; takes a plain-data `RaceFrame` |
| `oag-gpu` | `oag-render` | The lowest GPU layer: perf probe, timing, target formats |
| `oag-mesh` | `oag-render` | Mesh pipeline and mesh rendering |
| `oag-post` | `oag-render` | The post-processing chain |
| `oag-fx` | `oag-render` | Particles, exhaust, mist, flash, cloud, beam |
| `oag-pob` | `oag-vex` | The `.pob` particle container; depends on `oag-formats` only |
| `oag-weapons` | `oag-gameplay` | Pickups, projectiles, disruption, slowdown; reaches a craft through the `Craft` trait |
| `oag-hud` | `oag-game` | HUD layout and draw list, and the sprite sheet |
| `oag-livery` | `oag-game` | Livery and the ship entry-name rules |
| `oag-present` | `oag-game` | Upscale, dynamic resolution scaling, perf overlay data |
| `oag-ui-screens` | `oag-ui` | Campaign, endrace, picker and the other screens; `oag-ui` stays the core |
| `oag-raceplay` | `oag-game` | The whole race (`Race`, loading, scene, weapon visuals), and `oag-headless-sim` |
| `oag-source` | `oag-game` | Title and DLC source opening, `Remix`, cache dirs |

### Layering

```
oag-core -> oag-physics -> oag-weapons -> oag-gameplay      (simulation, determinism rules apply)
oag-gpu -> oag-mesh, oag-post -> oag-fx -> oag-render       (render side, exempt from determinism)
oag-ui -> oag-ui-screens;  oag-hud;  oag-present            (front end and presentation)
oag-music -> oag-sound                                      (audio side)
all of the above -> oag-raceplay -> oag-game                (composition root)
```

The dependency rules are unchanged and still enforced by `just check-deps`:
no gameplay crate reaches render, audio, input, `winit` or `wgpu`; no crate
depends on `oag-game`. `oag-weapons` is a gameplay crate and is scanned by
`just check-determinism`. Every other new crate is classified in
`NOT_TITLE_PACKAGES`, because it reads titles rather than being one.

### Seams and how they were cut

A split that needs something still in `oag-game` has three honest options, used
in this order: move the small self-contained leaf with it, pass plain data in,
or define a small trait the host implements. Three examples now stand as
precedent: `oag_sound::library::Library`, `oag_weapons::Craft`, and
`oag_ui::screen::Model` moved down so screens depend on the core and not the
reverse. Where a piece needs `oag-game`'s `Renderer` (the HUD's GPU overlay, the
headless capture) it stays there as a thin module.

### Deliberately not split

`oag-physics`, `oag-tables`, `oag-ai`, `oag-rcs` and the title packages are each
one cohesive concern. `oag-game` keeps `main/` (the session and stage wiring), the
boot sequence, movies and capture: that is the composition root's own work.

## Consequences

- A contributor finds a concern by crate name, and a crate's `Cargo.toml` states
  what it may touch.
- **More crates is more bookkeeping**: 41 workspace members, each with a row in
  `workspace-layout.md` and `CLAUDE.md`, a classification in the dependency
  script, and often a `[profile.dev.package.*]` entry. The rows are checked by
  `just check-deps` and `just check-docs`, but they must be written.
- **Visibility widened.** About 40 items in `oag-ui` and a few in `oag-mesh`
  went from `pub(crate)` to `pub` because the extracted code uses them. Nothing
  enforces that they stay internal by convention.
- **`oag-raceplay` is itself 51k lines** and `oag-game` still 51k. This ADR
  moved the seams that were cheap to cut; the next candidates are the race's own
  `load` and `scene`, a profile/progress crate (records, unlock, ghosts,
  campaign state), and the boot and movie cluster.
- `crates/pob/src/lib.rs` is exactly 1,000 lines, so its next addition trips
  `just check-size` and needs a split first.
- No build-time improvement is claimed or expected; see ADR-0051.
