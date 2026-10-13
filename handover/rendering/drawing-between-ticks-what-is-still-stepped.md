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
4. **Seen only headlessly.** The 240 Hz proof is the ground truth test and a
   `--draw-alpha` sequence; nobody has watched a windowed race on a 144 or
   240 Hz panel since. A player's look at one (`just play`, frame limit 240)
   is the check this thread cannot do from a member's Xvfb at 5-7 fps.

## Next Steps

1. Watch a windowed race at the 240 limit on a high-refresh panel, chase view
   and cockpit, and note anything that still steps. Ten minutes, maintainer.
2. Fractional-age trail rings (item 1), with a unit test that the ribbon is
   continuous in alpha across a tick boundary. Two to three hours.
3. Particle positions between ticks (item 2), if item 1's look says it is
   worth it. Half a day.
