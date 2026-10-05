# The LeachBeam against the original: what still differs after the 2026-09-23 rebuild

2026-09-23. The Pulse ribbon was rebuilt from a full re-read of
`LeachBeam_Advance`/`BuildStrip`/`InitLocked`/`SubmitStrip`. The firing
craft's hull overlay was wired, and the result was compared against a PPSSPP
capture of a real locked beam. Evidence and cadences are in
`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-09-23:
the ribbon re-read, and measured in play". Side by side, the arc now reads as
the original's jagged lightning, and the shooter's hull lights and pulses.

## Open

- **The track-tube bend is measured, and the port stands.** 2026-10-01: six
  PPSSPP captures at Talon's Junction (585 chain transitions, all three plane
  tests, floor-then-wall in one call) reproduce `LeachBeam_KeepInTrack`/
  `ReaimChain` as ported, the far-end hook included, to `1e-4`; the "corrected"
  reading (point kept, step from it) misses by 1.3 to 81 units, a thousand times
  the tolerance. `KeepInTrack`/`ReaimChain` are 92 on the page. No captured table
  is committed (`legal.md`); a `data/`-backed `#[ignore]` test that replays a
  capture is possible if the lead wants one. See `cannon-quake-leachbeam.md`, "2026-10-01: the
  tube against PPSSPP".
- **`oag_raceplay::Spline::tube_frame` does not interpolate; the original's
  locator does.** For the `pulse-shine` lane (that file is theirs): the located
  frame moves linearly with the query inside a segment (query moved `0.65`/
  `1.29`, same nearest row: `pos` moved `0.42`/`0.85`, left half width `0.22`/
  `0.45`). What it interpolates between is not recovered (lerping our rows
  leaves `0.19` units of mostly lateral residual at any row spacing from 1 to
  8). With our nearest-row locator the whole chain still lands within `0.25`-
  `0.54` units of the original's, so this is invisible; the doc comment on
  `tube_frame` ("chosen, not measured") should say it is now measured as
  *different*, and that the `100.0` radius is still unexercised (no query came
  within 80 units of failing it, no all-zero record in 585 calls).
- **The `100.0` search radius is unmeasured.** It needs a query more than 100
  from every spline sample held for a frame. Talon's folds back on itself, so
  a lateral pin from the corner lands inside the radius; a shooter written inside
  `Weapons_DispatchFire` keeps its body but the first chain starts from its
  previous pose. A track with a gap, or Pulse's Metropia with the player
  lifted on the Y axis, would do.
- **A fired beam disconnects on its first update in most captures** (five of
  six), then lingers `0.5` s: the cause was not isolated. The disconnect test is
  read (distance over `range 250`, target invulnerable or shielded, the owner's
  weapon record `+0x120` above zero, or either craft's `entity+0x8c` not `1`),
  and a finished race (`+0x8c` of `2`/`6`) makes every fire die at once.
- ~~**Our bloom spreads much further than PPSSPP's around the lit hull.**~~ **Closed
  2026-10-01 (`pulse-hull-bloom`), for the absorb overlay and on the software renderer.**
  The bloom's arithmetic was right (`pulse-glow`); the hull overlay's `0xff` stamp was the
  difference. A GE list of a real absorb shows the craft's ordinary batches and the
  overlay's ten batches drawn before the shadow pass, whose full-screen stencil quad
  (`REPLACE`, reference 4, every outcome) puts the whole mask back to 4; only the glow batch
  and its own overlay (prim after the reset) keep a stamp. Completed frames of the original
  hold the hull at 4 through the window; the 2026-09-23 reading of `255` was a mid-frame
  halt. `hull_overlay::stamps_mask` now writes the mask only over a batch with a glow of its
  own: bloom mean in the hull's neighbourhood 33 before, 5 after, 5 in the original, at five
  ages, two boots; `crates/game/tests/absorb_mask_ground_truth.rs`. **The OpenGL backend of
  PPSSPP draws the white blob** ours drew before; the software renderer is the reference
  here, and no PSP hardware arbitrates (`docs/rendering/glow-mask.md`, "The hull overlay's
  mask is wiped"). **Open on it**: the LeachBeam overlay is the same routine and takes the
  same rule, but no LeachBeam frame was read; the rule is read off Assegai alone (the other
  seven hulls' glow batches are not checked); the HUD's white energy-bar flash stamps the
  original's mask and ours does not.
- ~~**The lock sight stays up on our HUD while the beam is live.**~~ **Closed
  2026-10-01 (`pulse-cull`), and the claim did not reproduce as stated**: the pickup
  is spent on the fire tick and the sight was already gone about `0.47` s later in
  a `Race::tick` test. What the original does is a different law: the LeachBeam's
  four arrowheads have their own function, `HudSight_UpdateLeachBeam`
  (`0x0881e8c8`), gated on the held weapon (`view+0x48 == 11`; the shot clears the
  slot), with no hold timer, the lock taken on the extent arriving at `6.0`, the
  whole figure spinning (`2.0` rad/s seeking, `4.0` locked), opening at `1.5x` and
  gone about `0.24` s after the shot with the beam still live, the fade in the
  colour's alpha byte. Ported (`oag_race::sight::leach`, switched on for the Pulse
  dialect by `Race::set_sight_dialect`) and replayed against 298 frames read off
  PPSSPP (error under `1e-5`). Open on it: no frame pair of the arrowheads (the
  pinned-target capture presents stale frames, so their art and size are unverified
  here), and Wipeout HD's own `Hud_UpdateLeachBeamSight` keeps the Missile's law.
  `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`, "The LeachBeam's reticle is
  its own function".
- ~~**Ready to wire: the LeachBeam locks only when its reticle has locked.**~~ **Done
  2026-10-01 (`pulse-weapon-laws`).** `Race::spend_pickup`'s LeachBeam arm filters
  `sight_target()` through `self.view.sight.locked()` for slot 0 under the Pulse law only
  (`Sight::leach_law`); Wipeout HD keeps firing off the window alone, and an opponent has no
  reticle. `race/tests/leach_beam.rs::the_pulse_leach_beam_locks_only_once_its_reticle_has`
  presses at 5 ticks (unlocked) and 40 ticks (locked) and shows HD locked at 5. Not checked
  against the original with a press during the first 0.34 s: that is read off
  `Ship_FireHeldWeapon`, not measured.
- **An orange-white flash at the target in some original frames** has no
  counterpart in ours. Candidates: the victim's damage feedback, or
  `WO_SHIP_SPARK_DAMAGE_*`. Not identified.

## Next Steps

0. A way to fire a real LeachBeam on PPSSPP (2026-10-01): writing the held slot to `10` and
   holding fire neither halts nor fires (`craft+0x85c` stays `-1`, no lock); a beam needs
   `Ship_AcquireLock` to have found a target. Try an AI craft close ahead on a straight, the
   lock reticle's seek time (0.34 s) elapsed with the pickup *received*, not written.
1. Isolate why a fake-node fire disconnects on its first update (break at
   `0x08866cf4`/`0x08866d08` and read which of the five tests set `s2`), then
   hold a beam connected for the whole `range` window and capture ours at the
   same pose through the same fire script, so the comparison can be a matched
   frame pair and not only the original's.
2. With `pulse-shine`: make `Spline::tube_frame` interpolate once the
   interpolation's keyframes are recovered (they are not control points of
   spacing 1-8 rows of our export), or leave it and say it is measured
   sub-unit.
