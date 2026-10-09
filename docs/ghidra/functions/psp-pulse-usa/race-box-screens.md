# `TrackSelection` and `TeamSelection`, decompiled

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

This is the follow-up to the string archaeology on
[`docs/formats/race-setup.md`](../../../formats/race-setup.md): both screen
classes were unreachable by cross-reference until the PSP relocation patch
landed (2026-09-07, see `HANDOVER.md`'s "Traps that are live"). With it
applied, `get_xrefs_to` on both class-name strings resolves cleanly -
`s_TrackSelection_08a84a00` to `0x088ee9f8`, `s_TeamSelection_08a84688` to
`0x088eb8e8` - and both classes decompile in full. **Nothing here is
runtime-verified**; every score below is capped by the confidence rubric's
70-84 "Probable" band unless stated otherwise.

## The registration pattern

Both classes register the same way the `Movie` widget does
([frontend-video.md](frontend-video.md)): a global-constructor-style function
with no callers found by `get_xrefs_to` (consistent with a `.ctors`/init-array
table Ghidra does not track as a call), that

1. inserts the class into a name-sorted registry (`FUN_0888fc68`, confidence
   73 - full body read, but the registry's own consumer, presumably the
   screen factory that looks a class up by name when the front-end XML says
   `type="TrackSelection"`, was not traced),
2. sets the object's vtable pointer directly (`DAT_08ad0ce4` for
   `TrackSelection`, `DAT_08ad0b04` for `TeamSelection`), and
3. prepends the object to a second, generic linked list (`FUN_08971afc`,
   not renamed - it is a plain list-prepend used by every registered
   widget and screen alike, so it carries no class-specific evidence).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x088ee9f8` | `TrackSelection_Construct` | 75 |
| `0x088eb8e8` | `TeamSelection_Construct` | 75 |
| `0x0888fc68` | `ScreenClass_Register` | 73 |

## The vtable: eight slots shared with `Movie`, more shared between the two screens

Both classes' vtables were read directly (`read_memory`, 128 bytes each) and
compared word-for-word against `Movie`'s (`0x08acdacc`). Every vtable in this
binary follows the same odd layout: **each slot is 8 bytes, a zero word
followed by the function pointer**, not 4 bytes as a plain MIPS vtable would
be. Confirmed identically on all three vtables inspected; the reason for the
zero word (RTTI companion word, a this-adjustor always zero for
single-inheritance calls, or a `struct { int; void*; }` pair) was not
determined.

Eight slots (word indices 3, 5, 7, 11, 15, 19, 21, 23) hold the **identical**
function pointer across `Movie`, `TrackSelection` and `TeamSelection` -
`0x089444d0`, `0x0894479c`, `0x089447a4`, `0x089447f4`, `0x0894484c`,
`0x08944864`, `0x089449a4`, `0x08944a30` - a common root class's virtuals
(destructor variants, `IsA`, and similar - none traced). Two more slots
(word indices 13 and 25: `0x08944804`, `0x08944a94`) are shared between
`TrackSelection` and `TeamSelection` but **not** `Movie`, consistent with an
intermediate `Screen` base class that `Movie` (a widget) does not inherit
from. Neither was traced either.

The remaining four slots diverge per screen, and that divergence is where
the actual screen behaviour lives:

| Slot (word idx) | `TrackSelection` | `TeamSelection` | Role |
| --- | --- | --- | --- |
| 9 | `0x088ed7dc` | `0x088eab14` | `Update`, called once a frame |
| 27 | `0x088ed4f4` | `0x088e9ab8` | scalar deleting destructor |
| 29 | `0x088ed63c` | `0x088e9c00` | `OnEnter` |
| 31 | `0x088ed6cc` | `0x088e9e5c` | `OnExit` |

## `TrackSelection`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088ed63c` | `TrackSelection_OnEnter` | 74 | resets the cached selection index to `-1`, seeds `+0xd8` from the current `Mode` global |
| `0x088ed7dc` | `TrackSelection_Update` | 80 | up/down cycling with a 0.25 s repeat, list (re)build on a state-change flag |
| `0x088edf3c` | `TrackSelection_PopulateList` | 80 | builds the reachable-circuit list; see below |
| `0x088ee2f8` | `TrackSelection_ApplySelection` | 80 | loads the preview mesh and the per-mode stat panel for the newly selected circuit |
| `0x088ed6cc` | `TrackSelection_OnExit` | 80 | writes the chosen circuit's name into the `Track` front-end global |
| `0x088ed4f4` | `TrackSelection_Destruct` | 72 | restores the base vtable, frees |

### `TrackSelection_PopulateList` closes "what populates the track list is unread"

`docs/formats/race-setup.md`'s open item is answered: `FUN_088edf3c`

1. seeds `+0xd8` (the current `Mode`, read once via the `Mode` front-end
   global on first entry) if not already set,
2. frees the previous list and collects **every** `PI_Track` definition
   through a generic allocate-and-grow helper (`FUN_088895b8`, not renamed -
   its allocation tag reads `"file"`, a source-location debug tag rather than
   a class name, so it carries no evidence of being track-specific beyond
   this call site),
3. keeps an entry only if `Definition_IsUnlocked` (`FUN_0888e29c`, 72 -
   see below) passes **and**, when `Mode == 6`, a second per-track byte flag
   at `+0x16e` is also set - this is the mechanism behind
   `race-setup.md`'s "why Custom Race offers three circuits": it is not
   only the absence of `<Unlock>`, it is a *second*, mode-gated flag that
   happens to be set on exactly those same three tracks. `Mode == 6` was not
   independently confirmed to mean "Custom Race" (no enum table was located),
   so read that identification at 55, below the naming floor - the
   PopulateList mechanics that use it are read directly from code and score
   higher on their own,
