# Why a Quake hit feels stronger on the original: what differs, measured

**Status: measured 2026-10-26 (`quake-feel-capture`), nothing wired.** The maintainer: "the impact
just feels stronger on the original, hard to explain". An opponent's Quake reaching the player was
captured per frame on PPSSPP and compared with ours. Evidence, method, the full table and the
harness traps:
[cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md),
section "2026-10-26: why a Quake hit feels stronger on the original". The original-side tool is
`scripts/psp-quake-hit.py` (its docstring is the four traps).

**What already matches** (do not re-open): the camera shake (12.512 degrees at impact, 36 frames,
to 0.01 degree), the two hit sparks and their 0.8 s cooldown, the screen flash (alpha within
0.07), the picture at the hit (mean RGB within 6 of 255 on rows +1 to +8), the energy bar going red,
the damage (15 raw, halved with Weapons off), and the `QUAKEHIT` call (volume 1.0).
**No FOV or HUD shake exists for a Quake**: the original's one-cycle `Hud_Update` shake starts at
wall contacts and never on the hit.

## Open

Ranked by what a player sees and hears. Each gap is ready to wire except the third.

1. **The cockpit view's static (`CockpitHitFx`): ready to wire, Pulse PSP.** The original, on
   every weapon hit to the human (the same site as the shake: `Ship_Damage`'s `source == 2` block,
   outside game modes 2 and 12), runs a 0.6 s render-only node and draws it only in the internal
   view:
   - the node: `age` from 0, `p = age / 0.6`; while `p < 1` the strip width is `288 * (1 - p)`
     (both sides), the alpha is `(1 - p) * 255`, and a texture offset pair is redrawn every frame as
     `rand(0..7) / 8`; **an arm while it runs is ignored** (no restart, no stacking);
   - the draw, in the screen flash's layer: texture `Data\Tex\noisematt.mip` (64 x 64 RGBA8888 on
     the disc), blend ADD / SRC_ALPHA / ONE_MINUS_SRC_ALPHA, texture wrap repeat; **two gradient
     strips**, full height, **left** over `x` in `[0, w]` and **right** over `[480 - w, 480]` (`w` the
     strip width, in the PSP's 480 x 272 space, so scale by the frame's width over 480). Vertex
     colours: white at alpha `(1 - p) * 255` on the screen-edge pair, `0x00000000` (alpha 0) on the
     inner pair, so each strip is opaque noise at the edge and clear at its inner edge. `u` runs 0 to
     `w / 64`, `v` is static 0 to 4.25 (272 / 64), both multiplied by the texture scale `(0.125, 1.0)`
     and shifted by the offset pair (the noise is stretched about 8:1 horizontally);
   - checked against 36 live rows on two boots: `p` 0.027 at +1, 0.973 at +35, active 36 rows, the
     offsets drawn from eighths. The captured cockpit frames agree: the vertical noise energy of the
     outer 30 px columns is 41 to 43 at rows +2 and +4 against 4 before the hit, 14 to 15 at x 120 to
     150, and 9 to 10 in the middle (baseline 1.5 to 7.5). Edge-heavy, not a full-frame wash.
     Confidence 88 on the picture, 85 on the draw's exact UV law;
   Where: a render-side node beside `oag_fx::flash` (same layer, same "HUD over it" order), armed
   from `Race::arm_weapon_hit_shake`'s sites (the hit sparks' and the correction's), shown when the
   seat's camera view is internal. The existing [cockpit eye lift](quake-craft-open-leads.md) is
   the other cockpit-only piece. Needs `noisematt.mip` through the title's texture path (Pulse PSP
   has it; the PS2 pressing was not checked). Test: the node's timeline (`p` series above) and the
   strip width at three frames, with no disc. To exercise it in ours, `oag-game --force-hit TICK:0`
   fires a weapon hit on seat 0 at a tick (also the test path for item 2).
2. **`.COLLISIONS` twice per struck craft: ready to wire, Pulse PSP.** `Ship_Damage` calls
   `ShipCollisionFx_Trigger` once per locator it picks (two, or one when both draws coincide) and
   each call plays `.COLLISIONS` positionally at the locator, gated by the same 0.8 s per-locator
   cooldown that gates the spark; the live `Sound_Play` log shows `QUAKEHIT` plus two
   `.COLLISIONS` at the player's hit. Ours raises only `Cue::QuakeHit`
   (`oag_raceplay::weapons::single_instance`). The sparks already hold the cooldown and the pick
   (`oag_raceplay::hit_sparks::throw_hit_spark`): raise `Cue::Collision` beside each
   `play_riding`, on the struck craft (**chosen**: the original is at the locator, a few units
   off the hull's centre). It is the weapon-hit branch, not Quake-specific: every weapon that reaches
   the `source == 2` block gets it, the LeachBeam's kind-1 trigger included. Test: a Quake hit raises
   one `QuakeHit` and two `Collision`, none inside 0.8 s of the first.
3. **The slowdown's depth and decay: open, unattributed.** The first original reading (112.3 units/s
   at the hit to 57.0 at +76, then +1.3 a frame; thrust word 0 for 78 frames) against ours (110.5
   to 62.6, +1.0 a frame; both timers 1.28 at +1 and 0 at +79) differs, and the decel laws differ
   (`0.0077 v + 1.7e-5 v^2` against `0.0007 v + 8.1e-5 v^2`). **But the save state has the
   Autopilot running**, so the original's craft was steering and airbraking throughout and the
   reading is not a physics reference (maintainer, 2026-10-20). Next measurement: clear the bit
   (`scripts/psp-quake-hit.py --no-autopilot`) on a stretch of straight road at least 1.5 s long;
   this state's straight ends 5 frames after the hit, so use a state made on a long straight, or a
   shooter near enough that the hit lands in the first 10 rows. Ours: `--pilot-assist off`, the
   same hold, a counterfactual run with nothing fired on both sides. The ours-side reading used a
   scratch hook that was reverted and is **not pasted here**: it launched
   `Wave::launch(slot, progress - QUAKE_BACK, 1.0, &stats)` for an opponent at a fixed tick and logged
   the player's speed and timers per tick; rebuild it from those two names.

Smaller, not worth a lane alone:

- The hover dip under the slowdown's sink: ours peaks 0.3 deeper (-1.20 against -0.88), inside the
  noise of an Autopilot-flown reading; re-read with item 3.
- The flash's edge point `A` (ours `left`, 30 units from the eye at the hit; the original's reads
  46): `Quake_SampleSpan`'s two points, which is `A`, is unread.
- Every contact shakes the original's whole HUD and the FOV (`Hud_Update`, 15 frames, peaks of
  21 to 95 pixels); ours has neither. A contact gap, not a Quake one: the writer of `entity+0x80`
  is unnamed.
- Pure, HD, 2048 and Omega: whether they carry a cockpit overlay or a collision cue on a weapon hit
  was not checked.

## Next Steps

1. Item 2 first: about 20 minutes, one cue beside an existing cooldown, the loudest of the three.
2. Item 1: half a day with its test and a cockpit still against
   `scripts/psp-quake-hit.py --cockpit --frames 0,2,4,8`.
3. Item 3: a clean-straight capture, then compare the decel law before touching
   `oag_physics::slowdown`.
