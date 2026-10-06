# Wipeout HD/Fury's own EndRace screens: `Results`, `Menu`, `Rewards`, `Podium`

**Status: `DATA02`'s copy read widget by widget; `DATA03`/`DATA04`/`DATA05`/`DATA06`
diffed against it. `Rewards` settled as never entered (2026-09-25).** `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` - a
named plugin, plain UTF-8, **not** `oag_tables::fexml`-shortened, the same
divergence [`hd-frontend.md`](hd-frontend.md) already records for
`skin.xml`/`CellMode_Definition.xml` - in five of HD's seven archives, no two
copies alike by MD5:

| Archive | Size | Screens it declares |
| --- | --- | --- |
| `DATA02` | 30,110 | Results, **Rewards**, Menu |
| `DATA03` | 30,623 | Results, **Rewards**, Menu |
| `DATA04` | 31,053 | Results, **Rewards**, Menu |
| `DATA05` | 42,406 | Results, **Rewards**, Menu, **Podium** |
| `DATA06` | 41,190 | Results, Menu, **Podium** - no Rewards |

Every copy also carries `Kill Game Transition`, `InGame Restart Transition`,
`EndRaceSaveGhost` and `EndRaceDeleteGhost` (transition/memory-card screens,
not read here). Sizes match [`hd-frontend.md`](hd-frontend.md#the-end-screens-are-located-not-read)'s
own inventory exactly. This page is the widget-by-widget reading that
inventory was missing.

`oag_assets::Archives::holder_of` serves **`DATA02`'s** copy: it walks `data`
(`DATA00`, no copy) then `fe` (`DATA02`) before `extra` - the same precedence
[`hd-frontend.md`](hd-frontend.md#six-skins-one-layout) documents for
`skin.xml`. **This is this build's own mount order, not a measurement of the
original's** - nothing here settles which copy a real PS3 loads. Confidence
94 for the inventory table (a file list plus five direct reads); confidence
90 for the widget table below (quoted from `DATA02`'s and `DATA06`'s copies
directly, per the [confidence rubric](../reverse-engineering/confidence-rubric.md)'s
"clean read of authored data" band) unless a row says otherwise.

## Reading it yourself

```sh
python3 scripts/psarc.py cat \
    'data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC' \
    /data/plugins/frontend/gui/endrace_definition.xml
```

`fd`/`rg` respect `.gitignore` and return nothing under `data/`; use
`--no-ignore`, or `/bin/ls`/`find`/`grep` directly.

## `EndRace Results`: a whole-field standings grid, not a per-lap table

Pulse's own `EndRace Results` ([`endrace-screens.md`](endrace-screens.md)) is
the player's own per-lap splits. HD's is the finishing order of the **whole
field** - `Grid{col}.{row}`, **four columns by ten rows**, not the "eight rows
by ten columns" the inventory pass guessed before this one read the file.

| Widget | Authored as | Confidence |
| --- | --- | --- |
| `ResultsTitle` | `idstring="ER_RES"`, `font="Title"`, inside `Item OffsetX="375" OffsetY="143"` | 95 - direct read |
| `Line1` | `string="race complete!"` **literal**, no `idstring` at all | 95 that it is a literal placeholder; see below |
| `GridHead1` | `<Block>`, `x="0" y="24" width="497"`, `IDString="IG_HUD_POS"` | 95 |
| `GridHead2` | `<Block>`, `x="505" y="24" width="286"`, `IDString="IG_HUD_TIME"` | 95 |
| `GridHead3`/`GridHead4` | `<Block>`, `x="0" y="24" width="0"`, `IDString="X"` | 95 that these are inert - `width="0"` and the idstring is the literal letter `X`, not a real label |
| `Grid{col}.{row}` | 40 `<Text>` widgets, `col` 0-3, `row` 0-9, every one `x="0" y="0"` | 90 - the shape is measured, the positions are a runtime fill (see below) |
| `Gridp.{row}` | `<Text>`, `IDstring="ER_PERFECT"`, `row` 0-9 | 90 - a per-row "perfect lap" indicator, direction not traced |
| `Grid{n}.h` | `<Text>`, `n` 0-2 only, `color="0xffffffff"` (white, against every other Grid text's `0xff646464` grey) | 60 - three widgets, not four or ten; a highlighted-row variant of some subset of columns, which subset undetermined |
| `Gridi.{row}` | `<Image>`, `row` 0-9, `src="Data\Ships\<Team>\fe\miniBW.tga"` | 90 that this is a per-row ship badge; 40 on which team, since every copy names a fixed one or two teams (`Feisar` twice in `DATA02`'s own padding rows, `Auricom`/`Harimau` in `DATA03`/`04`/`05`) regardless of who raced - a runtime fill, not real data |
| `GridStrikeThrough` | `<Image>`, `x` 5-20, `y="391"`, differs across copies | 55 - a disqualified/eliminated-row strike, untraced |
| `GridHighlight` | `<Image>`, `color="FEGlobals->HD_Blue"`, no `src` - a [`Fill`](../../crates/ui/src/screen.rs), `x="1" y="391" width="789" height="38"` | 90 that this is the selected/player row marker (its authored `y` sits on the grid's own bottom edge, the same "default position is a template, not a real row" shape Pulse's `tablehighlight` already is) |
| `GridSideBarL`/`GridSideBarR`/`GridTopBar`/`GridBottomBar`/`GridBottomBlock` | plain colour fills, `x`/`y`/`width`/`height` all authored (`y="44" height="347"` on the two side bars) | 95 - the grid's own frame |
| `Target Title` / `MedalBlock` | two widgets at the **identical** `OffsetX="1180" OffsetY="532"` | 85 that these are mutually exclusive (target display vs. medal-earned display); untraced which condition picks between them |
| `Target0`/`Target1`/`Target2` | `<Text>`, placeholder `string="value"`, each paired with a `Hexmedal_HD.mip` icon | 90 - the three medal-target figures (gold/silver/bronze cutoffs), same idiom as `IG_HUD_TARGET` elsewhere |
| `MedalModelGold`/`Silver`/`Bronze` | `<ImageModel>`, `Src="Data\FE\Trophies\hd_gold.vex"` etc., `StartEnabled="false"` | 90 - a 3-D trophy, not a 2-D image; `oag_ui::screen::Screens::collect_widgets` has no `"imagemodel"` arm, so none of the three is collected at all |
| `RecordNotifyBlock` | `<Text>` group, `ER_RECORD_NOT` caption, `RecLogoSL`/`TT`/`Z` mode icons, `RecValue` | 85 - a "new record" banner, untraced trigger |
| Loyalty block (`Item OffsetX="1180" OffsetY="368"`) | `ER_LOYSTAT` caption, two `<Block>`s (`ER_LOY`/`IG_HUD_TOTAL`), `loyalty1.1`/`loyalty1.2`/`loyalty2` placeholder text (`"834 POINTS"`/`"3745"`) | 85 - **on `Results`, not a separate `Rewards` screen the way Pulse keeps it**; read and drawn 2026-10-02 - [`endrace-loyalty.md`](../ghidra/functions/ps3-hdfury-eu/endrace-loyalty.md), no bar |
| `DelayPostMsg` | `idstring="ONL_MSG_DELAYPOST"` | 95 - online-only |
| `RecordsCycleButton`/`RecordsCycle` | `δ` glyph / `idstring="ER_GLOB_REC"` | 95 - online leaderboard cycling, no binding possible offline |
| `ControlTextConfirmButton`/`ControlTextConfirm` | `FE_CONFIRM_BUTTON`/`FE_CONFIRM` | 95 - direct idstrings, same convention as Pulse; see "A malformed tag upstream" below for the parser bug that used to keep this screen alone from drawing either one, fixed 2026-09-25 |
| `EndRaceMenuRedirect` | `<Default goto="Race End Save">` | 90 - confirm always goes through this intermediate save-transition screen, not straight to `Rewards`/`Menu` |

### The grid is four columns, not eight rows - and only two of the four are real

`GridHead1`..`GridHead4` are the disc's own column headers, and only the
first two carry real idstrings (`IG_HUD_POS`, `IG_HUD_TIME`); the other two
are the literal string `X` at `width="0"` - inert placeholders, not a gap
this reading introduced. `Grid{col}.{row}` follows that same 4-wide, 10-tall
shape exactly: `Grid0.0`..`Grid3.9`, forty widgets, confirmed identical in
`DATA02` and `DATA06`. **This project's own build draws only columns 0 and
1** (position, time) - see [`oag_ui_screens::endrace::hd`](../../crates/ui-screens/src/endrace/hd.rs)'s
module doc.

### Every `Grid{col}.{row}` cell is authored at `x="0" y="0"` - position is computed, not stated

Unlike Pulse's own per-lap table, which authors each `lap{n}.{c}` at its own
real `x`/`y`, none of HD's forty grid cells carries a position at all. The
frame around them **is** measured (`GridSideBarL`'s own `y="44" height="347"`,
inside the `Item` that offsets the whole block by `375, 368`), so the row
geometry this project's own drawing uses was first derived from that frame
divided by a row count this file does not itself state.

**2026-09-28: the positions are the code's, and now read.** The screen's own
code lays the grid out per mode, and the frame this file authors is the Time
Trial / Speed Lap one: on a race, `EndRaceResults_LayoutGrid` (`0x0022c068`)
stretches the side bars to `443`, drops the bottom bar to `487` and hides
`GridBottomBlock`, and `EndRaceResults_FillRaceRows` (`0x0022b688`) puts row
`r` at `y = 96 + 45 r` with columns at `x = 40`/`200`/`545` - so the time
is `Grid2`, not `Grid1`, and `Grid1` is a per-racer string not yet
identified. Confidence 75 for "mode 3 is a single race", 78-80 for the
numbers. Full reading:
[endrace-results-grid.md](../ghidra/functions/ps3-hdfury-eu/endrace-results-grid.md).

### `Line1` carries no idstring - the headline is a runtime fill

`Line1`'s `<Values string="race complete!">` has no `idstring` attribute at
all, unlike every other headline-shaped text on this file. This project draws
it anyway, reusing the same idstring table Pulse's own `Line1` resolves
through (`ER_TT_COM`/`ER_SL_COM`/`ER_1STP`..`ER_8THP`/`ER_SHIP_DES`) -
**chosen, not measured**: no capture confirms HD fills this literal
placeholder with these exact strings, but `EndRace Menu`'s own `<Block>`
idstrings (`ER_RACE_AGAIN`, `ER_RETURN_GRID`, etc.) are verbatim identical to
Pulse's, which is the corroboration that this is shared engine vocabulary
rather than a Pulse-only table.

### A malformed tag upstream swallowed `NavigationController` on this screen alone - fixed 2026-09-25

`crate::screen::Screens::collect_widgets` walks a `NavigationController` the
same as any other container - see [campaign-screens.md](../ui/campaign-screens.md)'s
2026-09-25 entry for the mechanism - and this drew `CONFIRM` at the authored
position on `EndRace Menu`/`EndRace Rewards` immediately. **`EndRace
Results` alone did not, and it was not that reading's own bug**: `DATA02`'s
own copy of the screen authors `MedalModelGold`/`Silver`/`Bronze` (the
loyalty-block trophy widgets, `x="1180" y="532"` `Item`) as

```xml
<ImageModel name="MedalModelGold" ...>
  <Values Src="Data\FE\Trophies\hd_gold.vex" ... RotY="-0.5"</Values>
</ImageModel>
```

three times, one per medal - every `<Values>` here is missing the `>` that
should close its own opening tag before `</Values>` appears. `oag_tables::fexml::parse`'s
own `tag_end` (`crates/tables/src/fexml.rs`) tracked quoted `>` correctly but
had no recovery for a missing one: with no unquoted `>` anywhere in the
malformed `<Values ...>`, the scan ran on into the *literal text*
`</Values>` and treated **its** `>` as the one that closes the opening tag -
so the `Values` node was pushed onto the parse stack still open, and
everything the file authors afterwards (`MedalModelSilver`/`Bronze`,
`RecordNotifyBlock`, `DelayPostMsg`, the divider `<Image>`, and
`NavigationController` itself) became a *descendant* of that wrongly-open
node instead of a sibling of `EndRace Results`. Confirmed directly:
extracting just the `NavigationController` fragment into its own
well-formed file parsed and resolved both `Confirm` widgets correctly in
isolation; extracting the whole screen (malformed tags included) found
none of `screen.texts` named `ControlText*` at all, out of 69 collected.
This was a pre-existing gap in `fexml::tag_end`'s own recovery, not
specific to `NavigationController` - `MedalModelGold`/`Silver`/`Bronze`
were already uncollected before the 2026-09-25 prompts pass (no `"imagemodel"`
arm in `collect_widgets`, per the table above), so nothing that used to draw
stopped drawing, but nothing downstream of the first malformed tag in this
one screen ever reached `screen.texts` either.

**Fixed the same day, in `oag_tables::fexml` directly**: `tag_end` now
treats an unquoted `<` met mid-tag as proof the previous tag was never
closed, recovering the missing `>` in place - see
[fexml.md](fexml.md#recovering-a-start-tag-missing-its-own--fixed-2026-09-25-876b5d4d) for
the rule, its evidence, and a census proving it changes nothing on every
other fexml file this project reads except a small, named set of *other*
instances of the identical authoring bug (also fixed by the same change).
`EndRace Results` now collects `NavigationController` and draws `CONFIRM`
the same as `EndRace Menu`/`EndRace Rewards` - confirmed live
(`--menu-page endrace-results`, 1280x720) and by
`crates/game/tests/hd_endrace_ground_truth.rs`'s
`hd_endrace_results_draws_its_confirm_prompt_despite_the_malformed_tag`
against the real disc.

## `EndRace Rewards`: authored, never entered

`DATA02`/`03`/`04`/`05` all carry an `EndRace Rewards` screen; `DATA06` does
not. **The original never enters it** - confidence 85, settled from the data
and the executable's strings without a decompile (2026-09-25):

1. **No redirect names it.** Every `goto=` in every `.xml` of all seven
   archives (`DATA00`-`06`, extracted with `scripts/psarc.py extract ... .xml`)
   was grepped for `EndRace Rewards`: zero hits. The only references are the
   screen's own `<Screen name=...>` lines. HD's `EndRace Results` authors one
   exit, `EndRaceMenuRedirect` -> `Race End Save`; `Race End Save`
   (`ingame_definition.xml`, `SkipTarget="EndRace Menu"`) goes to `EndRace
   Menu`. Rewards' own `EndRaceRewardsMenuRedirect` also goes to `Race End
   Save` - an exit with no entrance.
2. **The executable registers no screen class for it.** The decrypted
   `EBOOT.elf` (`data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/`) has a type
   string and a source file for each end screen it can build -
   `EndRace Results`/`EndRaceResults_Screen.cpp`,
   `EndRace Menu`/`EndRaceMenu_Screen.cpp`,
   `EndRace Podium`/`EndRacePodium_Screen.cpp`,
   `EndRace Photo`/`EndRacePhoto_Screen.cpp` - and **zero** strings matching
   `reward` in any case, ASCII, UTF-16LE or UTF-16BE. None of the Rewards widget names
   (`RewardLine1`, `BigPos`, `MedalImg`, `loyaltybar`) appear either, while
   Results' own `loyalty1.1`/`loyalty1.2`/`loyalty2` sit in the
   `EndRaceResults_Screen.cpp` string cluster (`0x007945b0`), beside
   `ER_POINTS`. HD moved the loyalty readout onto `Results`; `ER_TOT_LOY` is
   referenced from the `ShowUnlocks_Screen.cpp` cluster (`0x788e88`), not
   from any end screen.
3. **The redirects are data-driven.** Neither `EndRaceMenuRedirect` nor
   `EndRaceRewardsMenuRedirect` is a string in the executable, so no code
   picks between them by name.

Not checked: `DFENGINE.SPRX` beside the EBOOT is SCE-encrypted and was not
grepped. It would have to build the screen from a type string it holds
itself, with no redirect naming the screen - which is why this is 85, not
higher. A live RPCS3 end-of-race walk would settle it.

Widgets, off `DATA02`'s copy (confidence 90, direct read):

| Widget | Authored as | Drawn by this build |
| --- | --- | --- |
| (unnamed backdrop) | `<Image>`, `x="-288" y="-200" width="2496" height="1480" color="0xc0000000"` | yes |
| (unnamed title) | `idstring="ER_REWARD"` (`"REWARDS"`), `font="Title"`, `x="480" y="240"` | yes |
| three dividers | `<Image>`s at `y` 292/465/690, `width="960" height="4"`, white | yes |
| `MedalImg` / `LoyaltyImg` | `<Image>`, `32x32` at `600,360` / `600,530`, `Color="0xff8AC0CA"`, **no `src`** | no - a src-less icon the original would assign at run time |
| `BigPos` | `<Text>`, `string="1"`, `align="centre"`, `x="610" y="370"` - centred inside `MedalImg`'s square | the player's finishing place (chosen) |
| `RewardLine1` | `idstring="ER_MEDAL_AWARD"` (`"MEDAL AWARDED:"`), `x="670" y="360"` | the medal tier on a campaign race (chosen) |
| `RewardLine2` / `RewardLoyaltyPoints` / `RewardLoyaltyActive` | placeholders `"test"` / `"points!"` / `"line 2"` | no - HD never enters this screen; its loyalty is on `Results` (`loyalty1.1`/`loyalty2`, drawn since 2026-10-02) |
| `loyaltybar` | `<Slider>`, `idstring="ER_TOT_LOY"`, `minSlide="0" maxSlide="100000"` | no - `oag_ui::screen` does not collect a `<Slider>` |
| `ControlTextConfirmButton` | inside a `<NavigationController>`, `font="buttons"` | no - `oag_ui_screens::endrace::hd`'s own `text_draw` has no `face_role` check, so this screen still risks the raw codepoint (a Greek letter) rather than the disc's own glyph even though a `Buttons`-role atlas now loads (2026-09-25, `oag_ui_screens::campaign::footer::face_role`); a real gap, left open rather than fixed in the same pass that found it - see `oag_ui_screens::endrace::hd::hd_results_draw_list`'s own doc |
| `ControlTextConfirm` | inside the same `<NavigationController>`, `idstring="FE_CONFIRM"` | **yes, since 2026-09-25** - `crate::screen::Screens::collect_widgets` now walks a `NavigationController` the same as any other container; confirmed live, `--menu-page endrace-rewards` |
| `EndRaceCountDown` | `string=""` | no |

`oag_game::endrace::load_hd` reads the layout when the served copy has it
and leaves `EndRaceScreens::rewards` `None` otherwise, so `DATA06`'s copy
still loads Results and Menu. The drawing rules, and which of them are
chosen, are `oag_ui_screens::endrace::hd::hd_rewards_draw_list`'s own doc; the
picture half is [`docs/ui/endrace-screens.md`](../ui/endrace-screens.md)'s HD
section.

## `EndRace Menu`: one `<Block>` per option, not a populated list

| Widget | Authored as | Confidence |
| --- | --- | --- |
| (unnamed title text) | `idstring="FE_MENU"`, `font="Title"` | 95 |
| `Endrace Difficulty` | `<List>`, `IDstring="ER_RACE_AGAIN"`(!), entries `Easy`/`Medium`/`Hard`, same `x="375" y="235"` as the `race_again`/`next_race` Blocks | 55 - untraced binding, same confidence Pulse's own `endrace-screens.md` gives its counterpart |
| `next_race` | `<Block>`, `IDString="ER_NEXT_RACE"`, `x="375" y="235" width="520"` | 95 - Tournament-only, unreachable by this engine |
| `race_again` | `<Block>`, `IDString="ER_RACE_AGAIN"`, `x="375" y="235" width="520"` | 95 |
| `return_to_grid` | `<Block>`, `IDString="ER_RETURN_GRID"`, `x="375" y="285" width="520"` | 95 |
| `return_to_menu` | `<Block>`, `IDString="ER_RETURN_MENU"`, `x="375" y="285" width="520"` | 95 |
| `quit_tournament` | `<Block>`, `IDString="ER_QUIT_TOUR"`, `x="375" y="285" width="520"` | 95 - Tournament-only, unreachable |
| `return_to_lobby` | `<Block>`, `IDString="IG_PAUSE_QUIT"`, `x="375" y="285" width="520"` | 95 - multiplayer-only, unreachable |
| `view_again` | `<Block>`, `IDString="ER_VIEW_AGAIN"`, `x="375" y="335" width="520"` | 95 |
| `view_MP_again` | `<Block>`, `IDString="ER_VIEW_AGAIN"`, `x="375" y="235" width="520"` | 95 - multiplayer-only, unreachable |
| `ControlTextConfirmButton`/`ControlTextConfirm` | `FE_CONFIRM_BUTTON`/`FE_CONFIRM` | 95 |
| `<Redirect>` table | `next_race`->`Load Next Race`, `race_again`->`InGame Restart Transition`, `return_to_grid`/`return_to_menu`->`Kill Game Transition`, `quit_tournament`/`return_to_lobby`->`Kill Game Multiplayer`, `view_again`/`view_MP_again`->`EndRace Results`, `Endrace Difficulty`->`InGame Restart Transition` | 95 - direct read |

**Every option Block also authors its own look**, which a first read of
this table left out: `selectable="true" arrowcolor="0xffffffff"
shaped="true" Color="0xff646464" textcolor="FEGlobals->HD_White"`. What those
mean - a grey box that turns `ActiveColor` (unauthored here, so the
constructor's `0xff8ac0ca`) when focused, grows sixty units, and blinks a
32x32 arrow - is `Block_ParseXml`/`Block_Update`, on
[menu-blocks.md](../ghidra/functions/ps3-hdfury-eu/menu-blocks.md#a-standalone-block-parse-selectable-update-2026-09-28).
Confidence 80.

Five of these eight idstrings (`ER_NEXT_RACE`, `ER_RACE_AGAIN`,
`ER_RETURN_GRID`, `ER_RETURN_MENU`, `ER_VIEW_AGAIN`) are **byte-identical**
to the ones [`oag_ui_screens::endrace::MenuOption::idstring`](../../crates/ui-screens/src/endrace.rs)
already returns for Pulse - not a coincidence of two titles reusing similar
English, but the same engine-wide idstring vocabulary, which is why this
project's build reuses `MenuOption` unchanged for HD rather than inventing a
parallel enum. `quit_tournament`/`return_to_lobby`/`view_MP_again` have no
`MenuOption` variant and are not drawn - Tournament and multiplayer, neither
implemented by this engine, the same "a cell whose mode maps to nothing must
not launch" rule [`race_mode_for_cell`](../../crates/game/src/campaign.rs)
already applies elsewhere.

There is **no ghost row on this screen at all** (`GhostLine1`/`GhostLine2`/
`GhostTime1`/`GhostTime2`, which Pulse's own `EndRace Menu` carries, do not
appear anywhere in `EndRace_Definition.xml`) - not a gap either build
introduces, the disc's own HD copy simply does not author one.

### `DATA06` differs only in colour and completeness, not in vocabulary

Diffed directly against `DATA02`: every `<Block>` on `DATA06`'s `EndRace
Menu` carries an extra `ActiveColor="FEGlobals->HD_Blue"` attribute (a
selection tint `DATA02`'s copy does not author), and the backdrop `<Image>`
is `FEGlobals->HD_transBG` rather than a literal `0xe0ffffff` - cosmetic
differences the widget names and idstrings are identical across. `DATA03`/
`DATA04`/`DATA05` add `RetrievingRecords` (`FE_RETREC`, an online-status
line) and pad `Gridi.8`/`Gridi.9` with different placeholder teams
(`Auricom`/`Harimau` plus two `Gridi.dummy10`/`dummy11` entries at
`color="0"`, i.e. invisible) - all consistent with the "placeholder, filled
at runtime" reading above, and none of it changes what a real race's own
values would be.

## `EndRace Podium`: read and drawn, multiplayer only

`DATA05` and `DATA06` only (`DATA02`, the copy this build serves for the
others, has none, so `oag_game::endrace::load_hd_podium` walks every copy of
the screen file with `Archives::read_every_name` and takes the first that
authors it - `DATA05` - for this one screen). `pod_head.{1,2,3}` all author
`IG_HUD_1ST`, but that is a placeholder the code overwrites; the slot setter
([endrace-podium.md](../ghidra/functions/ps3-hdfury-eu/endrace-podium.md), 78)
puts the winner in the middle column, second left and third right, each name,
heading and `dot.gtf` plinth at a computed position (plinths 352 wide, `80 (4 -
place)` tall, all ending on `y = 565`), the local player in `HD_Blue`. The
eight `b_b.N` panels are a per-player badge list (`"BADGE NAME TEST"`/`"NAME OF
PLAYER X"` are placeholders) with no state in this project.

**Entered by the multiplayer race managers** (74): `FUN_000459d8`, a member of
the `MPRaceManager` family, goes to it at `0x00045e4c` once its end deadline has
passed (and the series is not mid-way), holds it 16 s and leaves for `Kill Game
Transition`. No `goto=` in any archive names it, which is why the XML alone
never showed an entry. This project has no multiplayer, so the live flow stays
Results -> Menu and the screen is drawn by `--menu-page endrace-podium` only.
The earlier note that the screen has "no honest single-player mapping" stands in
the sense that matters: the slot fill is generic (first three records) but the
only entry found is multiplayer.

Drawn (synthetic names, labelled in the code): the authored backdrop, title
arrow and rules, and per place a heading (`1ST`/`2ND`/`3RD`), name and plinth.
Not drawn: the ship portraits (`pod_img`, a texture the code picks per record),
the badge panels, and the title - `FE_ENDRACE_PODIUM` is in `DATA05`/`DATA06`'s
English table and not in `DATA02`'s, which is the table this build reads.
**Chosen, not measured**: only `HD_Blue`'s value (`DATA06`'s; the code reads
the archive's own) and the absent title. No pointer targets: the screen
authors nothing to select.

Verified in `crates/game/tests/hd_endrace_ground_truth.rs`
(`hd_endrace_podium_draws_three_places_off_the_copy_that_authors_it`).

### Omega (checked, applies, not wired)

Omega's `data09.psarc`, `data08.psarc` and `data00.psarc` each carry an
`EndRace_Definition.xml` that authors `EndRace Results`, `EndRace Menu` and
`EndRace Podium` and **no** `EndRace Rewards` - HD's `DATA06` shape, and the
same "no Rewards" result as HD's code. The loader is HD's own
(`omega_endrace_definition_authors_the_screens_hd_does` in
`crates/omega/tests/omega_title_ground_truth.rs`); Omega's `endrace_entry`
stays `None`, so nothing is wired there (the Menu blocks draw wrongly, per
`omega-status.md`).

## Column/row geometry and the seam this project reads it through

- [`oag_title::FrontEnd::endrace_entry`](../../crates/title/src/lib.rs) is
  the per-title axis: Pulse keeps `Data\Plugins\PI001\GUI\EndRace_Definition.xml`,
  HD's is `oag_hd::frontend::names::ENDRACE_DEFINITION` (this page's own
  file). Pure/2048/Omega are `None` - not checked, not measured absent.
- [`oag_game::endrace::load`](../../crates/game/src/endrace.rs) dispatches on
  `title.name` exactly the way `crate::campaign::load`/`load_hd` already
  does, and its own `load_hd` reads `EndRace Results`/`EndRace Menu`, plus
  `EndRace Rewards` when the served copy authors it.
- [`oag_ui_screens::endrace::hd`](../../crates/ui-screens/src/endrace/hd.rs) is the drawing
  half - what draws, what does not, and the exact chosen-vs-measured split
  for the row/column geometry this page's own tables leave open, in its own
  module doc rather than repeated here.
- Which copy the runtime actually loads, and whether the language picker
  or any of the "not this pass" widgets above ever fire, are readings an
  RPCS3 capture would settle - `just rpcs3-race`, unavailable to this lane
  (another lane holds the emulator). See `HANDOVER.md`.
