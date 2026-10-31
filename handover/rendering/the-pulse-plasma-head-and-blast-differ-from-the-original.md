---
status: re-first
titles: [pulse]
updated: 2026-10-31
summary: "Plasma blast wired from the GE dumps (tail matches); the head's template playback and the onset flash are open; detonation timing fixed"
---

# The Pulse Plasma head and blast differ from the original's

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

1. ~~Blast envelope~~ **wired 2026-10-30 (`pulse-plasma-blast-wire`), `plasma.md` "the blast wired from the dumps"**:
   tracks at the age, the per-tick scale ease, the tint ramps, the world-fixed hemispheres and the
   camera-oriented halo. The tail (k96-140) area x excess went 1.95 M to 0.92 M. **Open inside it**:
   (a) the onset k76-92 reads 4.02 M against 3.69 M because our bolt detonates 10 units short and 3
   ticks early, so the eye is inside `hemisphere1` by shot 88 where the original's is at 94 (item 3);
   re-score the onset after that lands; (b) the ease targets (135.3, 112.75, 320) and the 0.025 model
   unit are fitted (conf 70-75), and the halo's end (0.65 s, chosen inside 0.55-0.717); (c) the frame
   is the nearest spline sample's, inferred from `PlasmaBlast_Construct`'s track fit; (d) the tint
   multiplies vertex colour where the original's lit pass ignores it; (e) the onset flash at shots
   k78-k82 is **not** these models (a 125 x 67 unit additive quad at craft z -28.8 and a 192-vertex
   `0x32` prim, emitter ownership open, conf 45; byte-match against `WO_PLASMA_FLASH.POB` not completed).
2. **Head and charge orb (0.24 M): the loader is not the gap.** `WO_PLASMA_HEAD` builds all five draws
   on their own textures and blends (two emitters, three sprite templates, none skipped; pinned).
   Open is the **templates' playback**: in the original their extents scale exactly with the charge
   severity (glow quad 7.0 to 21.0 units, ring 1.7 to 5.1, dark disc 1.0 to 3.3), i.e. the size
   channels read as constant, where ours samples their keys over a 60000-65535 tick life (the glow
   and ring sit at 0.007, the dark disc's looping channel decays to 0.02 in five ticks), so only the
   star shows. Which age a template samples its channels at, and what else scales its size, is the
   next read (a `ParticleSystem_DrawParticle` probe on `glow2` at two severities).
3. ~~**Detonation 3 ticks early**~~ Closed 2026-10-31 (`pulse-plasma-timing`): k76 now, per-update
   distance within 0.05 unit of the original. The cause was not the distance leg: the floor arm's
   steer, the 0.75 launch scale, a 3-update no-ride window, the Plasma's ride height 4.0 and a 2.23
   muzzle, plus one tick of f32 residue in the hold (`plasma.md`, "the detonation leg fixed").
   Left open there: what gates the first three updates (Rocket shows it too), the speed ramp's age
   being one update behind, Missile and Shuriken ride height, other crafts' muzzle node. The head's
   hold position moved 2.6 units toward the craft; item 1(a)'s onset re-score can run now.
4. ~~Craft hit~~: the second blast is wired (2026-10-30, `plasma.md`); still uncaptured on the original (needs a craft ahead on the bolt line).

## Next Steps

- Item 2 needs an `oag-re` read of the template age/size rule; then re-score the blast onset (item 3
  has landed).
- Pulse PS2 is unmeasured here; its pair is `scripts/pcsx2-pulse-pair.py`.
