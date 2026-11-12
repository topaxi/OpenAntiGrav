---
status: ready-to-wire
titles: [all]
updated: 2026-11-11
summary: "Born normal and jump lips answered 2026-11-11 (see the cage thread); left: HD's own projectile update unread, segment-landing speed chosen"
difficulty: medium (35)
---

# A projectile decided wall from floor by angle; the surface class was there all along

2026-09-13, [projectile-floor.md](../../docs/gameplay/projectile-floor.md) - the
page to read, not this row. Player report on HD/Fury: "in a looping or wavy
section the rockets disappear or go straight, instead of going with the floor."
Measured on eight circuits across both titles with
`crates/raceplay/tests/rocket_floor_trace.rs`, and **Pulse broke the same way** -
52 of 120 rockets on Talon's Junction flew their whole 10 s cap without meeting
anything, against 58 of 123 on HD's Anulpha Pass - so it was the shared
geometry and HD was incidental. The cause was `WALL_FACING`: `nearest_hit`
dropped any geometry hit the chord met at under ~15 degrees as "floor
clipping", which let a rocket pass *through the barrier* at a shallow angle and
fall off the world, and detonated it on any floor met steeply after a crest.
`Collision_SweepSegment` (`0x0883198c`, decompiler-level read) returns the
struck collider's `+0x6c` surface type - the `oag_physics::Surface` numbering -
and `Rocket_Update` switches on that, never on an angle. The engine's raycaster
had returned `RaycastHit::surface` the whole time under a comment saying it
did not. Fixed by branching on the class: wall or craft ends a flight, floor or
mag floor is ridden or landed on. Pulse-measured `6.0`/`3.0`/`50.0` untouched;
nothing on the title axis; the gameplay determinism reference did not move.

## Open

- ~~The born normal is `Vec3::Y`.~~ Wired 2026-10-07 as the firing craft's up
  (`projectile-floor.md`). **2026-11-11:** the original's is the track's up at the craft's located
  point, not the craft's up; **wired 2026-11-12** (`Ship::track_up`, see Done in
  [projectiles-leave-the-track-the-original-cages-them.md](projectiles-leave-the-track-the-original-cages-them.md)).
- **A segment landing keeps its speed here**; `Rocket_Update` writes
  `(next - prev) / dt` and rescales on the next probe. One tick of difference
  in the original, none here, chosen and recorded.
- ~~Rockets fired up a ramp that ends in the air still sail off it; whether the original's do is
  a PPSSPP play-test on 07's ramp.~~ **Run 2026-11-11 (`weapon-bank-follow`): the original's do
  not.** All three rockets of a volley fired up Outpost 7's first ramp ended on the collision cage
  within 0.37 s; ours fly on. The missing law and its wire are
  [projectiles-leave-the-track-the-original-cages-them.md](projectiles-leave-the-track-the-original-cages-them.md).
- **HD's own projectile update is unread.** `ps3-hdfury-eu` is imported; the
  fix is Pulse's model on HD's geometry, which the measurement supports.
- **The HD autopilot beaches on Sebenco Climb (t=1560), Chenghou Project,
  Talon's Junction, Amphiseum and Vineta K** within 90 s at VENOM, so a
  full-lap HD volley trace exists only for Sol 2 and Anulpha Pass. Not this
  thread's subject; recorded because the next person tracing HD will hit it.

- **`crates/weapons/src/projectile.rs` is at 997 lines** against the
  1,000-line ratchet. The next change to `Projectiles::advance` should move
  its body to `projectile/flight.rs` first, the way `geometry.rs` and
  `rocket.rs` were split out, rather than trimming comments to fit.

## Next Steps

1. Seed `Projectile::surface` from the firing craft's up in `launch` and the
   Missile/Plasma/Shuriken launches, regenerate the gameplay determinism
   reference in its own commit, and re-run the Amphiseum trace to see the drop
   volley ride the wall down instead of free-falling to it.
2. Read HD's projectile update in `ps3-hdfury-eu` far enough to confirm the
   probe length, the ride height and the fall term, and put anything that
   differs on `oag-hd`.

## From the HANDOVER.md index (moved 2026-09-25)

player report on HD, "rockets go straight or disappear in a wavy section", measured on eight circuits across both titles and **Pulse broke the same way** (52 of 120 rockets on Talon's Junction flew off the circuit; 58 of 123 on Anulpha Pass). `nearest_hit` dropped any geometry hit under `WALL_FACING` as floor clipping, so a rocket passed *through the barrier* at a shallow angle, and it detonated on any floor met steeply. `Collision_SweepSegment` (`0x0883198c`) returns the struck collider's surface type and `Rocket_Update` switches on it; the raycaster had carried `RaycastHit::surface` all along. Fixed 2026-09-13 by branching on the class; see [projectile-floor.md](../../docs/gameplay/projectile-floor.md). Open: the born normal is still `Vec3::Y` where the original seeds it from the craft, and HD's own update is unread.
