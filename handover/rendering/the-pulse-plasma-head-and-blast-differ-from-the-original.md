---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-03
summary: "Plasma head drawn right; open in the sim: the bolt releases at the muzzle node (4.24), not the 2.23 hold point"
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
   (a0) the hemispheres' 3.5 drop is applied since plasma-dome-specks (whole-frame k92 79.4 to 29.5 against the original), the dark discs are `WO_PLASMA_FLASH` particles and match; (a) the onset k76-92 reads 4.02 M against 3.69 M because our bolt detonates 10 units short and 3
   ticks early, so the eye is inside `hemisphere1` by shot 88 where the original's is at 94 (item 3);
   re-score the onset after that lands; (b) the ease targets (135.3, 112.75, 320) and the 0.025 model
   unit are fitted (conf 70-75), and the halo's end (0.65 s, chosen inside 0.55-0.717); (c) the frame
   is the nearest spline sample's, inferred from `PlasmaBlast_Construct`'s track fit; (d) the tint
   multiplies vertex colour where the original's lit pass ignores it; (e) ~~the onset flash at shots
   k78-k82~~ is not these models but `WO_PLASMA_FLASH` itself (2026-11-03, `plasma.md` "the onset flash is
   `WO_PLASMA_FLASH`", conf 85): the glow quad measures 125.2 x 66.8 and the root is 192 vertices; already played,
   one shot late (the `391 + k` mapping).
2. **Head and charge orb (0.24 M)**: ~~the templates' playback~~ **wired 2026-11-03 (`pulse-plasma-head`)**,
   `plasma.md` "the head's template playback": the three sprite templates (glow quad, ring, dark disc)
   are born once at the press (severity 0) and now draw at the instance's current severity, so their
   extents follow the charge as the original's do (7 to 21, 1.7 to 5.1, 1.0 to 3.3). Head box area x excess
   168,685 to 102,300 on k20-72 (five shots). **Open inside it**: (a) ~~the root emitter's white crescent
   arcs far smaller and dimmer~~ closed 2026-11-03 for size and brightness: the arcs are local-space (flag `0x2`)
   and now ride the craft (box luminance k60 127.0 against the original's 127.0); their shapes still differ
   (straighter bars, strips built in world not view space; checked quad by quad against the k50 dump 2026-11-03, `plasma.md` "the head's arcs against the dump": the view-space build is in, the geometry and colours match, the rest is random draws, so no construction fix is open); (b) ~~the glow reads
   dimmer~~ not the glow: its colour, blend and extent equal the dump's, the ring's alpha is a random channel (0.518 dump, 0.364 ours); (c) the draw-time severity read is inferred from the extents, not probed on
   `ParticleSystem_DrawParticle` (conf 70).
3. ~~**Detonation 3 ticks early**~~ Closed 2026-10-31 (`pulse-plasma-timing`): k76 now, per-update
   distance within 0.05 unit of the original. The cause was not the distance leg: the floor arm's
   steer, the 0.75 launch scale, a 3-update no-ride window, the Plasma's ride height 4.0 and a 2.23
   muzzle, plus one tick of f32 residue in the hold (`plasma.md`, "the detonation leg fixed").
   Left open there: what gates the first three updates (Rocket shows it too), the speed ramp's age
   being one update behind, Missile and Shuriken ride height, other crafts' muzzle node. The head's
   hold position moved 2.6 units toward the craft; item 1(a)'s onset re-score can run now.
4. ~~Craft hit~~: the second blast is wired (2026-10-30, `plasma.md`); still uncaptured on the original (needs a craft ahead on the bolt line).

## Next Steps

- Item 2(a) closed for shape 2026-11-03 (`pulse-plasma-arc-shape`, `plasma.md` "the arcs' size follows the charge"):
  the arcs draw at the head's live severity (strip extent 2.5, 5.4, 7.2 at k20, 36, 44 against the dump's 2.4, 5.4, 7.5)
  and the strips are built at one depth. Still open: (i) ~~draw order and disc colour~~ closed 2026-11-03 (`pulse-plasma-core-order`, `plasma.md` "the head's draw order"):
  the arcs draw in a layer before the alpha-over disc, the colour was the order (box k60 122.5, k72 115.1 against 127.0, 111.1); (ii) box luminance overshoots 9 to 12 at k36, k44, k72;
  (iii) ~~the arcs are 1.3 to 2.7 units too long from k56 on~~ closed to 0.9 / 0.8 units 2026-11-03 (`pulse-plasma-release-arcs`, `plasma.md` "the arcs after the release": the bolt matrix's 0.75 rows scale the local-space offsets at draw time, conf 60); what is left is a 1.3 pre-release gap at k56 (single random draws) and the arcs' shape (the original's curved crescents, ours straighter bars); (iv) Pure's `Plasma_Init` is unread, so Pure keeps the old arcs.
- 2026-11-03 (`pulse-plasma-crescents-opus`, `plasma.md` "the head's matrix and node"): the "straighter bars" were mostly
  placement. The charging head sits on the craft's weapon node (`(0, 0, 5.653)` model units, 4.24 ahead of the body, read
  live: the hull's `Ship Muzzle` locator, per team) where ours used the bolt's hold point (2.23), so ours drew 2 units nearer the eye; the 0.75 rows hold through the
  charge too. Both wired, head box 103,807 to 76,048; k36/k44 overshoot 8 to 4 and 2. The arcs' Y damping is the original's
  (live pool read, `|y|/|x|` 0.75); what still differs frame by frame is the random stream. Open:
  (v) ~~**sim, `oag-weapons`**: ours' bolt trails the original's by 2.19~~ closed 2026-11-03 (`plasma-release-step`): not a
  lag. The 2.19 pairs the bolt after an update with the craft of the previous hook read; read as one instant per record the
  original is 2.22, 2.41, 2.60, 2.76, 3.81, 4.77, 5.75 from the craft and ours matches to 0.1 for seven frames
  (`plasma_timing_ground_truth`, 0.12 at the seventh). The original's head also steps back about 2 at the release. The
  ~~k68 box luminance (131.9 against 121.3) is therefore not the release position and stays open~~ closed
  2026-11-03 (`pulse-plasma-fable`, `plasma.md` "the head after the release"): it was the release position. The
  drawn head is the bolt's own matrix, launched from the node, so it never steps back; ours anchored it on the sim
  bolt 2.01 short and drew the glow 26 % too large. The view now keeps the node's lead: k68 126.7 (120.9 one tick
  later) against 117.3-122.1 over five runs, k20-72 box 76,047 to 72,351;
  (vi) ~~the release offset is the node of the frame before minus nothing: `4.24 - craft step`~~ corrected 2026-11-03
  (`pulse-plasma-fable`): the 2.22 series paired a pre-move bolt with a post-move craft; at one instant the bolt is 4.43
  ahead on its first frame, so **the release point is the `Ship Muzzle` locator itself (4.24 on Assegai), speed-independent**,
  and `MUZZLE_FORWARD` (2.23) is 2.01 short in the sim (collision and detonation point). **Open, `oag-weapons`**: a per-craft
  muzzle lead in the `World` (set from the hull by the composition root), the hash reference regenerated, and
  `plasma_timing_ground_truth`'s series re-pinned at the draw instant (4.43, 4.62, 4.81, 5.79, 6.79, 7.81, 8.84, 9.89);
  the view's `plasma_lead` then goes;
  Pure's candidate locator is class `0x394` at `(0, 0, 6.068)` on Assegai's v4 hull, unconfirmed (no name table);
  (vii) ~~k60 box luminance 117.0 against 127.0 (reference boot) and 121.1 (fresh boot)~~: five runs read 108.4 to 127.7 at
  k60 (the release frame), so 117.0 is inside the original's own spread; closed 2026-11-03.
- Pulse PS2 is unmeasured here; its pair is `scripts/pcsx2-pulse-pair.py`.
