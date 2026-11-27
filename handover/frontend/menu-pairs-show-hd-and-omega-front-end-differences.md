---
status: ready-to-wire
titles: [hd, omega, 2048, pulse, pure]
updated: 2026-11-27
categories: [frontend, rendering]
summary: "Menu pairs on the compare page, ranked differences; Omega chooser, 2048 card art; Pulse list screens and pause now pairs"
difficulty: medium (30)
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

1. **Omega Campaign cell and grid: the Fury card is whole since 2026-10-27 (`omega-flyer-shell`); what is left is pose and chooser.**
   Shell, hexagon logo, sponsors, translucent arrows, wordmark, edge stripes and `TALON'S JUNCTION` draw
   (`docs/ui/campaign-screens.md`, "Shell, art and track name"). Pair numbers: cell 13.4 to 12.7, grid 40.9 to 42.9 (the card is now
   right but sits in the wrong place: it is 83 per cent of the grid's grey error). **Open**: (a) ~~the grid card's pose and size on Omega~~ (done 2026-10-27, `omega-grid-card-pose`: grid 42.9 to 16.4; the pose is
   measured off the frame and chosen, `Title.campaign.grid_card_pose`; Omega's own `<Flyer>` widget numbers still uncompared with HD's); (b) the backdrop (ghost tower against blurred streaks); (c) cell: `CHOOSE RACE` and the unlock line dark
   where the original is grey, the selected hex's white outline, the missing `RECORDS` prompt; (d) ~~the Omega `Campaign Selection`
   chooser (three-way: 2048, HD, Fury; `campaign-select` still refuses)~~ (done 2026-11-05, `omega-chooser`: drawn off the screen's own XML by the reader HD uses, pair `omega/menu-campaign-select` 52.7 whole frame, 78.4 over the bracket; the `2048_campaign` card's art done 2026-11-12 (`omega-campaign-card-art`: the card's UV keys from the `.vex` were unplayed, `CardSpec::keyed`; card box mean grey 69.3 to 32.3, whole frame 56.3 to 46.2; the HD and Fury cards' ring logos draw too; the grid cards stay unkeyed until a frame of one is captured, `docs/ui/campaign-screens.md`, "Omega: `Campaign Selection`"); **left open**: the unselected cards stand 45 and ~190 px left of the original's and turn by `RotY` as radians where the frame fits ~0.75 rad, and choosing 2048 opens no grid: `grid_16`..`grid_18` are cell-less stubs the grid reader skips and its events are `Campaign2048_Definition.xml`'s `FE3DCanvas` map, so it shows `OAG_CAMPAIGN_NOT_PLAYABLE`; only the 2048-selected state has an original frame, HD and Fury selected are not captured); (e) **a PS4-wide lead found on the way**: the PS4 build
   leaves every vertex's coverage channel at 0 (red), so any blended or alpha-tested PS4 draw whose coverage is a texture's alpha
   reads red instead (`mesh::rcs::psp2` `slots`; fixed for the flyer cards only in `psp2_scene::alpha_from_the_picture`): census
   Omega's circuits and hulls (and 2048's Vita models, which share the builder: checked, applies, not wired) for draws it changes before
   flipping the builder's default; the dark grey hull silhouette on Omega's base-campaign grid card (`grid-select-hd`) is unchecked against an original; (f) ~~`Track Select` on Omega still lists
   `NN_Track` ids (`CircuitNames::choose` needs one copy naming all 38; the table names only some)~~ (done 2026-11-05: the ten 2048 circuits author `StringTableId`, `choose` takes `(id, key)` pairs, all 38 are named, `omega_track_model_ground_truth::every_circuit_a_menu_reaches_has_a_name_not_an_id`). Pair each of Omega's other two
   campaigns against its own original frame when one is captured.
2. **Omega Ship Select: hull and logo** (34 grey). Ours draws the hull about twice the original's size in another livery
   (dark teal and yellow against pale blue and yellow) and a ringed Feisar logo against the flat one; the cursor starts on
   the top hex where the original's fresh profile lands on the first unlocked craft.
