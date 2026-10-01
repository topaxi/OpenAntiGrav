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
  reading misses by 10-160 units. `KeepInTrack`/`ReaimChain` are 92 on the
  page, and `tube::tests::the_port_reproduces_a_chain_read_off_the_original`
  replays one captured chain. See `cannon-quake-leachbeam.md`, "2026-10-01: the
  tube against PPSSPP".
- **`oag_game::race::Spline::tube_frame` does not interpolate; the original's
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
- **Our bloom spreads much further than PPSSPP's around the lit hull.**
  With bloom off, the hull overlay and the arc are as crisp as the original's.
  With it on, a white blob covers the nose. The ribbon's own `0x28` stamp is
  not the cause: zeroing it changes nothing visible. The hull overlay's
  `0xff` stamp and our composite are the candidates. That is the
  `hull_overlay`/bloom modules' calibration, not this weapon's, and nothing
  here was tuned toward the picture.
- **The lock sight stays up on our HUD while the beam is live.** In the
  original's frames the brackets vanish once the beam fires. HUD lane, not
  read.
- **An orange-white flash at the target in some original frames** has no
  counterpart in ours. Candidates: the victim's damage feedback, or
  `WO_SHIP_SPARK_DAMAGE_*`. Not identified.

## Next Steps

1. Isolate why a fake-node fire disconnects on its first update (break at
   `0x08866cf4`/`0x08866d08` and read which of the five tests set `s2`), then
   hold a beam connected for the whole `range` window and capture ours at the
   same pose through the same fire script, so the comparison can be a matched
   frame pair and not only the original's.
2. With `pulse-shine`: make `Spline::tube_frame` interpolate once the
   interpolation's keyframes are recovered (they are not control points of
   spacing 1-8 rows of our export), or leave it and say it is measured
   sub-unit.
