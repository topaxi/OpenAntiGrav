# The campaign grid draws and does not launch

**Update, 2026-10-02, `pulse-cursor-live` lane: the cursor measured and fixed, the loyalty slope pinned.**
`Cell Selection`'s cursor is **one `(x, y)` slot shared by every grid**, kept across leaving the campaign
(PPSSPP, two boots, a breakpoint reading the screen's `+0xdc`): `grid0_3_2` then `grid1` lands on `grid1_3_2`,
`grid1_3_1` then `grid0` lands on `grid0_3_1`. A slot is carried only when the new grid's cell there shows no lock glyph (the screen's own predicate, not the
`Locked` byte: a cell a gold neighbour unlocked keeps the cursor with its byte still 1); a glyph-visible one
(including re-entering the same grid on it) falls to the default scan.
`CellCursors` is replaced by `CellCursor` plus `Session::campaign_cursor`. Not measured: a cell whose lock glyph a
medal cleared, a slot the new grid lacks, HD/Fury (still chosen), and a grid reached by the game's own unlock path
(`grid1` was opened by writing its `Locked` byte). The loyalty bar slope is measured at 20080 and 50080 totals,
123 and 49 screen px against 124.2 and 49.8 predicted: no code change. Detail: `docs/ui/campaign-screens.md`'s
"Where the cursor lives" and `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`'s "The slope, measured at two
totals".

**Update, 2026-10-02, `pulse-loyaltybar` lane: where the original keeps the cursor
across a back-out is decompiled.** `CellSelection_OnEnter`/`_Update` hold it as the
`Selector` widget's own `(x, y)` slot, one position shared by every grid; `+0xdc` (the
selected cell) is derived from it each frame, and `OnEnter` re-resolves it by the name of
the *current* grid at that slot, running the first-visit default scan only when that fails
(and not around `Cell Help`). This build's per-grid `CellCursors` is therefore **chosen and
probably wrong for a second grid**; not changed, because the same-grid case is the only one
measured and the tile-flag filter (`FUN_088a37cc`) is unread. ~~Next: a PPSSPP walk that backs out of `grid0_3_2` and enters another unlocked grid, then key `CellCursors` by `(x, y)` if it reproduces.~~ Done by the update above. Detail: `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
"Where `Cell Selection` keeps its cursor across a back-out".

**Update, 2026-09-28, `pulse-cellsel` lane: the two live capture findings
below settled, plus a full decompile of the medal/difficulty write this
thread's own item 1/2 needed.** `Race_RecordResult`'s own `record+8`/
`record+9` tail, `Profile_DifficultyRC`/`Profile_SetDifficultyRC`
(`0x08809980`/`0x088098f0`, renamed and named this pass) and
`CellSelection_Update` (`0x088d6430`, decompiled and named this pass) are
all now read in full - see `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s
"The `record+8`/`record+9` write" and "The `DifficultyRC` persisted rung"
sections for the decompile itself.

1. **Fixed: the medal now remembers its difficulty, on every title.**
   `Session::handle_campaign` (`crates/game/src/main/session/campaign.rs`)
   used to gate recording a difficulty on `cell.difficulty_targets.is_some()`
   - `None` for every Pulse cell by construction, since a Pulse cell's own
   medal never varies by difficulty. The decompile shows the original does
   not gate on that at all: `CellSelection_CommitSelection` persists the
   screen's own browsed rung on every `Confirm`, on every cell, as metadata
   alongside whichever medal the race earns. The gate is gone;
   `oag_game::records::Store::record_campaign`'s own tie-break logic is
   rewritten to match the decompile exactly (a strict medal improvement
   overwrites the stored difficulty unconditionally, even *downward* - the
   previous "harder rung always wins outright" implementation was wrong on
   this branch, right only on an exact-tie branch); and
   `oag_ui_screens::campaign::draw::medal_line` now appends `"(<rung>)"` to `Line7`
   exactly the way `CellSelection_PopulateDetail`'s own `"%s (%s)"` format
   does, `"Gold (Medium)"` rather than a bare medal word. Verified by new
   unit tests (`crates/game/src/records/tests.rs`,
   `crates/ui-screens/src/campaign/draw/tests.rs`) reproducing the decompiled
   branches directly, and a live `--menu-page cell-select` capture of a
   worktree-local `records.toml` row predating this fix (`Best: Gold`, no
   suffix - correct, since that row's own `best_difficulty` is `None`).
   **Not verified this pass**: a full interactive Confirm -> race -> finish
   -> re-entry loop producing a *new* difficulty-tagged medal live - the
   write path is unit-tested against the decompile directly rather than
   played end to end.
2. **Fixed: the square-button prompt now reads the runtime template, not
   the disc's authored `"Change Difficulty"`.** `CellSelection_Update`
   rebuilds `DifficultyButton`'s text every frame,
   `sprintf("%s (%s)", resolve("RB_AI_DIF"), resolve(rung))` off the
   screen's own browsed rung (`Easy`/`Medium`/`Hard`, bare idstrings, no
   `MSC_`/`FE_` prefix) - `oag_ui_screens::campaign::draw::difficulty_button_line`
   reproduces it. Verified against `data/reference/psp-campaign-screens/
   cell-selection-difficulty-hard.png` (`"AI difficulty (Hard)"`) and a live
   `--menu-page cell-select` capture of this build reading `"AI difficulty
   (Medium)"` at its own measured default rung. **HD's own analogous
   `"DIFFICULTY (<rung>)"` mismatch is a related but not identical gap, not
   touched this pass** - see `docs/ui/campaign-screens.md`'s own note on why
   the two titles' wording differs on every axis (label text, rung-word
   idstrings, default rung).