3. **HD Campaign Select, grid and Track Select/Ship Select backdrop** (27.7, 15.0, 21.6, 22.0 grey): the original's red
   particle cloud is much fainter in ours on Track Select, Ship Select and Campaign Select (the known backdrop lead of
   `hds-front-end-differs-from-the-original-in-eight-ranked`); ~~the HD card on Campaign Select is white and cyan against a muted grey-green~~ (done 2026-10-27, `hd-campaign-card-dim`: whole 28.2 to 9.4; the unfocused card is
   dimmed by the screen's `0xff505050`, picture factor fitted; Fury's unfocused look unverified, no original frame has HD selected; the dimming may also animate in the original, a ramp not found), and the gold-medal icon
   is missing. What is left of that pair is the header/footer text sizes, the `GOLD MEDALS` block (~16 % together) and the card x-extent (ours 1107-1678, theirs 1100-1646).
4. **HD layout offset** (every HD pair): ours' rules sit 27 px left of the original's centre and the page is 4.1 % taller
   (rules 863.5 px apart against 829.5), see the doc's layout paragraph. Omega's columns match exactly and ours is 1.6 %
   shorter. Replaces the earlier "0.904 at (+150, -8)" lead with a measured pair of numbers (confidence 75, one pair of
   rules per title); the save's `Safe Area Setting` is 96, so a fresh-profile run would still settle whether the offset
   follows it.
5. **Track Select, both titles**: no circuit photo (`no slideshow chain to read`), no length or distance values, the model
   turned and scaled differently (the turntable phase is not matched), ~~Omega's name reads `01_Track`~~ (done 2026-11-05, item 1(f)).
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

**2026-11-27 (`2048-menu-pairs`):** Home 9.73 to 8.13, Game Mode 6.30 to 2.55, card page 0
7.83 to 7.01, page 1 7.00 to 5.83 mean grey (`docs/ui/campaign-screens.md`, "2048 menu pairs:
what the four worst differed by"). Closed: the header icon (a capture-tick
artifact plus a draw-order defect on Game Mode), the rows' label face and tint, the card's
subtitle, tabs and `CURRENT BEST` bar. Left: the header rule, the card's `TouchHomeButton`
composite (home square and badges; `0x810fc422`, `FUN_813cf8cc`), the blurred ground
(`CampaignEventCard_Draw` `0x810f1196`), and, on Omega, the card tabs in `NEOSANS_BOLD`
(applies, not wired: Omega ships the face and `FUN_013a9040` names it; the hosted map loads
no role face).

1. **Select Team** (was 56.7 %, mean grey 94.9; 2026-10-27 `2048-team-grid`: 38.3): the grid, badges, cursor cell, header square and
   confirm tick now draw from the disc (`TouchTeamGrid` read in Ghidra). Still differs: the 19 locked cells (rule needs the save and the
   ship record's rank, `FUN_812d16f6`), the statistics table, the `FE_SHIP_STATS_TUT` bubble and the hull `<Model>` preview.
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
1. **Track Select** (16.2 %, 14.6 % re-made 2026-10-29): the list now matches (`1/3`; the 11 were eight maintainer DLC circuits, fixed 2026-10-29 by an empty `$OAG_DLC` in the pair run), the photo cards show another view, distance, the `Distance(m)` label and the `-.--.--` empty records closed 2026-10-29 (the ticker's message choice is a weighted random draw over lifetime statistics, not wired).
2. **Ship Select** (7.1 %, 7.0 % re-made): the hull's livery and size (the original's blue-white Assegai, large and tilted; ours dark and smaller), `1/8` on both sides since 2026-10-29 (it was
   `1/12`: four DLC teams).
3. **Title** (2.6 %, `--until "Show Logo" --press cross --ticks 200`; the `--screen` route lacks the ray backdrop and reads 6.0 %): the
   wide `PRESS START BUTTON` and legal-line type face is small mixed case in ours; the rays are at another phase.
4. **Grid Selection, Cell Selection** (0.57 %, 0.65 % on 2026-11-23 grabs; `docs/tools/compare-page.md`, "Pulse Cell Selection: the footer, the tag and the title's glow phase"): the ticker, the tag and the title's glow phase are wired (the halo is the front end's bloom, drawn). 2026-11-23 (`pulse-fe-leftovers`): the glyph rings (the button glyphs now draw from `Pulse_14`, `docs/ui/menus-original.md`) and the bar row (the footer clip) are closed, Cell 0.04 % on two fresh grabs; left: the Confirm and Back prompts' dim top rows (the original's lower footer strip is likely drawn over them; ours draws every frame image under every text). `menu-grid` and `menu-cell` in `data/reference` are re-made by the lead's adopt step.

