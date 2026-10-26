---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-10-26
categories: [frontend, rendering]
summary: "Menu pairs HD 5, Omega 4 on the compare page; ranked differences, Omega campaign selector, XML main/racebox/options gaps"
---

# Menu pairs show HD's and Omega's front-end differences, ranked by visible area

`menu-pairs-hd-omega` (2026-10-26) made original-vs-ours pairs of the front-end screens ours draws from the disc's XML:
HD/Fury (RPCS3) Campaign Select, Campaign grid, Campaign cell, Track Select, Ship Select; Omega (shadPS4) Campaign grid,
cell, Track Select, Ship Select. Method, per-pair overlap, the XML-versus-`menu.toml` table and the layout numbers are in
`docs/tools/compare-page.md`, "Menu pairs". Pairs sit under `data/reference/compare-page/{hd,omega}/menu-*`; gaps (ours has
no XML-driven page, the original's frame kept) under `gap-*` beside them. This lane measured and changed no renderer or UI
code. It supersedes the HD lead "Main Menu and Racebox setup are our own tree" with a pair-based list below and corrects
"shadPS4 reaches the main menu only": the pad reaches Ship Select on every walk.

## Open

Ranked by visible area on the 1920x1080 frame (read off the pair images, each pair run twice).

1. **Omega Campaign cell and grid: no flyer card, wrong grid** (largest: mean grey difference 129 and 55). The original's
   dark-red panel (cell) and red flyer (grid) cover 60 to 80 % of the frame; ours is a white page with the ghosted backdrop
   tower. Ours also draws Omega's own first grid (18 points, `01_TRACK` unresolved) for `grid-select` and `grid-select-hd`,
   neither of the original's three campaigns (2048 Cup, HD, Fury), and `campaign-select` refuses on Omega. Needs: the
   Omega campaign selector and the flyer card on Omega's source (HD's cards draw on HD).
2. **Omega Ship Select: hull and logo** (34 grey). Ours draws the hull about twice the original's size in another livery
   (dark teal and yellow against pale blue and yellow) and a ringed Feisar logo against the flat one; the cursor starts on
   the top hex where the original's fresh profile lands on the first unlocked craft.
3. **HD Campaign Select, grid and Track Select/Ship Select backdrop** (27.7, 15.0, 21.6, 22.0 grey): the original's red
   particle cloud is much fainter in ours on Track Select, Ship Select and Campaign Select (the known backdrop lead of
   `hds-front-end-differs-from-the-original-in-eight-ranked`); the HD card on Campaign Select is white and cyan against a muted grey-green, and the gold-medal icon
   is missing.
4. **HD layout offset** (every HD pair): ours' rules sit 27 px left of the original's centre and the page is 4.1 % taller
   (rules 863.5 px apart against 829.5), see the doc's layout paragraph. Omega's columns match exactly and ours is 1.6 %
   shorter. Replaces the earlier "0.904 at (+150, -8)" lead with a measured pair of numbers (confidence 75, one pair of
   rules per title); the save's `Safe Area Setting` is 96, so a fresh-profile run would still settle whether the offset
   follows it.
5. **Track Select, both titles**: no circuit photo (`no slideshow chain to read`), no length or distance values, the model
   turned and scaled differently (the turntable phase is not matched), Omega's name reads `01_Track`.
6. **HD grid flyer card**: pose (two large arrows tilted in the original), previous and next grid arrows missing, `EVENT 01/08`
   bold capitals in the original against small mixed case.
7. **HD Ship Select**: hull pose and size, red markers on the statistic bars that the original lacks.
8. **HD Cell**: the selected hex's white outline, the red haze right of the card (closest pair, 16.7 grey).

Gaps that need a page before a pair: HD Main Menu (`OPENANTIGRAV`, no ONLINE, row text `RACE CAMPAIGN`), Racebox setup
(`menu.toml`'s `race`), Options+ (`Additional`), Race Records, the pause menu; Omega the same four plus Campaign Select. Each
is drawn from `menu.toml` or missing, so a pair would set ours' stand-in against the disc's XML (ADR-0066 keeps the main
page ours by design, so the main-menu gap is a decision, not a bug).

## Next Steps

1. Omega's campaign selector (2048, HD, Fury cards) and the flyer on Omega's source: one lane, the biggest area.
2. A `--menu-page` still for the HD pause menu and Racebox setup read from `racebox_definition.xml` / `ingame_definition.xml`
   if the maintainer wants those XML-driven; then pair them with `scripts/rpcs3-hd-menu-pair.py` (add a `WALK` step, change
   `drawn_from` to `xml`).
3. Re-measure the HD layout offset on a fresh RPCS3 profile (Safe Area default) to see whether the 27 px and 4.1 % are the
   save's.
4. Capture the original's pause menu (a Racebox race, Start) and HD's `Studio Logo` for gap records.
