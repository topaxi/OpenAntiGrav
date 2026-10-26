---
status: measure
titles: [pulse]
updated: 2026-10-26
summary: "Cage flash fade/scale need one live capture; hull sample order measured but held (05 Autopilot hits the cage); sparks draw as glow, not spokes"
---

# The collision cage's leftovers

2026-10-20, `wall-contact-miss`. Pulse's collision cage (an invisible elliptical
tube along the AI track data, `Cage_ResolveCraft` `0x08831aa4`) is read, measured
live and ported: `oag_physics::cage`, `oag_gameplay::cage`, run by
`oag_raceplay` after every craft's step. Evidence:
[collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20).

## Open

- **`CageEffect_Spawn` (`0x08832b0c`) has its trigger now**: one call per cage
  contact, at the boundary point, before the resolve. Its visual
  (`cage_collision_curved.vex`, four pooled instances) is unwired, and **not ready to
  wire**: 2026-10-26 decoded the pool, the pose rows, the 5 s life and the park
  ([collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#the-cages-spark-and-sound-and-the-flashs-decode-2026-10-26-collision-cages-and-contact-sparks)),
  but not the fade, the scale's origin or what the section quad at `+0x60` is. One capture at
  a cage contact (a GE list or a frame pair on `03_Track`, the craft driven into the tube's edge) settles them.
- ~~**Cage contacts make no spark or sound in ours.**~~ **Landed 2026-10-26.** The cage's
  contact goes through the ring loop's dispatch with friction `0`: the no-damage spark from the
  nearest locator and `.COLLISIONS` inside the locator's gate, nothing for a shielded craft
  (`Race::throw_cage_sparks`; `cage_sparks_ground_truth`). A craft-to-craft contact now sparks
  both hulls the same way, live-read on `talons-boost`
  ([contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md#live-2026-10-26-a-craft-to-craft-contact-sparks-both-hulls-and-the-dispatch-has-no-distance-gate)).
  The shape of the burst against the original's spokes is open: [compare-page.md](../../docs/tools/compare-page.md).
- **The finished player keeps the cage in ours.** The original skips a craft
  whose `entity+0x8c` reads `6` (`beq` against `s1 = 6` at `0x08842364`); a single
  race's wreck in state 6 is skipped since 2026-10-20 (`oag_physics::damage::in_state_6`,
  [collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#ported)),
  the finished player (AI craft read `2`) still is not: ours has no state 6 for it.
- **The denominator** (contact-response.md's open question): Fort Gale 1341, a
  cage contact on a rolled craft, leans body-frame (share ratio `0.91` against
  the world-axis `0.82`, cosine `0.99` both). One capture, confidence 55.
- **Our hull's sample order is the mirror of the original's: measured, held (2026-10-26).** The original's
  sample 0 is lower LEFT front; the faithful order matched every sample of 4,200 captured ticks (commit
  `18cfe598f`, reverted before merge). Held because with it
  `autopilot_original_ground_truth::the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_05`
  fails: the original Autopilot meets 05's cage and resets. Likely another mirrored term the old order was
  hiding. Next lane: re-apply the order, find why 05's run reaches the cage, fix that, land both.
- **HD Chenghou Project's pull-down (2026-10-20, `hd-chenghou-jumps`): closed.** HD's
  original runs no cage. Ours now holds it off by the title
  (`CraftLaws::cage_from_track_header`), and the jumps' vy follows the original's
  ([ps3-hdfury-eu/collision-cage.md](../../docs/ghidra/functions/ps3-hdfury-eu/collision-cage.md)).
  Still open from that lane: ours flies about 2 units lower than the original from the
  takeoff on, with takeoff vy 8.8 against 13.0 at `pz -100` on jump 1. That is a ramp or
  launch residual, not the cage.
- **Our own track format** carries the bytes (`_OAG_CAGE_A`/`_OAG_CAGE_B`) but not
  the header flag, so a track file runs no cage. A manifest or node field for the
  flag is the port (v1 can be edited in place).
- **The 200-unit and 15-unit resets were never exercised live**; the up-facing
  normal one is what fires in our races (a craft under the tube's floor).

- **`wall_contact_ground_truth.rs`'s "3-5 ticks late" is its own method**: it
  tests the recorded row pose, before the frame's integration. Stepped from the
  row, our hull's contacts match the original's tick for tick (collision-cage.md).
  The test's bound and doc comment still describe a residual that is not there.
- **The four disagreeing angular ticks** (time-trial 2458, clean lap 1257, 1681,
  2724): unchanged, the cage does not fire there, not examined further.

## Next Steps

1. The cage flash: one capture at a cage contact on `03_Track` (PPSSPP software renderer; a
   write watch on a pooled object's `+0x44` shows it start, a GE list or screenshot pair shows the
   sphere), then draw `cage_collision_curved.vex` at the pose the page decodes. An afternoon.
2. Re-capture with `scripts/psp-autopilot-contacts.py --weave 100 --pulse 12` on
   other circuits for the contact-by-contact check (only Talon's Junction is
   done; its two runs are at `data/reference/wall-contact-miss/`, flattened by
   `scripts/psp-contacts-flat.py`).
3. Re-base `wall_contact_ground_truth.rs` on a stepped pose and tighten its bound
   to the zero-tick onset. An hour.
