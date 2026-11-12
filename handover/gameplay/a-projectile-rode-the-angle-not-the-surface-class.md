---
status: re-first
titles: [all]
updated: 2026-11-12
summary: "Launch normal is track up on every title (landed); HD's own projectile update unread; Omega Ubermall misses more often with it"
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
- **Omega, Ubermall as Phantoms misses more often with the track's up.** Green unperturbed, but
  over `perturb::GATE` x `perturb::BOUND_OFFSETS` plus the four other `perturb::SCALES` it breaks
  its bounds in 10 of 49 runs against 3 of 49 with the craft's up (Amphiseum reversed: 11 and 8).
  The next test likely to redden on an unrelated change; if it
  reddens on an unrelated change, judge it on `GATE` the way Amphiseum is
  (`omega_original_ai_ground_truth.rs`), with its own miss rate, rather than re-picking a seed.
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

## Done

- **2026-11-12 (`hd-launch-normal-flip`): HD, 2048 and Omega launch along the located track's up.**
  `Weapons::launch_normal_is_track_up` is `measured(true)` in `oag-hd`, `oag-omega` and
  `oag-2048`; `weapon-lineage.md`'s row moved; pinned by
  `projectile_cage_ground_truth::an_hd_plasma_bolt_rides_the_located_tracks_up`. The three tests
  that blocked it were re-based on distributions
  ([projectile-launch-normal.md](../../docs/ghidra/functions/ps3-hdfury-eu/projectile-launch-normal.md),
  "Landed"): Amphiseum on `perturb::GATE`, two of five (8 of 45 misses with either normal); the
  Novice pace counts an unended race at its laps and asserts the median cannot depend on it
  (median 10); the board check runs on every race the pace tests end. The read that was here:
  - **HD, 2048 and Omega: read 2026-11-12, ready to wire, not flipped.** HD's `Rocket_Init`
    (`0x00124d90`), `Missile_Init`, `Plasma_Init`/`Plasma_Launch` and `Shuriken_Init` (`0x00159a30`)
    each store `0 - craft+0x7850`, the located track sample's down, so the carried normal is the
    located track's up, Pulse's law (confidence 86). Omega's `Plasma_Init` stores `-(craft+0x8040)`,
    the same record (80); 2048's stores `-(craft+0x6e90)` (75). Evidence and the locate chain:
    [projectile-launch-normal.md](../../docs/ghidra/functions/ps3-hdfury-eu/projectile-launch-normal.md).
    **The wire:** `Weapons::launch_normal_is_track_up` to `measured(true)` in `oag-hd`, `oag-omega`
    and `oag-2048`, the `oag_title::weapons` doc comment, and `weapon-lineage.md`'s row. **Why it is
    not landed:** with it on, the three tests below go red again, and that page's "Turning it on"
    section shows each is a chaotic divergence at a margin rather than a wrong normal:
    - `hd_eliminator_hud_ground_truth::a_novice_score_eliminator_reaches_its_target_at_about_the_originals_pace`:
      eleven of twelve seeds end, median 10 laps (window 6 to 12); seed 1 does not reach 200 in
      1,000 s. Off, seed 1 already takes 857 s.
    - `::a_finished_eliminator_leaves_a_board_ranked_on_points`: seed 1 at the default difficulty
      has not ended by 30,000 ticks.
    - `omega_original_ai_ground_truth::the_field_finishes_amphiseum_reversed`: the runs part at the
      first volley (its last rocket ends one tick early) and slot 5 respawns 4,700 ticks later with
      no hit.
    Next: decide with the maintainer whether these seeds are re-chosen or the bounds re-measured
    against the original (Vineta K's nine laps, n = 1), then land the flip with them. Do not land
    the flip by loosening a bound unmeasured.