4. restores the previously-chosen circuit's index by name from the `Track`
   global if it is still present in the filtered list, and
5. runs a bubble-style adjacency sort that groups a track's `Black`/`White`
   pair together and always orders the entry whose name matches `Black`
   before the one matching `White` - the first **direct evidence** for how
   the two runs end up adjacent in the wrapping list `race-setup.md`'s
   PPSSPP capture already walked.

### `Definition_IsUnlocked` and the two-axis unlock read

`FUN_0888e29c` (confidence 88, `Definition_IsUnlocked`; raised from 72 on 2026-10-02 by the full decompile below) reads:

- `+0x99`: a hard-hidden byte: nonzero always fails.
- a "cheat/dev unlock" global (`sceKernelGetGPI() & 1`, or a flag byte at
  `DAT_08b31774+0x45f`): if set, everything is unlocked. **Confirmed live
  2026-09-29**: `DAT_08b31774` holds a pointer (the profile object), and
  writing `1` to the byte at that pointer `+0x45f` in PPSSPP makes Track Select
  offer all 24 circuits - see
  [ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29).
- otherwise it walks a linked list at `+0x9c` - the `<Unlock>` rows
  `race-setup.md` already reads from `Definition.xml` - and calls one of nine
  unnamed predicate functions per entry (`FUN_0888e6e8`,
  `FUN_0888e86c`/`FUN_0888e9e0`/`FUN_0888ea30`/`FUN_0888ebd8`, and four
  companion "has this unlock type" gates). **Named-unlock entries OR
  together** (any one passing unlocks the item) while **empty-name entries
  AND together** - matching an `Exclusive="true"` row needing every
  condition against a plain named unlock needing only one. None of the nine
  predicates were individually traced; that is real remaining work, not
  guessed at here.

**Traced 2026-10-02 (headless decompile of `/pulse/BOOT-psp-pulse-usa.BIN`),
confidence 88: the row combine and the `Team` row.** Per `<Unlock>` row
the loop first runs `FUN_0888e6e8`, the **context filter**: a row naming
`Class`, `Team`, `Track`, `Tournament`, `Mode` or `Grid` context attributes
(row fields `+0x8..+0x1c`) applies only when each matches the live front-end
global of that name, case-insensitively; a row that does not match is
skipped. Then:

- **Non-`Exclusive` row** (`*row == 0`): each condition the row names
  (`Medal`, `MedalCount`, `Loyalty`, `Grid`) is ANDed into a running result
  that starts true.
- **`Exclusive` row**: the result is set false, then the four `*_Met`
  predicates are tried in order and **any one passing returns true at once**.
  So `Exclusive` rows are **alternatives**: a variant with an own-team row and
  a `Team="any"` row is unlocked when *either* holds. A failed `Exclusive`
  row leaves the item locked unless a later row passes.
- No rows at all: unlocked.

`Unlock_LoyaltyMet` (`0x0888ea30`, now confirmed in full): the row's
`Team` string is compared against the literal `any` (`0x08a7d5ec`).
For `any` it collects every team definition (`FUN_088892d8`, a
grow-until-it-fits enumerator over the team class, 100 entries then +20) and
returns true as soon as **one** team's per-team record word (`record+8`, the
total `Loyalty_AccumulateTotal` writes) is `>=` the row's `loyalty`. It is
therefore **the best single team, not the sum**: two teams at 40,000 do not
meet a 60,000 `any` row. For any other name it looks up that team's record by
`Libc_HashString(name)` and compares the same word. A team with no record
fails (a zero requirement is also a fail - `Unlock_LoyaltyValue` returning 0
makes the predicate false).

**How the original presents a locked variant: absent.** `FUN_088ea08c`
builds `Team Selection`'s list as each team that passes
`Definition_IsUnlocked(team, 1)`, then a null-skin entry (`Classic`), then
each child definition that passes `Definition_IsUnlocked(child, 1)`; the
second argument of 1 also requires a class-level virtual check
(`vtable+0xd4`) that is not read. A locked skin is simply not appended, not
greyed and not priced. `TeamSelection_Update` draws the `Loyalty` block with
`FEScreen_SetStatBar(screen, "Loyalty", total, 100000, 150)` - see
`FEScreen_SetStatBar` above: the number beside the bar is the selected
team's own total and the bar is `150 * min(total, 100000) / 100000` pixels in
integer arithmetic. Not run live on PPSSPP: the static read is unambiguous
and the lane drew no breakpoint.

### `Top->Ship` has a confirmed referent after all

`race-setup.md` recorded `Top->Ship` as "no confirmed referent" from a
bounded live-capture probe. `TrackSelection_ApplySelection` resolves it:

```c
iVar3 = FUN_08890f30(*(int *)(param_1 + 0xf0) + 0x168, "Top->Ship", 0);
```

`param_1 + 0xf0` is **not** a static screen widget - it is the *currently
selected track definition object*, replaced every time the selection
changes (the old one is torn down via `FUN_088c4410` first). So `Top->Ship`
is a node inside **each track's own dynamically created preview scene**, not
a persistent widget on the `Track Creation` screen itself - which is exactly
why no button probe on the static widget tree ever found it. Once resolved,
the node is fed the mesh path `race-setup.md` already confirmed
(`%s\FE\%s.vex`, forward/reverse) through `FUN_089724b4` (sprintf) and
`FUN_088b9200` (a model-source setter, not independently named here).
Confidence 78 for the mechanism - unambiguous decompilation, matches the
already-confirmed path template exactly, not runtime-verified. The name
`Top->Ship` most likely survives from a shared preview-scene template also
used for an actual ship (compare `TeamSelection`'s equivalent node, named
`Info->Ship`, which *does* hold a craft) rather than describing what this
screen puts in it.

