---
status: ready-to-wire
titles: [all]
updated: 2026-10-29
summary: "Trails, particles, reticle, flaps, swell, flare axis, mist landed 2026-10-27; Deck CPU re-measured, no rise; open: a 240 Hz look, extrapolation"
---

# Drawing between ticks: what is still stepped

2026-10-13. The window draws a race between its last two 60 Hz ticks since
this date, so craft, camera, projectiles, the ghost, trail heads and the
animation clock move on every frame at 144 or 240 Hz, and captures still draw
each tick byte for byte. How it works, what is blended, the snaps and the
evidence are on [`docs/rendering/between-ticks.md`](../../docs/rendering/between-ticks.md);
motion blur measures per drawn frame now, see
[`docs/rendering/motion-blur.md`](../../docs/rendering/motion-blur.md)
("Blur length"). This thread is only what is left.

## Open

- **Enhanced weather splashes** (`weather-enhanced` branch, ours) are placed and aged per tick, so
  the ring and droplets step at 60 Hz like the particles they come from; their motion is
  analytic (age and gravity), so drawing at `age + draw_alpha / 60` is a few lines when wanted.

Landed 2026-10-27 (`between-ticks` lane; how each works is on
`docs/rendering/between-ticks.md`): trail ribbon bodies at fractional ages
(old item 1), particles on the GPU (old item 2), the HD sprite flare axis,
airbrake flaps and shield swell (old item 3; the cannon muzzle flash stays a
stepped flash), the lock-on reticle moved to its target as drawn (old item 4).
Captures at alpha 1 hash identically before and after (seven fixed races,
Pulse PSP and HD Anulpha Pass, rockets and cannon included).

1. ~~The Deck re-measure~~ **Closed 2026-10-27 (`deck-between-ticks-measure`): no rise.** HD Anulpha
   Pass, moving race, unlocked, four interleaved runs each of `e9baf8c42` (`85e7f2b5d^1`) and
   `1a7e06314`: `total` 7.34 -> 7.42 ms, `scene` 2.80 -> 2.82, `submit` 3.43 -> 3.42, CPU row
   7.36 -> 7.45; the old build's own spread is 0.23 ms on `total`. Table in
   `docs/rendering/frame-thread-cpu.md`, "Between-ticks drawing on the Deck".
2. **Seen only headlessly.** The proof is unit tests and `--draw-alpha`
   sequences; nobody has watched a windowed race on a 144 or 240 Hz panel
   since. A player's look at one (`just play`, frame limit 240), chase view
   and cockpit, is the check a member's Xvfb at 5-7 fps cannot do. Ten
   minutes, maintainer.
3. **The mist overlay landed 2026-10-27** (`oag_fx::mist::Layers`, blended with a
   snap on a re-roll; `docs/rendering/between-ticks.md`, "The mist overlay").
4. **What still steps inside the blended items, by choice**: a particle's
   size, colour, sprite frame and roll; the weather's rain and snow (field
   coordinates placed with the drawn camera); the reticle's own chase while
   it seeks (it steps per tick; a locked reticle sits on the drawn target).
5. **Optional, off by default: extrapolate the local craft and chase camera**
   (maintainer, 2026-10-13). Interpolation draws up to one tick (16.7 ms)
   late; drawing the player's own craft and the chase camera *past* the
   current tick by the same alpha wins that back. It overshoots on contact, so
   a tick with a wall hit (`Evaluated::wall.impact` in `Race::tick`) or a
   craft-to-craft contact must not extrapolate: the snap keys in
   `between_ticks` gain a contact flag for those two poses. Remote craft,
   projectiles and everything else stay interpolated. **Convention:** in this
   code alpha `1.0` is the current tick, where captures, pauses and parked
   races draw; the maintainer's "capture at alpha 0" is that same tick.
   Extrapolation would draw at `t = 1 + alpha` along `previous -> current`
   (`blend_transform` with `t > 1`), so a capture still draws `t = 1`.
   **Pairs with the swapchain-acquire hoist**, which landed 2026-10-13
   (`vk-amd-anti-lag-is-the-next-latency-lever.md` in this directory): the
   frame acquires its image before it reads the clock, so the alpha an
   extrapolation would draw at already describes the moment after the
   acquire's wait. The particle shift, trail ages and reticle offset would
   need the same `t > 1` treatment.

