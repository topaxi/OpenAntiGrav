---
status: measure
titles: [2048, omega, pure]
updated: 2026-11-12
summary: "Compare pairs by payoff: 2048 race pairs checked (5 re-captured, 6 caveated), talons road bar open, Pure pairs not viewable"
size: S (30)
difficulty: medium (30)
---

# Comparison pairs ranked by payoff for effort

2026-11-12, lane `compare-pairs-scout` (read-only). Question: which compare pairs have a large visible error that a
low-to-moderate change could close. Scope: the 140 pairs on `data/reference/compare-page`; Omega race shots and
Fort Gale out; menus in. Race area x error is already ranked in
`compare-pairs-ranked-by-area-times-error.md`; the 2048 craft box is in `2048-grid-dark-bias-is-the-craft-not-a-frame-term.md`.
Both are cited, not repeated.

## Open

Impact figures are estimates from box area x error. Confidence is 0 to 100.

1. **Omega campaign select: the 2048 card's art and pose** (already threaded,
   `menu-pairs-show-hd-and-omega-front-end-differences.md`, Open 1(d)). Pair `omega/menu-campaign-select`, rmse 0.369,
   the largest of the 140. Visible: ours shows a blue disc and a placeholder hex where the original shows the 2048 banner,
   wordmark and roundel on a lattice card. Impact estimate: the card is about 30 % of the frame, error high (estimate).
   Split in two: the card's banner and wordmark are wiring, **done 2026-11-12** (`omega-campaign-card-art`: the card's UV keys from its `.vex`,
   `CardSpec::keyed`; the 2048 card box mean grey 69.3 to 32.3); the turned cards' pose needs the executable's `Campaign Selection` update,
   which is reverse engineering (oag-re).

2. ~~**2048 race pairs compare a different craft: a capture fix first**~~ **done 2026-11-12** (`vita-2048-race-pairs`; method and numbers:
   [compare-page.md](../../docs/tools/compare-page.md), "Real events for the substituted 2048 pairs"). The premise was half wrong: it is **eleven** substituted
   pairs (`dlc-vineta-k` is the authored `grid0_3_1` node), and **the craft is the same** on both sides (Feisar HD, the `hdships` hull; the HD and Fury Campaigns both
   open Select Team on it), so the per-pair hull luma sign flip is the circuits' lighting, not two crafts. Every substituted circuit has an authored Race, Venom, 3-lap cell;
   five were re-captured from it by tapping the node (Talons Junction, Modesto Heights, Anulpha Pass, Tech De Ra, Amphiseum), and each original differs from its substituted
   capture by RMSE 0.055 to 0.065, the spread between two boots of one node (0.063). The six others (Chenghou Project, Metropia, Moa Therma, Sebenco Climb, Sol 2, Ubermall)
   keep the rename and a `caveats` entry; their HD grids were not reached (open, Next Steps 1). Confidence 80.

3. **2048 talons-junction: a bright road bar in ours** (unblocked: item 2 re-captured it from the authored node). Measured on `2048-vita/talons-junction/ours.png`:
   one full-width bar, rows 289 to 304 of 960x544, about 60 % of the centre row above 90 % grey; the original has no such
   row (0 % in rows 284 to 312). `moa-therma` and `ubermall` have a weaker bright row (18 %) that was not examined. The pair
   is one of the substituted set (item 2), so the bar may be the track's own geometry in the 2048 build and not a render
   fault. The `alpha_emissive` family wired on 2026-11-11 (`four-hd-scenery-material-readings-are-not-wired-on-2048-and-omega.md`,
   item 1) is the first suspect and the pair was rendered after that change (12:55, 2026-11-12). Effort: measure is low.
   Confidence 20. Re-measured 2026-11-12 on the authored `talons-junction` pair (ours at `4d76ba866`, the original from `grid8_3_1`): rows 284 to 312, the share of
   pixels above grey 230 is at most 0.64 in ours (8 rows above 0.3) and 0.00 in the original, so the bar is not an artefact of the substituted event (confidence 70).
   `amphiseum` (authored) shows no bar in those rows (0.00 in both), `moa-therma` 0.18 / 0.20 and `ubermall` 0.00 / 0.04 (ours / original), so the weaker bright rows
   on the other two are not a bar.
   Measure: toggle the `alpha_emissive` family off on a copy, and count pixels above grey 230 in rows 285 to 308.

4. **Pure Classic league: not viewable** (unranked). Pairs `pure-psp/manor-top`, `mandrashee`, `sagarmartha`, `karbonis`,
   rmse 0.18 to 0.26. `ours.png` is absent on disk for all six Pure pairs (the manifest's `images = "../compare-page"`
   does not supply it), so their error cannot be seen. The state block reads class `venom` while the file name and argv
   say `vector`; this is unresolved, not a proven capture error. No cause is claimed.

## Already threaded (cited, not ranked)

- `omega/menu-ship-select` (0.260): menu-pairs thread, Open item 2 (hull size and livery).
- `2048/menu-team` (0.222), `2048/menu-map` (0.210, the pair shows the title and the map pose; the gap is in the 2048
  section of the menu-pairs thread, items 1 to 6), `2048/menu-card-*`: the 2048 front-end thread and menu-pairs 2048 section.
- `omega/menu-track-select` (0.199), `menu-cell-select` (0.184), `menu-grid-select` (0.159): menu-pairs Open 1(a), (c), (f)
  and 5.
- `hd/menu-grid-select`, `hd/menu-track-select`: `hds-front-end-differs-from-the-original-in-eight-ranked.md`, items 3, 5, 6, 8.

## Not read

`data/reference/hd-capture/*/` was listed and not read. `2048/menu-card-0` and `card-1` were not viewed.

## Next Steps

1. Item 2, the six circuits still on the rename (oag-capture, optional): reach HD `grid1` to `grid6` on the map (the HD map shows the next grid as a sliver under the two
   bottom-right buttons; a drag and the d-pad do not pan it) and tap `grid1_3_1`, `grid2_3_1`, `grid3_3_1`, `grid4_3_1`, `grid5_2_3` and `grid6_3_1` with `vita-2048-pair.py
   --node-template`. Accept when each pair's `original.event` names its cell. The rename is shown to render the same (compare-page.md), so no render lane waits on this.
2. Item 1a (oag-wire): the `2048_campaign` card's banner and wordmark on the Omega campaign select. Box: the card on
   `omega/menu-campaign-select`, approximately x 200 to 1040, y 170 to 900 of 1920x1080 (set the exact box from the pair).
   Accept when the pair's rmse falls below 0.369.~~ (done 2026-11-12).
3. Item 1b (oag-re): the turned campaign cards' pose on Omega, from the executable's `Campaign Selection` update.
4. Item 3 (oag-re to name the bar's material, then oag-wire). Accept when the row measure above is zero on the authored `talons-junction` pair.
5. Item 4 (lead): decide whether the four Pure Classic pairs need a re-render or a state fix before any lane.