### The hexagonal window is a per-circuit `screen.xml`, and it shows stills

Read 2026-09-10, off the same function. Every branch of
`TrackSelection_ApplySelection` that changes the selection calls
`FUN_088c4410` on the definition object it just installed at `+0xf0`:

```c
FUN_088c4410(old_definition, "", 0);          // teardown of the previous one
...
if (definition->availableInZone /*+0x16e*/ == 0 || mode != 6)
    FUN_088c4410(definition, "Info", 0);
else
    FUN_088c4410(definition, "Zone", 0);
```

The strings at `0x08a84948`/`0x08a84950` are `Zone` and `Info`. The callee -
now **`TrackDefinition_EnterScreenState`** (`0x088c4410`, confidence 78) -
reads:

1. if the definition has no state machine at `+0x168`, return;
2. `StateMachine_FindState(+0x168, name)`; when the state is **not** there
   and `name` is non-empty, load a file into the machine first:
   `"%s\screen_zone.xml"` of the definition's `location` (`+0x94`) when
   `availableInZone` (`+0x16e`) is set and the `Mode` global reads `6`,
   `"%s\MP_Screen.xml"` when the third argument is set (split screen), and
   `"%s\screen.xml"` otherwise - through `FUN_08891448`, now
   **`StateMachine_LoadXml`** (`0x08891448`, 72: opens the file with
   `Xml_OpenFile`, allocates a state under the named parent and feeds every
   root element to `FUN_0889165c`);
3. `StateMachine_TransitionTo(+0x168, name, ...)`.

So the `Top->Ship` node of the previous section is a node of **that file**,
and the file is on the disc: `Data\Environments\16_Track\screen.xml`
(2,204 bytes on `pulse-psp-usa.chd`, dictionary-compressed like every other
front-end XML), plus `screen_zone.xml` and `MP_Screen.xml` beside it. Its
`<Screen name="Top">` holds the `<Mode3D><Model name="Ship">` the outline
ribbon is loaded into (`Src="%s\FE\forward.vex"`, `x=0 y=-300 z=-3700
RotX=1.5 RotY=0.1`, `OriginX=-145 OriginY=13 nearZ=1000 farZ=5000`) and,
under it, **a chain of `Info` states two seconds apart, each one nested
inside an anonymous `Screen` carrying one more `Image`**:

```text
Info -> Info2 -> Info3 -> Info4 -> Info3 Close -> Info2 Close -> Info
  1        2        3        4          3             2         cards
```

The cards are `%s\FE\image_01.mip` .. `image_04.mip` at `(50,70)`,
`(43,65)`, `(36,60)`, `(29,55)`, each card after the first over a
`Data\FE\Images\track_sel_shadow.mip` four units down and right of it,
`0xCF000000`. **The window shows stills, not a flythrough** - the reading
`race-setup.md` recorded off two captures a second apart was two different
stills, and the "stack of frames" visible in one of them is the chain three
deep. The hexagon is the still's own shape: `image_01.mip` is 256x128, 8bpp,
hex-cropped with its border baked in. Confidence 92 for the mechanism
(decompiled, and the file it names is on the disc and reads as described)
and 95 for what the window shows (the file's own content, drawn and
compared against the capture). Implemented in
`oag_ui_screens::picker::slideshow`; see `docs/ui/selection-screens.md`.

`FUN_088c4410` has four more callers (`FUN_088b09b4`, `FUN_088b12c8`,
`FUN_088ecb54`, `FUN_088ed05c`) besides `TrackSelection_OnExit`'s teardown -
untraced, presumably the tournament and split-screen track pickers, which
`MP_Screen.xml` and the PS2's `Track CreationSplit` screen both suggest.

The PS2 pressing's `screen.xml` (7,244 bytes, plain XML) is the same file
scaled onto its 640x448 grid - cards at `(67,115)`, `(57,107)` .. and the
shadow `301x211` - with **five** cards rather than four, and the `Zone`
chain (`zone_01.mip` .. `zone_05.mip`) authored in the same file rather than
in a `screen_zone.xml` of its own.

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088c4410` | `TrackDefinition_EnterScreenState` | 78 | loads `<location>\screen.xml` (`screen_zone.xml`, `MP_Screen.xml`) into the definition's state machine on first use, then transitions to the named state |
| `0x08891448` | `StateMachine_LoadXml` | 72 | opens a front-end XML file and adds its root elements as states under the named parent |

### The `Info Track %d.%d` layout hypothesis, still not settled either way

`TrackSelection_ApplySelection` switches on the `Mode` value cached at
`OnEnter` and picks one of two widget-count layouts via `FUN_088edc08`
(not traced further): a 2-widget layout for the default case (only
`Info1`/`Info3`, both cleared to blank) and a 7-widget layout for five other
`Mode` values, which additionally computes summed lap/stage records from a
handling-stats-shaped table (`FUN_088085d0`, not traced - **and that reading is
now corrected: it is the profile's saved-record store**, keyed by a definition's
name hash, so the summed figures are saved bests rather than handling stats. See
[`race-campaign.md`](race-campaign.md)). This dispatch is by
**`Mode`**, not by the circuit's own row-count property, so it neither
confirms nor refutes `race-setup.md`'s `Info Track %d.%d` = (count, index)
reading - it shows a *different* axis (which stat rows are shown) is
mode-driven, orthogonal to the still-open question of which numbered layout
template renders them.

## `TeamSelection`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088e9c00` | `TeamSelection_OnEnter` | 72 | resets cached music-select flag, seeds `+0xd8` |
| `0x088eab14` | `TeamSelection_Update` | 78 | craft/livery cycling, stat-bar refresh, Eliminator/Normal variant pick |
| `0x088e9e5c` | `TeamSelection_OnExit` | 74 | writes the pre-race music selection string, frees the roster list |
| `0x088e9ab8` | `TeamSelection_Destruct` | 72 | restores the base vtable, frees |
| `0x088ea6b8` | `FEScreen_SetStatBar` | 82 | shared bar-widget setter (see below) |

