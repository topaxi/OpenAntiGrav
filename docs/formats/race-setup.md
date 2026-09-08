# Race setup: what each title authors between the menu and the grid

**Status: read, not implemented.** This page describes the screens each title
uses to configure a single race - what the community calls the "race box" - and
what each one actually selects. Nothing here is built. The menu tree this
project draws stays [its own](../architecture/menus.md); what this page
supplies is the **contents** of the rows, which that page already says belong
to the release rather than to this project.

Confidence scores follow the
[rubric](../reverse-engineering/confidence-rubric.md).

## The naming trap, first

**A screen named `Racebox` is not the race box.** Confidence 95.

On Wipeout Pulse, `MainMenu_Definition.xml` redirects
`Main Menu->Mode == FE_RACEBOX` to a screen named **`Single Player`**. The
screen actually named `Racebox` offers `RB_SP` / `RB_LOAD_GRID` /
`RB_EDIT_GRID` and is the **custom-grid editor**.

Wipeout HD does the same and comments on it in the file: its
`racebox_definition.xml` carries
`<Entry item="Mode" equals="FE_RACEBOX" goto="Single Player">` beside an
authoring note saying it formerly went to `Racebox`
([hd-frontend](hd-frontend.md)). Both titles moved the destination and kept the
old idstring.

So: **`FE_RACEBOX` is the entry, `Single Player` is the screen, and the
grid editor kept the name.** Code or documentation that greps for a screen
called `Racebox` finds the editor.

## The shape four titles agree on

```
Main Menu -> Single Player -> Track Creation -> Team Selection -> Launch Game
              (settings)       (circuit)         (craft)
```

Pulse (PSP), Pulse (PS2) and HD/Fury use exactly these three screen names in
exactly this order. Pure keeps `Track Selection` and `Team Selection` but
splits the first page into a chain of single-choice screens. Wipeout 2048 has
no such flow at all.

| | Pulse PSP | Pulse PS2 | Pure | HD/Fury | 2048 |
| --- | --- | --- | --- | --- | --- |
| Settings on one page | yes, 5 rows | yes, 5 rows | **no, one per screen** | yes, 6 rows | none |
| Speed class | 4 | 4 | **5** | 4 | n/a |
| AI difficulty | 3 | 3 | **none** | 3 | n/a |
| Weapons on/off | yes | yes | multiplayer only | yes | n/a |
| Lap count selectable | **no** | **no** | **no** | **no** | n/a |
| Track preview | mesh | mesh | unresolved | mesh | n/a |
| Craft preview | mesh | mesh | unresolved | mesh | touch grid |

## Pulse - `Single Player`

`Data\Plugins\PI001\GUI\RaceBox_Definition.xml`, `<Screen type="RaceBox"
name="Single Player">`. Five `<List>` widgets, read directly. Confidence 92.

| Row label | `<List name=>` | `global=` | Entries | Default |
| --- | --- | --- | --- | --- |
| `RC_RT` | `Mode` | `Mode` | Arcade, Head2Head, Time Trial, Speed Lap, Tournament, Zone, Elimination | Arcade |
| `RC_SC` | `Class` | `Class` | venom, flash, rapier, phantom | venom |
| `RB_WEAP` | `Weapons` | `Weapons` | FE_ON, FE_OFF | FE_ON |
| `RB_AI_DIF` | `Difficulty` | **`SkillLevel`** | Easy, Medium, Hard | Easy |
| `IG_HUD_KILLS` | `Eliminations` | `Eliminations` | 5, 10, 15, 20, 25 | 10 |

**The widget name and the persisted key differ on the difficulty row** -
`Difficulty` against `SkillLevel` - on all three titles that have it. Code
keying off the widget name will not find the saved value.

Two `<Text>` widgets sit at coordinates already occupied by a list and are
swapped in for particular modes: `Zone` over the `Class` row, and
`DifficultyNaText` (`FE_NA`) over the `Difficulty` row. Confidence 80 - the
coordinate collision is exact, that the class performs the swap is inference.

Redirect out: `Mode == Tournament` goes to `Tournament C`, everything else to
`Track Creation`. Confidence 95.

### `GSDisableEntriesBitField`

The `Mode` list carries `GSDisableEntriesBitField="0xFA"`; HD's carries
`0xF0`. Read as **"bit N may disable entry N"**, confidence 72.

Both are self-consistent under that rule: Pulse's `0xFA` over seven entries
leaves Arcade and Time Trial ungated, HD's `0xF0` over five leaves only Zone
gated. In **both** cases every bit at or above the entry count is set, which is
what a fixed 8-bit mask does. Two titles agreeing on a non-obvious pattern is
the corroboration; the consuming code is unread, which is what holds the score
under 85. The field is listed as undecoded in
[fe-menu-definitions](fe-menu-definitions.md).

## Pulse - `Track Creation` and `Team Selection`

