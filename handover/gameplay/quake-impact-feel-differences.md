# Why a Quake hit feels stronger on the original: what differs, measured

**Status: measured 2026-10-26 (`quake-feel-capture`); the two missing channels wired the same day
(`quake-feel-wire`), the slowdown's depth still open.** The maintainer: "the impact just feels
stronger on the original, hard to explain". An opponent's Quake reaching the player was captured per
frame on PPSSPP and compared with ours. Evidence, method, the full table, the harness traps and what
was wired (`.COLLISIONS` twice per struck craft, the cockpit view's `CockpitHitFx` static, the
`--force-quake TICK:SLOT` aid, per-title results):
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
sections "2026-10-26: why a Quake hit feels stronger on the original" and "Wired 2026-10-26
(quake-feel-wire lane)". The original-side tool is `scripts/psp-quake-hit.py` (its docstring is the
four traps).

**What already matches** (do not re-open): the camera shake (12.512 degrees at impact, 36 frames,
to 0.01 degree), the two hit sparks and their 0.8 s cooldown, the screen flash (alpha within
0.07), the picture at the hit (mean RGB within 6 of 255 on rows +1 to +8), the energy bar going red,
the damage (15 raw, halved with Weapons off), and the `QUAKEHIT` call (volume 1.0). Since the wiring
lane: the `.COLLISIONS` pair and the cockpit static (band metric within the original's shape and size).
**No FOV or HUD shake exists for a Quake**: the original's one-cycle `Hud_Update` shake starts at
wall contacts and never on the hit.

## Open

What is left of the gaps the capture found: the slowdown.

1. **The slowdown's depth and decay: open, unattributed.** The first original reading (112.3 units/s
   at the hit to 57.0 at +76, then +1.3 a frame; thrust word 0 for 78 frames) against ours (110.5
   to 62.6, +1.0 a frame; both timers 1.28 at +1 and 0 at +79) differs, and the decel laws differ
   (`0.0077 v + 1.7e-5 v^2` against `0.0007 v + 8.1e-5 v^2`). **But the save state has the
   Autopilot running**, so the original's craft was steering and airbraking throughout and the
   reading is not a physics reference (maintainer, 2026-10-20). Next measurement: clear the bit
   (`scripts/psp-quake-hit.py --no-autopilot`) on a stretch of straight road at least 1.5 s long;
   this state's straight ends 5 frames after the hit, so use a state made on a long straight, or a
   shooter near enough that the hit lands in the first 10 rows. Ours: `--pilot-assist off`, the
   same hold, a counterfactual run with nothing fired on both sides. Ours: `--force-quake TICK:SLOT`
   launches the rival's wave with its bump on the player (`Race::force_quake`); a shooter further
   behind needs the same launch at `progress - distance`, and `--trace-out` logs the speed and timers.

Smaller, not worth a lane alone:

- The hover dip under the slowdown's sink: ours peaks 0.3 deeper (-1.20 against -0.88), inside the
  noise of an Autopilot-flown reading; re-read with the slowdown.
- The flash's edge point `A` (ours `left`, 30 units from the eye at the hit; the original's reads
  46): `Quake_SampleSpan`'s two points, which is `A`, is unread.
- Every contact shakes the original's whole HUD and the FOV (`Hud_Update`, 15 frames, peaks of
  21 to 95 pixels); ours has neither. A contact gap, not a Quake one: the writer of `entity+0x80`
  is unnamed.
- **`.COLLISIONS` on a weapon hit outside Pulse.** Every executable names the cue (Pure once, HD
  once, reached through a data pointer at `0x008a8e34`, 2048 twice), but the weapon-hit call site is unread
  on all three, and the hit sparks it rides are unwired there too (Pure's `Ship_Damage` branch, HD's
  `Cannon_ApplyCraftHit` kind 1 spark, 2048's own). Wire the cue with each title's sparks.
- **The static outside Pulse** is settled: Pure inherits Pulse's (its disc ships the same noise), HD,
  2048 and Omega draw none. Pure's `Ship_Damage` is unread, so a read could still change Pure's rule.
- **The two choices the wiring made without a measurement**: the noise is sampled linearly, and the
  static draws before the screen flash inside their shared layer `0x4f000000` (order unread).

## Next Steps

1. The slowdown: a clean-straight capture, then compare the decel law before touching
   `oag_physics::slowdown`.