`TeamSelection_Update` is the single largest function in either class. Once a
team and craft are picked it:

1. resolves the four rating bars (`Speed`, `Thrust`, `Handling`, `Shield`)
   and the `Loyalty` bar through `FEScreen_SetStatBar`, reading the ratings
   off a per-craft record via `FUN_08808664` (offsets `+0xb4`/`+0xb8`/`+0xbc`/
   `+0xc0`) - a **different** table from the `<Handling>` XML
   [handling-stats.md](../../../formats/handling-stats.md) documents; this
   is a display-rating record, not the physics tuning block, and its own
   source file/struct was not located,
2. distinguishes three variant classes by comparing the resolved skin id
   against `6`, `8`/`0x12`, or neither - the `6` branch wires the fixed
   skin flag against the literal string `"Eliminator"`, the `8`/`0x12`
   branch against `"Normal"` (Eliminator mode itself does not exist in this
   project yet - see `docs/gameplay/race-modes.md` - but the disc's own
   Eliminator-variant livery selection is wired here, in code, for whenever
   that mode is picked up), and
3. picks `"ER_SUG_SHIP"` vs `"ER_FOR_SHIP"` help text depending on whether
   the current craft appears on a per-team suggestion/forced-ship list
   (`FUN_08973424`), which is a live mechanism `race-setup.md` did not
   record at all.

`FEScreen_SetStatBar(screen, label, value, max, scale)` (confidence 82,
clean and unambiguous) sets a `<Text name="{label}">` to `%d` of `value` and
a sibling `<Image name="{label} Bar">`'s fill fraction to
`scale * min(value, max) / max` - the generic widget both `Speed`/`Thrust`/
`Handling`/`Shield`/`Loyalty` on `Team Selection` go through.

## `LeftLayer`'s own `transition` attribute is a fade duration

Both screens' info panel is wrapped in `Data\Plugins\PI001\GUI\
Selection_Definition.xml`'s own `<LeftLayer transition="0.5">` -
`docs/ui/selection-screens.md` already read that the attribute exists and
that a live capture shows a card mid-arrival, but not what the number means.
It is a fade-in/fade-out duration in seconds, on every widget kind alike,
not a slide or a scale.

**The evidence is the generic widget constructor, not the screen classes.**
Every XML element - `LeftLayer`, `Item`, `Text`, `Image` and so on -
passes through `Widget_CreateFromElement` (`0x088920fc`, confidence 72:
the attribute set matches the disc's own XML exactly and unambiguously, but
the surrounding class-dispatch machinery was read only at the call site).
It reads a fixed base-attribute set off the element - `Global`, `Focus`,
`Default`, `Transition`, `EnableTransition`, `DisableTransition`,
`OffsetX`/`OffsetY`/`OffsetZ`, `Delay`, `StartEnabled`, `PushWidth`,
`PushHeight`, `GSDisable`, `DontUsePass2` - before creating the widget
through the class registry and recursing into its children. The relevant
four lines:

```c
fVar10 = local_768;                          // EnableTransition
if ((local_768 == FLT_MIN) && (fVar10 = local_76c, local_76c == FLT_MIN)) {
    fVar10 = *(float *)(iVar7 + 0x68);       // inherited, if neither is authored
}
*(float *)(iVar7 + 0x68) = fVar10;           // the widget's own enable-fade seconds

if (local_764 == FLT_MIN) {                  // DisableTransition
    fVar10 = local_76c;                      // falls back to Transition
    if (local_76c == FLT_MIN) { fVar10 = *(float *)(iVar7 + 0x6c); }
    *(float *)(iVar7 + 0x6c) = fVar10;
} else {
    *(float *)(iVar7 + 0x6c) = local_764;    // DisableTransition, when authored
}
```

(`local_76c`/`local_768`/`local_764` are `Transition`/`EnableTransition`/
`DisableTransition`; `FLT_MIN` is this reader's sentinel for "not authored",
same idiom the rest of the function uses.) So `transition="0.5"` on a
`LeftLayer` sets both the enable- and disable-fade of every widget under it
to half a second, unless a widget declares its own `EnableTransition`/
`DisableTransition` - neither of which either selection screen's own XML
does, only the plain `transition` name.

**A live capture confirms the shape is a fade, not a slide.** Walking
`pulse-psp-usa.chd` under PPSSPP into `Track Creation` and screenshotting
every ~130ms of the first 1.2 seconds: the panel (`transition="0.5"`) and
the hexagonal window's own first card are both faint-to-invisible at 0-130ms
and fully settled by 320-480ms, while the title bar's own group
(`<LeftLayer transition="0">`) is solid from the very first frame - matching
`transition`'s reading as a duration exactly, and ruling out a position or
scale change (nothing in the frames moves or resizes; only opacity climbs).
The *curve* (linear vs eased) is not settled by four screenshots a tenth of
a second apart - implemented as a linear ramp, **chosen, not measured**.
Confidence 85 on "a per-widget alpha fade, `transition` seconds long" as the
mechanism (static decompile plus a live capture at the predicted timing);
no score on the ramp's own shape.