`Data\Plugins\PI001\GUI\Selection_Definition.xml`.

**Neither screen authors its rows.** `Track Creation` (`type="TrackSelection"`)
declares no `<List>` and no `<Menu>` at all - only an `up arrow` at (117,30), a
`down arrow` at (117,215), a `1/1` counter (`<Text name="honey">`) and an info
panel. The arrows being vertical is the authored evidence that the circuit list
moves with up/down rather than left/right. Confidence 93.

The info panel shows the circuit name in one of three layouts - `Info Track
1.1`, `2.1`/`2.2`, `3.1`/`3.2`/`3.3`, selected at runtime by a formatter the
executable spells `Info Track %d.%d` - plus three stat rows labelled
`IG_HUD_DISTANCE`, `IG_HUD_LAP_REC` and `ER_RR`.

`Team Selection` (`type="TeamSelection"`) *does* author a list, as a template
of sixteen placeholder entries `Team Entry 01`..`Team Entry 16`, `wrap="true"`.
Beside it sits the variant selector: a `<Text name="skin">` flanked by
`skin left arrow` and `skin right arrow`, with a `skin fixed` text for teams
with nothing to cycle. **So the team moves on up/down and the livery on
left/right, on one screen.** Confidence 92.

Five bars, each a dim backing `<Image>` plus a bright fill plus a numeric
`<Text>`: `Speed`, `Thrust`, `Handling`, `Shield`, `Loyalty`. HD authors the
same five in the same order as `Slide_0`..`Slide_4` with idstrings `RC_SPEED`,
`RC_THRUST`, `RC_HANDLING`, `RC_SHIELD`, `ER_LOY`.

## The previews are rendered 3D meshes

