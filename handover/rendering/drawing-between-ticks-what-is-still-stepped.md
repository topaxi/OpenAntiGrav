---
status: ready-to-wire
titles: [all]
updated: 2026-10-27
summary: "Trail ribbons, particles, reticle and per-tick scalars still step at 60 Hz; maintainer priority; particles via the GPU, no Deck CPU cost"
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

1. **The trail ribbons' bodies step per tick.** Only the newest sample of the
   PSP ribbon (`oag_fx::exhaust::Exhaust::extend_trail_vertices`) and the HD
   tube (`oag_fx::exhaust::hd::Tube::extend_vertices`) follows the drawn
   nozzle. Each tick every older sample moves one index down the ring, so its
   stretch (`trail_stretch(k)`), taper, fade and `u` step with it: a small
   shuffle along the ribbon at 60 Hz. Fix: build the ring at a fractional age
   (sample `k` at age `k - 1 + alpha`, the drawn nozzle at age 0), which
   changes the segment count by one at the tail, so check the trail
   pipeline's vertex budget (`MAX_TRAIL_VERTICES`).
2. **Particles advance per tick and draw where the tick left them**:
   `psys::Stage` (every `.pob` effect, rocket flares, sparks, wreck fire,
   damage smoke), mist, clouds. Spawns are discrete in the originals and stay
   so; continuous particle motion could draw at `position + velocity * (alpha
   - 1) * dt` if the stage kept velocities. Not started.
3. **Smaller per-tick reads in the draw**: the HD sprite flare's nozzle axis
   (`Race::hd_sprite_quad`, `-body.forward()`), the cannon muzzle flash
   (`cannon_muzzle_matrix`, a flash and so arguably stepped), the airbrake
   flap angles (`Race::airbrake_flaps`), the shield shell's swell. Each is
   a scalar or an axis a tick old; none has been seen to judder.
4. **The lock-on reticle is projected in the tick** (`Race::update_sight`
   runs from `Race::tick` and feeds a stateful tracker,
   `oag_race::sight`), so it draws where the tick left the target while the
   target and the camera are drawn between ticks. Projecting the target at
   draw time needs the tracker split from its projection.
5. **Seen only headlessly.** The 240 Hz proof is the ground truth test and a
   `--draw-alpha` sequence; nobody has watched a windowed race on a 144 or
   240 Hz panel since. A player's look at one (`just play`, frame limit 240)
   is the check this thread cannot do from a member's Xvfb at 5-7 fps.
6. **Optional, off by default: extrapolate the local craft and chase camera**
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
   acquire's wait.

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

1. Watch a windowed race at the 240 limit on a high-refresh panel, chase view
   and cockpit, and note anything that still steps. Ten minutes, maintainer.
2. Fractional-age trail rings (item 1), with a unit test that the ribbon is
   continuous in alpha across a tick boundary. Two to three hours.
3. Particle positions between ticks (item 2), in the vertex shader (see "Cost on
   a CPU-bound target"). Half a day.
4. The reticle split (item 4) and the per-tick scalars (item 3). Two to three hours together.
5. Local-craft and chase-camera extrapolation (item 6), behind an off-by-default
   setting, with a test that a contact tick never extrapolates. After the
   acquire hoist lands and an input-to-photon number exists to judge it by.
   Half a day to a day.

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
| PSP ribbon, HD tube | samples pushed per tick | `extend_trail_vertices`, `Tube::extend_vertices` | per-tick history | head landed, body open (item 1) |
| HD engine lights | `nozzle_local_of` on the hull | `hd_engine_lights` | continuous | landed |
| Projectile bodies and billboards, HD rocket lights | `world.projectiles` | `*_model_matrices`, `projectile_sprites`, `hd_weapon_lights` | continuous | landed |
| Replay ghost | `ghost_pose` (now, before) | `scene/ghost.rs` | continuous | landed |
| Animation clock | `World::tick / 60` | `Race::clock_seconds` | continuous | landed |
| Motion blur's previous pose | `MotionState` | `prev_mvp` | per frame | landed (per drawn frame) |
| Lock-on reticle | `update_sight` in the tick | HUD | continuous | open (item 4) |
| Airbrake flaps, shield swell, HD sprite axis, cannon flash | per tick | various | small scalars | open (item 3) |
| Particles, sparks, smoke, mist, clouds | `stage.advance` | `upload_particles` and siblings | spawn discrete, motion continuous | open (item 2) |
| HUD numbers, messages, sprite frames, countdown | per tick | HUD | discrete | stepped, correct |