**The hexagonal window's own stills author no `transition` of their own** -
the circuit's per-entry `screen.xml` (`docs/ui/selection-screens.md`) has no
`LeftLayer` and no `transition` attribute anywhere in it - so their fade in
this build (`oag_game::preview::CARD_FADE_SECONDS`) reuses the panel's own
measured `0.5`s rather than inventing an unrelated number, on the strength
of the capture above showing both arrive in the same window. Implemented in
`oag_ui_screens::picker::body` (the panel, reading `Text`/`Image`/`Fill::transition`
off `oag_ui::screen`'s own `LeftLayer` inheritance) and
`oag_game::preview::fade_draw` (the cards, shared between the live picker
stage and the `--menu-page --menu-picker-seconds` still capture). See
`docs/ui/selection-screens.md`'s own "cards slide in" section, now closed.

## The per-widget fade is confirmed linear, 2026-09-28

Chasing [menus-original.md](../../../ui/menus-original.md)'s own open item on
`Tween::eased`'s invented page-transition curve. **The generic
per-widget fade ramp is exactly linear, both in the decompile and live.**
`Widget_UpdateTransitionFraction` (`0x0888d8e4`, confidence 80) is the
function every widget kind's own `Update` calls (found by intersecting
`lwc1 ...,0x68(a0)` and `lwc1 ...,0x6c(a0)` - reading both transition
durations on the same register - across the whole binary, which turns up
exactly one function):

```c
undefined4 Widget_UpdateTransitionFraction(float dt, int widget) {
    float enable_dur = widget->0x68;     // EnableTransition seconds
    if (enable_dur == 0.0) {
        // +0x90 hold, then jump straight to the enabled/disabled state -
        // only reachable when no EnableTransition is authored at all.
    } else if (widget->0x60->0xbe == 0) {         // parent says "disabled"
        float disable_dur = widget->0x6c;          // DisableTransition seconds
        widget->0x8c = clamp(widget->0x8c - dt, 0.0, disable_dur);
        widget->0x64 = disable_dur == 0.0 ? 0.0 : widget->0x8c / disable_dur;
    } else {                                        // parent says "enabled"
        if (widget->0x90 > 0.0) { widget->0x90 -= dt; }   // hold, enable branch only
        else {
            widget->0x8c = clamp(widget->0x8c + dt, 0.0, enable_dur);
            widget->0x64 = widget->0x8c / enable_dur;
        }
    }
}
```

No curve anywhere: both the enable and disable branches are `elapsed / duration`,
a plain linear ramp, clamped at the ends. `+0x64` is the fraction every widget's
own `Draw` presumably reads (not traced this pass) - it is what
`Widget_CreateFromElement`'s existing "fade duration" finding actually drives.

**Confirmed against a live capture, not just the decompile.** `pulse-psp-usa.chd`
under PPSSPP 1.20.4, breaking on this function on every hit:
across several thousand hits spanning many different widget kinds and screens
(the front end's own attract-idle timer having fired at least once mid-session,
corroborating `MainMenu_Update`'s own 60-second idle-to-demo reading elsewhere
in this doc), `dt` sat at `0.033360`-`0.033381` on every single hit - six
distinct values within 21 microseconds of each other, consistent with a
per-frame `dt` genuinely measured off the front end's own clock (`DAT_08b317b0`,
a pointer - `*DAT_08b317b0 + 0x40` is the float seconds field) rather than a
hand-tuned constant, and with real per-frame variance too small to explain any
visible curvature on its own. `+0x68`/`+0x6c` matched `CellMode_Definition.xml`'s
authored `0.5` and `0.7` durations exactly on the widgets checked, and `+0x64`
was pinned at exactly `0.0` or `1.0` on every steady-state sample - never a
partial value - which is what a ramp that reaches its endpoint and clamps there
should look like.

**This does not settle the page-level zoom-and-crossfade `Tween::eased`
exists for, and it should not be read as though it does.** Two static reads
this pass ruled out as the source of that curve:

- **`Widget_SubtreeTransitionFraction`** (`0x0889050c`, confidence 75) is a
  recursive `max()` of `Widget_UpdateTransitionFraction`'s own `+0x64` across
  a widget's active subtree, used only to answer "is anything under me still
  fading" - rate-limited to `1.0` (busy) for the first `0.1` s after a widget
  is shown (`+0xc8`, stamped by `FUN_0889007c`'s own widget-arrival handler).
  It never computes a value anything draws.
- **`Widget_DestroyIfTransitionSettled`** (`0x08890358`, confidence 72), the
  first thing `MainMenu_Update` calls every frame, is pure garbage collection:
  once the subtree fraction above reports `0.0`, it queues the widget for
  destruction. Also not a drawn value.

**A live-measured discrepancy argues against `Widget_UpdateTransitionFraction`
alone driving the outgoing page's own alpha**, even though it is the same
`transition="0.5"` mechanism `Widget_CreateFromElement` already ties to
`LeftLayer`. [menus-original.md](../../../ui/menus-original.md) measured the
outgoing page's alpha at `0.55` eleven presented frames into a `0.5` s (fifteen-
frame) transition; a raw linear disable ramp from `1.0` predicts `1 - 11/15 =
0.267` at that frame, not `0.55`. Two live-capture attempts this pass tried to
resolve this directly (breaking on `StateMachine_TransitionTo`,
`0x0889123c`, to catch the transition's own first frame with no free-running
gap) and neither landed a clean capture of the actual page-swap in progress -
the attract-mode idle timer and a stray extra button press both derailed
individual runs, and the sessions available this pass ran out before a third
attempt. **Left open, not guessed at.** A plausible unifying account - the
outgoing widget still takes the *enable* branch (with its own `+0x90` hold)
for the first frame or two, until `+0xbe` actually flips once the state's own
`OnExit` propagates down the tree - is written down here as a hypothesis for
whoever picks this up next, not as a finding: nothing in this pass traced
`+0xbe`'s setter or confirmed the timing.

