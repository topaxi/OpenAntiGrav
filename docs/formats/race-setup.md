# Race setup: what each title authors between the menu and the grid

**Status: read; the unlock axes below are implemented on Pulse (PSP and PS2).** The rest of this page is read, not built. This page describes the screens each title
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
| Track preview | mesh | mesh | **sprite** | mesh | n/a |
| Craft preview | mesh | mesh | **sprite** | mesh | touch grid |

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

**When the `Eliminations` row shows** (read 2026-09-30 off the screen's per-mode visibility
routine `FUN_088e715c`, called with the mode index, Pulse PSP USA; confidence 75 - the
helpers are read, the mode index to mode name mapping is by elimination, so only Elimination
(index 8) is relied on): the row and its `EliminationsText` are shown **greyed** by default
(`FUN_088e6400`: visible plus the dimmed flag), **enabled** for Elimination
(`FUN_088e62d4`), and hidden for indices `0xe`, `0xf` and `0x10` (`FUN_088e6378`). This
build greys it in every mode but Eliminator. Whether the list wraps past 25 is **not read**
(the XML authors no `wrap`); this build's generic choice row decides, **chosen, not measured**.
The values are read at boot by `oag_game::boot::screens::read_race_setup` via
`oag_title::FrontEnd::race_setup`.

**When the `Weapons` row is editable, and what it shows when it is not** (read 2026-09-30, the
same routine `FUN_088e715c`, Pulse PSP USA; confidence 85). The row is the pair at `param_1+0xf0`
(text) and `+0xec` (list); per mode index the routine either enables it (`FUN_088e62d4`) or greys
it (`FUN_088e6400`) **and forces its value** with `FUN_088a9ab8(list, "FE_OFF"|"FE_ON")`, first
saving the player's own index at `+0x138` and restoring it (`FUN_08888574`) when a mode that
allows a pick comes back:

| Mode index | Row | Shows |
| --- | --- | --- |
| 5 (Time Trial), 10 (Speed Lap), 6 (Zone), 9 (Head2Head), 0xf | greyed | forced `FE_OFF` |
| 8 (Elimination), 0x12 | greyed | forced `FE_ON` |
| 3, 4 (Single Race and Tournament; which is which is unread), 0xc, 0xe, 0x10 | **enabled** | the player's pick |

Three independent sources agree on the first row: `Race_ReadSetupOptions`' weapons-off set
(`shield.md`), `Mode::weapons_enabled`, and the 2026-08-10 PPSSPP observation of the greyed `OFF`
in Time Trial, Speed Lap, Zone and Head to Head with Tournament's row editable. Only the
Eliminator's greyed `ON` is read from the routine alone (index 8 is the one that also enables
`KILLS`, which is what fixes it as Elimination). The list's own `Default="FE_ON"` is the row's
default. The two entries are `idstring="FE_ON" value="On"` and `idstring="FE_OFF" value="Off"`;
`Race_ReadSetupOptions` compares the value against `"On"`. This build stores `On`/`Off`
(`race.weapons`, empty = untouched = `On`), shows the disc's own `FE_ON`/`FE_OFF` strings, and
offers the pick in a single race only, the one mode of its five that is in the third row (the
mapping of indices 3 and 4 to Single Race and Tournament is by elimination against the greyed
rows and the 2026-08-10 capture, not read).

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

**Corrected 2026-09-09: `Track Creation` shows *one* mesh, and the
`%s\FE\%s.vex` file is it.** `16_Track\FE\forward.vex` decodes
to a single 364-vertex ribbon with a bounding radius of 805 - the circuit's
plan shape at the circuit's own scale - and rendered it is the glowing
outline on the info panel. **Corrected again 2026-09-10: the hexagonal
window is not a flythrough and not a mesh at all.** It is a slideshow of
four hex-cropped stills, `<location>\FE\image_01.mip` .. `image_04.mip`,
authored as a two-second state chain in the circuit's own
`<location>\screen.xml` - the file `TrackDefinition_EnterScreenState`
loads into the definition's state machine, and the file `Top->Ship` is a
node of. The "camera visibly further along the corridor" reading below was
two different stills a second apart. See
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#the-hexagonal-window-is-a-per-circuit-screenxml-and-it-shows-stills)
for the code and [selection-screens.md](../ui/selection-screens.md) for the
file, the capture and what this build draws. Confidence 95.

