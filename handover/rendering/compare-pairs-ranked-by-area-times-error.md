---
status: measure
titles: [hd, pulse, pure, 2048]
updated: 2026-11-10
summary: "Race pairs by area x error: HD gantry GO boards and the top-right RECORD band lead; 2048 tone runs dark; frame-wide PS2 and Talon's rows unexplained"
---

# Race compare pairs ranked by area x error

2026-11-09, lane `pair-impact-scout` (read-only). Question: where do our race
frames differ most from the original, weighted by how many pixels the
difference covers, so the next render lane is picked by measured impact rather
than by how ready its evidence is. Omega in-game shots and every Fort Gale shot
are out of scope; menu pairs too.

## Open

**Method, once, so a lane can re-measure.** Both sides of each race pair in the
manifests `fresh.toml`, `race-shots.toml`, `zone.toml`, `2048.toml` and
`pure.toml` (under `data/reference/compare-page/`) are blurred with a Gaussian
of sigma 7 at native size. Error per pixel is the mean absolute RGB difference
of the blurred pair, in grey levels. The frame is cut into 32 blocks across; a
block is hot when its mean error is at least 6; hot blocks are merged
8-connected. A region's px is its pixel count, err its mean error, and
impact = px x err, an estimate of the region's share of the frame's error. 283
regions over 48 pairs; the summed impact is 172 million.

Read px with the title in mind. A Pulse PSP frame is 480x272 and an HD frame
1920x1080, sixteen times the pixels. Read `frame_frac` when comparing titles.

**Checks made on this list (2026-11-09).**

- Edge bars and sizes: none of the 48 race pairs has letterbox bars on either
  side, and every pair is the same size on both sides, so no pair was resized.
  The region box `960,540` is the block grid's edge (18 blocks of 30 px), not a
  size error.
- The HD GO banner is the start gantry board: a card drawn in the scene,
  phased by the gantry clock (`docs/rendering/hd-glow-mask.md`,
  `docs/rendering/start-gantry.md`). It is layer 3, not HUD.
- The HD RECORD readout is HUD. Its digits differ on `hd-talons` (9.43.9 against
  9.43.8) and match on `hd-talons-assegai-c1` and `hd-anulpha-pass`: a state
  difference where they differ, not a render one.