`FUN_08890248` (`StateMachine_TransitionTo`'s own OnEnter/OnExit/OnLeave
dispatch) and `LeftLayerTransition` (a `Dialog`-only XML attribute name,
distinct from the generic `Transition`/`EnableTransition`/`DisableTransition`
trio) were also read this pass and are vtable dispatch and an unrelated
attribute respectively - neither is a curve either.

**`Tween::eased` (`crates/ui/src/anim.rs`) is unchanged, and its `invented`
marker (confidence 30) stays.** Nothing this pass found is evidence for *any*
particular shape of the zoom-and-crossfade curve, linear included - see the
discrepancy above. Swapping the invented `t*t` for an equally unproven `t`
would trade one unverified guess for another; the project's own rule against
tuning a curve to look right cuts against doing that on the strength of "linear
elsewhere in this engine" alone.

## `Mode3D`'s own fixed camera, and the child `Model`'s own pose, 2026-09-28

Chasing this thread's own open item - the race-box outline was framed by a
capture-read orbit (`oag_game::preview::orbit_for`), not by
`Track Creation`'s `screen.xml` own `<Screen name="Top"><Mode3D><Model>`
(`docs/ui/selection-screens.md`: `x=0 y=-300 z=-3700 RotX=1.5 RotY=0.1` on
the `Model`, `OriginX=-145 OriginY=13 nearZ=1000 farZ=5000` on the `Mode3D`
around it). The camera is now read from the disc and drawn from
(`oag_game::preview::mode3d_view_projection`, `crates/game/src/preview.rs`).

**The `Mode3D` widget's own attribute set is `Mode3D_ReadValues`**
(`0x088b7c8c`, confidence 85: exact attribute-name strings - `OriginX`,
`OriginY`, `nearZ`, `farZ`, `mirror`, `mode` (compared against the literal
string `orthographic`), `FirstPass` - plus a live capture at `Track
Creation` reading the exact authored values, `0.0/0/0/-145.0/13.0/1000.0
/5000.0` for `mode`/`FirstPass`/`mirror`/`OriginX`/`OriginY`/`nearZ`/`farZ`,
off the widget `Mode3D_EnterView` and `Mode3D_DrawQueued` both hit on
`0x08fcd150`, `mode3d_capture.json`). The widget's
own constructor (`FUN_088b7a04`, not renamed - clear from context but no
attribute strings of its own to anchor a name to) defaults `OriginX`/
`OriginY` to `0`, `nearZ`/`farZ` to `20.0`/`100.0` and `mode`/`mirror` to
false, matching `Mode3D_ReadValues`'s own field offsets exactly.

**No camera position or rotation exists on the `Mode3D` widget at all** -
only `Origin`/`nearZ`/`farZ`. The camera sits at the origin, always looking
down `-Z`; what moves is the child `Model`'s own authored pose.

