---
status: re-first
titles: [pulse]
updated: 2026-10-30
summary: "Plasma matched pair ranked: blast envelope and head orb differ, detonation 3 ticks early, craft-hit second blast unwired; head and blast law unread"
---

's head and blast differ from the original's

2026-10-30 (`pulse-plasma-fidelity`, measure lane). Matched pairs on Talon's Junction, Time
Trial, Venom, Assegai, fire at GO + 120 (speed 106.2), original on PPSSPP software, ours at tick
`391 + k`, a no-fire control each side, three original restarts. The ranked table, method and
numbers are in `docs/ghidra/functions/psp-pulse-usa/plasma.md`, "2026-10-30: the Plasma matched
against the original". Pairs: `data/reference/pulse-plasma-pairs/`. Scoring tool:
`scripts/weapon-pair-score.py`. The craft trajectory matches to about 3 units through fire+180,
so these are effect gaps, not scene gaps.

## Open

Read 2026-10-30 (`pulse-plasma-blast-law`): `docs/ghidra/functions/psp-pulse-usa/plasma.md`, "the
blast and the head read off GE dumps of the original" has the draw lists, per-frame laws,
confidence scores and the replay evidence. What is left is wiring and two unread leaves.

1. **Blast envelope (3.7 M + 1.9 M px-grey): law read, ready to wire.** Three models, all additive
   (`0xa2`, fixed white destination), no tint or alpha state: scale curve plus UV scroll
   (`hemisphere2` u = v = 0.4987 age, `hemisphere1` v = 0.0202 - 1.2205 age, halo (0.503, -1.008)
   age), halo gone by age 0.72, hemispheres to 1.5 s. **The white-out is `hemisphere1` alone, one
   sided (back-face culled) and world-fixed: it ends when the eye passes through its wall** (dump
   k91 to k92, shot k93 to k94; cull-off replay recovers it, removing the model gives the post-cut
   frame). Wire: draw `hemisphere1` culled with a world-fixed basis at the construct anchor; keep the
   scale curve exact; stop using the camera-facing billboard for it. **Unread:** the scale curve is
   not a function of age alone (neighbouring steps on different boots), and its source is open
   (conf 60); the flash at shots k78-k80 is the `WO_PLASMA_FLASH` effect, not these models.
2. **Head and charge orb (0.24 M): law read, ready to wire.** Five draws per frame from the
   `WO_PLASMA_HEAD` pob's own embedded textures (crescent-arc atlas strip, dark-disc quad under
   source-alpha blend, ring quad, spike strip, soft glow quad) with the extents in the plasma.md
   table (about 2.3x growth k20 to k36, then slower); ours draws none of the arcs, the dark disc
   and the ring.
3. **Detonation 3 ticks early: half isolated.** Original: released fire+59 at launch speed 634.61
   km/h, **first four flight ticks at 0.75 x that (475.9 km/h)**, then pinned to the ramp
   (850 km/h class), detonating 50.4 units on at fire+76. Ours flies the first four ticks at the
   full launch speed (about 1 tick of the 3). The remaining 2 ticks need a per-tick projectile
   log on ours (position at launch and detonation); `--probe plasma_fly` is the original side.
4. Craft hit: see `the-pulse-plasma-craft-hit-spawns-two-detonations.md` (ready-to-wire, own thread).

## Next Steps

- Spawn an `oag-wire` member on items 1 and 2 (they are independent), then a small lane for item 3
  that logs our bolt per tick first.
- Pulse PS2 is unmeasured here; its pair is `scripts/pcsx2-pulse-pair.py`.
