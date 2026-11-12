---
status: measure
titles: [2048, omega, pure]
updated: 2026-11-12
summary: "Compare pairs ranked by payoff for effort: Omega campaign card first (threaded), 2048 race hull livery census, 2048 white road bars to measure"
size: S (30)
difficulty: medium (30)
---

# Comparison pairs ranked by payoff for effort

2026-11-12, lane `compare-pairs-scout` (read-only). Question: which compare pairs have a large visible error that a
low-to-moderate change could close. Scope: all 140 pairs on `data/reference/compare-page`; Omega race shots and
Fort Gale out; menus in. Race area x error is already ranked in
`compare-pairs-ranked-by-area-times-error.md`; the 2048 craft box is in `2048-grid-dark-bias-is-the-craft-not-a-frame-term.md`.
Both are cited, not repeated.

## Open

Impact figures are estimates from box area x error, labelled as such. Confidence is 0 to 100.

1. **Omega campaign select: the 2048 card's art** (already threaded, `menu-pairs-show-hd-and-omega-front-end-differences.md`,
   Open 1(d)). Pair `omega/menu-campaign-select`, rmse 0.369, the largest of the 140. Visible: ours shows a blue disc and a
   placeholder hex where the original shows the 2048 banner, wordmark and roundel on a white lattice card. Impact estimate:
   the card is about 30 % of the frame, error high, so the largest single area on any menu pair (estimate). Effort: low to
   medium (the card's banner and wordmark materials are the open part). Confidence 40. Suggested lane: oag-wire.

2. **2048 race hull: livery identity first** (new angle on the craft box). Pairs: 2048-vita talons-junction, ubermall,
   moa-therma, modesto-heights, tech-de-ra, sebenco-climb (and the other race pairs there). Visible: on talons-junction
   ours has yellow flanks and the original dark olive with a blue core glow; the same yellow where the original is cyan
   shows on `2048/menu-team` (Feisar Speed). Reading (alpha removed, box x 36 to 54 %, y 50 to 82 %, original / ours luma):
   talons-junction 29 / 61, ubermall 97 / 49, moa-therma 110 / 61, modesto-heights 57 / 45, tech-de-ra 28 / 53,
   sebenco-climb 72 / 71. The sign flips by circuit, so this is not one tone term, and the threaded craft-box note's
   ambient probe does not explain it. Impact estimate: the craft box is about 6 % of a 960x544 frame, so each pair is worth
   roughly 1 to 2 M in the area x error units of the ranked thread (estimate). Effort: census is low; a fix is medium.
   Confidence 30. Suggested lane: oag-re, first the census of each pair's team and paint, then oag-wire.
   The 2048 gantry reads 3 in the original and 2 in ours on these pairs: a capture-clock caveat, which may also move the
   ring and glow with the countdown phase. Cause not known. Evidence: the pair JSON's state block carries no team for 2048 (`original` has emulator and
   hull_shake only), so the team must be read from the save state the pair names before a hull lane is picked.

3. **2048 road white bars** (measure first). Pair `2048-vita/talons-junction`. Visible: ours draws two bright horizontal
   white bars across the road in the upper third, and the original shows none. The bars look like an emissive term.
   `alpha_emissive` was wired on 2048 on 2026-11-11 (`four-hd-scenery-material-readings-are-not-wired-on-2048-and-omega.md`,
   item 1), and this pair was rendered at 12:55 on 2026-11-12, after that change. Impact estimate: about 10,000 px at error
   near 150, about 1.5 M for this pair (estimate), more if the other 2048 circuits carry the same surface. Effort: measure is
   low; a fix is low if it is the `alpha_emissive` family. Confidence 25. Suggested lane: oag-re to name the material of the
   bars, then oag-wire.
   Measure: toggle the `alpha_emissive` family off on a scratch copy of the pair and count pixels above grey 200 in
   the band y 130 to 200 of the 960x544 frame.

4. **Pure Classic league: re-render first** (unranked). Pairs `pure-psp/manor-top`, `mandrashee`, `sagarmartha`,
   `karbonis`, rmse 0.18 to 0.26. `ours.png` is absent for all six Pure pairs, so the error cannot be seen. The pair's
   state reads class `venom` while the file name and argv say `vector`, and the caveat says the original draws another hull.
   No cause is claimed. Effort: a re-render. Suggested lane: the lead re-renders with `just compare-rerender-ours` after
   the copies are checked, then oag-capture if the state is wrong.

## Next Steps

1. Items 1: oag-wire, the 2048 campaign card's banner and wordmark, box = the card on `omega/menu-campaign-select`
   (approximately x 200 to 1040, y 170 to 900 of 1920x1080; set the exact box from the pair). Accept when rmse drops on that pair.
2. Item 2: oag-re, census of team and paint per 2048 race pair from each pair's state file. Accept when each pair's team
   is named in the thread.
3. Item 3: oag-re, name the material of the road bars on `2048-vita/talons-junction`, then oag-wire. Accept when the
   bar pixel count in the measure above drops to zero with no change to the other 2048 pairs' rmse.
4. Item 4: lead re-renders the four Pure Classic pairs; then decide whether the state class is wrong (oag-capture).