**Two functions build the projection matrix from `nearZ`/`farZ`, and only one
of them is the one that actually reaches the polygons.** Both read cleanly as
a standard symmetric perspective frustum - `Y`-scale `1/tan(halfFovY)`,
`X`-scale that divided by a hardcoded `1.7647059` (`480/272`, the PSP's own
screen shape, not the panel's), the usual near/far terms - and both are
reached only through the widget's own `+0x38` class-info block, never called
directly (`get_xrefs_to` on either address returns nothing):

- **`Mode3D_EnterView`** (`0x088b7f24`, confidence 75) runs from
  `FUN_08944160`, the generic widget-tree draw traversal (`(**(vtable+0x34))
  (...)` on every visible widget), with a fixed half-FOV `0x3f11361e`
  (`FUN_0897e414`, a range-reduced `tanf` - confirmed by its call into
  `__rem_pio2f` - so this is the input angle, not its tangent) - `0.566896`
  rad, `~64.97 deg` full FOV. It never touches the GE viewport offset, and
  ends by calling `Gfx_Enqueue` to submit the widget for a **deferred** draw.
- **`Mode3D_DrawQueued`** (`0x088b8548`, confidence 78) runs from
  `Gfx_FlushRenderManager`'s own queue-replay loop (`(**(vtable+0x44))(item,
  manager, sort_key)`, the callback every queued `Gfx_Enqueue`d item gets at
  actual polygon-submission time, after the queue's own sort) - the return
  address sits right after that indirect call. Its own half-FOV is a fixed
  `0.5` rad exactly, `~57.30 deg` full FOV - **not** the same number
  `Mode3D_EnterView` computes. It also calls `Gu_SetOffset` (`0x08811220`,
  confidence 85: its own default-argument call passes `0x710, 0x778` =
  `1808, 1912` = `2048 - 240, 2048 - 136`, the PSP's screen half-extents
  subtracted from the GE's `2048` viewport-centre convention exactly) with
  `OriginX + 1808, OriginY + 1912` - i.e. `OriginX`/`OriginY` are a plain
  screen-pixel shift of the viewport's own centre. `Mode3D_LeaveView`
  (`0x088b8488`, confidence 75, the matrix-stack pop paired with
  `Mode3D_EnterView`) restores the same default `1808, 1912` on its own way
  out, corroborating the pair.

  Because the deferred callback is the one still running when the GE
  actually consumes the vertex buffer, **`Mode3D_DrawQueued`'s numbers -
  `0.5` rad and the `Origin` shift - are read as the ones that reach the
  screen**, and `Mode3D_EnterView`'s own `0.566896` rad is superseded before
  that happens. Not single-stepped through an entire frame to watch the
  supersession directly, so this is a call-graph reading, not a breakpoint on
  the rasteriser itself.

**The child `Model`'s own attribute set is `Model_ReadValues`** (`0x088b8cf4`,
confidence 85: exact attribute-name strings again - `X`, `Y`, `Z`, `RotX`,
`RotY`, `RotZ`, `orthoScale`, `orthoScaleX/Y/Z`, `colour`, `src`,
`startPaused`, `Ztest` - matching the disc's own `x y z RotX RotY` on this
circuit's `Model` element letter for letter). Loading a mesh
(`Model_SetSrc`-shaped, `0x088b9200`, not renamed: it calls `Vex_LoadModel`
with the read `src` and, at its own end, calls the transform builder below)
and every `Values` read both end by calling **`Model_BuildTransform`**
(`0x088b98b4`, confidence 78): builds a `4x4` from `X/Y/Z` as a translation,
then composes `RotX`, `RotY`, `RotZ` in that order via three `vmmul_q`s, each
overwriting the running matrix - `M := RotAxis * M` - then applies
`orthoScale`/`orthoScaleX/Y/Z` as a per-axis scale, then commits the result
through `FUN_08945284` (not renamed - it just copies 16 floats into the
node's own transform slot, no attribute strings of its own).

**Confirms radians, not degrees, for `RotX`/`RotY`/`RotZ`.** Each rotation's
own sine/cosine comes from `vcos_s`/`vsin_s` fed `angle * vcst_s(5)` -
`vcst_s(5)` is the Allegrex VFPU's `2/pi` constant, the standard
pre-multiply the PSP's own trig microcode needs because it treats its input
as a fraction of a half-turn rather than a plain radian - so the value
multiplied in is the angle already in ordinary radians, exactly what
`RotX="1.5" RotY="0.1"` on the disc read as.

**What this pass did not settle: the rotation's own composition order in
this renderer's convention.** `Model_BuildTransform`'s own matrix ends up
`RotZ * RotY * RotX * T` as a literal left-to-right product (`RotZ` was the
*last* `vmmul_q`, so it sits leftmost). Read as PSP code's own row-vector
convention (translation sits in the matrix's last row, the row-vector
signature), a point transforms as `v * RotZ * RotY * RotX * T` - `RotZ`
applied first, `T` last. `oag_game::preview::mode3d_view_projection` instead
builds `T * RotX * RotY` in this renderer's column-vector convention (`RotZ`
dropped, since this circuit authors none) - a plausible transpose of the
row-vector reading, not one confirmed by stepping through an actual frame.
A live PSP screenshot at this reading looks right against a live PPSSPP
capture of `Track Creation`
(`track-select-psp.png`
against a live capture of the same panel) - the ribbon sits in the right
place, at the right scale, right-side up - but `RotY` on this circuit is a
small `0.1` rad, small enough that a wrong composition order would be hard
to see on this circuit alone. Left open for whoever picks up a circuit with
a larger `RotY`, or a breakpoint on `FUN_08945284` dumping the matrix it
commits.

**The PS2's own `OriginX`/`OriginY` sign was not read from `psp-pulse-eu`'s
PS2 sibling, and is inferred instead.** This circuit's two pressings author
`OriginX=-145 OriginY=13` (PSP) against `OriginX=193 OriginY=-21` (PS2) -
within rounding of `-(-145, 13) * (640/480, 448/272)`, i.e. the PS2's own
values read as the PSP's own, negated, scaled to the PS2's `640x448` grid.
Applying the PSP's own sign to the PS2's raw values landed the outline
bottom-left of the panel instead of inside it
(confirmed live, `track-select-ps2.png`
before the flip, `track-select-ps2-fixed.png` after); `oag_game::preview`
negates `OriginX`/`OriginY` on the PS2's own grid to match. No PS2 Ghidra
program was consulted for this pass - the sign comes from the two pressings'
own authored numbers agreeing on a scale factor, not from PS2 code.

## What is still open after this pass

- **Implemented 2026-10-02 (`oag_game::unlock`)**: a locked circuit is absent from
  Track Select, not greyed - the list filter above plus the 3-vs-24 live capture.
  The `+0x16e` mode-gated byte **is `availableInZone`** (2026-10-02,
  [zone-start.md](zone-start.md): the definition loader writes it from that attribute, and
  Mode 6 is `Zone`), already modelled by the Zone list. Not modelled: the `+0x99`
  always-hidden byte.
  The `Team="any"` combine was traced 2026-10-02 (above) and craft variants
  are now gated; see `oag_game::unlock::loyalty_unlocked`.

- Of the nine unlock-predicate functions inside `Definition_IsUnlocked`, the
  Medal/MedalCount/Loyalty/Grid ones are named and read (`race-campaign.md`);
  `FUN_0888e6e8` (the context filter above) and the four companion "has this
  unlock type" gates are still unnamed.
- `Mode == 6` reading as "Custom Race" is a 55-confidence hypothesis, not a
  finding - no enum table was located. **An enum table has since been located**
  (`0x08ab062c`, see [`race-campaign.md`](race-campaign.md)) and in *that* table
  `6` is **`Zone`**, not a custom race - which would make
  `TrackSelection_PopulateList`'s extra `+0x16e` gate "the circuits Zone is
  playable on", a reading that fits three circuits at least as well. Whether the
  `Mode` global this screen caches is the same enumeration was **not** checked,
  so this stays a lead: 60, and the `Mode == 6` identification above is
  superseded but not yet replaced.
- The per-craft rating record `FUN_08808664` resolves (`+0xb4..+0xc0`) is a
  distinct table from `HandlingStats.xml`, unlocated.
- `FUN_088edc08`'s layout dispatch and `FUN_088085d0`'s record lookup are
  read only at the call site, not decompiled themselves.
- None of this is runtime-verified. A PPSSPP capture stepping through
  `TrackSelection_PopulateList` or `TeamSelection_Update` with a breakpoint
  would move several of the 70s here into the 90s.
- **The page-level zoom-and-crossfade `Tween::eased` exists for is still
  unlocated.** `Widget_UpdateTransitionFraction` is confirmed linear but does
  not by itself explain the outgoing page's measured `0.55` alpha at frame 11
  (see above); no function that reads a widget's own scale, or that shapes
  `+0x64` into anything but a straight pass-through, has been found.
  `+0xbe`'s setter (who actually flips a widget's parent-enabled flag on a
  state exit) is untraced, which is what a live capture of the transition's
  first two or three frames - not yet landed - would settle.

## Cross-platform: one exact-hash match on `psp-pulse-eu`

Per this project's EU-preference (`psp-pulse-eu` is the Ghidra target of
record, [ADR-0048](../../../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)),
all fifteen functions named above were checked against `/pulse/BOOT-psp-pulse-eu.BIN`
using the exact-opcode-hash method
[`exact-hash-transfer.md`](../psp-pulse-eu/exact-hash-transfer.md) already
established: `get_function_hash` on each USA address, matched against a full
`get_bulk_function_hashes` pull of all 10,672 EU functions.

**Only `Definition_IsUnlocked` (`0x0888e29c` USA) has an exact hash match**,
at `0x0888e0c4` on `psp-pulse-eu`. `diff_functions` confirms it: 123/123
instructions equal, zero added, zero removed - the callee list differs only
in naming (`FUN_0888ea30` etc. on one side, differently-addressed but
logically identical callees on the other), the same asymmetry
`exact-hash-transfer.md` already documents for every one of its 115 rows.
Applied to `docs/ghidra/functions/psp-pulse-eu/names.tsv` at USA confidence
minus 5 (72 → 67, below the 70 floor so the applier suffixes it `_q`).

**The other fourteen have no exact-hash match on `psp-pulse-eu` at all**, and
a single-address fuzzy check (`find_similar_functions_fuzzy`) against the
same binary returned nothing above 0.9 and only a 0.55 candidate at 0.5 for
`FEScreen_SetStatBar` - too weak to act on under this project's rule against
renaming off a score alone. That is a real difference from
`exact-hash-transfer.md`'s collision/physics-code sweep, where 115 of 131
candidate functions matched exactly: **this front-end/menu code region
diverges more between the USA and EU pressings than the simulation code
does**, plausibly because the front end carries region-specific text and
menu wiring where the simulation does not. Confirming that with a proper
`bulk_fuzzy_match` sweep (descending thresholds, collision-filtered, each
candidate individually diffed) rather than one address at a time is left for
whoever picks EU coverage of this area up next.

## History

- 2026-09-28, `pulse-easing` lane: `Widget_UpdateTransitionFraction`
  (`0x0888d8e4`), `Widget_SubtreeTransitionFraction` (`0x0889050c`) and
  `Widget_DestroyIfTransitionSettled` (`0x08890358`) named - the per-widget
  linear fade ramp `Widget_CreateFromElement`'s own `+0x68`/`+0x6c` feed, and
  the subtree-completion/garbage-collection pair built on top of it. See "The
  per-widget fade is confirmed linear" above. Chasing
  [menus-original.md](../../../ui/menus-original.md)'s page-transition-curve
  item; did not close it - the page-level zoom-and-crossfade curve is still
  unlocated, see "What is still open" above.
- 2026-09-28, `pulse-mode3d` lane: `Mode3D_ReadValues` (`0x088b7c8c`),
  `Mode3D_EnterView` (`0x088b7f24`), `Mode3D_LeaveView` (`0x088b8488`),
  `Mode3D_DrawQueued` (`0x088b8548`), `Gu_SetOffset` (`0x08811220`),
  `Model_ReadValues` (`0x088b8cf4`) and `Model_BuildTransform` (`0x088b98b4`)
  named - the race box's fixed `Mode3D` camera and the child `Model`'s own
  pose, closing this thread's own "outline framed off the capture" item. See
  "`Mode3D`'s own fixed camera, and the child `Model`'s own pose" above and
  `oag_game::preview::mode3d_view_projection`. Left open: which of the two
  projection functions actually reaches the rasteriser is a call-graph
  reading, not a breakpoint on the draw itself; the rotation composition
  order is a plausible transpose; the PS2's `Origin` sign is inferred from
  the two pressings' own numbers, not read off `psp-pulse-eu`.
- 2026-09-28: `Widget_CreateFromElement` (`0x088920fc`) named - the generic
  XML-element-to-widget constructor every front-end screen's tree goes
  through, and the one that reads `LeftLayer`'s own `transition` attribute
  as a per-widget fade-in/fade-out duration (`EnableTransition`/
  `DisableTransition`, falling back to plain `Transition`). Confirmed
  against a live PPSSPP capture: the panel and the hexagonal window's first
  card are both faint at 130ms into `Track Creation` and settled by
  320-480ms, matching the XML's own `transition="0.5"`, while the title
  bar's `transition="0"` group is solid from the first frame. Built in
  `oag_ui_screens::picker::body` and `oag_game::preview::fade_draw` - see the new
  section above and `docs/ui/selection-screens.md`.
- 2026-09-10: `FUN_088c4410` and `FUN_08891448` named, off
  `TrackSelection_ApplySelection`'s own call; the per-circuit `screen.xml`
  found and read on both pressings. The "flythrough" reading is retired.
- 2026-09-08: first pass, immediately after the PSP relocation patch
  (`HANDOVER.md`, 2026-09-07) unblocked `get_xrefs_to` on both class-name
  strings. Confirmed live before decompiling anything:
  `get_xrefs_to("TrackSelection")` → `FUN_088ee9f8`,
  `get_xrefs_to("TeamSelection")` → `FUN_088eb8e8`, both "No references
  found" as recently as 2026-09-05.