3. **Refined again, still open: `SelectorGlow` is confirmed absent from
   both files this screen is built from, not merely unreferenced in code.**
   `oag-wad cat --expand` on `Data.wad`'s own `CellMode_Definition.xml` and
   `Skin.xml` - the two files `crate::campaign::load` reads, in full -
   contain the literal string `"SelectorGlow"` zero times. The burst's own
   visible halo (still real, still phase-correlated, per the prior pass) has
   to come from somewhere this pass did not chase - a blend-mode effect on
   `Selector`'s own already-implemented colour pulse is the next lead, not a
   decided explanation. Left undrawn, per this project's own rule against a
   stand-in shape with no located geometry. See
   `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s own section for
   the full extraction detail.

**Update, 2026-09-28, `pulse-campaign` lane: the first live PPSSPP capture of
this leg, and four of the open items above/below settled by it - two closed,
one refined, one found new.** Walked `Main Menu` -> `Grid Selection` ->
`Cell Selection` -> `Cell Help` by hand under Xvfb `:93`/PPSSPP port 45001
against `pulse-psp-usa.chd` (fresh and returning profiles), aligned-cropped
every frame against this build's own `--menu-page grid-select`/`cell-select`
at the same 960x544. `scripts/psp-frontend-capture.py` now walks the same
leg as a committed step (commit `7f2b7855`, same day, same finding on
`TournamentLoad` below - not duplicated here). Full detail and every crop
path: `docs/ui/campaign-screens.md`'s own "## Open" section, in place at each
item below.

1. **Closed: `Cell Selection`'s own hex-outline colour was never actually
   open.** `crate::campaign::load`'s `fallback_globals` plumbing (landed
   2026-09-14) already resolves `Outline_x_y`'s `i="FEGlobals->CM_HEX_Outline"`
   to `Skin.xml`'s own declared `0x7F34ACC2`, and `cell_draw_list` already
   applies it through the generic `image_draw` path - the "not one of the
   two confirmed `FEGlobals->` names" sentence in this thread's own bullet
   below was stale the day it was written, not re-checked against the code
   that already existed. A locked hex's outline stroke averages RGB
   `(31, 58, 63)` on the PPSSPP capture against `(26, 56, 62)` on this
   build's own render of the same hex - within anti-aliasing noise.
2. **Fixed: the detail panel's `default`-role text (`Line1`..`8`/`Title`/
   `Track Line`/`Speed class`, both screens) was drawing at roughly 60% the
   real size - a genuine bug, not a "too broad a constant" risk.**
   `crates/ui-screens/src/campaign/draw.rs`'s `text_draw` was multiplying `scale` by
   `layout.face_scale("default")` (`13/22`) *on top of* routing through the
   real `Default`-role atlas `face_atlas_slot` already loads at its own
   native size - double-shrinking glyphs that needed no shrinking at all.
   `oag_ui_screens::campaign::footer::face_scale` already carries the correct form
   (`"default" => 1.0`) for the footer's own prompts, landed the same day as
   `face_atlas_slot` (2026-09-21) but never mirrored onto
   `Layout::face_scale` (`crates/ui-screens/src/campaign.rs`), which every panel
   label reads instead. Now does. **`FaceScales::default()` itself
   (`crates/ui-screens/src/picker.rs`) was never touched** - every other screen that
   reads it is unaffected. Verified against the same aligned crop (this
   build's `"Speed class"` row now within a pixel of the reference) and
   `cargo nextest run -p oag-ui campaign`/`-p oag-game campaign`, both green.
3. **Refined, not closed: `SelectorGlow` is a real, phase-synced draw this
   build still does not produce - not confirmed absent from the original.**
   An eight-frame burst (a scratch directory, not kept,
   ~180ms apart) shows a soft halo tracking `Selector`'s own cyan/white pulse
   exactly, and `Selector`'s own sprite crop has zero alpha in its padding
   (`docs/ui/campaign-screens.md`'s own texture read), so a plain colour tint
   of that one sprite cannot explain the halo bleeding past the hex edge.
   `get_xrefs_to` on the `"SelectorGlow"` string still returns exactly one
   hit (`GridController_UpdateSelectorPulse`'s own lookup, no creation site)
   - real, but not proof either way of what draws it. Left undrawn, per this
   project's own rule against a stand-in shape with no located geometry.
4. **Found and fixed, same pass: `Cell Selection`'s initial cursor is not
   "first cell in document order".** Measured live first, then decompiled:
   a genuinely fresh profile's first-ever `Cell Selection` selects
   `grid0_3_1` (`"SINGLE RACE"`/`"Moa Therma White"`), never `grid0_2_1`
   (`grid_00.xml`'s own first-listed cell, and what `CellSelection::new`'s
   old `index: 0` picked). `CellSelection_OnEnter` (`0x088d59c4`) decompiles
   to a plain linear scan for the first cell in document order whose own
   `Locked` byte (`+0xb9`) parses as the literal value `false` - an absent
   attribute and an explicit `true` both skipped alike, which is exactly
   `grid0_3_1` (fourth listed, first explicit `false`) and not `grid0_2_1`
   (first listed, no `Locked` attribute at all). See
   `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s new
   "`CellSelection_OnEnter`'s own default-cursor scan" section for the full
   decompile and confidence. `CellSelection::new`/`with_medals_and_records`
   (`crates/ui-screens/src/campaign.rs`) now reproduce it, with two new tests
   (`crates/ui-screens/src/campaign/tests.rs`). **Separately measured, still
   unreproduced**: the cursor **persists** across a back-out to `Grid
   Selection` and a re-entry (moved to `grid0_3_2`, backed out, came back in
   on `grid0_3_2`, not reset to `grid0_3_1`) - this build's own screen
   already holds `index` as live state for the session, so re-entering
   `Cell Selection` without tearing down the model would carry this for
   free, but that plumbing (does `CampaignStage` keep the `CellSelection`
   instance alive across a back-out today, or rebuild it) was not checked
   this pass.