**Pure**
1. **Team Selection** (8.6 %, 7.5 % re-made 2026-10-29, 3.2 % on 2026-11-01 against a fresh capture, 7.57 % the same fresh capture before): the large type, the hull's angle (the slideshow's clock: `team` is now in `PURE_CLOCKED`) and the stat labels (the `Stats` face) are closed. 2026-11-23 (`fe-pulse-pure`): ~~the cursor on ASSEGAI~~ (a fresh profile opens on FEISAR, `RaceDefaults::fresh_team`; the pair's ours had been re-rendered without its `race` override), ~~the description line~~, ~~the dashed and solid footer rules~~, ~~the pink cursor arrow and the stray scroll arrows~~ all drawn from `Skin.xml`/the string table (`docs/ui/selection-screens.md`, "Pure's cursor arrow and footer frame"); share over 40 grey 6.15 % to 1.86 % (whole), 3.86 % to 1.32 % (footer box) on a fresh capture. ~~the typed line's baseline and advance~~ (closed by `pure-sr-track` and `pure-leftovers`: 0.2 px mean glyph residual on `menu-main`, `docs/ui/menus-original.md`, "Pure's right-aligned text ends on its ink"; what is left is one pixel on the first glyph of a word, unexplained, and `menu-mode`'s reveal count). ~~Open: the title screen's `PRESS START BUTTON` (scale 1.5 menu face): edge sampling differs~~ (closed by `pure-title-press-start`: the sprite's texture span, `docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`, "The quad"; title pair 0.32 % to 0.01 %). Open: the `_` the original types after the selected row's label (`QIREX_`, `VECTOR_`); on Team Selection only, a white gapped sprite over ours' top rule at x 304-366, y 38-39 (likely the ship slideshow card's top rows drawn opaque; the chain screens' rule is whole); the scroll arrows' show-while-scrolling rule is read off the widget names, not measured (no captured Pure list scrolls).
2. **Track Selection** (5.5 %, 4.2 % on 2026-11-01, 5.63 % the same fresh capture before): the `SINGLE RACE - VENOM - ALPHA` context line is gone from the still (the original draws none on any chain screen; it stays on the live screen), the `m` unit and the labels come from the `Stats` face. 2026-11-23 (`pure-sr-track`): the text pens of every Pure screen step up to whole pixels (`docs/ui/menus-original.md`, "Pure's text pens step up to whole pixels"), the description line is 0.8 and one pixel lower; share over 40 grey 2.73 % to 1.30 % on a fresh capture (list 834 to 76 px, title 570 to 174, description 695 to 330, stat labels 1,098 to 871), seven Pure pairs all better. Open: glyph edges of the `Stats` face are 1 px wider in ours (a left fringe the original has and ours lacks, 815 to 871 px of the remaining 1,617; the Pulse HUD's measured half-unit glyph quad was tried and moved the share 1.30 % to 1.24 % with a worse mean grey, so it is not adopted), and the record's separators. At 8x the original's `0.00.00` reads `0 . 00 . 00` with small mid-height dots and wide gaps, not `0'00"00` (the 2x reading); ours prints `.` at the baseline, and a sweep of every Latin-1 character as the separator found none that matches, so the format string is not recovered.
3. **Every chain screen** (2.4 to 3.4 %): the context line (ours' chosen stand-in; Class, League, Tournament Selection), the description
   line is drawn settled since 2026-10-29 (the original types it in and its frame holds a prefix, so the share rose 0.8-1.6 points; its type-in rate is
   not measured, its scale 0.8 and 1 px top offset measured since 2026-11-23; its face and the footer's `Confirm`/`Back` are the disc's `Scroll` face since 2026-11-01, the `Stats` glyph slot), the footer `Back` has its triangle; the two frame rules, the cursor arrow and no scroll arrows are drawn since 2026-11-23 (re-rendered copies: sr-class 2.32 % to 0.88 %, mode 2.95 % to 1.51 %, t-sel 2.65 % to 1.21 %; the lead re-renders the page). The context line is no longer in the compare still.
4. **Title** (0.3 %): no visible difference.

Gaps (original kept, no pair): none on Pulse or Pure. Pure's `Main Menu` and pause menu are pairs since 2026-11-23 (`pure-main-pause`: Main Menu 2.97 %, pause 2.82 % over 40 grey; `docs/tools/compare-page.md`, "Pure Main Menu and pause menu pairs"); Pulse's `Main Menu` (0.97 %), `Racebox` (0.44 %), `Single Player` (1.04 %), `Additional` (0.44 %) and pause menu (1.56 %) are pairs since 2026-11-23 (`pulse-gap-pairs`, "Pulse list screens and the pause menu"). Largest regions left on them: the pause (the worst, 1.56 %: the rows' bloom); Single Player's step arrows (the original dims the left arrow at a list's first entry, ours lights both), its greyed KILLS ink and `Help` prompt; the pause rows' bloom (ours' glow pass needs a cleared target) and its song pill; every list screen's footer ticker and prompt rings.

## Next Steps

0. Pulse Track Select (`pulse-psp-track`, `pulse-fe-leftovers`, 2026-11-23: 0.54 % and 2.32 % to 0.14 % and 0.15 % over 40 grey on fresh grabs): ticker, tag, footer clip, heading pens (`MenuSkin::bearing_pens`), prompt icons (`MenuSkin::small_font`) and the circuit name's glow phase (`--menu-title-seconds`) are done (`docs/tools/compare-page.md`, "Pulse Cell and Track Select: glyphs, pens and the name's glow"). Open: the Confirm and Back prompts' dim top rows (draw order against the footer's lower strip), the circuit map's travelling highlights (94 px, a clock not read), and Pulse PS2's third glyph slot (its skin has none, so its footer glyphs keep the menu face).

1. ~~Omega's campaign selector (2048, HD, Fury cards)~~ (drawn 2026-11-05). What is left of it (the `2048_campaign` card's art landed 2026-11-12): the turned cards' pose (needs the executable's `Campaign Selection` update, `ghidra-mcp` was down that session, and an original frame per selection), and 2048's grids (`grid_16`..`grid_18` stubs, `Campaign2048_Definition.xml`'s map): `docs/ui/campaign-screens.md`, "Omega: `Campaign Selection`".
2. A `--menu-page` still for the HD pause menu and Racebox setup read from `racebox_definition.xml` / `ingame_definition.xml`
   if the maintainer wants those XML-driven; then pair them with `scripts/rpcs3-hd-menu-pair.py` (add a `WALK` step, change
   `drawn_from` to `xml`).
3. Re-measure the HD layout offset on a fresh RPCS3 profile (Safe Area default) to see whether the 27 px and 4.1 % are the
   save's.
4. Capture the original's pause menu (a Racebox race, Start) and HD's `Studio Logo` for gap records.
5. 2048: a still route for Options, Profile, Extras and the Select Team grid (a `--screen` that draws the shell around the screen, or an
   `--until` that takes a button script), then pair them with `scripts/vita3k-2048-menu-pair.py` (move the `WALK` entries from
   `route: None` to a route).
6. Pulse and Pure: (Track Select, Ship Select and Pure's team list closed 2026-10-29) make `--menu-page cell-select` read the persisted campaign difficulty on Pulse (only the HD branch does, so a non-default rung cannot be paired); draw the record separators; read the `Stats` face's glyph-edge fringe (ours 1 px wider, see item 2) and the typed line's second row (`TextInfoGap`; the pen, scale and top offset closed 2026-11-23); give `mode` (Single Player) its description line (the original types one, ours draws none); a Pure walk on a stick with no profile stops on the system's "There is no data" dialog and still passes all seven parity checks, because Pure's screen is declared, not read: check `shots/` by eye after a walk; then re-run
   `scripts/psp-menu-pair.py pairs`.
7. Pulse's pause menu is a pair since 2026-11-23 (`pulse-gap-pairs`). Open on it: the header bar's slide (`InGame Pause SP`'s `<Animation delay="1">`, `Y -24` to `0`; ours draws the bar at rest, so the pair is grabbed 4 s after START), the front-end glow stamps on a menu over a parked race (`Renderer::render`'s `glowing` needs a `LoadOp::Clear` target; the original's rows peak (65, 204, 225) against ours' (49, 155, 173)), the now-playing pill (ours runs silent) and pinning the focused row's pulse on the drive route. Pulse `Single Player`: dim the left step arrow at a list's first entry and the right at its last on a compare page (the original's lists stop at the ends; the live `race` page wraps), the greyed row's ink (peak 36 grey on `KILLS`) and the `Help` prompt the screen authors (a `Text` and `FE_HELP` at y 252). Pure's pause pair: the HUD digits' glyph edges (the race HUD lane's) and the moving pennants are open; the words are the same width as the original's, 1 px left on two of three rows (centring rounding unresolved, `docs/tools/compare-page.md`, "The band's words and the HUD top, measured again"; next: capture the Free Play and Tournament pause at scale 1.0); the race clock is matched by `--pause-clock`. Pure Main Menu: row 4 `DOWNLOAD` is now drawn (the USA manifest's `PI012` overlay); the typed description's `Scroll` advance (6.5 px a character against 7.6) is the largest region.
8. Pulse PS2 menu pairs (2026-11-23, `ps2-ship-select`, `ps2-grid-cell`): Ship Select, Grid Selection and Cell Selection carry the original's footer ticker message and tag (`docs/tools/compare-page.md`, "PS2 menu pairs: the footer
   ticker and the tag are read from PCSX2 RAM", and "`menu-grid` and `menu-cell` re-made"); footer box >40-grey share 17 % to 8.6-9.0 % on the two campaign pages, whole frame 3.3-3.5 % to 2.8-3.0 %; translucent fills under an outlined atlas were drawn twice
   (the largest region on both pairs, 31-50 % of the pixels over 40 grey) and are drawn once. Open: the PS2 ticker widget's own fields (item index, weight) are not recovered, so only the six tips are pinnable and a frame with a statistic message (menu time, favourite music) is not grabbed;
   ~~the PS2 front end's pulse clock~~ (closed 2026-11-24, `ps2-pulse-clock`: the `Selector` phase is read from EE RAM at the grab and passed, `docs/ghidra/functions/ps2-pulse-eu/frontend-pulse.md`); the PS2 title shows no pulse (its `Text_Draw` ladder runs in RAM, the frame never changes): ~~ours' title halo on PS2 Grid and Cell Selection~~ (closed 2026-11-24, `ps2-title-no-pulse`: `MenuSkin::pulse_text_glow` is `false` on the PS2 skin, title box over 40 grey 13.3 % to 4.9 % on Grid and 11.0 % to 10.1 % on Cell; why the counter never reaches the pixels stays unknown; the `Selector`'s halo shape, small and offset up-left in the original, a broad round bloom in ours, is still open); the `RACE CAMPAIGN` header face (bold dark in the original). The committed `menu-grid`/`menu-cell` pairs need re-grabbing with the new script.