## Cost on a CPU-bound target (2026-10-27, the Steam Deck)

The Deck is CPU bound (`docs/rendering/frame-thread-cpu.md`, "On the Steam Deck": wgpu's bookkeeping in
`submit`, and the frames that also run a 60 Hz tick are the slow ones at 90 fps), so every item here must
add no measurable CPU per frame. What exists runs on the CPU but per object, not per vertex: one blend per
camera, craft, projectile, ghost, trail head and clock, a few dozen in all, into matrices that are uploaded
every frame anyway, so the interpolation changes which numbers go up, not how many. The `original` shadow
tier's floor rays were the one per-vertex case and already run per tick with a lerp between.

The open items, by where they should run (estimates, not measured):

- Reticle (item 4), flare axis, muzzle flash, flaps, shield swell (item 3): CPU, a handful of blends; negligible.
- Trail rings (item 1): CPU; the ribbon's vertices are already rebuilt every frame for its head, only the ages change.
- Particles (item 2): **GPU**. Keep a velocity per particle and pass one alpha uniform; the vertex shader draws
  at `position + velocity * (alpha - 1) * dt`. Moving each particle on the CPU and re-uploading per frame is
  the one way this lane could cost real time on the Deck: do not build it that way.
- Extrapolation (item 6): the same blend with `t > 1`; negligible.

Acceptance for any of it: a before/after `frame-phases` total on the Deck (HD Anulpha Pass, moving race, the
recipe in `the-decks-60-fps-is-anulpha-pass-submit-cost.md` in this directory) with no measurable rise.

## Next Steps

1. Watch a windowed race at the 240 limit on a high-refresh panel (item 2).
   Ten minutes, maintainer.
2. Local-craft and chase-camera extrapolation (item 5), behind an off-by-default
   setting, with a test that a contact tick never extrapolates. Half a day to
   a day.

## Survey (2026-10-13), for reference

Every per-tick pose the race scene draws from, where the tick sets it, and
what happened to it. "Landed" means it draws between ticks now.

| Item | Set per tick in | Read per frame through | Kind | Status |
| --- | --- | --- | --- | --- |
| Camera view (chase spring, cockpit, flyby override, destroy and spectator cameras, impact shake) | `Race::tick`, `tick_intro`, `tick_cosmetics` | `Race::view`, `view_unshaken`, so every camera reader | continuous | landed |
| Camera cut, view cycle, pose imposed or lifted | `camera_cuts`, `set_camera_view`, `camera_override` | snap keys | discrete | snaps |
| FOV speed widen and boost kick | body velocity, `boost_kick` | `Race::vertical_fov` | continuous | landed |
| Craft model matrix | physics | `Race::ship_model_matrix_of` | continuous | landed |
| Craft respawn, active | `sim.respawns`, `ships[].active` | snap keys | discrete | snaps |
| Blob shadow ray | body pose | `Race::shadow_placements` | continuous | landed (`body_between`) |
| Exhaust flare sprite, HD sprite position | `nozzle_of` | exhaust gather | continuous | landed (nozzle on the blended hull) |
| PSP ribbon, HD tube | samples pushed per tick | `extend_trail_vertices`, `Tube::extend_vertices` | per-tick history | landed (fractional ages) |
| HD engine lights | `nozzle_local_of` on the hull | `hd_engine_lights` | continuous | landed |
| Projectile bodies and billboards, HD rocket lights | `world.projectiles` | `*_model_matrices`, `projectile_sprites`, `hd_weapon_lights` | continuous | landed |
| Replay ghost | `ghost_pose` (now, before) | `scene/ghost.rs` | continuous | landed |
| Animation clock | `World::tick / 60` | `Race::clock_seconds` | continuous | landed |
| Motion blur's previous pose | `MotionState` | `prev_mvp` | per frame | landed (per drawn frame) |
| Lock-on reticle | `update_sight` in the tick | HUD | continuous | landed (`Race::drawn_sight`) |
| Airbrake flaps, shield swell, HD sprite axis, cannon flash | per tick | various | small scalars | landed; the flash stays stepped |
| Particles, sparks, smoke, mist, clouds | `stage.advance` | `upload_particles` and siblings | spawn discrete, motion continuous | landed on the GPU, mist blended on the CPU with a re-roll snap; clouds static |
| HUD numbers, messages, sprite frames, countdown | per tick | HUD | discrete | stepped, correct |

