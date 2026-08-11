# The in-race HUD

**Status: the layout is understood, confidence 95.** Pulse's HUD geometry is
**authored data, not code**, and it decodes with the front-end XML machinery this
project already had. Every rectangle, atlas sub-rectangle, colour, font role and
alignment is read off the player's own disc. Implemented in
[`oag_game::hud`](../../crates/game/src/hud.rs); pinned against all five shipped
layouts by `crates/game/tests/hud_layout_ground_truth.rs`.

95 rather than higher because the *geometry* is read directly from shipped data
and closes on independent counts, while **what drives each widget** - which are
visible when, how a bar's fill maps to a value - is still inference. Those are
scored separately below.

## The correction that produced this page

The first pass concluded the HUD was drawn from code and planned to recover
element rectangles by measuring emulator screenshots and reading the decompiler.
That was wrong, and the way it was wrong is worth keeping:

- [`fexml.md`](../formats/fexml.md)'s known-element table has no HUD, gauge,
  meter or bar element.
- [`frontend-boot.md`](../architecture/frontend-boot.md) has no in-race screen in
  the `Skin.xml` tree.

Both true, and the conclusion drawn from them was false. **A table of what a
parser happens to model is not evidence about the format.** The layouts were in
`Data.wad` the whole time, one directory away from files this project already
read. The cheap check that would have caught it - grep the archive listing for
`HUD` - was never run, because the negative evidence felt sufficient.

## Where it is

| Asset | Location | Notes |
| --- | --- | --- |
| 5 layout XMLs | `Data.wad`, `Data\XML\*_HUD.xml` | shortened front-end XML; expand with the file's own `<code>` dictionary |
| Atlas | `Data\HUD\Textures\PulseHUD.mip` | 256x256, 8 bpp. **84 of 100** `Src=` references across the five layouts. Ships **twice**, in `FE.wad` and `Data.wad`, byte-identical at 66,576 bytes |
| `HUD` font | `Data\FE\Fonts\PulseHud.fnt` | 25 px line height. See [fnt.md](../formats/fnt.md) |
| `HUDSmall` font | `Data\FE\Fonts\small.fnt` | 10 px line height |
| `Default` font | `Data\FE\Fonts\pulse_text.fnt` | 13 px; used only by the eight opponent name tags |

### The PS2 ships the same HUD and hides both halves of it

Same five layouts under the same names, the same widget counts, and - the part
that settles it - the **same `U`/`V`/`TxtrWidth`/`TxtrHeight` boxes**, so both
discs index the identical 256x256 atlas arrangement. Only the on-screen
geometry differs, and **mostly** by one ratio: `SpeedBarBg` is
`x=8 y=16 w=224 h=43` where the PSP has `x=6 y=10 w=168 h=26`, which is
640/480 horizontally and 448/272 vertically.

**Not every coordinate follows it**, and the exceptions are the reason a PS2
sweep is not a one-line change. Measured across `Arcade_HUD.xml`'s 141
coordinate values on both discs, 121 are the PSP's own scaled and rounded to an
integer; the 20 that are not are the nine `<Mode3D><Model>` placements, which sit
at the PSP's `x="-240" y="136"` unchanged in an orthographic mode, and
`TimeDiffIcon`'s two small negative offsets. This paragraph said "exactly one
ratio" until 2026-08-09. That is the front-end coordinate space, not a HUD
question - see the `SCREEN` thread in [`HANDOVER.md`](../../HANDOVER.md).

Two things kept it off the screen entirely, and each hid the other:

| | PSP | PS2 |
| --- | --- | --- |
| Layout XML | shortened, `<code>` dictionary | **plain `<?xml`** |
| Atlas entry | `Data\HUD\Textures\PulseHUD.mip` | hashes to nothing; entry **3518** holds it |

**Shortening is per file, not per platform.** The PS2's own `Skin.xml` *is*
shortened while its `Data\XML\*_HUD.xml` are not, so a reader that calls
`fexml::expand` unconditionally rejects a perfectly good layout with "no
`<code>` dictionary element" and the whole HUD goes with it.
[`fexml::text`](../../crates/formats/src/fexml.rs) decides from the blob's own
first bytes instead.