**Confidence 97 for the track, 95 for the craft, on the PSP pressing** - raised
from string archaeology to a direct capture; see
[below](#captured-live-in-ppsspp-2026-09-05). This is the result most
likely to be guessed wrong: a selection screen with a circuit on it invites the
reading "a flat 2D map" or "a prerendered image", and on every title where the
question is resolved it is neither.

Neither PSP screen authors a preview widget in XML, so the answer is in the
screen classes. Those cannot be reached by cross-reference - see
[Limits](#limits) - but each class's strings sit contiguously in `.rodata` and
terminate in the class name, which is enough.

### `TrackSelection`, `/psp-pulse-usa/BOOT.BIN` `0x08a848a0`-`0x08a84a10`

The block holds, in order: `Info Track 2.2`, `3.1`, `3.2`, `3.3`,
`Info Track %d.%d`, `linebg%dl`, `linebg%dr`, `%s Title`, `-.--.--`, `%d`,
`--`, `Mode`, `Black`, `White`, `Top->Ship`, `Zone`, `Info`, **`forward`**,
**`reverse`**, **`%s\FE\%s.vex`**, `%d / %d`, `honey`, `Class`, `Info1`,
`MSC_DISTANCE`, `Info2`, `IG_HUD_LAP_REC`, `Info3`, `ER_RR`, `Info3 Title`,
`zone`, `ER_ZONE_SCORE`, `Info1 Title`, then `TrackSelection`.

**`Black` and `White` are the two circuit runs, not a preview-view toggle.**
Confirmed live: the screen's own Help overlay (`triangle`) reads "Select your
circuit, and choose the BLACK RUN or WHITE RUN for the circuit. Black and
White runs vary in size, direction or difficulty, and can provide different
challenges at different speed classes." Confidence 92 - the disc's own help
text, verbatim. That leaves `Top->Ship` as the one string in this block with
no confirmed referent; see the capture below.

Every widget name in that list - `Info Track 2.2`, `linebg1l`, `honey`,
`Info1`, `Info2`, `Info3`, `IG_HUD_LAP_REC`, `ER_RR` - is one the XML authors
on `Track Creation`. That correspondence is what makes the block attribution
evidence rather than an address guess.

**One key differs between the two sides, and the difference is real.** The XML
labels the first stat row `IG_HUD_DISTANCE`; the class carries `MSC_DISTANCE`.
Checked both ways rather than assumed: `IG_HUD_DISTANCE` occurs exactly once
across every definition file that resolves in `Data.wad` and **not at all** in
the executable, while `MSC_DISTANCE` occurs at `0x08a84998` - inside this block
- and nowhere in the XML. So the class names its own string key for that row
rather than using the authored one, which is the same overwrite-the-authored-
value pattern the placeholder circuit names and the `Team Entry NN` rows
already show. Confidence 85 for the observation; **what makes the class prefer
its own key is not established.**

**The constructed paths resolve on the disc**, in `Data.wad`:

| Path | Size |
| --- | ---: |
| `Data\Environments\16_Track\FE\forward.vex` | 18,704 B |
| `Data\Environments\16_Track\FE\reverse.vex` | 18,704 B, byte-different |
| `Data\Environments\03_Track\FE\forward.vex` | 22,736 B |
| `Data\Environments\03_Track\FE\reverse.vex` | 17,840 B |
| `Data\Environments\02_Track\FE\reverse.vex` | 23,184 B |

Each carries `VEXX` at offset `0x0C`, the same magic as the real
`Data\Environments\16_Track\track.vex` (4.25 MB) - so these are genuine
[meshes](vex.md), around 1/200th the size, i.e. purpose-built preview geometry.

**The two `%s` are the circuit's `location` and its direction, not its id.**
`Data\Environments\18_Track\FE\*.vex` does not resolve, because
`<PI_Track name="18_Track">` in `Data\Plugins\PI001\Definition.xml` carries
`location="Data\Environments\02_Track" Reversed="True"` - the same
entry-is-not-a-directory trap
[the menus](../architecture/menus.md#what-still-comes-off-the-disc) already
describes. Its preview is `02_Track\FE\reverse.vex`, which does resolve. The
`forward`/`reverse` pair sitting immediately before the template in the string
block is the corroboration.

### `TeamSelection`, `0x08a84560`-`0x08a846a0`

The block holds `skin left arrow`, `skin right arrow`, `IMAGE.DAT`, `Custom`,
`%d`, `%s Bar`, **`Info->Ship`**, **`%s\%s_FE.vex`**, `zone`, `FE_TeamModel`,
`%s\%s.dat`, `FE_ModelSkin`, `Info`, `Loyalty`, `Speed`, `Thrust`, `Handling`,
`Shield`, `%d / %d`, `honey`, `Team Ship Help`, `skin fixed`, `Zone`,
`Eliminator`, `Normal`, `Suggest`, `ER_SUG_SHIP`, `ER_FOR_SHIP`, then
`TeamSelection`.

So the screen loads `<team dir>\<variant>_FE.vex` into a widget it addresses as
`Info->Ship`, picks the variant through `FE_TeamModel` / `FE_ModelSkin`, and
swaps the livery with `%s\%s.dat` - the skin file decoded in
[ship-skin](../ghidra/functions/psp-pulse-usa/ship-skin.md).
`Data\Ships\Assegai\ship_FE.vex` resolves at 76,832 bytes, and
[handling-stats](handling-stats.md) independently lists
`Data\Ships\<Team>\ship_FE.vex` as the front-end preview model.

`IMAGE.DAT` and `Custom` sit in the same block and are **unattributed** - they
are recorded here because the rest of the block is, not because anything places
them. On the evidence here neither is a preview asset: no path template joins
them to a directory, and `Custom` is also the vocabulary of the grid editor
(`CustomGridMC`). Do not read `IMAGE.DAT` as evidence against the mesh reading
without checking what loads it.

**`%s\%s_FE.vex` occurs twice in the executable** - at `0x08a7ff6c`, among the
ship path templates, and at `0x08a845a8`, inside this block. That duplication
is why attributing *this* copy to `Team Selection` is safe: it is not a
re-reading of the multiplayer-lobby preview site that the ship-skin page
already records at a lower confidence, but a second, separately located copy
whose neighbours are this screen's own widget names.

### The PS2 pressing ships the same two families

Both resolve in `54748/WADS2.WAD`, with their own geometry:
`Data\Environments\16_Track\FE\forward.vex` at 10,432 bytes and
`Data\Ships\Assegai\ship_FE.vex` at 126,784 bytes. Confidence 90.

### Captured live in PPSSPP, 2026-09-05

Both screens were walked and screenshotted end to end: PPSSPP v1.20.4, SDL
build under Xvfb, `pulse-psp-usa.chd`, a fresh profile, following
[the debugger page's](../reverse-engineering/ppsspp-debugger.md) menu walk as
far as `Track Creation` and then driving custom input past that point instead
of straight through to `InGame`, since the ordinary walk never stops there.
Screenshots are game content and were not committed; described here with
measurements instead.

**Both previews are confirmed live, animated 3D renders, not stills.** Two
screenshots taken one second apart on the same track show the same static
architecture with the camera visibly further along the corridor and the
motion-blur streak in a different position; the same pair on the ship preview
shows the identical craft rotated a few degrees further round its own turntable.
That is what raises the confidence at the top of this section from
string-archaeology to directly observed.

**`Track Creation`'s preview is a first-person flythrough down the circuit's
own corridor, framed in a hexagonal window - never a top-down map and never a
ship.** Confirmed across all three reachable circuits and after pressing
`square`, `triangle` and `select` on the screen (the first is inert here,
the other two open a `Track Help` overlay): no ship model, silhouette or
top-down framing appeared under any of them, which agrees with the code now
that it is readable: **`Top->Ship` has a confirmed referent, and it is not a
static screen widget at all.** It is a node inside *each track's own*
dynamically created preview scene - replaced every time the selection
changes - fed the already-confirmed `%s\FE\%s.vex` mesh path directly. The
name most likely survives from a shared preview-scene template also used for
an actual ship (`Team Selection`'s equivalent node is named `Info->Ship` and
does hold a craft). See
[`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md#top-ship-has-a-confirmed-referent-after-all),
confidence 78.

**The wrapping three-entry list was walked all the way round** (`down` x4,
landing back where it started): `1/3` Talon's Junction White (`Distance(m)
5178`), `2/3` Moa Therma White (`5350`), `3/3` Metropia White (`4419`), then
back to `1/3`. That is `16_Track`, `03_Track` and `18_Track` in list order,
confirming both the "exactly three, wrapping" reading and the
`18_Track` = `02_Track Reversed` = "Metropia" identification live rather than
only from XML. Confidence 95.

**Every one of the three used the same single-row info-panel layout** -
`Distance(m)` / `Lap record` / `Race record` over one value column - with no
layout ever switching to a multi-row `2.x`/`3.x` template. So this capture
does not confirm the `Info Track %d.%d` = (count, index) hypothesis; it is
consistent with "always 1.1 when nothing multi-line is needed" just as much as
with any other reading, and stays unresolved. The rendered label for the
disputed stat row read **`Distance(m)`** regardless of which idstring drove it
(`IG_HUD_DISTANCE` from the XML or `MSC_DISTANCE` from the class) - the two
evidently resolve to the same or near-identical English text, so a screenshot
alone cannot discriminate which key actually rendered; the localisation table
that could was not located this session. The 85-confidence conflict stands.

**The locked-circuit info panel is confirmed unreachable from `Track
Creation`, not just inferred from `<Unlock>` XML.** All three list entries
are the three ungated `PI_Track`s on every lap around the wrapping list;
there is no key sequence that reaches a fourth. Answering what a locked
circuit's panel looks like needs a different entry point - `Tournament C` or
campaign-grid progression, both untried here, both a different screen from
`Track Creation` per this page's own reading above. Left open.

**`Team Selection`'s preview is the craft alone on an unlit black background**,
turntable-rotating, with all five bars rendered exactly as authored - `Speed`,
`Thrust`, `Handling`, `Shield` as bars, `Loyalty` as a separate bar below a
divider - and the livery name (`Classic`) between two small arrow glyphs under
the team name. `down` moved the team from Assegai (`1/8`) to Qirex (`2/8`);
`right` on a fresh, `Loyalty 0` team's livery selector produced no visible
change, which is consistent with the variant-unlock gating this page already
reads out of `Definition.xml` (`Alternative` costs 4,000/60,000) and is now a
live corroboration of it rather than only an XML reading. Confidence 80 for
the gating read (one team, one press, not an exhaustive test). `square` opened
`Pre Race Music Select` (the bottom bar's "Music playlist" label) and
`triangle`/`select` both opened `Team Help` - both button bindings confirmed
live.

### HD/Fury authors its previews as widgets

HD needs no string archaeology: `track_selection_definition.xml`'s first
element is `<Model name="TrackModel">` and `team_selection_definition.xml`'s is
`<Model name="ShipModel" OriginX="1220" OriginY="412" nearZ="1.0" z="-24.0"
RotX="0.4" RotY="-0.5">`. Both are `<Model>` widgets - rendered meshes, with a
camera stated. Confidence 90. `TrackModel` is a sibling of the screens rather
than a child, so `Track Creation` and `Tournament C` share it.

## Two unlock axes, and they are not the same axis

From `Data\Plugins\PI001\Definition.xml`. Confidence 95.

**Circuits gate on a named grid.** A `<PI_Track>` either carries an
`<Unlock Grid="...">` child or it does not. The `Grid` attribute holds a
**name** - `Grid0`, `Grid1`, ... - not an integer, so an unlock stored as an
index will not round-trip. **Exactly three `PI_Track` entries carry no
`<Unlock>` at all**: `16_Track`, `03_Track` and `18_Track`. Those three are
therefore the only circuits available before any campaign grid is cleared,
which is a much shorter list than "everything raceable". **`Track
Creation`'s own list code confirms this from the executable side, and adds a
second, mode-gated flag on top of it**: `TrackSelection_PopulateList` filters
on `<Unlock>` (via `Definition_IsUnlocked`) and, only when the current `Mode`
equals a specific value, an additional per-track byte that happens to be set
on exactly the same three tracks. See
[`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md#trackselection_populatelist-closes-what-populates-the-track-list-is-unread).

**Craft variants gate on loyalty, per team.** Each `PI_TeamModel` /
`PI_ModelSkin` leaf carries *two* `<Unlock>` rows - a cheap own-team price and
an expensive `Team="any"` one, both `Exclusive="true"`. Consistent across every
team read:

| Variant | own team | any team |
| --- | ---: | ---: |
| `Alternative` | 4,000 | 60,000 |
| `Eliminator` | 8,000 | 75,000 |
| `Zone` | 16,000 | 90,000 |
| `Concept` | 25,000 | 100,000 |

That second axis is what the `Loyalty` bar on `Team Selection` displays, and it
is why craft availability cannot be modelled with the circuit unlock's shape.

## The Race Campaign: the disc's own campaign grid, shape yes, content no

**Status: read, not implemented**, same as the rest of this page - and the
subject of a dedicated scoping pass (2026-09-08) into how much of Wipeout
Pulse's campaign structure already exists as data on the disc. Short answer:
**the screen flow and the grid's visual shape are fully authored; what races
at each cell, and its medal target times, are not found authored anywhere
this pass could read.** All extracted directly with `oag-wad cat` against
`pulse-psp-usa.chd`'s `PSP_GAME/USRDIR/Data.wad`; nothing here needed Ghidra.

### The path in, and it is a fourth destination `FE_RACEBOX` does not touch

`MainMenu_Definition.xml`'s `Mode` list carries `FE_RACE_CAM` **before**
`FE_RACEBOX` - "RACE CAMPAIGN" is its own top-level entry, not a mode of the
custom race box this page already covers. Its redirect chain:

```
Main Menu (FE_RACE_CAM) -> TournamentLoad -> Grid Selection -> Cell Selection -> Cell Help
                                                                       |
                                                          Team Selection -> Launch Game
```

`TournamentLoad` (`MainMenu_Definition.xml`) is a loading/dialog screen -
title `MSC_LOADTOURN`, a `MSC_MSG_AUTOSAVE3` warning dialog gated behind
`MSC_SQ_MSG7` - before redirecting on to `Grid Selection`. That autosave
dialog is the first concrete evidence the campaign persists between sessions,
consistent with `MSC_EVENT_TOURN`'s own text (below) saying a tournament can
be saved between races. Confidence 90 - clean widget/redirect reads, not
runtime-verified.

### `Grid Selection` and `Cell Selection`: two authored levels, `CellMode_Definition.xml`

`Grid Selection` (`type="GridSelection"`) shows one row of up to four
`GridController` tiers (`MaxX="4" MaxY="1"`), each labelled `GRID 1`, `GRID
2`... (`Title`, `string="GRID 1"` - a template, filled per grid at runtime)
and three stat lines per tier: `Medals` (`"00/16"`), `Points` (`"000/110"`)
and `Required` (`"20"`), each with its own idstring (`RC_GM` "Gold medals",
`RC_TP` "Total points", `RC_PN` "Points needed"). **All three are literal
placeholder strings in the XML**, the same pattern `race-setup.md` already
reads on `Track Creation`'s `1/1`/`1/3` counters and the placeholder circuit
names - a template the executable substitutes into, not shipped content.

Selecting a tier opens `Cell Selection` (`type="CellSelection"`): the same
**35-cell** (`MaxX="7" MaxY="5"`) hex `GridController` as `Cell Creation`'s
custom-grid editor (below), each cell a `Medal_x_y` / `Outline_x_y` /
`Lock_x_y` triple - filled hex, outline, and a lock glyph overlay. Confirms
the campaign's per-tier grid and the player-built custom grid share one
widget shape, 35 cells, not merely a similar look.

### A selected cell's detail panel is authored down to the medal-target colours - the values are not

Selecting a cell opens a panel with:

- **`Title`**, `string="SINGLE RACE 01"` - placeholder.
- **`Line1`..`Line8`**, each an id/value pair (`l1`..`l4` placeholders, then
  literally `--7`/`--5`/`--6`/`--7` for lines 5-8) - eight scoreboard-shaped
  rows, unpopulated.
- **`Target0`/`Target1`/`Target2`**, each a coloured hex swatch
  (`0xfffaeb38` gold, `0xffdae3e4` silver, `0xffdf942f` bronze - the ordering
  and the colours themselves are the evidence these are the three medal
  tiers) plus an `IG_HUD_TARGET` ("Target") label and a `string="value"`
  placeholder for the actual time or score.
- **`Event Help`**, `idstring="MSC_EVENT_SR"` by default - the same
  `MSC_EVENT_*` family `docs/gameplay/race-modes.md` already reads for the
  four implemented modes, confirming a cell's help text is one of these
  per-mode strings substituted in, not bespoke per-cell prose.

`MSC_HELP_RC_CS`, the screen's own help text, says outright: *"Details and
medal requirements for this event are listed to the right of the grid. For
races that feature AI ships, you can change their difficulty level with δ."*
So the disc's own documentation confirms medal requirements are meant to be
shown per cell - the widgets to show them are authored - **and no source read
this pass carries the numbers that would fill them in.** Confidence 88 for
the widget/placeholder reading (direct, unambiguous XML); the absence claim
that follows is scored on its own, below.

### Circuit and craft unlocks already read (above) are the campaign's own gate

The eleven named `Grid0`..`Grid10` tiers this page's unlock section already
documents on 21 of 24 `PI_Track`s and every `PI_TeamModel`/`PI_ModelSkin` are
now read in context: they are what `Cell Selection`'s `Lock_x_y` overlay and
`Grid Selection`'s tier progression gate against. Clearing enough of one
grid's cells is what should unlock the next `PI_Track`s carrying that grid's
name - that inference was already implicit in the unlock section; this pass
adds the screen that displays the lock, not a new mechanism.

### What is authored for a *custom* grid, and it is a different answer

`Racebox` (`RaceBox_Definition.xml`, `RB_EDIT_GRID` -> `Cell Creation`) is the
**player-built** equivalent of the same 35-cell shape, and here the medal
targets genuinely are authored - **by the player, at the console, not by the
disc**. `Cell Creation`'s `Records` button opens `SetTargetTimes`, whose three
`TargetInput` widgets (`type="time"`) are labelled `RB_GOLD_TARG` "Gold
target", `RB_SILV_TARG` "Silver target", `RB_BRON_TARG` "Bronze target" - a
free-text time entry, one per medal, per custom cell. `Cell Creation`'s
`Tournament` button opens `Tournament C`
(`Selection_Definition.xml`, `type="TournamentSelection"`, title
`RB_TOURN_SET` "Set tournament") - the multi-track list this page's "Nobody
offers a lap count" section already reads the serialisation format for
(`<TournamentTrack size="%d">` wrapping `<Values track=... mode=... laps=...>`
records, four slots on PSP, twelve on PS2).

**So the same three medal-target widgets exist in two contexts and are filled
two different ways**: typed by the player for a custom grid, and (on the
evidence available) supplied from somewhere unread for the built-in Race
Campaign. That a custom grid needs `TargetInput` at all is itself evidence
the built-in campaign's numbers are not entered the same way - a player never
sees a `TargetInput` on `Cell Help`, only a rendered value.

### What this pass could not settle

**Confidence 55, genuinely unresolved, and explicitly a question for whoever
has Ghidra next**: where the built-in campaign's per-cell content - which
track, which mode, which medal target times, how many points a race is worth,
how many points a grid requires to clear - actually lives. Two readings
remain open and this pass could not discriminate them from XML alone:

1. **A compiled table inside `BOOT.BIN`**, read the way `Definition_IsUnlocked`
   reads `<Unlock>` rows - i.e. the grid's *shape* is XML but its *content* is
   executable data, the same split `TrackSelection_PopulateList` already shows
   for the ordinary track list (XML declares candidates, code filters and
   orders them).
2. **A procedurally-built campaign**: grid contents derived at runtime from
   `Definition.xml`'s existing `PI_Track`/`Grid` associations (e.g. "one cell
   per unlockable track, event type and target time computed from the track's
   own difficulty fields") rather than a separately authored table at all.

Nothing in `Data.wad`'s 17 resolving GUI/plugin definition files distinguishes
these. The five files this page's own Limits section already flags as
unresolved (`Controls_Definition`, `Credits_Definition`, `Debug_Screens`,
`MemoryStickBootScreens`, `MemoryStickScreens`) are unlikely campaign-shaped
by name, but were not ruled out. **The concrete next step is Ghidra, not more
XML reading**: find what populates `Cell Selection`'s `Title`/`Line1..8`/
`Target0..2` and `Grid Selection`'s `Medals`/`Points`/`Required` widgets at
runtime, the same cross-reference approach
[`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md)
used for `TrackSelection`/`TeamSelection` once the PSP relocation patch made
`get_xrefs_to` usable on these screen classes too. That is outside this pass's
lane (Ghidra is another contributor's) and is recorded as a next step in
[the handover thread](../../HANDOVER.md) this pass opened.

### The full mode list the disc authors, seven against this project's four

`RaceBox_Definition.xml`'s `Mode` list (`Single Player`, confidence 92 - see
above) is the complete enumeration: **Arcade, Head2Head, Time Trial, Speed
Lap, Tournament, Zone, Elimination.** `oag_race::Mode::ALL` implements four of
the seven (Time Trial, Speed Lap, Zone, and Single Race for the disc's
`Arcade`). English text for all seven is in `Data.wad`'s language tables
(`Data\Text\...`, one `.cod` file per language - English is the entry hashing
to `01141`, confirmed by content, not by a resolved path):

| idstring | English text |
| --- | --- |
| `MSC_EVENT_SR` | "Single Race: take on a full grid of opponents, weapons optional. A gold medal awaits those who emerge victorious." |
| `MSC_EVENT_TT` | "Time Trial: beat the clock in this solo race. ... You will be given a free turbo pickup once per lap. Your energy will also recover automatically..." |
| `MSC_EVENT_SL` | "Speed Lap: focus all your efforts into tearing up the track and beating the single best lap time in this solo event. ..." |
| `MSC_EVENT_ZONE` | "Zone: your ship accelerates automatically and the top speed increases after every ten second period... Clear the target number of zones to win the event." |
| `MSC_EVENT_TOURN` | "Tournament: take part in a series of single races. Points are awarded between races, and tallied to determine the overall winner. ... You can also save your tournament progress between races." |
| `MSC_EVENT_ELIM` | "Eliminator: race for kills, not position, against a full grid of trigger happy contenders in a weapons-heavy environment. Weapons do more damage, and you cannot absorb pickups, but you regain health after each lap. The lap count is not fixed, and the race will end when the kill count is reached." |
| `MSC_EVENT_HTH` | "Head to Head: an intense game of rivalry where you will be pitted against an opponent. ... Track the distance between you and your opponent on the HUD." |

Confidence 92 for every string (direct text extraction, five languages
cross-checked to identify English by content). Three mechanically new facts
inside Eliminator's own text, none previously recorded: pickups cannot be
absorbed (health regenerates per lap instead), weapon damage is scaled up,
and the race ends on a kill-count target rather than a lap count -
corroborating `docs/gameplay/race-modes.md`'s existing reading that
`ER_DEATHS`/`ER_YOU_ELIM` imply a respawn-based mode.

**`Arcade_HUD.xml` is confirmed (by `docs/ui/hud.md`, independently) to be
both the single-race and the tournament in-race HUD** - no separate
`Tournament_HUD.xml` exists or was found, and no `Head2Head_HUD.xml` either.
`Elimination_HUD.xml` does exist (`docs/ui/hud.md` already reads it) and is
its own layout. So an in-race tournament leg looks like an ordinary single
race; only the surrounding menu chrome (`Cell Help`, its target/points
widgets, the end-of-tournament placement text below) is campaign-specific.

**Per-speed-class lap counts, read from help text rather than a table, and
qualified in the source's own wording**: `MSC_LOAD_VENOM` "Most Venom events
last for 3 laps", `MSC_LOAD_FLASH` "Flash events usually last 4 laps",
`MSC_LOAD_RAPIER` "a lap count of 4 for most events", `MSC_LOAD_PHANTOM` "a
punishing 5 laps of racing for most events". Confidence 78 - direct quotes,
capped because the text itself hedges ("most", "usually") rather than stating
a fixed rule, and no `<Values laps="%d">` record was cross-checked against it.
This bears directly on
[race-modes.md](../gameplay/race-modes.md#single-race)'s "the lap count is
ours" gap for Single Race: the original's own manual text says a Venom race
(the default class) is normally 3 laps, agreeing with this project's guess,
but Rapier/Phantom races are described as longer, and nothing here ties a lap
count to a class programmatically.

**End-of-tournament placement text, all eight positions, previously
unrecorded**: `ER_END_TOUR_1`.."Congratulations! 1st Place!" through
`ER_END_TOUR_8` "Bad luck! Maybe next time!", plus `ER_TOUR_COM` "Tournament
complete - ", `ER_WON_TOUR` "You have won the tournament!", `ER_QUIT_TOUR`
"Quit Tournament", `ER_CONT_TOUR` "Continue tournament", `ER_TOUR_STAN`
"Tournament standings", `ER_RACE_POINTS` "Race points:", `ER_RC_POINTS` "Race
Campaign points", `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` "Gold/Silver/Bronze/No
medal awarded". None of these carry a numeric points table alongside them -
they are presentation strings, not data, the same distinction the rest of
this section draws.

## Nobody offers a lap count

**No title authors a lap-count row on its race-setup page.** Confidence 88,
on four independent measurements: a widget-shaped search across Pulse's 18
resolving definition files, the PS2's seven, Pure's eleven and HD's eighteen.
HD's only lap widget is `NumberOfOnlineLaps`, online-only.

Laps are nonetheless a real race parameter. Pulse's executable carries the
serialised race record at `0x08a783d0`:

```
<Values track="%s" mode="%s" class="%s" Weapons="%s" Locked="%s"
        damage="%s" AICount="%d" skill="0" laps="%d" ship="None"
        ShipChoice="Yes"></Values>
```

wrapped by `<TournamentTrack size="%d" >`. Eleven fields, of which five are
rows on `Single Player` and six - `Locked`, `damage`, `AICount`, `laps`,
`ship`, `ShipChoice` - are set by the authoring side. Confidence 90. HD's
`Track Creation` displays `RB_CIRCUIT_LEN` and `RB_RACE_DIST` read-only, with
an infinity glyph for the modes that have no fixed end.

So the race length is **mode- and circuit-determined, not player-chosen**.

## Pure differs in shape, not only in value

Pure has no `Racebox`, no hex grid and no grid editor. Confidence 94. Its chain:

```
Main Menu -> Single Player (mode only) -> Class Selection -> League | Tournament
          -> Track Selection -> Team Selection -> Launch Game
```

Zone short-circuits to `Zone Track Selection` and straight on to `Launch Game`,
with no class, league or team step.

- **Five speed classes** - Vector, Venom, Flash, Rapier, Phantom - against four
  everywhere else. Confidence 94. This does **not** establish that Pulse has a
  fifth; [handling-stats](handling-stats.md) records that question separately
  and it stays open.
- **No AI difficulty row anywhere** in Pure's front-end XML. Confidence 94.
- **No unlock machinery at all**: zero `<Unlock>`, zero `Grid=`, zero
  `GSDisableEntriesBitField` across all eleven files. Confidence 94. Pure's
  `Show Unlocks` subtree is a post-race reward reveal, not a gate.
- **Its one authored preview is a 2D stat graph on `Class Selection`** - a
  shared `class_background.mip` and `classgraph.mip` plus a per-class
  `<class>graph2.mip` / `graph3.mip` pair, inside five `<Watch watch="Class">`
  blocks. All eleven textures resolve. Confidence 94. **This is the only flat
  2D preview found in any title, and it previews the speed class rather than a
  circuit or a craft.**
- Its variant axis is a two-state `<MenuBitmap name="Livery" maxItems="2">`
  toggle, against Pulse's four-way cycler.
- `Track Selection` and `Team Selection` author an empty `<Menu allocate="16">`
  and a `<Viewport>` and nothing else.

## The PS2 pressing's divergences

The race-setup page itself is identical to the PSP's - same lists, same
globals, same idstrings, same defaults, same bitfield. Confidence 92. What
differs sits around it:

- **The `Racebox` menu is bypassed.** `FE_RACEBOX` goes straight to
  `TournamentLoad` and on to `Single Player`, one hop shorter, and the file
  says so in three `<!-- Was goto=Racebox -->` comments. Confidence 95.
- **Split screen is the real addition** - `FE_RACE_CAM_SS` and
  `FE_RACE_CAM_SSH` route to `Split ScreenV` / `Split ScreenH`. Confidence 95.
- **Every `Team Selection` widget gains a `0` suffix** (`Team0`, `skin0`,
  `Speed Bar0`). Read as a player index, since the pressing's one new feature
  needs two instances of the screen. Confidence 70.
- **`Tournament C` holds twelve track slots** against the PSP's four.
  Confidence 92.
- `Skin.xml` loads 17 definition files against the PSP's 23, a strict subset;
  no PS2-only file exists. Confidence 92.

## Wipeout 2048 has no race box

Confidence 85. Across the 27 files under `/data/plugins/frontend/NEWGUI/` there
is no `Track Creation`, no `<List name="Track">`, no `<Model name="TrackModel">`
and no `Single Player`-shaped settings page. The top-level `Home` screen offers
five `TouchButton`s and none is "race": racing is entered from the campaign
event grid, which is drawn by code rather than authored as lists.

**That is a property of the title, not a gap in the measurement.** 2048
replaced the flow with a campaign grid. It keeps a craft-selection screen
(`team`, HD's renamed, with a `TouchList` of skins) and its only authored
track-plus-class picker is the crossplay lobby vote.

## Limits

- **HD's chain is declared, not measured.** It is read off the front-end XML's
  own redirects, not captured from a running game, and nothing on this page
  upgrades it - see
  [ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
  Pure's ordering has the same limit and is capped at 88 for it.
- **The PSP screen classes could not be reached by cross-reference until
  2026-09-07, when the Allegrex relocation patch was applied to all four
  PSP Ghidra databases** (`HANDOVER.md`, "Traps that are live"). Before that,
  `get_xrefs_to` on `TrackSelection` (`0x08a84a00`) returned nothing, a byte
  scan for the literal pointer found nothing, and an instruction search for
  the matching `addiu` immediate found zero matches across 525,049
  instructions - because relocations were not applied at import, so operands
  did not hold loaded addresses. **With the patch applied, both classes
  decompile cleanly** - see
  [`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md).
  Kept here as a record of what an unrelocated PSP binary looks like from
  this tooling, since Pure and HD's PSP databases carried the same defect
  and may still, if their own databases were not among the ones reimported.
- **What populates `Track Creation`'s list is now found on Pulse** - see
  [`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md#trackselection_populatelist-closes-what-populates-the-track-list-is-unread).
  HD's equivalent is still unfound - see
  [track-selection-screen](../ghidra/functions/ps3-hdfury-eu/track-selection-screen.md).
- **Pure's track and craft previews are unresolved.** A craft candidate exists
  (`%s\Phantom.vex` inside the `fe::TeamSelection_Screen` string span, with
  `Data\Ships\Feisar\Phantom.vex` resolving) but no cross-reference was
  recovered, so it rests on string adjacency alone: confidence 45, below this
  project's floor for acting on.
- **No screen class was renamed in Ghidra.** The strings prove the classes
  exist; they are not evidence of what any function does, and the rubric's
  floor is 50 to rename at all.