- Talon's pack (`hd-race/talons-pack`, rank 1, err 42.6) **was** a state mismatch
  (2026-11-10, `hd-talons-pack-align`): the original's frame was at least 5 to 8
  ticks older than the RAM it was paired with (its HUD clocks say so), with a
  weapon effect, pause banner and a classic-hull state RPCS3 can no longer load.
  Re-made from a regenerated state, the pair's grey difference fell 47.6 to 17
  and the opponent aligns to 4 px (`docs/tools/compare-page.md`, "A moving HD
  craft"). Still open there: the player's hull is drawn about 1.3 times larger
  and lower in the original than the recorded body and camera project it. The
  hue shift (ours greener) of the old frame was not looked at again.
- Tight boxes: for 20 of the top 31 regions the pixels above 15 grey levels
  fill the whole region, so the box is the error, not one banner inside it.
  The labels below use the box's visual centre, not a measured sub-rectangle.

**Top 15 overall (excluding rank 1; rank numbers from the measure):**

2. Pulse PS2, `pulse-ps2-moa-therma-white`, 1280x720, px 707200, err 17.4, impact 12.3 M. Layer 3, camera or geometry (luma bias near zero). Low.
3. 2048, `2048-empire-climb-grid`, 960x544, px 401700, err 24.1, impact 9.67 M. Layer 3, tone: ours darker by 11 to 18 luma. Medium.
4. 2048, `2048-mall-grid`, 960x544, px 356520, err 20.6, impact 7.33 M. Layer 3, tone: ours darker by 5 to 8 luma. Medium.
5. HD, `hd-metropia`, box 180,0 1740x480, px 500400, err 13.4, impact 6.70 M. Layer 3, GO gantry board. Medium.
6. 2048, `2048-dlc-vineta-k-grid`, 960x540, px 229500, err 22.7, impact 5.22 M. Layer 3, tone: ours darker by 2 to 10 luma. Medium.
7. HD, `hd-the-amphiseum`, box 600,0 1080x600, px 277200, err 16.1, impact 4.47 M. Layer 3, GO gantry board. Medium.
8. HD, `hd-sebenco-climb`, box 0,0 1560x480, px 198000, err 13.8, impact 2.73 M. Layer 3, sky and scenery at the top edge. Low.
9. HD, `hd-sol-2`, box 840,120 660x300, px 140400, err 17.8, impact 2.50 M. Layer 3, GO gantry board. Medium.
10. HD, `hd-moa-therma`, box 0,540 840x540, px 216000, err 11.0, impact 2.38 M. Layer 3, wall and track (a lap readout sits in the corner). Low.
11. HD, `hd-modesto-heights`, box 1320,0 600x360, px 115200, err 14.4, impact 1.66 M. Layer 4, RECORD readout, top-right band. Medium.
12. HD, `hd-anulpha-pass`, box 0,0 780x300, px 82800, err 12.6, impact 1.04 M. Layer 4, lap badge. Medium.
13. HD, `hd-moa-therma`, box 1020,720 660x360, px 75600, err 13.0, impact 0.98 M. Layer 4, speed and boost gauge over a track edge. Low.
14. HD, `hd-sebenco-climb`, box 1500,0 420x480, px 93600, err 10.0, impact 0.93 M. Layer 4, RECORD readout over sky. Medium.
15. 2048, `2048-dlc-vineta-k-grid`, box 300,390 390x150, px 42300, err 21.9, impact 0.93 M. Layer 1, craft and the ring under it. Low.
16. HD, `hd-tech-de-ra`, box 660,720 900x120, px 72000, err 11.8, impact 0.85 M. Layer 1, craft hull, lower centre. Medium.

**Top 3 per layer** (from the 31 regions the visual check covered; past 31 nothing is labelled):

- Layer 1, hull and plume: HD `hd-tech-de-ra` box 660,720 (impact 0.85 M, medium); Pulse PSP `pulse-psp-race-talons-boost` box 105,90 360x182 (impact 0.80 M, medium, the boost plume; the boost bar bottom right is HUD); 2048 DLC `2048-dlc-vineta-k-grid` box 300,390 (impact 0.93 M, low).
- Layer 2, effects: none confident in the top 31. Low-confidence picks: HD `hd-tech-de-ra` box 120,780 360x180 (bright light on a pod, 0.45 M); HD `hd-talons-assegai-c1` box 720,60 240x180 (blue striped shape, 0.43 M, unresolved between effect and billboard).
- Layer 3, track, lighting, tone: 2048 `2048-empire-climb-grid` (9.67 M, tone, measured luma bias), 2048 `2048-mall-grid` (7.33 M, tone), Pulse PS2 grid (12.3 M, camera or geometry, low).
- Layer 4, HUD: HD RECORD readout `hd-modesto-heights` (1.66 M), `hd-sebenco-climb` (0.93 M, top-right), `hd-anulpha-pass` lap badge (1.04 M).

**What the list says.**

- The largest localised HD error is the start gantry's GO board: five regions
  (ranks 5, 7, 9, 17, 24) sum to about 15 M of impact. That matches the
  "gantry cards" item of 14.0 M in the HD grid-family read
  (`hd-render-gaps-ranked-by-pixel-impact.md`), where the phase is not closable.
  So this is not a new lane; it is the same item seen from the frame list.
- The 2048 grid rows are a tone difference, not a camera one: ours is darker
  across all three bands (by up to 18 luma), and the affine fit removes part of
  the error. Haze, fog and bloom are the first suspects (the 2048 fog and
  bloom threads below).
- The Pulse PS2 grid row has near-zero luma bias in every band, so its error is
  spatial. It is a camera or geometry question, not tone.
- The HD top-right RECORD band is layer 4. Its ours-brighter bias was partly
  landed on 2026-10-26; the open part is in `hd-render-gaps-ranked-by-pixel-impact.md`.
- Layer 2 (effects) has no confident region in the top 31.

**Existing threads that overlap** (none is closed by this list):
`hd-render-gaps-ranked-by-pixel-impact.md` (HD gaps by tile; gantry cards;
top-right band), `2048s-sky-turns-and-fog-draws-on-hds-curve.md` (2048 fog),
`2048-and-omega-run-no-bloom-chain.md` (2048 bloom), `2048-composite-leftovers.md`
(2048 composite), `the-race-cameras-fov-is-fitted-to-the-psps.md` (PS2 aspect and
camera), `hds-engine-light-washes-the-hull-on-boost.md` (HD hull),
`bloom-has-no-setting-and-pulse-reads-a-little-strong.md` (Pulse plume),
`hd-glow-mask.md` in `docs/rendering/` (gantry phase).

Measure a region by re-running the same blur-and-block measure on the pair
before and after a change, and quote the region's impact and px. The number to
move is the region's impact, not the frame mean.

## Next Steps

1. ~~Lead: the 2048 tone rows are the first lane: a luma-band check ... fog and bloom on and off.~~ **Done 2026-11-09 (`tone-2048-grid`)**: no frame term closes them; the bottom third is the craft, and a display gain recovers at most 8 % of impact (`2048-grid-dark-bias-is-the-craft-not-a-frame-term.md`, `exposure.md`). Rows 3, 4 and 6 are spatial error.
2. HD gantry GO board: not a new lane. Its phase stays open per `hd-render-gaps-ranked-by-pixel-impact.md`; do not chase it from this list.
3. Pulse PS2 grid (rank 2): match the camera first. If the craft's screen position moves between the pair, it is camera; if not, geometry.
4. Talon's pack (rank 1): re-check the hue shift on the matched pair before any lane; the affine fit leaves most of its error.
5. Delete this thread once the list is re-measured or its rows have landed.