The atlas is an ordinary [PS2 texture](../formats/ps2-texture.md) that the
archive does not file under the name the layout asks for. **How the game
performs that lookup is unknown** - 60 spellings of the name miss all 7,393
hashes on the disc, the directory-position rules that solve models and fonts do
not apply, and a repeated 3,656-byte blob beside the HUD assets that looked
like a name table was checked and ruled out. The entry was found by its picture
and is recorded as a hash in
[`oag_pulse::PS2_IMAGES`](../../crates/pulse/src/lib.rs), which is
what that module exists for. See its documentation for the evidence and
`crates/assets/tests/ps2_image_ground_truth.rs` for the check that re-derives
it rather than asserting the constant against itself. **Confidence 90.**

The atlas's flag byte at `+0x07` is `2`, so `FLAG_SWIZZLED` is **clear** and it
takes the linear path in [`texture.rs`](../../crates/formats/src/texture.rs) - it
is one of the 7 of 13 `FE.wad` textures that are *not* pre-swizzled, unlike the
five `.fnt` atlases, which all are. Size arithmetic closes exactly:
`16 + 256*4 + 256*256 = 66576`.

```sh
just wad list <image>:PSP_GAME/USRDIR/Data.wad
oag-wad cat --expand <image>:PSP_GAME/USRDIR/Data.wad 'Data\XML\TimeTrial_HUD.xml'
```

## The five layouts

One per race mode. Counts are `<Image>`, `<Text>` and `<Mode3D><Model>` elements,
measured with `grep` over the expanded files - independently of the parser, so the
ground-truth test is not checking the parser against its own output.

| Entry | Images | Texts | Models |
| --- | ---: | ---: | ---: |
| `Data\XML\Arcade_HUD.xml` | 35 | 35 | 11 |
| `Data\XML\Elimination_HUD.xml` | 33 | 35 | 11 |
| `Data\XML\TimeTrial_HUD.xml` | 10 | 25 | 2 |
| `Data\XML\Zone_HUD.xml` | 7 | 24 | 2 |
| `Data\XML\MPTag_HUD.xml` | 0 | 8 | 0 |

Most widgets are inactive in any given frame; these are sizes of the layout, not
of what is on screen.

## The schema

Small and closed: 8 element names and 33 attributes across all five files. No
`.mip` or `.vex` reference outside the table above.

### Elements

| Element | Count | Meaning |
| --- | ---: | --- |
| `Values` | 264 | attribute carrier for its parent, the format-wide convention |
| `Text` | 127 | a string, a font and a position |
| `Image` | 85 | a rectangle cut out of the atlas - or, with no `Src`, a solid fill |
| `Model` | 26 | a `.vex` model in the 3D overlay |
| `Item` | 26 | **a translation group**; children are positioned relative to it |
| `Variable` | 20 | a named colour or number constant |
| `Screen` | 10 | container |
| `Mode3D` | 6 | container for the 3D overlay layer |

### Attributes that carry geometry

- `x`, `y`, `width`, `height` - destination, in the PSP's 480x272 space.
- `U`, `V`, `TxtrWidth`, `TxtrHeight` - **source** rectangle in atlas pixels.
  Genuinely differs from the destination: `TimeDiffIcon` samples 28x23 and draws
  it at 14x12.
- `OffsetX`, `OffsetY` on `<Item>` - added to every child's `x`/`y`. **The single
  most important attribute in the format**: dropping it piles the whole HUD into
  the top-left corner, which reads as a layout bug and is a parsing bug.
  Nothing in the shipped data nests `<Item>`s.
- `Centred="true"` (54 uses) - `x`/`y` is the middle of the rectangle, not its
  top-left. Always a screen-centre piece; the pickup box is at `x="240"`, half of
  480.

### Attributes that carry appearance

- `Color`, `BorderColor` - `FEConst->Name` into the `<Variable>` table, or a
  literal `0xAARRGGBB`. Time trial declares five: `HudColour1` `0xFFFFFFFF`,
  `HudColour2` `0xFF7DEFC0`, `HudColour3` `0xFF0DDFDD`, `HudColour3A`
  `0x60B5D7C8`, `HudBGColour` `0x40000000`.
- `font` - `HUD`, `HUDSmall` or `Default`. The role names are the language
  plugin's `<Font Src="...">` attributes, not code-side; `HUDSmall` does not
  appear in the binary at all.
- `align` - `left`, `centre`, `right`.
- `vertalign` - `bottom` (41), `middle` (37), `centre` (16). **Both `middle` and
  `centre` ship and mean the same thing**, unlike `align`, where only `centre`
  appears.