**Also settled the same day: the per-craft rating record is authored, in
`Definition.xml`.** Each `PI_Team` carries `<FE speed=".." thrust=".."
handling=".." shield="..">`, and the four values match the capture's bars
digit for digit on every team checked (Assegai `8/8/9/7`, Qirex `8/7/8/9`,
AG Systems `7/9/9/8`, Piranha `10/6/6/9`). That is the table
[`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md)
reads through `FUN_08808664` at `+0xb4..+0xc0` and this page recorded as
unlocated; `oag_raceplay::catalogue::Team::rating` reads it. Confidence 95.

Neither PSP screen authors a preview widget in XML, so the answer is in the
screen classes. Those cannot be reached by cross-reference - see
[Limits](#limits) - but each class's strings sit contiguously in `.rodata` and
terminate in the class name, which is enough.

### `TrackSelection`, `/pulse/BOOT-psp-pulse-usa.BIN` `0x08a848a0`-`0x08a84a10`

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

**The craft preview is a live, animated 3D render; the track preview is
not.** *(Corrected 2026-09-10: this paragraph read both as live renders.)*
Two screenshots taken one second apart on the ship preview show the
identical craft rotated a few degrees further round its own turntable. The
same pair on a track show two *different* stills of the circuit - the
"camera visibly further along the corridor" was card two of the slideshow
over card one, and the "motion-blur streak" is baked into the still. The
file that says so is `<location>\screen.xml`, read above.

**`Track Creation`'s preview is a first-person *still* of the circuit's
own corridor - four of them, cycled - each cropped to a hexagon, never a
top-down map and never a ship.** *(The word here was "flythrough" until
2026-09-10.)* Confirmed across all three reachable circuits and after pressing
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

### Captured live in PPSSPP, 2026-09-10 (Pure)

`pure-psp-eu.chd`, PPSSPP v1.20.4 SDL under Xvfb, a fresh profile, driven by
blind `cross`/`down`/`right` taps with a screenshot after each rather than a
state-name-verified walk - Pure's `G_STATE_MACHINE` address is unmeasured,
unlike Pulse's, so `dbg.state_name()` cannot be used here yet. Screenshots
are game content and were not committed.

**Neither `Track Selection` nor `Team Selection` renders a mesh. Every
preview on both screens is a pre-rendered 256x128 sprite shipped in
`PSP_GAME/USRDIR/FEData.wad`.** Confidence 97. *(This paragraph read "both
render a real 3D preview" until 2026-09-10; see ["How the 3D reading went
wrong"](#how-the-3d-reading-went-wrong) below.)*

The proof is offline and bit-exact. PPSSPP's `DumpTextures` wrote 12 textures
for `UCES00001` while the two screens were up, every one 256x128 grayscale +
alpha, at VRAM addresses `0x041c6940` / `0x041ced80` / `0x041d71c0`. Each one
matches an `FEData.wad` entry at **RMSE 0** - the pixels on screen are the
shipped texture, unaltered:

| What it shows | `FEData.wad` entry |
| --- | --- |
| craft wireframes | 126, 130, 134, 138 |
| circuit line art | 175, 176, 182, 188, 193, 194 |

`FEData.wad` holds 242 entries, of which **137 decode as exactly 256x128**:

- **~24 craft wireframes**, three consecutive indices per group at stride 4
  (124-126, 128-130, 132-134, 136-138, 140-142, 144-146). Within a group the
  three read as a top, a side and a 3/4 perspective view of one craft, and the
  capture drew the **third** of each group.
- **~28 circuit line-art images**, three per group at stride 6 (150-152,
  156-158, 162-164, 168-170, 174-176, 180-182, 186-188, 192-194). Two
  different members of one group were seen drawn (175 *and* 176; 193 *and*
  194), so a screen shows more than one image per circuit - whether that is a
  cycling slideshow, a rotation, or a selection change between screenshots is
  **not settled**.
- the rest are record-label logos, the boot splash and ad banners.

**Which still belongs to which entry is authored, not inferred** - recovered
the same day the sprite finding was, and entirely offline. Every entry's own
`<location>\screen.xml` names its three stills by full path:

```xml
<Screen name="Info"> <Image transition="0.25"><Values x="246" y="45"
    Color="FEGlobals->TrackColor"
    src="Data\Environments\01_Vineta_K\Images\Track_1.mip"/></Image>
  <Redirect delay="2"><Default goto="Side"/></Redirect> </Screen>
<Screen name="Side"> ... Track_2.mip ... goto="Top"  </Screen>
<Screen name="Top">  ... Track_3.mip ... goto="Info" </Screen>
```

`hash_name(r"Data\Environments\01_Vineta_K\Images\Track_1.mip")` is
`0xd6404ac7`, which is `FEData.wad` entry 194 - and 193 and 192 are `Track_2`
and `Track_3`. The two entries the capture caught on that circuit were
`Track_2` and `Track_1`: **the screen is a three-state chain that cycles
`Info -> Side -> Top -> Info` at two seconds a step**, and that cycle is what
a pair of screenshots read as a rotating model.

Craft are the same shape with different names, and **the names are not
derivable from the team id**: `feisar3D.mip` / `feisarside.mip` /
`feisartop.mip` are lowercase, `Medievil3D.mip` / `Medievilside.mip` /
`Medieviltop.mip` are capitalised. A composed `format!` would have been
wrong on the second team it met, which is the concrete reason the right
answer is to read `screen.xml` and play what it names.

Pure's Zone circuits name their chain `screen_z.xml` (the `%s\screen_z.xml`
template in its own executable) and still start it at `Info`, where Pulse's
Zone chain is a `Zone` state inside `screen_zone.xml`. `screen_m.xml` and
`screen_tt.xml` (split screen, time trial) exist beside them and are **not
read**: choosing between them needs a race-mode parameter
`oag_game::preview::slideshow` does not take, and on the circuit checked all
three name the same three stills.

#### How the 3D reading went wrong

The earlier pass described "a distinct wireframe-outline ship per team -
Feisar, Auricom and Qirex, three different silhouettes" and read per-team
distinctness as proof of a per-entity 3D render. **Per-team distinct
*sprites* produce exactly that observation**, and the same is true of "the
model appears to turntable-rotate on its own" against a cycling image set.
Two tells were available and not taken:

1. The art is anti-aliased, with soft grayscale gradients along the ribbon.
   PSP hardware has no anti-aliasing; a 256x128 render target on real hardware
   would be jagged. This is offline-rendered art, shipped as a texture.
2. The preview stays blurry as PPSSPP's internal resolution is raised.
   PPSSPP upscales geometry *and* render targets with internal resolution,
   but not a fixed 256x128 source image. (This was the maintainer's own
   observation, and it is the cheapest live check for the next case.)

The correction also **closes** an open question rather than leaving one:
neither `TeamSelection_ApplySelection` nor `TrackSelection_ApplySelection`
composes a default preview mesh path because **there is no default preview
mesh**. Hunting the record class that owns a mesh-loading virtual was
chasing something that does not exist for this screen. The Phantom special
case (`%s\Phantom.vex` / `%s\VR\Phantom.vex`) is unaffected and still stands
at confidence 65 - see
[`race-box-screens.md`](../ghidra/functions/psp-pure-eu/race-box-screens.md).

- **`Track Selection`** on Vineta K (Alpha league, first entry): the circuit
  line-art sprite plus a stat block - `RACE RECORD 0.00.00`, `LAP RECORD
  0.00.00`, `LENGTH 4446 M`, `HEIGHT 222 M` on a fresh profile. No
  hexagon-cropped photo slideshow anywhere on this screen, unlike Pulse's
  `Track Creation` - nothing in `Selection_Definition.xml` references one the
  way Pulse's `TrackDefinition_EnterScreenState` chain does.
- **`Team Selection`** shows the craft wireframe sprite plus
  `SPEED`/`HANDLING`/`SHIELD`/`THRUST` bars. No `Loyalty` bar and no
  livery-name readout were visible on the entries checked, consistent with
  Pure's two-state `Livery` `<MenuBitmap>` being a different, simpler axis
  than Pulse's named-skin cycler.
- **`Class Selection`, on this fresh profile, lists only `Vector` and
  `Venom`** - `Flash`/`Rapier`/`Phantom` do not appear at all, confirming the
  unlock gating this page's XML reading could not see (no `<Unlock>` element
  anywhere in the file) is real and happens in code.
- **A `sceIoOpen` breakpoint swept across `down`-triggered team and track
  changes found no hits** (all four `zz_sceIoOpen` stub addresses armed via
  the websocket debugger, 6 s window each). That is consistent with the
  sprite reading: `FEData.wad` is open for the life of the front end, and
  changing selection only re-uploads a texture already in memory.

#### The colours are in a second skin, and Pure is the only title with one

Pure's stills each carry `Color="FEGlobals->ShipColor"` / `TrackColor`, and
**both of those live in `Data\Skins\Default\Skin.xml`, not in the front-end
root**. Its `Data\Plugins\PI001\GUI\Skin.xml` declares 13 globals against
Pulse's 56; the other 37 - every colour the selection screens, the stat
panels and the title text draw in - are the *style skin's*, which
`Data\Plugins\PI001\Definition.xml` activates alongside the UI one:

```xml
<PI_Skin name="UI">      <Values location="Data\Plugins\PI001\GUI" activate="true"/>
<PI_Skin name="Default"> <Values location="Data\Skins\Default" style="true" activate="true"/>
<PI_Skin name="Hacker">  <Values location="Data\Skins\Hacker" style="true"/>
```

`Hacker` is the same file with a black background and is **not** activated.
Pulse declares exactly one `PI_Skin`, the UI one, whose `Skin.xml` is the
front-end root already loaded - so following `activate` changes nothing there,
measured on both pressings.

This matters twice over. Without it a still tints white, and on Pure's white
front end that is not a wrong colour but an absent picture. And four of the
globals it authors - `TitleColor` `0xFFED4796`, `DesignColor` `0xFF5FDBF6`,
`TextColor` `0xFF11ACD0`, `FrameLineColor` `0xFE99C9D8` - were being supplied
by `oag_pure::frontend::FALLBACK_GLOBALS`, a table sampled off captured frames
at confidence 60-65. The disc's own values differ from three of the four, most
of all on `TextColor` (`0x11ACD0` against the sampled `0x88D6E8`).

#### The stat panel is read and drawn, off the entry's own `screen.xml`

**Implemented 2026-09-27.** The `RACE RECORD`/`LAP RECORD`/`LENGTH`/`HEIGHT`
block and the `SPEED`/`HANDLING`/`SHIELD`/`THRUST` bars above draw now -
`oag_ui_screens::picker::body` never had a chance at them, because they are not in
`Selection_Definition.xml` at all: they sit above any named `Screen` in the
*entry's own* `screen.xml`, the same file the slideshow stills come from
(`Data\Ships\Feisar\screen.xml`, `Data\Environments\01_Vineta_K\screen.xml`).
`oag_ui_screens::picker::slideshow::Slideshow` reads them the same way it already read
the stills - inherited into every state, since they sit outside any of the
three (`Info`/`Side`/`Top`) - as a `Vec<Fill>` (the colour-only bars, no
`src`) and a `Vec<Text>` (`idstring="SPEED"` resolved through the title's own
string table the same way `Layout::read` resolves `Selection_Definition.xml`'s
own text, plus the literal fraction beside it - `"2/5"` on Feisar's own
`Speed` row, authored directly rather than computed from any `<FE>` rating,
which Pure's `Definition.xml` does not author at all - see
["Pure differs in shape, not only in value"](#pure-differs-in-shape-not-only-in-value)).
An `idstring` the table does not carry falls back to itself
(`StringTable::get_or_id`) rather than drawing nothing.

Confidence **90**: every value drawn is the disc's own literal XML content -
no bar fraction is computed, no rating is read, nothing is guessed - checked
against a fresh `--menu-page track-select`/`ship-select` capture on
`pure-psp-eu.chd` (German: `RENN-REKORD`/`LÄNGE`/`RUNDEN-REKORD`/`HÖHE`,
`TEMPO`/`HANDLING`/`SCHILD`/`SCHUB`, all idstrings resolving through
`Data\Plugins\PI009\Definition.xml`). Short of 94 because it is not checked
against a real PPSSPP frame in the same language - the 2026-09-05 capture
above is English and a fresh profile, whose `RACE RECORD 0.00.00` differs in
punctuation from this build's `00' 00' 00`, both being the entry's own
authored placeholder text (`docs/formats/pure-status.md`'s "Show the records
and fill them in for this track" comment, still unfilled here: `DisplayRecords`
is read as a node but nothing populates it, so a set record still shows the
placeholder).

**One overflow, not fixed here**: German's `RUNDEN-REKORD` (`LAP RECORD`) is
wider than English's own and its authored `x` runs into `HÖHE`'s (`HEIGHT`)
column, which is otherwise a separate authored `x`. Both are drawn exactly at
their own authored position with no wrapping, which is what
`Selection_Definition.xml`'s own English-authored text widgets already do
everywhere else in this crate - nothing here adds a wrap Pulse's own screens
do not have either. Whether the real PSP also overlaps at this string length
in German is unmeasured; recorded as an open item below rather than guessed
at with a wrap this format gives no evidence for.

#### Pure lists, Pulse shows one

The two titles disagree on the *shape* of a selection screen, not just on how
it previews. Pulse's `Selection_Definition.xml` authors named `<Text>` widgets
that show the **selected** entry alone (`Info Track 1.1`, `Info1`, the stat
bars) and no `<Menu>` at all. Pure authors one `<Menu name="Track">` /
`<Menu name="Team">` per screen, at `x=21 y=45 scale=1.15
color="FEGlobals->TextColor" allocate="16"`, and no named text - it **lists
every entry** down the left the way its own `Language Selection` lists
languages. `oag_ui_screens::picker` draws both shapes; each title's own XML decides
which it gets.

The widget states no row pitch, so the step is one line of its own font at its
own scale - the rule `oag_ui::frontend::draw`'s `DisplayLanguages` block
already used for this same widget on this title. **`allocate` is a capacity,
not a count, and nothing pages**: a source offering more entries than fit runs
off the bottom. The base disc's 8 circuits and 10 teams are inside 16; its DLC
packs are uncounted.

One trap the style skin exposed: **the `<Menu>`'s own `color` attribute is the
*selected* row's ink on Pure, not the unselected one.** It resolves to
`TextColor` `0xFF11ACD0`, and the Main Menu capture behind
`oag_pure::MENU_SKIN` samples the selected row at `0xFF16AED1` and the
unselected rows at `0xFF88D6E8` - lighter. While `TextColor` was the sampled
`0xFF88D6E8` the two agreed by accident and the language list looked right;
once the disc's real value arrived, taking the attribute literally painted
every row the same colour and the list lost its selection cue. Both list
drawers now take the title's measured pair.

**Net effect on this page's `PreviewSource` reading below**: Pure has no entry
in it, and needs none. `oag_game`'s pickers preview Pure with the entry's own
`screen.xml` chain and Pulse with a mesh plus its hexagon-window chain -
`oag_title::FrontEnd::preview_meshes` is the axis. Neither substitutes
`Ship.vex` or `track.vex`, which are the in-race hull and the full 4.25 MB
racing circuit; wiring those was a plausible-looking stand-in of exactly the
kind [CLAUDE.md's "never invent what the assets already author"](../../CLAUDE.md)
forbids.

### HD/Fury authors its previews as widgets

HD needs no string archaeology: `track_selection_definition.xml`'s first
element is `<Model name="TrackModel">` and `team_selection_definition.xml`'s is
`<Model name="ShipModel" OriginX="1220" OriginY="412" nearZ="1.0" z="-24.0"
RotX="0.4" RotY="-0.5">`. Both are `<Model>` widgets - rendered meshes, with a
camera stated. Confidence 90. `TrackModel` is a sibling of the screens rather
than a child, so `Track Creation` and `Tournament C` share it.

`ShipModel`'s values are read since 2026-09-29 (`oag_ui_screens::picker::hd::ShipModel`)
and nothing draws them yet: the one preview path this build has reads
`<team>\ship_FE.vex`, which HD's own per-team `screen.xml` names and no HD
archive carries. See `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Team
Selection`" section for the rest of that screen.

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

The `Loyalty` bar on `Team Selection` shows the selected team's own total, not
either price. The two rows are alternatives and `any` means the best single
team (`race-box-screens.md`, "Traced 2026-10-02"); that is why craft
availability cannot be modelled with the circuit unlock's shape.

**Status 2026-10-08 (implemented, Pulse PSP and PS2).** Track Select drops a circuit whose
`<Unlock Grid>` is unmet (`oag_game::unlock::Gate`, list absent not greyed, as `TrackSelection_PopulateList`
does); a fresh profile lists 3 circuits on PSP and on PS2 (same `Definition.xml` rows, same shared code, no
title branch: `Campaign::circuit_unlocks`). Craft variants gate on the two loyalty rows
(`loyalty_unlocked`). `--unlock-all` is ours. Per title census, `<Unlock` elements in `PI001/Definition.xml`
(`oag-wad cat --expand`): Pulse USA and EU identical, 85 rows, 21 with `Grid=`, 24 `PI_Track`; **Pure USA and EU
identical, 17 rows, none with `Grid` or loyalty: `MedalCount` 10/25/40/60/60/70 on the Classic circuits,
`Track`+`medal` chaining Zone 1..4, `Tournament` Alpha/Beta - checked, differs, unread, not wired**; HD: see
`hd-status.md` (its own progression is `hd-campaign`'s).

## The Race Campaign: the disc's own campaign grid, shape yes, content no

**Status: read, not implemented**, same as the rest of this page - and the
subject of a dedicated scoping pass (2026-09-08) into how much of Wipeout
Pulse's campaign structure already exists as data on the disc. The screen
half, below, was read with `oag-wad cat`/`extract` against
`pulse-psp-usa.chd`'s `Data.wad`, `FE.wad` and `FEData.wad` and needed no
Ghidra: **the screen flow and the grid's visual shape are fully authored, and
every content slot on those screens is a template placeholder.**

**The heading's "content no" is about the screens, and it survives only in that
sense.** A Ghidra follow-up the same day found the campaign's content in full,
in sixteen `Data\Plugins\grids\grid_NN.xml` files this pass never opened - 236
`PI_Cell` records with their own tracks, modes, lap counts and gold/silver/
bronze targets. Read
[`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md) before
using anything below as evidence that a value is missing; the
["settled the same day"](#settled-the-same-day-in-ghidra-the-content-is-authored-in-files-this-pass-never-opened)
section corrects the specific claims.

**A region note, since it bears on the language and front-end findings
below**: `oag-unpack info` identifies this image as `UCUS-98712` (USA) by
its boot path, but its ISO 9660 volume id reads `SCEE` and it also carries a
full `PSP_GAME/USRDIR/UCES00465/` (EU-serial) subtree alongside the USA one -
an oddity not resolved this pass. The `Data.wad`/`FE.wad`/`FEData.wad` this
page reads sit under the USA `PSP_GAME/USRDIR/` root regardless, and the
five-language string table found there (English, French, German, Spanish,
Italian) is consistent with either a PAL disc or a USA disc that ships PAL's
language set - it does not by itself resolve which pressing this image
actually is. Flagged here rather than silently assumed.

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
hex `GridController` as `Cell Creation`'s custom-grid editor (below), each
cell a `Medal_x_y` / `Outline_x_y` / `Lock_x_y` triple - filled hex, outline,
and a lock glyph overlay. `MaxX="7" MaxY="5"` is the bounding box, not the
cell count: a hex grid staggers, and counting the authored `Medal_x_y` nodes
by column gives 5,4,5,4,5,4,5 - **32 cells**, not 35. Confirms the campaign's
per-tier grid and the player-built custom grid share one widget shape, 32
cells, not merely a similar look.

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
the widget/placeholder reading (direct, unambiguous XML). The absence claim was
**refuted the same day**: the numbers are `<Gold Target=>`/`<Silver Target=>`/
`<Bronze Target=>` on each `PI_Cell` in `Data\Plugins\grids\grid_NN.xml`, and
`CellSelection_PopulateDetail` (`0x088d68d8`) substitutes them into exactly
these three widgets - hiding all three for `Race`, `Tournament` and
`Head2Head`, where the target is a finishing position and therefore implicit.
See [`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md).

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
**player-built** equivalent of the same 32-cell shape, and here the medal
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

### `FEData.wad` authors real per-track numbers - race times, lap times, AI difficulty, and two mode-independent constants

**Found by widening the search past `Data.wad` alone**, after the negative
above looked too clean to leave unchecked: `FEData.wad` (6.7 MiB, 242
entries) carries **24** entries - exactly the `PI_Track` count - each a
flat `<code>`-dictionary record (extension `.cod`, the same guessed
extension the language tables use; unrelated content, same guess heuristic)
with this shape:

```
<RaceTimes Flash="138" Phantom="128" Rapier="119" Venom="117"/>
<LapTimes  Flash="33"  Phantom="25"  Rapier="29"  Venom="38"/>
<Targets Elimination="10" Zone="25"/>
<Physical Length="5178"/>
<l>
  <c Difficulty="Easy"   Class="Venom"   SkillScaleValue="0.9"/>
  ... 12 rows, {Easy,Medium,Hard} x {Venom,Flash,Rapier,Phantom}
  <f Class="Venom" HeadToHead="1.7" FullGridWithWeapons="0.0"
     HalfGridWithWeapons="0.0" FullGridWithoutWeapons="0.1"
     HalfGridWithoutWeapons="0.0"/>
  ... 4 rows, one per Class
</l>
```

(attribute names expanded from the file's own `<code>` shortening for
legibility - `ModeModifiers`/`ModeModifiers` values quoted directly). Read
this way, confidence 88: the tags (`RaceTimes`, `LapTimes`, `Targets`,
`Physical`, `SkillScaleValue`, `ModeModifiers`) are unambiguous once expanded,
and one record's `Physical Length="5178"` is an **exact** match to the
`Distance(m) 5178` this page's own PPSSPP capture already recorded live for
Talon's Junction White - independent corroboration this family is genuine
per-track data, not a coincidence of format.

Four things read off the 24 records directly:

- **`Targets Elimination="10" Zone="25"` is identical on every one of the 24
  files** - a fixed, track-independent kill-count for Eliminator and
  zone-count for Zone. This is the numeric value behind `MSC_EVENT_ZONE`'s
  "clear the target number of zones" and `MSC_EVENT_ELIM`'s kill-count
  ending (both text-only until now). Confidence 90 - flat census over all 24.
- **`RaceTimes`/`LapTimes` carry one figure per speed class**, varying track
  to track (Venom's `RaceTimes` ranges 87-135 across the 24). These read as
  target/record times, but each is a *single* number per class - there is no
  three-way gold/silver/bronze split inside this record. Whether/how it
  feeds `Cell Help`'s three `Target0..2` tiers is **not established**; a
  single baseline scaled by a fixed ratio into three tiers is plausible and
  unverified.
- **The `ModeModifiers` block's two "WithWeapons" fields
  (`FullGridWithWeapons`, `HalfGridWithWeapons`) are `0.0` on every class, on
  every one of the 24 files** - only `HeadToHead` and the two
  "WithoutWeapons" fields ever carry a nonzero value. Confidence 90, flat
  census. Reads as: a weapons-on grid gets no AI equalising offset (weapons
  themselves are the equaliser); a weapons-off grid and head-to-head do.
- **The element names above are now read from the parser, not guessed.**
  `TrackStats_ParseElement` (`0x088c46f8`) calls the `<l>` block
  `SkillLevels`, its 12 rows `Entry` and its 4 rows `ModeModifiers`, and
  `TrackStats_Load` (`0x088c454c`) opens these files as `"%s\stats.xml"` /
  `"%s\stats_reversed.xml"` under each track's own directory. The whole record
  is parsed onto the **`PI_Track` definition object**, not into any campaign
  structure. `SkillScaleValue` defaults to `1.0`/`2.0`/`3.0` for Easy/Medium/
  Hard before parsing, so it is an index on a three-point curve rather than a
  raw multiplier - see
  [`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md).
- **`SkillScaleValue` is now a located, valued field.**
  [`ai-stats.md`](../ghidra/functions/psp-pulse-usa/ai-stats.md#what-is-not-determined)
  already flagged it as "appears in the string table and in none of the nine
  functions [read], unchased" - this pass did not chase the consumer either
  (that is Ghidra, out of lane), but the *values* are now on disc, per track
  per difficulty per class, twelve rows to a file.

**File order was checked as a possible track-identity key and does not hold
up as one.** The 24 entries' extraction indices are consecutive
(`00218`-`00241`), and the first pairs (`00218`/`00219`, both length `5178`;
`00222`/`00223`, both `5350`) look like forward/reverse pairs sharing one
physical circuit, tempting a "file order equals `Definition.xml`'s `k`
order" reading. **That reading is contradicted by the third pair**: `k=3`
(`03_Track`) and `k=5` (`18_Track`) are independently known from this page's
own PPSSPP capture to measure `5350` and `4419` respectively, but a strict
positional mapping puts `4419` at position 3 and `5350` at position 5 -
swapped. So file order is *not* a reliable track key on its own evidence,
and no attempt to name all 24 tracks from this family is made here.

### Settled the same day, in Ghidra: the content is authored, in files this pass never opened

The paragraph that stood here asked where the built-in campaign's per-cell
content lives, and offered two readings - a compiled table in `BOOT.BIN`, or a
campaign built procedurally out of `Definition.xml` plus the `FEData.wad`
per-track family. **Both are wrong, and the answer was on disc the whole time.**
`Data\Plugins\grids\Definition.xml` in `Data.wad` lists **sixteen** files,
`grid_00.xml` .. `grid_15.xml`, each one `PI_Grid` holding 8-16 `PI_Cell`
records - **236 cells in all**, every one naming its track, mode, speed class,
lap count, weapons and damage switches, AI count, AI skill, and **its own gold,
silver and bronze targets**. The full decompilation of the parsers, the medal
evaluator, the points table and the unlock gate is
[`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md); the
headline numbers it recovers:

- **A medal is worth gold 3, silver 2, bronze 1** (`Cell_MedalPoints`,
  `0x088bf530`). There is no points-per-position table anywhere on disc because
  points come from the medal, not the position.
- **A grid unlocks when the points earned in the *previous* grid reach its
  authored `RequiredPoints`** - 12, 16, 20, 24, then 28 for every grid from
  `grid4` on (`Unlock_GridPointsMet`, `0x0888ebd8`). That is exactly what a
  `<Unlock Grid="Grid0"/>` row on a `PI_Track` evaluates.
- **The medal is a three-way threshold compare** against the cell's own
  `Target0..2`, with the comparison direction flipped for `Zone` and
  `Elimination` where more is better (`Cell_EvaluateMedal`, `0x088bf620`). The
  ordinal is `0 = gold, 1 = silver, 2 = bronze, 0xff = none`.
- **The campaign reads `FEData.wad`'s per-track family for exactly one thing:
  AI difficulty.** A cell's `skill`/`skillEasy`/`skillHard` is a position on the
  track's own `SkillScaleValue` curve, and `AI_ResolveSkillScale`
  (`0x08834df4`) interpolates the two. Nothing about a medal target, a mode or a
  lap count comes from there.
- **The 32-cell count above is the widget shape, not the content.** A
  `CellSelection` screen can display 32 hex positions; a campaign grid fills 8,
  10, 12, 14 or 16 of them, addressed by the `(x, y)` parsed out of each cell's
  own name (`grid0_2_1`).

The lesson for the next search, worth writing down: this pass read
`Data.wad`'s *resolving* GUI/plugin definition files and concluded the content
was absent, but `Data\Plugins\grids\*` resolves only if you already know the
path - the WAD's own name table does not carry it, and `Data\Plugins\grids`
alone (no file) does not hash to an entry. The string that gives it away is
`"%s\Definition.xml"` at `0x08a7d6a8` in `BOOT.BIN`, applied to the
`"Data\Plugins\grids"` at `0x08a7d38c`. **A negative over an archive whose
names are hashes is only ever a negative over the names you thought to try.**

### The full mode list the disc authors, seven against this project's four

`RaceBox_Definition.xml`'s `Mode` list (`Single Player`, confidence 92 - see
above) is the complete enumeration: **Arcade, Head2Head, Time Trial, Speed
Lap, Tournament, Zone, Elimination.** The executable's own `{ value, name }`
table at `0x08ab062c` gives the ordinals and two more names the front-end XML
never shows: **3 `Race`, 4 `Tournament`, 5 `Time Trial`, 6 `Zone`, 8
`Elimination`, 9 `Head2Head`, 10 `Speed Lap`, 11 `Custom Grid`, 12 `AI Race`**
- 7 is absent from the table and banks no result. Confidence 92, and the
speed-class table beside it at `0x08ab067c` is `0 Venom, 1 Flash, 2 Rapier, 3
Phantom`; see
[`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md). `oag_race::Mode::ALL` implements five of
the seven (Time Trial, Speed Lap, Zone, Single Race for the disc's `Arcade`,
and Eliminator, from 2026-09-08 - see
[race-modes.md](../gameplay/race-modes.md#eliminator)). English text for all seven is in `Data.wad`'s language tables -
five `.cod`-extension entries reached only through `oag-wad extract` (they
have no resolved path; the extractor names them by directory index and
hash), each a flat `<a c="IDSTRING" b="text"></a>` list. English is entry
**#1141** (hash `b96b2da0`), identified by content - `ER_YOU_ELIM` reads
"You have been eliminated!" there and something else in each of the other
four (French, German, Spanish, Italian - the PAL five-language set, see the
region note below):

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

**`docs/ui/hud.md` describes `Arcade_HUD.xml`, in passing, as "the
single-race and tournament layout"** - that page's own reading, cited here
rather than re-verified; this pass did not independently confirm the
tournament half of it. `docs/ui/hud.md`'s own census of the five layout XMLs
(`Arcade`, `Elimination`, `TimeTrial`, `Zone`, `MPTag`) is the evidence there
is no separate `Tournament_HUD.xml`. **This pass separately tried
`Data\XML\Head2Head_HUD.xml` directly (`oag-wad cat`) and it does not
resolve** - consistent with, but not the same claim as, hud.md's five-file
census. If both hold, an in-race tournament or head-to-head leg looks like
an ordinary single race, and only the surrounding menu chrome (`Cell Help`,
its target/points widgets, the end-of-tournament placement text below) is
campaign-specific - but that inference rests on hud.md's phrasing for the
tournament half, not on a probe run here.

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

**Superseded the same day by hard data, and the hedged prose turns out to be
exact.** Every one of the 236 `PI_Cell` records in
`Data\Plugins\grids\grid_NN.xml` carries a `laps` attribute, and across all 236
it is **3 for Venom, 4 for Flash, 4 for Rapier and 5 for Phantom** with no
exception, `7` for every `Speed Lap` cell and `0` (rendered `RC_INF`) for every
`Zone` cell. Confidence 90 - a flat census over 236 authored records. See
[`race-campaign.md`](../ghidra/functions/psp-pulse-usa/race-campaign.md).

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

- **The definition file is read, and it is Pulse's own file name.**
  `Data\Plugins\PI001\GUI\Selection_Definition.xml` - not a localised entry,
  resolves on `pure-psp-eu.chd` - holds `Class Selection`, `League Selection`,
  `Tournament Selection`, `Track Selection`, `Zone Track Selection` and
  `Team Selection` in full. Confidence 96 (direct read). This corrects an
  earlier "these files are unread" note that had never actually been checked
  against this exact path.
- **Five speed classes** - Vector, Venom, Flash, Rapier, Phantom - against four
  everywhere else. Confidence 94. This does **not** establish that Pulse has a
  fifth; [handling-stats](handling-stats.md) records that question separately
  and it stays open.
- **No AI difficulty row anywhere** in Pure's front-end XML. Confidence 94.
- **No unlock machinery authored in the XML** - zero `<Unlock>`, zero `Grid=`,
  zero `GSDisableEntriesBitField` across all eleven files. Confidence 94.
  Pure's `Show Unlocks` subtree is a post-race reward reveal, not a gate.
  **Gating still happens, just in code rather than data**: a fresh profile's
  `Class Selection` visibly lists only Vector and Venom (captured live in
  PPSSPP, 2026-09-10, see below) - Flash/Rapier/Phantom are hidden until
  something else unlocks them.
- **Its one authored preview is a 2D stat graph on `Class Selection`** - a
  shared `class_background.mip` and `classgraph.mip` plus a per-class
  `<class>graph2.mip` / `graph3.mip` pair, inside five `<Watch watch="Class">`
  blocks. All eleven textures resolve. Confidence 94. **This is the only flat
  2D preview found in any title, and it previews the speed class rather than a
  circuit or a craft.**
- Its variant axis is a two-state `<MenuBitmap name="Livery" maxItems="2">`
  toggle, against Pulse's four-way cycler.
- `Track Selection` and `Team Selection` author an empty `<Menu allocate="16">`
  and a `<Viewport>` and nothing else **in the XML** - the 3D preview each one
  draws is not authored there, the same way Pulse's two screens compose
  theirs in code rather than XML. Confirmed by direct capture, not left as a
  gap: see "Captured live in PPSSPP" below.

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
- **Pure's track and craft previews are sprites, not meshes** - 256x128
  images in `FEData.wad`, matched to the drawn pixels at RMSE 0 (see
  above). That is *why* neither `TeamSelection_ApplySelection` nor
  `TrackSelection_ApplySelection` composes a default preview path: there is
  no default preview mesh to compose. Only
  `TeamSelection_ApplySelection`'s Phantom special case composes a `.vex`
  at all (`%s\Phantom.vex`/`%s\VR\Phantom.vex`, confidence 65 on when it
  actually fires - the gating flags trace to campaign-progress and
  multiplayer-permission checks, not to the currently selected speed
  class). Which still belongs to which entry is **authored** in the entry's
  own `<location>\screen.xml` and needed no inference - see above.
- **No screen class was renamed in Ghidra.** The strings prove the classes
  exist; they are not evidence of what any function does, and the rubric's
  floor is 50 to rename at all.
