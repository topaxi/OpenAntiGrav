---
status: ready-to-wire
titles: [hd, omega, 2048, pulse, pure]
updated: 2026-10-27
categories: [frontend, rendering]
summary: "Menu pairs HD 5, Omega 4, 2048 7, Pulse 5, Pure 8 on the compare page; ranked differences, Omega campaign selector, XML main/racebox/options gaps"
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
   **Do not confuse the HD and Fury campaigns**: the maintainer recalls (2026-10-26, from play, unverified) that only the
   Fury campaign draws the red background rect; HD's does not. Check which campaign the original frame shows before
   matching a panel, and pair each of Omega's three campaigns against its own original frame.
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

## 2048 (Vita3K), `menu-pairs-2048` 2026-10-27

Seven pairs and seven gaps of the touch front end, two cold boots each (method and numbers: `docs/tools/compare-page.md`,
"Menu pairs: Wipeout 2048"). Ours is drawn from the disc's XML through `--until STATE --press`; 2048 has no `menu.toml` page. No
renderer or UI code changed. The campaign pair (map plus event card pages 0 and 1, Empire Climb, fresh save both sides) is made for the **base Single Player Campaign** ("2048 - Event 1") only; the HD and Fury DLC campaigns are not paired yet (their Game Mode tiles are known, HD at (477,162); pair like with like, only Fury is thought to draw the red background rect);
2048's map and card show no difficulty control, so there was none to match. Ranked by the share of pixels that differ by more than
40 grey:

1. **Select Team** (56.7 %): ours is two rows of labelled buttons and a `FE_SKIN_*` list (chosen stand-ins); the original is a 5x4 grid
   of locked hexagons under five team badges, a hull turntable and the statistics table. Needs the grid, hull and table drawn from
   `Team_Definition.xml` and the ship models (the screen's `<Model>` preview is not drawn, see `2048-frontend.md`).
2. **Game Mode** (19.3 %): the RACEBOX / REMIX / MULTIPLAYER column (`oag-extras` rows) and the check mark are ours alone; the
   original's header icon and underline are missing, Single Player is ringed on entry in ours, the rows sit about 14 px lower.
3. **Campaign map** (12.6 %): the `A.G.R.C 2048` banner, `TOUCH TO START` tag, home button, three badges, corner buttons, scroll
   bars and triangle ground are missing; the node layout is the known chosen mapping.
4. **Title** (11.0 %): ours is a white frame with the legal line on the entry tick; the logo, the pulsing `PRESS ANY BUTTON TO START`
   and the triangle pattern are missing (a capture-route limit as much as a draw gap: `--ticks` after `--until` does not advance it).
5. **Event card** (4.9 % page 0, 4.3 % page 1): subtitle `RACE` against `NO WEAPONS` (the card's second line), the map shows through
   ours instead of a blurred white ground, no home button or badges, `CURRENT BEST` bar grey-blue against orange.
6. **Home** (3.8 %): the header icon and underline and the blurred map backdrop are missing; Team is ringed in ours.

Gaps: PSN prompt (a system dialog), the campaign welcome panel `SPTutorial` (ours never opens it live), Options, Profile and Extras
(ours draws them live but no still route reaches them: `--press` is one mask, `--screen` draws on black), the pause menu (`GAME
PAUSED`, ours' pause is `menu.toml`'s). A `--script`-driven still route past Home, and a title still that settles, would close
most of these.


## Pulse and Pure (PPSSPP), `menu-pairs-pulse-pure` 2026-10-27

Five Pulse pairs and eight Pure pairs of the screens ours draws from the disc, two or three boots each (the Pulse walk three: two on one instance, one fresh; the title two further boots) (method and numbers:
`docs/tools/compare-page.md`, "Menu pairs: Wipeout Pulse and Wipeout Pure"). The **campaign pair** is made for both: Pulse's Grid
Selection and Cell Selection (the footer reads `AI difficulty (Medium)` on both sides), Pure's Tournament Selection (Alpha, class Vector;
Pure shows no difficulty control). No renderer or UI code changed. Ranked by the share of pixels that differ by more than 40 grey:

**Pulse**
1. **Track Select** (16.2 %): ours lists 11 circuits where a fresh profile has 3 (`1/11` against `1/3`; the same under both `race.mode`
   values and `[dev] unlock_everything = false` survives the rewrite, so the cause is open), the photo cards show another view, distance `5094` against `5178`, empty records `-` against `-.--.--`.
2. **Ship Select** (7.1 %): the hull's livery and size (the original's blue-white Assegai, large and tilted; ours dark and smaller), `1/12`
   against `1/8` craft.
3. **Title** (2.6 %, `--until "Show Logo" --press cross --ticks 200`; the `--screen` route lacks the ray backdrop and reads 6.0 %): the
   wide `PRESS START BUTTON` and legal-line type face is small mixed case in ours; the rays are at another phase.
4. **Grid Selection, Cell Selection** (2.1 %, 1.8 %): the cursor hex glow and the scrolling footer ticker (empty at tick 0).

**Pure**
1. **Team Selection** (8.6 %): 17 teams in small type against the original's eight in large, the hull outline's angle, crowded stat labels.
2. **Track Selection** (5.5 %): the `SINGLE RACE - VENOM - ALPHA` context line (chosen), record format `0.00.00` against `0'00"00`, `m` unit case, no description.
3. **Every chain screen** (2.4 to 3.4 %): the context line (ours' chosen stand-in; Class, League, Tournament Selection), no description
   line along the bottom (a missing element: ours is drawn 5 s in), and the footer `BACK BUTTON` drawn over itself where the original has a triangle and `BACK`.
4. **Title** (0.3 %): no visible difference.

Gaps (original kept, no pair): Pulse `Main Menu`, `Racebox`, `Single Player` (Racebox setup), `Additional` (Options+), the pause menu;
Pure `Main Menu` and the pause menu. Each is drawn from `menu.toml` or has no still route.

## Next Steps

1. Omega's campaign selector (2048, HD, Fury cards) and the flyer on Omega's source: one lane, the biggest area.
2. A `--menu-page` still for the HD pause menu and Racebox setup read from `racebox_definition.xml` / `ingame_definition.xml`
   if the maintainer wants those XML-driven; then pair them with `scripts/rpcs3-hd-menu-pair.py` (add a `WALK` step, change
   `drawn_from` to `xml`).
3. Re-measure the HD layout offset on a fresh RPCS3 profile (Safe Area default) to see whether the 27 px and 4.1 % are the
   save's.
4. Capture the original's pause menu (a Racebox race, Start) and HD's `Studio Logo` for gap records.
5. 2048: a still route for Options, Profile, Extras and the Select Team grid (a `--screen` that draws the shell around the screen, or an
   `--until` that takes a button script), then pair them with `scripts/vita3k-2048-menu-pair.py` (move the `WALK` entries from
   `route: None` to a route).
6. Pulse and Pure: find why Track Select lists 11 circuits and Ship Select 12 craft on a fresh profile where the original lists 3 and 8, check Pure's chain context line against the original's text, make `--menu-page cell-select` read the persisted campaign difficulty on Pulse (only the HD branch does, so a non-default rung cannot be paired), and give the `BACK` prompt its triangle (the doubled `BACK BUTTON`); then re-run
   `scripts/psp-menu-pair.py pairs`.
7. Pulse and Pure pause menus: a `--menu-page pause` still would turn those two gaps into pairs.
