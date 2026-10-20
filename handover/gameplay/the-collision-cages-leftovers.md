# The collision cage's leftovers

2026-10-20, `wall-contact-miss`. Pulse's collision cage (an invisible elliptical
tube along the AI track data, `Cage_ResolveCraft` `0x08831aa4`) is read, measured
live and ported: `oag_physics::cage`, `oag_gameplay::cage`, run by
`oag_raceplay` after every craft's step. Evidence:
[collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20).

## Open

- **`CageEffect_Spawn` (`0x08832b0c`) has its trigger now**: one call per cage
  contact, at the boundary point, before the resolve. Its visual
  (`cage_collision_curved.vex`, four pooled instances) is unwired. Render lane.
- **Cage contacts make no spark or sound in ours.** The original records them in
  the body's contact ring like any other, and `FUN_088418e0`'s ring loop
  dispatches collision effects from it without a friction test (damage is gated
  on friction, so a cage contact costs no shield).
- **The finished player keeps the cage in ours.** The original skips a craft
  whose `entity+0x8c` reads `6` (`beq` against `s1 = 6` at `0x08842364`); a single
  race's wreck in state 6 is skipped since 2026-10-20 (`oag_physics::damage::in_state_6`,
  [collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#ported)),
  the finished player (AI craft read `2`) still is not: ours has no state 6 for it.
- **The denominator** (contact-response.md's open question): Fort Gale 1341, a
  cage contact on a rolled craft, leans body-frame (share ratio `0.91` against
  the world-axis `0.82`, cosine `0.99` both). One capture, confidence 55.
- **Our hull's sample order is the mirror of the original's.** The original's
  sample 0 is lower *left* front (`body.right()` is `-row0`), ours lower right
  front. The order decides the sequence contacts resolve in. One-law change,
  measured against the weave capture below.
- **HD's `04_chenghou_project`** runs the cage, inherited from Pulse: the one HD
  circuit whose track header sets the flag, with bytes on every point. HD's own
  code is not read; the `hd-wall-resolver` lane can check it.
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

1. The hull sample order: swap to the original's order in
   `oag_physics::wall::hull_sample_points` and re-run the contact-by-contact
   sweep (`collision_cage_ground_truth::the_originals_contacts_against_our_mesh`)
   and the wall-angular test. An hour, plus gates.
2. Re-capture with `scripts/psp-autopilot-contacts.py --weave 100 --pulse 12` on
   other circuits for the contact-by-contact check (only Talon's Junction is
   done; its two runs are at `data/reference/wall-contact-miss/`, flattened by
   `scripts/psp-contacts-flat.py`).
3. Re-base `wall_contact_ground_truth.rs` on a stepped pose and tighten its bound
   to the zero-tick onset. An hour.