- `scale` - 0.5 to 1.5.
- `idstring` / `string` - a localisation key, or a literal (`"0 kmh"`, `"+0"`,
  `"/"`).

### Attributes not acted on

- `CalcBlur="1"` - on 200 of 264 `<Values>` and **always `1` when present**, so
  presence is the only signal and its meaning is unrecovered. Not modelled.
- `BorderColor` - parsed and carried, **not drawn**: the renderer has no outline
  pass, and faking one with four offset copies would quadruple the glyph count
  for something never compared against the original.
- `ztest`, `Delay`, `Enabletransition`, `cached`, `FirstPass`, `OriginX/Y`,
  `mode="orthographic"` - all on the `<Mode3D>` layer, which is deferred.

## Two properties of the data that a first reading gets wrong

Both cost a test failure here, so they are recorded rather than left to be
rediscovered.

**`HeadToHeadBar` is an `<Image>` with no `Src`.** It carries a `Color` and
`height="0"` - a solid bar whose length is supplied at runtime, the same
colour-only convention the front end already uses for backdrops
([`screen.rs`](../../crates/game/src/screen.rs)'s `Screen::fills`). Read as a
sprite it is a widget with no texture, and it vanishes. One widget across all
five layouts, in arcade and eliminator.

**`PosTag0`-`PosTag7` and `PlrTag0`-`PlrTag7` are not screen-positioned.** They
are anchored to a rival's projected screen position at runtime, and their authored
`x`/`y` plus the enclosing `<Item>` offset are a nudge from that anchor. The
arcade layout nests them in `<Item OffsetX="-40">`, so they resolve to **negative
coordinates** - correct for an offset, nonsense for a position. Any "is it on
screen" check has to skip them, which is what `hud::is_screen_positioned` is for.

## The HUD fonts are pre-outlined, and that cost a renderer change

The single biggest surprise in implementing this. `PulseHud.fnt` and `small.fnt`
are **not** plain coverage atlases like the three menu fonts: their palette carries
six distinct greys where the menu fonts carry one pure white, and alpha is the
silhouette of glyph *plus* a baked outline. The grey level is what separates body
from outline.

Drawing them the way the menu fonts are drawn - alpha as coverage, colour from the
vertex - fills the outline with more glyph. A `0` becomes a filled box and a
25-pixel lap time is unreadable. That is exactly what the first working version
did, and the screenshot is what caught it; no test would have.

The fix is in [`oag_game::font`](../../crates/game/src/font.rs) and
[`render.rs`](../../crates/game/src/render.rs): the glyph atlas is a two-channel
`Rg8Unorm` texture, `r` the body/outline mask and `g` the coverage, and text is
composited as

```text
rgb = mix(BorderColor, Color, mask)
a   = Color.a * coverage
```

which is exactly the two colours the layout supplies. Full measurements in
[fnt.md](../formats/fnt.md#the-rgb-is-a-second-channel-not-a-constant).

**The menu fonts are unaffected by construction**, their mask being a constant 255,
and that was checked rather than assumed: the main menu and the language picker
both still render correctly, accented characters included.

## What a reference frame settled

Captured 2026-07-30 from PPSSPP, a Venom time trial on Talon's Junction sitting on
the start line. Recipe, which works:

```sh
# PPSSPP with the debugger, then drive it to a race
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\n' > /tmp/dbg.ini
SDL_VIDEODRIVER=wayland PPSSPPSDL --appendconfig=/tmp/dbg.ini --windowed data/cache/pulse-psp-usa.iso &
uv run --with websocket-client python scripts/psp-drive.py --port <PORT> menu
# then, with the window focused:
grim -g "<x>,<y> 1142x648" /tmp/ref.png
```

Three traps, all of which cost time here:

- **`RemoteISOPort` does not pin the debugger port.** PPSSPP bound an ephemeral one
  (46659) and `psp-drive.py` defaults to 47810, so `preflight` reported no debugger
  at all. Read the real port off `ss -ltnp | grep PPSSPP` and pass `--port`.
- **The window renders black when unfocused** - the documented SDL throttle. A
  `grim` grab of an unfocused window is a black rectangle with no error. Focus it
  (`niri msg action focus-window --id N`) and check the grab's mean luma is not
  zero before reading anything off it.
- **`niri msg windows` does not print absolute window coordinates**, but
  `niri msg --json windows` gives `tile_pos_in_workspace_view`, which is what
  `grim -g` wants. Getting it wrong crops a corner of the frame, which looks like a
  HUD missing half its widgets.

### Confirmed, and now implemented

| Question | Answer |
| --- | --- |
| Does the bar fill grow from the left? | **Yes**, and the art is a wedge, so at low speed it reads as a small triangle. The horizontal-crop model is right. |
| Is there an outline on widgets with no `BorderColor`? | **Yes.** Every HUD-font widget on the frame is outlined, `CurrentTime` and `BestTime` included. The earlier "draw it transparent" reading was wrong. |
| What does `best` show with no best lap? | **`0.00.00`** - zeros, not a dash placeholder. |
| What precision do times use? | **Both.** `best` is `m.ss.hh` (`0.00.00`) and `current` is `m.ss.h` (`1.27.9`), a few pixels apart. Running clocks carry tenths, a set time carries hundredths. `record` top-right is also tenths (`0.29.0`). |
| What does `ShieldBarText` show? | **`100%`** - a percentage, not the raw `<Misc shield/>` pool. Confirmed from the writer as of 2026-08-10: `Ship_SetShield` (`0x0883e6f4`) calls `Hud_SetEnergyBar((shield / max) * 100, ...)` itself, so the percentage is the original's own arithmetic rather than a reading of a frame. |
| Does the shield bar move? | **Yes, now.** It read a constant full for as long as nothing depleted the pool; wall contact does as of the shield work, so `ShieldBar` and `ShieldBarText` are live. **In a time trial and a speed lap it will still sit near full**, and that is faithful rather than broken - both modes race with the original's `Damage` option off, which floors the pool at 20 and regenerates it at 4 a second. Zone is the mode where it visibly drains. See [shield](../ghidra/functions/psp-pulse-usa/shield.md). |
| Where is `SpeedBarMark`? | At its authored position at zero speed, just left of the bar. Whether it *slides* with speed is still open - one frame at speed settles it. |

### And one structural finding

**The caption text is not only the layout's.** `TimeTrial_HUD.xml`'s top-right
widget is `TotalTimeTxt` with `idstring="IG_HUD_TOTAL"`, whose English string is
`"Total"` - but the frame reads **`record`**, which is `IG_HUD_RECORD`
(`"Record"`). No shipped layout carries that key, and the unnamed XML entries
around the five were checked and hold no HUD layout either.

So a mode's code can **substitute a different string key** into a widget the layout
has already positioned. That explains the arithmetic: 38 `IG_HUD_*` keys exist in
the binary against 12 `idstring` values across all five layouts, so 26 keys are
reachable only from code. This build resolves the layout's own key and therefore
shows `Total` where a time trial shows `Record`; the substitution rule is unread and
is **not** worked around by hardcoding.

## Still open after the frame

- **The shield bar is not one colour, and this build draws it as one.** Found
  2026-08-11 in the two single-race frames that settled the position anchor, which
  were not looking for it: on the grid the bar is **cyan** at `100%`, and fifty
  seconds later, after wall contact, it is **solid red** at `79%` - same widget,
  same rectangle, same authored `Color`. So the original tints `ShieldBar` at
  runtime from the value, exactly as it substitutes a string key into a positioned
  widget (see the structural finding above). **What the rule is, is not
  established**: two samples cannot separate a threshold from a gradient, and 79 %
  is high enough that "red means critical" is already ruled out. `ShieldBarText`
  stays white in both. Frames at three or four known levels would settle it; a
  writer of the colour in `Hud_SetEnergyBar`'s neighbourhood
  ([shield.md](../ghidra/functions/psp-pulse-usa/shield.md)) would settle it
  better. The speed bar in the same frames is mint green and fills from the left,
  which is what this build already draws.
- **The outline colour is approximate.** The default is the layout's own
  `HudBGColour`, `0x40000000` - 25 % black - because that is what every widget that
  *does* name a border points at, which makes it data rather than invention. The
  original's outline reads crisper and darker than 25 % alpha produces. Measuring
  it off the frame is the fix; opaque black is the obvious candidate.
- **`TotalTime` overflows the right edge.** `align="left"` at `OffsetX="475"
  x="-75"` starts at 400, and a `HUD`-font time at scale 1.0 measures 92 px - 12 px
  past the 480-wide screen. Tenths rather than hundredths narrows it but not
  enough. Either that anchor means something other than a left edge, or the
  overflow is real and the original clips too.
- **Whether `SpeedBarMark` slides.** Needs a frame at speed.
- **`PulseHud.fnt`'s stylised lowercase.** The disc's captions render in
  lowercase-looking forms in both the original and here, so this is the typeface
  rather than a case fold going wrong - but it was not measured.

## Widget inventory

Grouped by what drives them. Confidence here is about the *binding*, not the
geometry - the geometry is 95 throughout.

| Group | Widgets | Conf | Note |
| --- | --- | ---: | --- |
| Speed | `SpeedBar`, `SpeedBarBg`, `SpeedBarMark`, `SpeedBarText` | 90 | `speed * 3.6`, the recovered km/h factor |
| Shield | `ShieldBar`, `ShieldBarBg`, `ShieldBarMark`, `ShieldBarText` | 75 | no runtime pool exists yet; see [roadmap](../overview/roadmap.md) M5 |
| Lap | `Lap`, `LapOf`, `Lap Outof`, `LapTxt` | 60 | **lap counting is unrecovered** - see below |
| Position | `Position`, `PositionOf`, `Position Outof`, `PositionTxt` | 95 | **drawn** as of 2026-08-11, from `Race::places()`, and checked against the original's own `pos 8 / 8`; only `Arcade_HUD.xml` carries them, and they take the anchor off `TotalTime` - see below |
| Times | `CurrentTime`, `BestTime`, `TotalTime`, `CountdownTime`, `TimeDiffText`, `TimeDiffIcon`, `TimeIcon` | 70 | format `m.ss.hh`, observed as `1.11.08` on the running game. **`TotalTime` is not drawn in a single race**: it shares an anchor with `Position` and yields it - see below |
| Weapon | `PickupBackground`, `SubWeapon`, **13** `<Type>Icon` widgets | 95 | **drawn**; the icon is found by *name* - see below. `SubWeapon` is not driven |
| Warnings | `ForwardWarningIcons`, `RearWarningIcons` and children | 55 | incoming-weapon indicators; nothing drives them |
| Countdown | `ReadyText`, `GoText`, plus the `Mode3D` models | 65 | |
| Wrong way | `WrongWay` | 70 | `dot(forward, tangent)` is a sufficient source |
| Zone | `Zone`, `Score`, `Zone_Bar_*` | 50 | Zone mode is a separate scope item |
| Eliminator | kill counters | 50 | |
| Tags | `PosTag0-7`, `PlrTag0-7`, `HeadToHeadBar` | 50 | runtime-anchored, multiplayer |
| Debug | `VersionTextOnHUD`, `Info1`-`Info4`, `Info`, `Info2nd` | 40 | present in shipped layouts; purpose inferred from the names |

### The pickup icon is found by name, and there are thirteen of them

**This row read "14 `*Icon` widgets" with "numeric" ids and no known mapping
until 2026-08-11, and both halves were wrong.** `Arcade_HUD.xml` authors
**thirteen** weapon icons - the fourteenth was `TimeDiffIcon` being counted -
and each is named after its weapon's own `type` string:

```text
TurboIcon ShieldIcon AutopilotIcon RocketIcon MissileIcon QuakeIcon CannonIcon
PlasmaIcon BombIcon MineIcon LeachBeamIcon RepulserIcon ShurikenIcon
```

That is exactly `oag_formats::weapons::Weapon::ALL`, misspellings (`LeachBeam`,
`Repulser`) included, so the lookup is `format!("{}Icon", weapon.as_type())` -
`oag_game::hud::pickup_icon_name` - and needs nothing recovered. Pinned against
the shipped file for all thirteen by
`crates/game/tests/hud_layout_ground_truth.rs`. The numeric ids are real
(`0x0883b3b8` forces `6`) and simply are not needed.

**The four layouts author three different pickup sets**, and the difference is
evidence rather than noise: `Arcade` and `Elimination` carry the backdrop and
all thirteen, `Zone` carries none, and **`TimeTrial_HUD.xml` carries the
backdrop and `TurboIcon` alone** - which is the disc's own event text
(*"a free turbo pickup once per lap"*) recorded a second time. See
[pickups](../gameplay/pickups.md).

**The backdrop's colour is substituted, and it has to be.** `PickupBackground`
and every `<Type>Icon` are authored `HudColour1`, which is `0xFFFFFFFF`, and
their art is a solid white hexagon and a white glyph - so drawn as authored the
icon is invisible inside an opaque white hexagon. The original must set a colour
at runtime and that is unrecovered; this build draws the backdrop in the
layout's own `HudBGColour` instead. The measurement and what is and is not
claimed are on [pickups](../gameplay/pickups.md).

### The place and the total time are authored at one anchor, so one of them has to go

`Arcade_HUD.xml` - the single-race and tournament layout, and **the only one of the
five that carries the place widgets at all** - authors `TotalTime` and `Position`
inside the same `<Item OffsetX="445" OffsetY="5">` with the same everything:

```text
<Text name="TotalTime"><Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" .../>
<Text name="Position"> <Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" .../>
```

Both resolve to `(445, 35)`, right-aligned, so the place's single digit lands on
the last digit of the time. The captions collide too, less exactly: `TotalTimeTxt`
is right-aligned to 445 and `PositionTxt` to 460, which is inside a `HUDSmall`
`"TOTAL"`.

**Coincidence is this dialect's way of saying "at most one of these is live"**, and
the precedent is in the same file: thirteen weapon icons, every one at `x=240 y=35`,
of which the code draws the one being carried. **Confidence 95** for the
coincidence - it is measured, and reproducible with

```sh
# The backslashes are doubled on purpose: `just` runs the recipe through a shell,
# which eats one layer. Single backslashes here hash to a name no archive has, and
# the error reads `no entry named DataXMLArcade_HUD.xml`.
just wad cat --expand <image>:PSP_GAME/USRDIR/Data.wad 'Data\\XML\\Arcade_HUD.xml'
```

**The place is the one that wins, and the original was asked.** Confidence **95**,
up from the 55 this section shipped with for a few hours: a single race driven on
the real game (PPSSPP under Xvfb, `psp-drive.py menu --single-race`,
`pulse-psp-usa`, 2026-08-11 - recipe on
[ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md#running-without-a-real-display-xvfb-works-no-compositor-needed))
reads

```text
pos
8 / 8
```

in the top-right corner, on the grid and again fifty seconds into the lap, with
**no total time anywhere on the screen**. Two frames, both `POS`, no clock. That
also squares with the layouts: `TimeTrial`, `Zone` and `Elimination` carry
`TotalTime` and no place, so the clock is not homeless without this anchor and the
place would be.

The rule is `oag_game::hud::place_owns_the_anchor` and it asks the *layout*, not
just the readout, so `Elimination_HUD.xml` keeps its clock. **The captions were not
separately confirmed** - the `TOTAL` caption is suppressed with its clock here on
the ~5 px overlap argument above, and the reference frames show `pos` where it
would have been, which is consistent with but does not isolate that.

**The same two frames settle a second question nobody asked them.** The original
places its own **parked player 8th of 8 on the grid**, before anyone has crossed
the line - not 1st. That is independent corroboration of the lap-1 rule in
`oag_race::Standing::distance`: a craft that has not reached the line is behind the
field, and the arithmetic that read it as almost a lap ahead disagreed with the
original as well as with common sense. See [ai.md](../gameplay/ai.md#the-field-is-placed).

Found by `no_two_live_widgets_share_an_anchor_on_any_shipped_layout`, which is a
new ground-truth check and reported this collision the first time it ran. Its unit
counterpart in `oag_game::hud` had claimed for months that the shipped layouts were
checked; they were not.

**A bar and its background share a rectangle exactly**, differing only in colour -
pinned by `each_bar_exactly_overlays_its_own_background`. So the fill can only be
a horizontal crop of the same art rather than separate geometry, and `*Mark` is a
slider riding on top. That reading drives the draw code and is **confidence 80**:
it follows from the geometry but has not been checked against a reference frame at
a known speed.

## Localisation keys

The `idstring` values resolve through the language plugin's string table. 38
`IG_HUD_*` keys exist in the binary (`IG_` = in-game), which is a more complete
element inventory than the layouts themselves - several have no widget in any
shipped layout:

`IG_HUD_LAP`, `IG_HUD_CURRENT`, `IG_HUD_TOTAL`, `IG_HUD_BEST`, `IG_HUD_LAP_REC`,
`IG_HUD_NEWLAP_REC`, `IG_HUD_PLAP`, `IG_HUD_FIN_LAP`, `IG_HUD_WRONG_WAY`,
`IG_HUD_1ST`, `IG_HUD_2ND`, `IG_HUD_GOLD`, `IG_HUD_SILVER`, `IG_HUD_BRONZE`,
`IG_HUD_RECORD`, `IG_HUD_KILL`, `IG_HUD_KILLS`, `IG_HUD_CONT_ELIM`,
`IG_HUD_ZONE`, `IG_HUD_ZONES`, `IG_HUD_PERF_ZONE`, `IG_HUD_PERF_BOOST`,
`IG_HUD_NEW_ZONE_RECORD`, `IG_HUD_NEW_SCORE_RECORD`, `IG_PAUSE_QUIT`.

## Parse the disc, or commit a derived definition?

A reasonable question, since the menus use **our own** format
(`assets/ui/menu.toml`, see [menus.md](../architecture/menus.md)) rather than the
disc's. Why not export the HUD geometry once and commit the result?

**Because [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) forbids
it.** The rule is "no game content in the repository, ever. No assets, no
executables, no extracted data, and **no derived data that reconstitutes the
original**." A file carrying all 211 widgets' pixel coordinates, colours and
atlas UVs is exactly derived data that reconstitutes the original's HUD - a
transcription in a different syntax, not a description of a format. It is the same
line [`handling-stats.md`](../formats/handling-stats.md) draws: *a field name is a
description of the format, a tuning table is the content itself.*

Note that `just audit-leakage` would **not** catch it: the check is
extension-based (`.chd`, `.wad`, `.elf`, …) and a `.toml` passes. The rule is the
constraint here, not the tooling - and committing one would be a new and much
larger instance of the breach [`HANDOVER.md`](../../HANDOVER.md) already flags as
an unresolved maintainer decision, where two physics pages quote shipped tuning
values.

### What the idea is right about, and how to get it

Everything except committing the file:

- **An exporter to a gitignored artifact.** A `just` target that reads the disc
  and writes our own format under `data/`. Inspectable, diffable, editable, and
  it makes the geometry visible without a hex editor. No leakage, because `data/`
  is gitignored and the artifact is the player's own.
- **Our own format as an *override*, committed.** A layout **we author** -
  widescreen anchoring, a modern HUD, a de-cluttered one - is our work and is
  fine to commit. That is the genuinely valuable version of the idea, because it
  does something the disc's data cannot: the original's layout is hard-authored
  for 480x272, and a 16:9 or ultrawide HUD wants anchors rather than absolute
  coordinates.
- **No fidelity cost.** Parsing the disc means the HUD automatically matches
  whatever region and revision the player owns. A transcription pins one disc.

### And the decision is cheap to defer

[`hud::Layout`](../../crates/game/src/hud.rs) is already the seam. It is a plain
geometry type; `Layout::from_xml` is *one constructor*. A `from_toml` beside it is
additive, and nothing downstream - `draw_list`, the renderer, the tests - changes.
So there is no lock-in either way, and no reason to decide now.

There is also no scenario where a committed definition unlocks playing without a
disc: a race needs track geometry, ship models and handling stats from the same
archive.

## The shipped art is raster, and the disc suggests it was not always

Worth writing down because the HUD *looks* vector-native - flat fills, hard edges,
geometric chevrons and bars - and it is fair to ask whether any of it is.

**Everything this build draws is rasterised, and all of it is authored for
480x272:**

| Asset | What it actually is |
| --- | --- |
| `PulseHUD.mip` | 256x256, **8 bpp paletted raster**, 84 of 100 `Src=` references |
| `PulseHud.fnt` | 512x256 **4 bpp paletted bitmap** glyph atlas |
| `small.fnt` | 256x128, likewise |

So every bar, chevron, icon and digit is pixels, and at a modern window size it is
being magnified: the 1440x816 captures in this project are already a ~3x upscale
with linear filtering, which is exactly why the zoomed glyph comparisons look soft.
At 4K it is ~8x. The one exception is `HeadToHeadBar`, which carries no `Src` and is
a solid `Draw::Fill` - resolution-independent by accident rather than by design.

**But 26 `Data\HUD\*.vex` files are polygonal geometry.** They are named exactly
after HUD elements - `Bar_1`, `Bar_2`, `Bar_3`, `Bar_outline_1`, `Speed`, `Shield`,
`Lap`, `Position`, `Thrust`, eleven `Weapon_*`, `Zone_Bar_*`, `Zone_outline_1` -
and their embedded Maya source paths and node names are unmistakably meshes:
`polySurfaceShape1`, `polySurface01_nolight`, `Cube1_nolight`, `Lap1_nolight`,
several carrying `AnimEnd` markers. The `_nolight` suffix is what you would name
flat-shaded overlay geometry.

Nothing references them: no layout's `Src=`, and `BOOT.BIN` holds no
`Data\HUD\...vex` string at all. They are also `.vex` **version 4** with root class
`0x0ee`, which [vex.md](../formats/vex.md) says does not occur in Pulse.

What that means is **not** established. The defensible reading is only that a
geometry representation of these elements existed in the pipeline; whether it was
the source the `.mip` atlas was baked from, an abandoned approach, or another
platform's path is unknown, and the class-`0x0ee` decoder does not exist yet to
look. Do not write it up as "the HUD was vector".

### Redrawing the art as vector is a good future option, and the seams are already right

Recorded as a direction, not a decision:

- **Resolution independence is the payoff.** A vector or procedural HUD is sharp at
  any window size, and the original's own layout is hard-authored for 480x272, so a
  16:9 or ultrawide HUD wants anchors rather than absolute pixels anyway - the same
  argument as the override format below.
- **Our own art is committable.** This is the useful asymmetry with
  [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md): a *transcription*
  of the disc's layout is derived data and cannot be committed, but art **we** draw
  is our work and can be. So the vector path is the part of a "commit an artifact"
  idea that is actually available.
- **The seam exists.** `hud::Sprite` carries a destination `rect` and a source `uv`
  independently, and `draw_list` decides only which widgets are live. Substituting a
  vector or procedural draw for the atlas sample is a renderer concern; `Layout`
  stays the geometry source and the tests stay valid.
- **Keep the layout disc-derived.** Only the *art* becomes ours. Parsing the disc's
  geometry is what makes the HUD match whatever region and revision the player owns,
  and it is what keeps this out of ADR-0006's way.
- **Some of it is already vector.** The bars sample a wedge out of the atlas, but
  they are a rectangle and a crop; `Draw::Fill` draws one with no art at all. The
  bars are the cheapest thing to convert and the most visible.

## Deferred, and known

Recorded so none of this reads as undiscovered work.

- **The `<Mode3D>` layer.** The countdown (`Pulse_Ready_Go`, `Cockpit_321GO`) and
  the weapon sights (`missile_sight_inner`/`_outer`, `leachbeam_sight`). Needs a
  second pass with its own projection.
- **Zone and Eliminator HUDs.** Their layouts parse; nothing drives them.
- **Medal targets.** `IG_HUD_GOLD`/`SILVER`/`BRONZE`/`RECORD` need progression
  data.
- **`IG_PAUSE_QUIT`.** There is no pause: leaving a race drops the `World` rather
  than suspending it.
- **26 unreferenced `Data\HUD\*.vex` models.** `Bar_1`, `Speed`, `Shield`, `Lap`,
  `Position`, `Thrust`, 11 `Weapon_*`, `Zone_Bar_*`, `ghostship`, `grid`,
  `memory_stick_anim`. Referenced by **no** layout's `Src=` and by no string in
  `BOOT.BIN`. They are also `.vex` **version 4** with root class `0x0ee`, which
  [`vex.md`](../formats/vex.md) says does not occur in Pulse and
  `vex::CLASS_NAMES` does not know. A separate open thread; the 2D HUD is complete
  without them.
- **Text outlines** (`BorderColor`) and **`CalcBlur`**.

## Lap counting is the one real blocker

`Lap`, `LapOf` and `Lap Outof` have geometry but no source.
[`track.md`](../formats/track.md#where-is-lap-counting) records `gate` (class
`0x3ca`) as having **no** runtime class registration - all 46 registrar callers
enumerated - and the `SplinePt.flags` (`+0x61`) reading as unverified and zero on
every control point of `01_Track`.

The layouts sharpen the search, because the widget names are in the binary:

- **`FUN_0881fbec`** (`0881fbec`-`08820d77`) is the widget bind: the only referrer
  of `"SpeedBarBg"` (`0x08a79f30`), with 20+ references to the `"HUD->"` path
  prefix. **Whatever writes the `Lap` widget's string is the lap counter's
  consumer.**
- **`FUN_0882d1a0`** is the only referrer of `Data\XML\TimeTrial_HUD.xml`, so it
  selects the per-mode layout.
- The **total** is configuration: the race-setup format string at `0x08a783d0`
  contains `laps="%d"`, and `RC_LAPS`/`RC_LAP` (`0x08a824bc`, `0x08a82f54`) look
  like config keys.

Nothing is renamed on the strength of the above - see
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md) and the
[confidence rubric](../reverse-engineering/confidence-rubric.md).
