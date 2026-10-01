# The LeachBeam against the original: what still differs after the 2026-09-23 rebuild

2026-09-23. The Pulse ribbon was rebuilt from a full re-read of
`LeachBeam_Advance`/`BuildStrip`/`InitLocked`/`SubmitStrip`. The firing
craft's hull overlay was wired, and the result was compared against a PPSSPP
capture of a real locked beam. Evidence and cadences are in
`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "2026-09-23:
the ribbon re-read, and measured in play". Side by side, the arc now reads as
the original's jagged lightning, and the shooter's hull lights and pulses.

## Open

- **The track-tube bend is built, but not measured against the original.**
  2026-09-30: `oag_render::beam::tube` ports `LeachBeam_KeepInTrack` and
  `LeachBeam_ReaimChain` as re-read from disassembly (the page's first
  reading of `ReaimChain` had lost the sign of its `f12` scale), fed by
  `Spline::tube_frame`. Headless captures of ours at tick 2690 and 2710 of a
  `--race --autopilot --give leachbeam` run show the arc bending instead of
  cutting the wall, and the far end hooks back inward (the original's
  re-aim measures its step from the point displaced the other way, so a bent
  chain ends `2 * s * dir` off the target; the port keeps that). **No PPSSPP
  capture of a bent beam exists yet**, so the hook is a reading, not a
  comparison. `AiTrack_LocatePosition`'s search radius (`100.0` taken as one)
  and whether it interpolates along a segment are unread; the port uses the
  nearest of four samples a segment.
- **Our bloom spreads much further than PPSSPP's around the lit hull.**
  2026-10-01 (`pulse-glow`): the bloom's *arithmetic* is not the cause. A numpy
  model of the four recovered passes on our own scene reproduces the original's
  bloom layer at Outpost 7's neon strip to about 6 %, and
  `crates/render/tests/bloom_gain.rs` pins the real shader to the same maths
  (`docs/rendering/glow-mask.md`, "The bloom's own arithmetic is right"). On
  that frame the composite does not spread further than the original's either.
  **What this leaves for the hull**: the `0xff` stamp of the absorb/LeachBeam
  overlay (`hull_overlay`) and the mask it leaves around the nose, which needs
  the original's own EDRAM mask and bloom layer read at an absorb frame on the
  *same hull and pose* - the same method works (own PPSSPP, software renderer,
  `trace_shot`-style read of `0x04000000` and `0x04110000`) but the pose has to
  be a stationary one, and an absorb is not. Not done.
- **The lock sight stays up on our HUD while the beam is live.** In the
  original's frames the brackets vanish once the beam fires. HUD lane, not
  read.
- **An orange-white flash at the target in some original frames** has no
  counterpart in ours. Candidates: the victim's damage feedback, or
  `WO_SHIP_SPARK_DAMAGE_*`. Not identified.

## Next Steps

1. Capture a bent beam on PPSSPP (Talon's Junction, a chord across a corner)
   and compare it with ours at the same pose: settles the far-end hook and
   the locate radius.
