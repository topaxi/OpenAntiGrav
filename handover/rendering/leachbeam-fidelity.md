# The LeachBeam against the original: what still differs after the 2026-09-23 rebuild

2026-09-23. The Pulse ribbon was rebuilt from a full re-read of
`LeachBeam_Advance`/`BuildStrip`/`InitLocked`/`SubmitStrip`. The firing
craft's hull overlay was wired, and the result was compared against a PPSSPP
capture of a real locked beam. Evidence and cadences are in
`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-09-23:
the ribbon re-read, and measured in play". Side by side, the arc now reads as
the original's jagged lightning, and the shooter's hull lights and pulses.

## Open

- **The track-tube bend is read, not built.** `LeachBeam_KeepInTrack`
  (`0x088734d0`) and `LeachBeam_ReaimChain` (`0x08873328`) re-aim the chain
  wherever a point leaves the track (below the road, or within `2.0` of an
  edge). On a straight nothing changes. Round a corner the original's arc
  bends and ours cuts through the wall. Buildable from `Spline`'s
  `pos`/`lateral`/`half_width_*`, which `advance_quake_visual` already reads.
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
- **`WO_LEACHBEAM_CHARGING` plays only on slot 0.** `FUN_0883f540` is the
  per-craft weapon update, so the original lights any craft holding id 10,
  and AI craft do fire LeachBeams (`fire_opponent_leach_beam`).
- **The ENERGY instance is detached at each re-spawn.** The original hands
  the old handle to `FUN_088f3298`, which was not read. Detaching lets the
  particles finish; a kill would cut them.

## Next Steps

1. Build the tube bend in `oag_render::beam::chain` from the spline, then
   re-capture a beam fired round Talon's Junction's first corner.
2. Read `FUN_088f3298` to settle detach versus kill.
3. Extend the charge effect to every slot.
