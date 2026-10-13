# The renderer does not interpolate between ticks

2026-10-13, queued by the maintainer. **Priority: high.** No work started.

Above 60 fps every frame inside one tick draws the same simulation state, so
a 144 Hz panel or the frame limiter's 240 shows craft, camera and everything
tick-driven moving in 60 Hz steps, each repeated two to four times unevenly:
judder, at any frame rate other than an exact 60. It is also the stated
prerequisite for VR (90-120 Hz), see
`stereo-3d-first-shaped-for-vr.md` in this directory.

## What the code does (read 2026-10-13)

- `crates/game/src/main/session/frame.rs` (around line 235):
  `self.clock.advance(nanos)` returns the tick count and the loop runs that
  many ticks; the frame then draws the latest state. Zero ticks means the
  previous frame's state is drawn again.
- `TickClock::interpolation_alpha` (`crates/core/src/tick.rs`) exists, and its
  doc comment says renderers use it. **It has no caller outside its own
  tests.**
- The chase camera, the exhaust and the audio cues advance per tick, inside
  `Race::tick` or the frame's tick loop, so the camera steps with the craft.
- `docs/rendering/motion-blur.md` ("Blur length keys off `race.world.tick`")
  already records the absence correctly and says what must change in motion
  blur when interpolation lands.

## Docs that claim it is there

Both are wrong today and should be corrected in the same change that lands
interpolation, or now if the work slips:

- `docs/architecture/determinism.md` (around line 74): "Rendering interpolates
  between the last two states using `TickClock::interpolation_alpha`."
- `docs/architecture/engine-lifecycle.md` ("Frame structure"): the pseudo-loop
  keeps `previous = world.clone()` and calls
  `renderer.draw(&previous, &world, alpha)`; neither happens.

## Open

1. **What to interpolate.** Draw-side poses only, never `World`: each
   drawable's transform, the chase camera's eye and target, and the per-tick
   effects that move (exhaust, trails, projectiles, beams). Keep the
   previous tick's pose beside the current one and blend by
   `interpolation_alpha`. Rotation by quaternion slerp/nlerp. Positions by
   lerp. The simulation stays untouched and no state hash moves.
2. **Discontinuities must not blend**: a respawn, a teleport, a camera cut
   (intro camera, view cycle, wreck cut to the circuit camera), a new lap's
   wrap of any looped value, a craft spawned or removed this tick. Each needs
   a "snap" flag for the tick it happens on.
3. **Latency trade.** Interpolating between the last two ticks shows state up
   to one tick (16.7 ms) late. Extrapolation avoids that but overshoots on a
   wall hit. Interpolation is the usual choice; state it as chosen, not
   measured.
4. **Motion blur** must switch from per-tick to per-frame previous transforms,
   with the shutter a fraction of the real frame delta, as
   `docs/rendering/motion-blur.md` already says.
5. **Per-tick visuals that are not poses** (sprite frame counters, HUD
   counters, particle spawns) stay stepped; that is correct, they are
   discrete in the originals too. Particles that move continuously can
   interpolate their positions later.
6. **Capture and replay paths** (`race_capture`, `--screenshot`) draw at tick
   boundaries and must keep producing identical images: alpha 0 there, or a
   test pins it.

## Next Steps

1. Pose double-buffer and alpha blend for craft and chase camera, with the
   snap flag for respawn and camera cuts. Prove a 240-limited run moves the
   player craft every frame, and that capture images are unchanged. Half a
   day to a day.
2. Extend to exhaust, trails, projectiles and beams. Half a day.
3. Motion blur to per-frame previous transforms (item 4). An hour or two.
4. Correct `determinism.md` and `engine-lifecycle.md` to describe what landed.
   Minutes.