Separately corrected, no doc change needed beyond `scripts/psp-frontend-capture.py`
itself (commit `7f2b7855`): **`TournamentLoad`'s autosave dialog was never
observed, on a fresh profile or a returning one, at a 50ms poll right on the
confirm press** - `MainMenu_Definition.xml`'s own `TournamentLoad` screen
authors a `NumOptions="0"` dialog with a `Redirect` gated on the same
`DialogMenu` value, which is consistent with the whole screen resolving in
fewer ticks than either walk's polling cadence catches, not with the screen
or dialog being unauthored. Say "no frame observed", not "never appears".

**Update, 2026-09-25, `pulse-campaign-nav` lane: `Grid Selection`'s own
left/right, reported dead by a maintainer playing this build, now works.**
`GridSelection::update` bound `Down`/`Up` to a single-tile wrapping step and
left `Left`/`Right` unbound, on the reasoning that no left/right arrow image
is authored - which does not follow, the screen's own `GridController` is a
four-hex row. Measured live against PPSSPP (`pulse-psp-usa.chd`, Xvfb):
`Down`/`Up` page by a full four-hex row, `Left`/`Right` step one tile within
the page, and **neither direction wraps** - both clamp at their own boundary,
and a page turn keeps the pressed-in slot rather than resetting to the new
page's own first tile. `GridSelection::page_step`/`tile_step`
(`crates/ui-screens/src/campaign/pointer.rs`) carry the corrected arithmetic; a new
`per_page` field keeps HD/Fury's own one-tile-per-page flyer pager on the old
wrapping `step` unchanged. Full walk in `docs/ui/campaign-screens.md`'s
"Measured against PPSSPP, 2026-09-25" section. **Separately confirmed
already fixed, not touched this pass**: the upper-case-label defect this
thread's own "What is still open after this pass" section below still names
as open was actually closed 2026-09-21 (`docs/ui/campaign-screens.md`'s own
"upper-case label note" is marked closed, `text_draw` routes through
`face_atlas_slot` now) - that bullet below is stale and should have been
struck when the fix landed; struck now. The three "headline differences"
(`"GRID N"` title, `"SINGLE RACE"` mode name, circuit display name) are also
already fixed, confirmed via a fresh `--menu-page grid-select` capture this
pass (`docs/ui/campaign-screens.md`'s `## Open` has the comparison). Left
open: everything else below, and see `docs/ui/campaign-screens.md`'s own
`## Open` for what this pass could not settle (the ticker in a `--menu-page`
capture, unrelated to this fix, was already a known, documented gap).

2026-09-14. `Grid Selection` and `Cell Selection` - the Race Campaign's own
two screens - now draw, off `Data\Plugins\PI001\GUI\CellMode_Definition.xml`
and the real 236-cell campaign
([`docs/formats/race-campaign.md`](../../docs/formats/race-campaign.md)).
This is the frontend-drawing half; the launch half is the other lane's, per
the gameplay handover's own instruction not to wire cell-selection without
first tracing the launch path.

**Update, same day: the launch landed too, in the `campaign-launch-wiring`
lane.** The title is kept only so this thread's index line still matches -
confirming a cell in one of the five modes this engine runs now opens `Team
Selection` and launches the cell's own race, evaluates the medal earned and
persists it keyed on the cell's own `name`, and feeds it back into
`Grid Selection`'s `Medals`/`Points` rows and `Cell Selection`'s `Line6`/
`Line7`. See `docs/ui/campaign-screens.md`'s "Confirming a cell launches"
and `docs/architecture/persistence.md`'s "where a career system attaches"
for the full detail. **Not closed by that pass**: everything below this
paragraph that is not struck through is still open, and it did not attempt
a live PPSSPP capture or an interactive play-test of the new launch either
- see `docs/ui/campaign-screens.md`'s own `Open` entry on why.

**Update, 2026-09-14, `campaign-picture` lane: the picture itself, against
the PPSSPP frames the `campaign-ppsspp-results` lane measured the same
day.** Most of six items landed: the hex grid draws `Outline_x_y` always
and `Medal_x_y` only where a medal/points are earned (was: `hex_filled.mip`
everywhere); `Lock_x_y`/`Lock_n_0` draw under
`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s measured
three-term rule on both screens, and a locked tile now refuses `Confirm`
too (chosen, not measured, on the exact refusal mechanism - see
`docs/ui/campaign-screens.md`'s "Grid tiers and cells lock and unlock");
`Grid Selection`'s `Title` resolves through a real per-grid idstring
(`"GRID 1"`, and `"PHANTOM GRID 1"` on `grid12`..`grid15` - not derivable
from a formula), `Cell Selection`'s `Title`/`Track Line` resolve through the
disc's own mode/circuit names, and the panel's five row labels (`Speed
class`/`Laps`/`Weapons`/`Points`/`Best`) resolve to real idstrings instead
of staying blank. **Not landed**: the tip ticker and the `Confirm`/`Back`
half of the button-legend footer - see this thread's `Open` section, which
replaces the two locking bullets below (struck through) with the measured
rule now implemented.

**Update, 2026-09-21, `pulse-campaign-ticker` lane: the tip ticker, the
`Confirm`/`Back` footer legend, `Cell Help`'s static overlay and `Line5`
(`Cell_SavedRecord`, Time Trial/Speed Lap) all now draw - and the podium walk
closed live**, `[ai] difficulty = "novice"` (worktree-local, never committed)
plus `--autopilot-skill ace` finishing `grid0_3_1` 1st of 8 and `Cell
Selection` reading `Points 3/3` / `Best Gold` back. New module:
`oag_ui_screens::campaign::footer`, reading `Skin.xml`'s `<NavigationController>`/
`<TextInfo>` directly off the raw parsed tree - neither tag is one
`oag_ui::screen::Screens::collect_widgets` recognises, and even parsed,
neither fits the per-screen widget model (a `NavigationController` picks
prompts *per screen*; the ticker is two alternating text buffers sharing one
clip viewport). Full detail in `docs/ui/campaign-screens.md`'s own
2026-09-21 section. **Still open below**: the `Grid`/`Grid1` duplicate
mystery, a live PPSSPP capture of the ticker/footer/`Cell Help` (this pass
verified only against this build's own live behaviour, not the original's),
the ticker's own scroll speed (chosen, not measured), and `Line8`/`Zone`/
`Elimination`'s own saved records (no raw count anywhere in
`oag_game::records::Record` to read).

**Update, 2026-09-25, HD/Fury's own three screens
(`Campaign Selection` -> `Grid Selection` -> `Cell Selection`): a
capture-only "impossible state" bug fixed, and the footer legend now draws
on all three screens rather than one.** `crate::capture::campaign_page`'s
HD arms read `campaign.grids` whole (all sixteen - base `Wipeout HD`'s
`grid0`..`grid7` plus `Fury`'s `grid8`..`grid15`) instead of one campaign's
own eight, so `--menu-page grid-select`/`cell-select` drew `Event 01/16`, a
state the live session never reaches (`CampaignStage::open_grid_selection`
always narrows first) and RPCS3's own frame never shows (`EVENT 01/08`).
Fixed by slicing to one campaign, defaulting to `Fury` - the measured
default and the campaign every RPCS3 reference frame on disk is actually
of. Separately, RPCS3's own frames show the `NAVIGATION`/`CONFIRM`/`BACK`
footer row on `Campaign Selection` and `Grid Selection` too, not `Cell
Selection` alone as the `pulse-campaign-ticker` lane's own note above
assumed ("matching Pulse's own scope") - `hd_grid_draw_list`/
`selection::draw_list` gained the same `footer_overlay` parameter
`hd_cell_draw_list` already had. Full detail, including the still-open
gaps (the button glyphs themselves, `Grid Selection`'s own third `CHANGE
DIFFICULTY` prompt, the 3-D flyer model), in
`docs/ui/campaign-screens.md`'s "`--menu-page` stills match a real screen
state, and the footer draws on all three, 2026-09-25" section.

**Update, 2026-09-25, `pulse-grid-look` lane: the coordinator's own crop
comparison against `grid-selection-page1-grid0-unlocked.png` found five real
render gaps the `pulse-campaign-nav` lane's "no new gap" re-check above (text
content only) had missed - all colour and number-format, none of them
launch-affecting.** `Medals`/`Required` were formatted backwards
(`"04/08"`/`"12"` instead of the disc's own unpadded `"0/8"`/padded
`"012"` - `race-campaign.md`'s own format-string table had `Required` wrong
too, `"%d"` where the literal actually reads `"%03d"`, now corrected).
Three colour gaps turned out to share one root cause: `CellMode_Definition.xml`
authors the selected-tile glow, the tier hex outlines and the page arrows all
at a flat, multiply-neutral default, and every one of them is actually
tinted by native code at runtime (`GridController_UpdateSelectorPulse`,
`GridSelection_PopulateTiles`, `GridSelection_UpdatePageTransition` - the
first and third renamed and documented this pass, `race-campaign.md`'s own
new "The runtime tint layer" section has the full decompile). Fixed:
`Selector` now draws the measured cyan endpoint of its own real pulse
animation (the pulse itself is not implemented - no clock reaches the
draw-list builder); `Grid Selection`'s tier hexes now draw the measured dim
cyan-teal tint; the page arrows now grey out at their own boundary. **Left
open, not guessed at**: `Cell Selection`'s own hex-outline tint (its XML
routes through an unresolved `FEGlobals->CM_HEX_Outline` this project has
not read the registry value of - `docs/formats/fexml.md`'s own open
question), the `SelectorGlow` halo widget (procedural, no authored
geometry/texture located), and a real but unconfirmed lead that the detail
panel's row-label/value text (`font="default"`) draws smaller than the
reference capture shows - pointing at `FaceScales::default()`'s own `13/22`
ratio rather than a per-widget fix, too broad a constant to change off one
pass's crude pixel measurement. The title bar's own face (`RACE CAMPAIGN`,
`handover/frontend/ps2-front-end-layout-is-hardcoded-to-480x272.md`'s open
question) was checked against this same frame and is **not settled by it** -
a scale-normalized crop reads the two as close to the same size, and a rough
measurement even reads the reference as slightly *larger*, the opposite
direction from what would justify shrinking this build's own title. Full
detail, every crop path and the exact pixel measurements:
`docs/ui/campaign-screens.md`'s 2026-09-25 bullet under `## Open`.

**Update, 2026-09-25 (second pass), the footer's own button glyphs and the
`GOLD MEDALS` denominator, both closed.** `ControlTextConfirmButton`/
`BackButton`/`DifficultyButtonIcon` (`font="buttons"`) now draw through a
genuine third face atlas (`oag_ui::language::roles::BUTTONS`,
`render::Renderer::set_buttons_atlas`) rather than being excluded - verified
against the disc first with a new scratch tool,
`cargo run -p oag-tools --example hd_buttons_font_probe`. The
`Grid Selection` "third prompt" the previous pass on this thread named as
open turned out not to exist: the RPCS3 capture it cited
(`rpcs3-grid0-3-2/00-default.png`) is `Cell Selection`, not `Grid Selection
Fury` - every clean frame in the same directory reads unambiguously as
`Cell Selection`, and neither archive's own `CellMode_Definition.xml`
authors a `DifficultyButton` on `Grid Selection` at all. `Campaign
Selection`'s own `GOLD MEDALS` denominator is now `{earned}/{total}` instead
of a bare numerator - `87` for `Wipeout HD`, `80` for `Fury`, both exactly a
campaign's own total cell count, matching RPCS3 on both sides. Closing that
surfaced and fixed a real, separate bug an earlier pass had found and
deliberately left alone: `grid_04.xml`'s own `<Values>` tag is missing a `>`
on the disc (all three copies), and `oag_tables::fexml::parse` used to drop
the whole grid because of it - RPCS3's own `"0 / 87"` is first-party
evidence the original tolerates the same break, so `fexml::tag_end` now
does too. Full detail, evidence and confidence in
`docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: the footer's button
glyphs, and the `GOLD MEDALS` denominator, 2026-09-25" section - **read that
before this file's own `## Open` below**, since it supersedes the "button
glyphs themselves" and "`CHANGE DIFFICULTY` prompt" lines the previous
paragraph left open.

Still open, not touched this pass: the `DIFFICULTY (<rung>)` runtime
template (`Cell Selection` still shows the disc's own authored `"Change
Difficulty"`) and this build's own difficulty default reading `SKILLED`
where RPCS3's fresh-profile default reads `NOVICE` - two mismatches on the
same widget, named together in the docs section above; `oag_ui_screens::endrace::hd`'s
own screens (`Results`/`Rewards`/`Menu`) still draw no button glyph at all,
since that module's own `text_draw` has no `face_role` check the campaign
screens' does; and whether Omega's own language plugins declare the same
`Buttons` slot is unverified (`omega-ps4-eu.pkg` is a raw PS4 package, not
directly openable the way the decrypted PS3 ISO is).

Milestone: **M7 - Shell and polish**.

Read first: [`docs/ui/campaign-screens.md`](../../docs/ui/campaign-screens.md)
(what draws, what is measured vs chosen, every number this pass read off the
disc), `oag_ui_screens::campaign` (`crates/ui-screens/src/campaign.rs`, model + layout +
draw), `oag_game::campaign` (`crates/game/src/campaign.rs`, the shared
read), `crates/game/src/main/campaign_stage.rs` and
`crates/game/src/main/session/campaign.rs` (the live flow).

## Open

- **2026-09-27, `pulse-ps2-fe` lane: fixed, not open any more - noted here
  only as a pointer.** The PS2 pressing drew both grid screens' lock icons
  with no hex cell shape under them at all (`hex_filled.mip`/
  `hex_outline.mip` never resolved: `crates/game/src/campaign.rs` read them
  with `archives.read_name` directly, which only tries the PSP-declared
  name, instead of `oag_pulse::read_image`, which retries the PS2 build's
  own `.mip`->`.pct` rewrite every other Pulse texture load already gets).
  Fixed in `crates/game/src/campaign.rs`; PSP output unchanged. This thread
  otherwise never mentions the PS2 pressing at all - everything below is
  PSP/HD, unaudited on PS2.
- ~~`Cell Selection`'s own hex-outline colour is unresolved.~~ **Done -
  never actually open, `pulse-campaign` lane, 2026-09-28.** The
  `fallback_globals` plumbing landed 2026-09-14 already resolves it to
  `Skin.xml`'s own `0x7F34ACC2`; this bullet's "not one of the two
  confirmed names" claim was stale against the code the day it was written.
  Confirmed against a fresh live PPSSPP capture, not just static reading -
  see this file's own 2026-09-28 update paragraph above.
- **`SelectorGlow` is not drawn - refined twice, still open,
  `pulse-cellsel` lane, 2026-09-28.** An eight-frame burst confirms a real,
  phase-synced halo the sprite's own zero-alpha padding cannot explain by
  itself; direct extraction of `CellMode_Definition.xml` and `Skin.xml`
  (`oag-wad cat --expand`) now confirms the literal string `"SelectorGlow"`
  appears in neither file at all, so the lookup's own null-guard always
  fails here and this is dead code for Race Campaign, not an unlocated but
  real widget. See this file's own 2026-09-28 update paragraph above and
  `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s matching
  section.
- ~~The detail panel's `default`-role text may be undersized.~~ **Fixed,
  `pulse-campaign` lane, 2026-09-28** - a real bug, not too broad a
  constant to touch: `crates/ui-screens/src/campaign.rs`'s own `Layout::face_scale`
  was double-scaling `"default"`-role text against the real `Default` atlas
  `face_atlas_slot` already loads at native size, the same shape
  `oag_ui_screens::campaign::footer::face_scale` was already fixed for one widget
  over. `FaceScales::default()` itself was never touched. See this file's
  own 2026-09-28 update paragraph above.
- ~~PPSSPP was not captured against this pass.~~ **Captured, `pulse-campaign`
  lane, 2026-09-28** - the first live capture of this leg, folded into
  `scripts/psp-frontend-capture.py` the same day (commit `7f2b7855`). See
  this file's own 2026-09-28 update paragraph above for what it settled and
  what it didn't (the `Grid`/`Grid1` crossfade question below is still
  open).
- **The `Grid`/`Grid1` duplicate `GridController` is read but not resolved.**
  `Grid Selection` authors two identical controllers at one position - `Grid`
  (`focus="true"`, a staggered `delay` reveal on its own hexes) and `Grid1`
  (`startenabled="false"`, otherwise byte-identical minus `delay`). This
  build skips whichever carries `startenabled="false"`, which is safe
  (`docs/ui/campaign-screens.md` says why - a name collision in
  `screen.images` otherwise) but does not settle *why* two exist: an
  entrance-animation buffer that gets swapped to the settled state, or a
  paging crossfade between four-tile pages. A PPSSPP capture across a page
  turn would likely settle it.
- ~~`Cell Help`'s own text is resolved but the overlay is not drawn.~~
  **Done, 2026-09-21** - draws as a static panel, `oag_ui_screens::campaign::draw::cell_help_draw`.
  Its `Viewport`/`Animation` scroll timeline is still not scripted, on
  purpose - see `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~Grid-tier locking draws every tier open.~~ **Done, 2026-09-14**, in the
  `campaign-picture` lane, once a per-cell save existed to compare
  `Grid_PointsEarned` against and `race-campaign.md`'s own later pass traced
  the actual glyph rule. See the update paragraph above.
- ~~`Locked`/`Lock_x_y` is deliberately unimplemented.~~ **Done, 2026-09-14**
  - `race-campaign.md`'s own confidence rose to 82-85 once its "Unlock
    rules, cell and tier" section traced the consumer in full; the
    `campaign-picture` lane implemented the draw side the same day. See the
    update paragraph above.
- **PPSSPP measured 2026-09-14 - `docs/ui/campaign-screens.md`'s "Measured
  against PPSSPP" section has the full readings and frame paths
  (`data/reference/psp-campaign-screens/`, gitignored).** Three corrections
  to this file's own prose above, all confirmed live rather than inferred:
  `Grid Selection`'s `Title` reads `"GRID 1"`/`"GRID 5"`/`"GRID 9"`, not the
  raw `grid->name` this file and `campaign-screens.md` both assumed - the
  "runtime placeholder" reading of the XML's own `string="GRID 1"` was
  backwards. `Cell Selection`'s `Title` is a localised mode name
  (`"SINGLE RACE"` for a `Race` cell, matching the Racebox's own `RACE TYPE`
  wording), not `cell->mode`'s bare enum spelling. The `Track Line` is the
  circuit's own display name (`"Moa Therma White"`, `"Talon's Junction
  White"`), not the raw `03_Track`/`16_Track` string. Also new: a scrolling
  tip ticker along the bottom of both screens (not documented anywhere
  before this pass, not drawn here at all), and behavioural evidence
  (confidence 70, not breakpoint-verified) that **grid-tier locking gates
  `Confirm` in the original** - pressing it on a `Locked` grid does nothing -
  which is stronger than "cosmetic lock glyph" and worth weighing against
  the "chosen, not measured" note two bullets up. `race-campaign.md`'s own
  `Locked`/`Lock_x_y` reading (the *cell*-level glyph, a different byte) was
  independently settled by a different pass the same day; this file's bullet
  above still describes this build's own choice not to implement it, which
  stands unless whoever picks this thread up next decides otherwise.

## Next Steps

- ~~Decompile `CellSelection_OnEnter` (`0x088d59c4`)'s own default-cursor
  logic.~~ **Done, `pulse-campaign` lane, 2026-09-28** - see this file's own
  2026-09-28 update paragraph, point 4: the first-visit default (skip a
  cell whose `Locked` byte is absent or `true`, select the first explicit
  `false`) is decompiled and implemented. **Still open**: the cursor
  persisting across a back-out/re-entry is a separate mechanism, measured
  live but not traced in the decompile. **Reproduced 2026-09-28**
  (`pulse-campaign-flow` lane, `CellCursors` in `campaign_stage.rs`), see
  `docs/ui/campaign-screens.md`'s Open entry; the per-grid keying and
  forgetting it on leaving the campaign are chosen, not measured, and the
  decompile of where the original keeps it is still untraced.
- ~~Wire the launch, once the other lane's trace lands.~~ **Done,
  2026-09-14**, in the `campaign-launch-wiring` lane -
  `Session::launch_campaign_cell` (`crates/game/src/main/session/campaign.rs`)
  is the caller; see `docs/ui/campaign-screens.md`'s "Confirming a cell
  launches" and `docs/architecture/persistence.md`. The track/mode spelling
  mismatch this bullet worried about turned out not to be one: a cell's own
  `track=` id already shares `catalogue::Track::id`'s spelling, so
  `Shell::track(mode, id)` resolves it directly.
- ~~Feed `oag_game::records::Observation::campaign_medal` once a cell is
  selected.~~ **Done, 2026-09-14.** `RaceStage::campaign_medal`
  (`crates/game/src/main/race_stage.rs`) evaluates the per-mode value -
  narrower than this bullet assumed: `Observation::tick` turned out to be
  the wrong quantity for `Speed Lap` (a mode that never finishes, so "the
  tick it was left on" is not "how fast" - checked against
  `grid_00.xml`'s own authored targets, see
  `docs/architecture/persistence.md`), which reads `best_lap_ticks`
  instead; `Zone`'s zone count and `Elimination`'s kill count did turn out
  to be reachable without widening `Observation` at all - `RaceStage`
  already has `self.race` in hand, so the value is computed there and only
  the already-evaluated medal crosses into `Observation`.
  `Grid Selection`'s `Medals`/`Points` and `Cell Selection`'s `Line6`/
  `Line7` all read real numbers now, fed through a `Fn(&str) ->
  Option<Medal>` closure rather than the store type itself - see
  `oag_ui_screens::campaign::GridSummary::from_grid_with_medals`/
  `CellSelection::with_medals`. ~~`Line5`/`Line8` are unchanged.~~ **`Line5`
  done, 2026-09-21** - `CellSelection::with_medals_and_records` reads the
  general per-track/mode/class `oag_game::records::Store` for Time Trial/
  Speed Lap. **`Line8` still blank on purpose** (collides with `Target0`'s
  own row) and **`Zone`/`Elimination` still have no raw count anywhere in
  `oag_game::records::Record` to read** - see
  `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~A PPSSPP capture of the walk above~~, to move every confidence score in
  `docs/ui/campaign-screens.md` from "read off the XML and the decompile"
  to "measured against a live frame". **Done, 2026-09-14**, in the
  `campaign-ppsspp-results` lane - see `docs/ui/campaign-screens.md`'s
  "Measured against PPSSPP" section, which that lane owns; do not edit
  inside it.
- ~~The scrolling tip ticker and the `Confirm`/`Back` half of the footer
  legend still do not draw.~~ **Done, 2026-09-21** - `oag_ui_screens::campaign::footer`.
  The ticker's own scroll speed is chosen, not measured, and the
  `Grid`/`Grid1` duplicate `GridController` question above is still open.
- **A previous live walk's cell (`grid0_2_1`) is locked under the rule this
  pass implemented.** The "interactive play-test" bullet below drove
  through `grid0_2_1` on a fresh profile; that cell authors no `Locked`
  attribute (defaults locked) and has no medal or medalled neighbour, so a
  repeat of that walk today would refuse `Confirm` rather than reach `Team
  Selection`. `grid0_3_1`/`grid0_3_2` (`Locked="false"`) are the cells to
  re-walk with.
- ~~An interactive play-test of the new launch.~~ **Driven live, 2026-09-14**,
  in the `campaign-pointer` lane, mouse-only under Xvfb: `Cell Selection`
  confirming, `Team Selection`, an `--autopilot` race finishing and the
  results table all confirmed by screenshot. ~~Not fully closed: both runs
  finished 4th of eight... `Line6`/`Line7` were only seen reading the *no
  medal* state.~~ **Closed, 2026-09-21** - a worktree-local `[ai] difficulty
  = "novice"` (never committed) plus `--autopilot-skill ace` podiumed
  `grid0_3_1` 1st of 8; `Cell Selection` read `Points 3/3` / `Best Gold`
  back. See `docs/ui/campaign-screens.md`'s 2026-09-21 section.
- ~~Cell Help's static overlay.~~ **Done, 2026-09-21.**

## What is still open after this pass

- **The `Grid`/`Grid1` duplicate `GridController`** - still read but not
  resolved, see above.
- **No live PPSSPP capture of the ticker, the `Confirm`/`Back` legend or
  `Cell Help`'s overlay** - all three were verified against this build's own
  live behaviour under Xvfb, not against the original.
- **The ticker's own scroll speed** is chosen (`crate::anim::MARQUEE_SPEED`
  reused), not measured.
- **`Line8` and `Zone`/`Elimination`'s own saved records** - `Line8` would
  collide with `Target0`'s own row if drawn unconditionally, and neither
  mode has a raw zone/kill count anywhere in `oag_game::records::Record` to
  read at all; both are real gaps, not implemented this pass.
- ~~This whole screen renders every label upper-case~~ **Fixed 2026-09-21**,
  stale bullet struck 2026-09-25 - the atlas gap this bullet itself named as
  blocking a fix landed (`crates/game/src/boot/fonts.rs`'s
  `face_atlas_slot`), and `oag_ui_screens::campaign::draw::text_draw` now routes
  through it. See `docs/ui/campaign-screens.md`'s own "upper-case label
  note", confirmed still fixed by this pass's own fresh
  `--menu-page grid-select` capture.
- ~~`AI difficulty (Medium)` reads `CHANGE DIFFICULTY`~~ - **fixed,
  `pulse-cellsel` lane, 2026-09-28.** `oag_ui_screens::campaign::draw::difficulty_button_line`
  reproduces `CellSelection_Update`'s own runtime template - see this
  file's own 2026-09-28 update paragraph above.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-21. `Grid Selection`/`Cell Selection` draw and launch, off `CellMode_Definition.xml` and the real 236-cell campaign; the tip ticker, the `Confirm`/`Back` footer legend, `Cell Help`'s static overlay and `Line5` (`Cell_SavedRecord`, Time Trial/Speed Lap) all now draw too, via a new `oag_ui_screens::campaign::footer` reading `Skin.xml`'s `<NavigationController>`/`<TextInfo>` directly (neither tag is one `Screens::collect_widgets` recognises). The podium walk closed live: a worktree-local `[ai] difficulty = "novice"` plus `--autopilot-skill ace` podiumed a campaign cell 1st of 8 and `Cell Selection` read `Points 3/3` / `Best Gold` back. Open: the `Grid`/`Grid1` duplicate `GridController`, no live PPSSPP capture of the ticker/footer/`Cell Help` (verified only against this build's own behaviour), the ticker's own scroll speed (chosen, not measured), `Line8` and `Zone`/`Elimination`'s own saved records (no raw count anywhere in `oag_game::records::Record`)
