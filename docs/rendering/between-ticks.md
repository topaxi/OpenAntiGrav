# Drawing between simulation ticks

The simulation steps at a fixed 60 Hz
([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)); the
window draws at whatever its frame limiter allows (240 by default, 144 on a
common panel). Until 2026-10-13 every frame inside one tick redrew the same
state, so above 60 fps the craft, the camera and everything tick-driven moved
in 60 Hz steps, each held for two to four frames unevenly: judder at any frame
rate but an exact 60. The window now draws each frame between the last two
ticks, by how far its frame clock has got into the next one.

This is an enhancement of this project's, not a recovery: the originals ran a
variable timestep and drew every frame from a fresh state.

## How it works

Code: `crates/raceplay/src/between_ticks.rs`. Tests: its unit tests in
`crates/raceplay/src/tests/between_ticks.rs` and the disc-backed
`crates/raceplay/tests/between_ticks_ground_truth.rs`.

- **A draw-side history, never `World`.** At the start of every tick path
  (`Race::tick`, `Race::tick_intro`, `Race::tick_cosmetics`) the race copies
  the poses a draw reads into `RaceView`, which is out of the state hash. So
  the previous pose is always exactly one tick old, including below 60 fps
  where one frame runs several ticks.
- **A draw alpha on the race.** `Race::set_draw_alpha` sets how far between the
  previous tick and the current one the next reads are. The window sets the
  frame clock's `TickClock::interpolation_alpha` before it draws a race and
  puts `1.0` back after the HUD; every tick path puts `1.0` back too, so
  nothing the tick computes reads a blend. `1.0` is the current tick exactly.
- **Blended at the choke points.** The accessors every continuous pose is read
  through blend by the alpha, so their readers follow without each being
  changed:

  | Accessor | What follows it |
  | --- | --- |
  | `Race::view`, `Race::view_unshaken` | every camera reader: the sky, the fog and PVS eye, opaque sorting, billboard axes, the lock-on reticle, the LOD eye, the motion blur's shake term |
  | `Race::vertical_fov` (the speed widen and the boost kick) | the projection and the LOD switch |
  | `Race::ship_model_matrix_of` | hull, shield shell and cockpit sphere, engine flame, boost plume, absorb overlays, wrecks, shine, both shadow tiers, the exhaust nozzle |
  | `Race::projectile_between` (inside the weapon model matrices) and the billboard fallback | every live projectile body |
  | `Race::ghost_draw_matrix` | the replay ghost |
  | `Race::trail_head_of` | the newest sample of the PSP ribbon and the HD tube, so the trail leaves the hull as drawn |
  | `Race::clock_seconds` | texture scroll, `Anim Transform` nodes, the flame surface's scroll |

  Positions lerp, rotations slerp along the shorter arc (glam's `Quat::slerp`),
  scales lerp; a view matrix is blended as the camera transform it inverts, so
  the eye moves in a straight line and turns about itself.
- **Stays stepped**, because it is discrete in the originals too: HUD numbers,
  message lines, sprite frames, particle spawns and the particles themselves,
  flashes and washes, the shield's flicker, the cannon muzzle flash.

## Discontinuities draw where they land

A blend is skipped, and the current tick drawn as it is, across:

- for the camera: a cut (`Race::camera_cuts`: the player's respawn, the destroy
  and spectator cameras, the flyby's shots), a view cycle, or a pose imposed or
  lifted (the flyby starting and ending);
- for a craft: its own respawn, or a change of whether it is racing;
- for a projectile: a slot that held nothing, or a different shot (another
  kind or owner, or a lifetime that went up), a tick ago;
- for the ghost and the clock: a tick path that did not step the world (the
  flyby and the cosmetic ticks after a non-line finish hold `World::tick`);
- for anything: a move of more than `SNAP_DISTANCE` (64 world units) in one
  tick, as a backstop for a teleport nothing counts;
- a paused race (the window passes `1.0` while paused), a race parked behind
  the menus (drawn at `1.0`), and a race just resumed from there
  (`Race::settle_poses` forgets its last tick).

## Captures stay byte-identical

A capture, a trace, `--screenshot` and the headless paths advance one tick per
drawn frame, and draw at alpha `1.0`; `race_capture` sets it explicitly. At
`1.0`, and on a snap, every blended reader returns the tick value untouched,
never a decomposed-and-rebuilt copy, so these paths draw exactly what they
drew before. Five captures (Pulse PSP standing and holding thrust at tick 420,
the same with motion blur `high`, the cockpit view, HD with motion blur
`medium`) hashed identically before and after the change.

`--draw-alpha A` with `--race --screenshot` draws the captured frame at alpha
`A` instead: captures at one `--ticks` and rising alphas are the frames a
fast window draws inside that tick.

## Evidence

`between_ticks_ground_truth` drives one second of frames on Pulse's first
circuit, the window's way (a `TickClock` fed a fixed frame period,
`between_ticks::frame_alpha`, the scene's accessors), with thrust held from
two seconds past the countdown:

| Frame rate | Frames | Craft and eye still, every frame at the tick | Still, between ticks | Per-frame step between ticks |
| --- | --- | --- | --- | --- |
| 240 Hz | 240 | 180 | 0 | 0.444 to 0.506 units |
| 144 Hz | 144 | 84 | 0 | 0.740 to 0.843 units |

At the tick every moving frame jumped a whole tick's travel, 1.78 to 2.02
units. Every step between ticks went forward along the craft's travel, and at
alpha `1.0` the accessor returned the tick's body position bit for bit.

## Chosen, not measured

- **Interpolation, one tick late.** Drawing between the last two ticks shows
  the state up to one tick (16.7 ms) behind the newest. Extrapolating past the
  newest tick avoids the latency but overshoots a wall hit by up to a tick.
- **Slerp, not nlerp.** A handful of matrices a frame; slerp keeps the angular
  speed even across the tick.
- **`SNAP_DISTANCE` = 64 units**: about ten times a craft's top speed per tick
  and well under any respawn or camera cut.

## Motion blur

[Motion blur](motion-blur.md) measures each drawn frame against the previous
drawn frame now, so its shutter is a fraction of the real frame delta; frames
drawn at a tick exactly (captures, a paused race) still measure one tick, as
they did. See that page's "Blur length" section.

## Open

- The PSP ribbon's and the HD tube's older samples are still placed per tick:
  each tick, every recorded sample moves one index down the ring, and its
  stretch, taper, fade and texture coordinate step with it. Only the head
  follows the drawn nozzle. Drawing the ring at a fractional age would make
  the whole ribbon continuous.
- Particles (`psys::Stage`), sparks, smoke, mist and clouds advance per tick
  and are drawn where the tick left them.
- The HD sprite flare's nozzle axis reads the body's forward at the tick, not
  the drawn hull's.
