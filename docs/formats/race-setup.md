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
top-down framing appeared under any of them. So on the evidence gathered,
**`Top->Ship` does not visibly draw a ship on this screen** - confidence 65
for the negative, since `square`/`triangle`/`select`/`up`/`down` is a bounded
probe, not an exhaustive one, and the widget path's actual behaviour is still
unread in code.

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
which is a much shorter list than "everything raceable".

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
- **The PSP screen classes cannot be reached by cross-reference.**
  `get_xrefs_to` on `TrackSelection` (`0x08a84a00`) returns nothing, a byte
  scan for the literal pointer finds nothing, and an instruction search for the
  matching `addiu` immediate finds zero matches across 525,049 instructions.
  Relocations are not applied at these binaries' import, so operands do not
  hold loaded addresses. **An empty result here is the tool failing, never
  evidence that a reference does not exist** - the same caution
  [ship-skin](../ghidra/functions/psp-pulse-usa/ship-skin.md) already
  documents for `.rodata` strings. Reading a class's contiguous string block
  works around it for *names*; it recovers no behaviour. **The 2026-09-05
  capture upgrades what the two screens draw from names to an observed,
  running frame** - see [above](#captured-live-in-ppsspp-2026-09-05) - but
  this is still a fact about *pixels*, not code: nothing about the capture
  reaches the functions that build the frame, so this limit's substance is
  unchanged.
- **What populates either list is unfound**, on Pulse and on HD alike -
  see [track-selection-screen](../ghidra/functions/ps3-hdfury-eu/track-selection-screen.md).
- **Pure's track and craft previews are unresolved.** A craft candidate exists
  (`%s\Phantom.vex` inside the `fe::TeamSelection_Screen` string span, with
  `Data\Ships\Feisar\Phantom.vex` resolving) but no cross-reference was
  recovered, so it rests on string adjacency alone: confidence 45, below this
  project's floor for acting on.
- **No screen class was renamed in Ghidra.** The strings prove the classes
  exist; they are not evidence of what any function does, and the rubric's
  floor is 50 to rename at all.
