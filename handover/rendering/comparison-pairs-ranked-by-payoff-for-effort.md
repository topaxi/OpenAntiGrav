---
status: measure
titles: [2048, omega, pure]
updated: 2026-11-12
summary: "Compare pairs ranked by payoff for effort: 11 of 14 2048 race pairs compare an HD craft (capture first), Omega card art done, Pure pairs not viewable"
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

2. **2048 race pairs compare a different craft: a capture fix first** (new). The `meta` block of 11 of the 14
   `2048-vita` race pairs (amphiseum, anulpha-pass, chenghou-project, dlc-vineta-k, metropia, moa-therma,
   modesto-heights, sebenco-climb, sol-2, talons-junction, tech-de-ra, ubermall) reads
   "HD Campaign Uplift ... Race, Venom, 3 laps ... its circuit set to <track> in the guest's memory", with craft
   **Feisar HD**. So the original is an HD event on a rewritten circuit name, and the pair's `caveats` are empty.
   Only `empire-climb` and `mall` are a real 2048 Single Player event (craft Feisar). Visible consequence: on
   talons-junction the original's hull is dark olive with a blue core glow, ours is yellow and teal. Per-pair hull luma
   also flips sign by circuit (alpha removed, box x 36 to 54 %, y 50 to 82 %, original / ours: talons-junction 29 / 61,
   ubermall 97 / 49, moa-therma 110 / 61, modesto-heights 57 / 45, tech-de-ra 28 / 53, sebenco-climb 72 / 71), which
   fits two different craft and not one tone term. Consequence for the threads: `dlc-vineta-k` is one of the substituted
   pairs, so the craft-box and tone rows that used it (`2048-grid-dark-bias-is-the-craft-not-a-frame-term.md`,
   `compare-pairs-ranked-by-area-times-error.md` rank 6) may be HD craft against 2048 craft; the real 2048 pairs are
   empire-climb and mall. Impact estimate: no render gain by itself; it decides which pairs can be trusted.
   Effort: low to medium (re-make from a real 2048 race state, or add the substitution to `caveats`). Confidence 45.
   Suggested lane: oag-capture.

3. **2048 talons-junction: a bright road bar in ours** (blocked by item 2). Measured on `2048-vita/talons-junction/ours.png`:
   one full-width bar, rows 289 to 304 of 960x544, about 60 % of the centre row above 90 % grey; the original has no such
   row (0 % in rows 284 to 312). `moa-therma` and `ubermall` have a weaker bright row (18 %) that was not examined. The pair
   is one of the substituted set (item 2), so the bar may be the track's own geometry in the 2048 build and not a render
   fault. The `alpha_emissive` family wired on 2026-11-11 (`four-hd-scenery-material-readings-are-not-wired-on-2048-and-omega.md`,
   item 1) is the first suspect and the pair was rendered after that change (12:55, 2026-11-12). Effort: measure is low.
   Confidence 20. Do this after item 2 re-makes the pair.
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

1. Item 2 (capture first, oag-capture): re-make the eleven HD-substituted `2048-vita` race pairs from a real 2048
   Single Player race state, or add the substitution to each pair's `caveats` so no lane is picked from them. Accept when
   every 2048 race pair's `meta.craft` names the craft the 2048 game drew. Until then, do not pick a hull lane.
2. Item 1a (oag-wire): the `2048_campaign` card's banner and wordmark on the Omega campaign select. Box: the card on
   `omega/menu-campaign-select`, approximately x 200 to 1040, y 170 to 900 of 1920x1080 (set the exact box from the pair).
   Accept when the pair's rmse falls below 0.369.~~ (done 2026-11-12).
3. Item 1b (oag-re): the turned campaign cards' pose on Omega, from the executable's `Campaign Selection` update.
4. Item 3 after item 2 (oag-re to name the bar's material, then oag-wire). Accept when the row measure above is zero on a
   real 2048 pair with the same track.
5. Item 4 (lead): decide whether the four Pure Classic pairs need a re-render or a state fix before any lane.
