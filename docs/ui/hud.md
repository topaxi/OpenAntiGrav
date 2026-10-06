# The in-race HUD

**Status: the layout is understood, confidence 95.** Pulse's HUD geometry is
**authored data, not code**, and it decodes with the front-end XML machinery this
project already had. Every rectangle, atlas sub-rectangle, colour, font role and
alignment is read off the player's own disc. Implemented in
[`oag_hud`](../../crates/hud/src/lib.rs); pinned against all five shipped
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

**Not every coordinate follows it**, which is why the PS2 sweep below needed
more than swapping in the PS2's grid size. Measured across `Arcade_HUD.xml`'s
141 raw `x`/`y` attribute values on both discs, keyed by tree path rather than
by document order, **119 are the PSP's own scaled by 640/480 or 448/272 and
rounded to an integer; 22 are not**. This paragraph said 121 and 20 until
2026-09-05 - the earlier count used a weaker key and, in one case, paired
`TimeDiffIcon`'s `y` with a value that actually belongs to
`RearWarningMissileIcon`. Of the 22: **18 are the nine `<Mode3D><Model>`
placements**, sitting at the PSP's own `x="-240" y="136"` unchanged - a
placeholder overwritten every frame by `HudSight_Update` (see
[`lock-sight.md`](../ghidra/functions/psp-pulse-usa/lock-sight.md)), so an
unscaled copy costs nothing; **4 are small integer
nudges** (`LapTxt`'s `y`, `RearWarningMissileIcon`'s and
`RearWarningRocketIcon`'s `y`, and most of `TimeDiffIcon`'s `x`) that stay
byte-identical across consoles while the `<Item>` enclosing every one of them
scales its own `OffsetX`/`OffsetY` correctly, dwarfing the nudge. See
`oag_hud::inside_screen`'s doc comment for the full tally and
`crates/game/tests/hud_layout_ground_truth.rs` for the check that runs against
both grids.

**The PS2 layout is now checked for a dropped `<Item>` offset, the same way the
PSP's has been from the start.** `every_widget_lands_on_screen_on_the_ps2`
reads `Data\XML\*_HUD.xml` unshortened - the PS2 pressing does not run these
through the `fexml` dictionary - and checks every composed rect against the
PS2's 640x448 rather than the PSP's 480x272. It passes: 187 screen-positioned
widgets checked, the same count as the PSP sweep, none of them off the PS2's
larger screen. The `<Mode3D><Model>` placements above are not part of that
check on either console - `Layout::models` is a separate field the sprite/label
walk never iterates - so their own coordinate space stays unrecovered without
blocking the widget check that matters.

Two things kept it off the screen entirely, and each hid the other:

| | PSP | PS2 |
| --- | --- | --- |
| Layout XML | shortened, `<code>` dictionary | **plain `<?xml`** |
| Atlas entry | `Data\HUD\Textures\PulseHUD.mip` | hashes to nothing; entry **3518** holds it |

**Shortening is per file, not per platform.** The PS2's own `Skin.xml` *is*
shortened while its `Data\XML\*_HUD.xml` are not, so a reader that calls
`fexml::expand` unconditionally rejects a perfectly good layout with "no
`<code>` dictionary element" and the whole HUD goes with it.
[`fexml::text`](../../crates/tables/src/fexml.rs) decides from the blob's own
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
takes the linear path in [`texture.rs`](../../crates/texture/src/texture.rs) - it
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

### Eliminator

`Data\XML\Elimination_HUD.xml` was drawn by no mode before
[`oag_race::Mode::Eliminator`](../gameplay/race-modes.md#eliminator)
(2026-09-08) - "Layouts no mode here reaches" used to name it explicitly
alongside HD's Detonator/Duel/MPTag. Wired the same way every other mode's
layout is: `oag_title::HudLayouts::elimination`, read by
`oag_raceplay::hud_layout`. Pulse and HD both author a dedicated file and
use it; Pure, whose own Eliminator layout has never been read off its disc,
falls back to `arcade` - the same "no dedicated file, reuse one that exists"
shape this page's own `speed_lap`/`time_trial` row already has.

**The layout, the `PosTag` kill column and the `KILLS (n)` header are wired**
(see the `PosTag` section above); `Standing::kills` reaches the screen through
`Race::readout`. Deaths and this layout's other counters are not drawn, which is
the same gap `docs/gameplay/race-modes.md` records for `Zone_HUD.xml`: the
substitution rule between a mode's own counters and the widgets a layout
positions is unread on every layout but the ones read one at a time.

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

- `x`, `y`, `width`, `height` - destination, in the PSP's 480x272 space. True
  of every `<Image>`/`<Text>` and of a `<Mode3D><Model>` whose enclosing block
  authors `mode="orthographic"`. **A `<Model>` in the other `<Mode3D>` dialect
  reads `x`/`y` differently** - see "Two `<Mode3D>` dialects" below.
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
- `ztest`, `Delay`, `Enabletransition`, `cached`, `FirstPass` - all on the
  `<Mode3D>` layer, still deferred. `mode` and `OriginX/Y` are **no longer** on
  this list - see below.

### Two `<Mode3D>` dialects, distinguished by `mode="orthographic"`

**Previously lumped together as one deferred `<Mode3D>` feature; they are
opposite ends of the same fork.** Every `<Mode3D>` block on both Pulse and
Pure, checked across all four layouts that carry one (`Arcade`, `TimeTrial`,
`Elimination`, `Zone`), is exactly one of two shapes and never a mix:

| | sights, Pure's weapon icons/bars | `ReadyGo`/`Cockpit321Go` (the countdown) |
| --- | --- | --- |
| `<Values>` carries | `mode="orthographic"` | `FirstPass="yes" OriginX="0.0" OriginY="35.0"` |
| `x`/`y`, every layout | real screen pixels (`240,250`; `-240,136`; `394,20`) | always `0.0, 0.0` |
| `z` | `0`, `-1` or `-10` - never used for depth | `-70.0` |

The orthographic dialect is the one `model_draw`'s convention was measured
against (`pickup_icon_ground_truth.rs`, `lock_sight_ground_truth.rs`): `x`/`y`
are literal HUD pixels. The countdown's own block never authors
`mode="orthographic"`, and reading its `x=0, y=0` the same way put the widget
off the left edge of the screen - the actual defect behind the countdown being
undrawable in practice, not the runtime-placement question
[countdown-widgets.md](../ghidra/functions/psp-pulse-usa/countdown-widgets.md)
settled separately (there is no runtime writer, and there does not need to be
one). `x=0, y=0` is where a symmetric perspective camera along `-z` always
projects the optical axis, for any FOV and any `z != 0` - so it reads as
screen-centre in this dialect, not a placeholder. `oag_hud::Layout::collect`
now parses `mode` and `OriginX`/`OriginY` per `<Mode3D>` block (`super::Model::orthographic`/`::origin`),
and [`countdown.rs`](../../crates/game/src/hud_countdown.rs)'s own module doc
has the full derivation, including why no projection matrix needs to be built
for the `(0, 0)` case every shipped layout actually authors, and why a future
widget authored at a nonzero `x`/`y` in this dialect is a documented gap
rather than a guess.

#### A `<Mode3D><Model>` quad carries its own blend, and two titles disagree about it

**Measured 2026-09-07 on `pulse-psp-usa.chd` and `pure-psp-usa.chd`,
confidence 95.** A `<Mode3D>` widget's art is not in a texture file - it is
embedded in a `.vex` model - and that model's batch declares a blend equation
in its `pass_mask` exactly the way any other batch does (see
[mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md), "Three
`pass_mask` bits decoded"). The 2D sheet path drew every one of them with an
ordinary alpha blend and never consulted it, which cost the lock-on reticle
its whole appearance:

| Model | `pass_mask` | class | embedded texture |
| --- | --- | --- | --- |
| `missile_sight_outer.vex` | `0x120e` | `Additive` (`0x200`) | `gunsight_ADD.tga`, 32x32 |
| `missile_sight_inner.vex` | `0x120e` | `Additive` | `gunsightdot_ADD.tga`, 32x32 |
| `leachbeam_sight.vex` | `0x120e` | `Additive` | `LeachBeamSight_ADD_nomip.tga`, 32x32 |
| Pure's 10 `Weapon_*.vex` + `grid.vex` | - | `AlphaOver` (`0x100`) | one apiece, 128x16 / 64x64 |
| `Pulse_Ready_Go.vex` | `0x1132` | `AlphaOver` | - |
| `Cockpit_321GO.vex` | `0x1112` | `AlphaOver` | - |

The three sight textures carry **no shape in their alpha channel at all** -
`gunsight_ADD.tga` is 784 texels at alpha 250 and 240 at 255, and its bracket
lives in the colour channels over a black field. Drawn with `SrcAlpha` over
`OneMinusSrcAlpha` that can only produce an opaque black tile with a white
bracket inside it, which is exactly what every reticle capture before this
date shows. Additive makes the black field contribute nothing and the bracket
glow, on both PSP titles.

Two consequences worth carrying:

- **The class is read per model, never tabulated per widget.** Pure's eleven
  icon models declare `AlphaOver` where its *sight* models declare `Additive`,
  in the same layout - so any table keyed on the widget's name would have been
  wrong for one of them. `oag_raceplay::hud::quad_blend` reads it off the
  batch at load and `oag_hud::sprite::Placed::blend` carries it to the draw;
  `pickup_icon_ground_truth::a_held_turbo_draws_its_own_authored_green_on_a_real_race`
  pins Pure's reading and `lock_sight_ground_truth` Pulse's.
- **The countdown was never affected.** Both its models are `AlphaOver`, and
  they reach the frame through the 3D mesh path, which has honoured
  `Batch::blend_class` since it was recovered.

`oag_game::frontend::Draw::BlendedSprite` is the variant that carries it. The
UI pipeline blends *premultiplied* alpha so an additive quad is one emitting
an alpha of zero - one pipeline, one pass, and the draw list's own
back-to-front order untouched; `ui.wgsl`'s `fs_main` has the algebra.

**`OriginX`/`OriginY` reads as a screen-space translation added to the
projected result, not as the projected result itself, and `OriginX="0.0"`
is the reason to believe that rather than the other way round.** The
alternative reading - "`OriginX`/`OriginY` *is* the final screen pixel,
full stop" - would put the countdown right back on the left edge it was
just taken off (`OriginX="0.0"` read as a literal pixel is column zero),
the exact failure this page exists to record. "No horizontal nudge" is
what an author who wants a centred widget actually writes; "column zero"
is not a coordinate anyone authors on purpose next to a `z="-70.0"` that
places the same widget in front of a camera. HD/Fury's own copy of this
element (`hud_ready_go.xml`'s `<aMode3D><Values OriginX="0.0" OriginY="140.0">`,
disabled by the tag-rename convention `hd-hud.md` documents, not
repositioned) is consistent either way and settles nothing on its own - see
[countdown-widgets.md](../ghidra/functions/psp-pulse-usa/countdown-widgets.md)'s
own section on it. The `OriginX="1220" OriginY="412"` data point on HD's
`Team Selection`'s `ShipModel` and `OriginX="960" OriginY="540"` on its
`<BackgroundAnim>` (`docs/formats/race-setup.md`, `docs/formats/hd-frontend.md`)
look like they support "`OriginX`/`OriginY` is the literal screen pixel"
instead, since `960,540` is exactly the centre of a 1920x1080 screen with no
further offset needed - but both carry `nearZ` and sit directly on a
`<Model>`/`<BackgroundAnim>` **widget**, a different XML dialect from
Pulse's and HD's own `<Mode3D><Values OriginX/OriginY>` **container**
attribute. The two attribute sets are disjoint - no Pulse or Pure `<Mode3D>`
block ever carries `nearZ`, and no HD `<aMode3D>`/`<Mode3D>` container ever
carries it either - which is what licenses reading them as two different
attributes that happen to share a name, not one attribute with two
data points pulling in opposite directions.

**Position at `x=0, y=0` is FOV-independent; apparent size is not, and only
the first was derivable here.** `countdown.rs` keeps the same "one model
unit is one HUD pixel" orthographic draw `model_draw` uses for its own
scale, rather than computing a foreshortened size from `z="-70.0"` and a
real perspective camera - correctly, since recovering a size that way needs
the original's actual FOV, which nothing recovered states, and guessing one
is exactly what this page's own derivation avoided doing for position. The
glyph's on-screen size today comes entirely from `Cockpit_321GO.vex`'s own
mesh extents and its authored `Anim Transform` scale-burst (rest ~3.5x,
peaking ~8x), not from a projection - recorded so a reader does not read the
position fix as having also derived the scale.

## Two properties of the data that a first reading gets wrong

Both cost a test failure here, so they are recorded rather than left to be
rediscovered.

**`HeadToHeadBar` is an `<Image>` with no `Src`.** It carries a `Color` and
`height="0"` - a solid bar whose length is supplied at runtime, the same
colour-only convention the front end already uses for backdrops
([`screen.rs`](../../crates/ui/src/screen.rs)'s `Screen::fills`). Read as a
sprite it is a widget with no texture, and it vanishes. One widget across all
five layouts, in arcade and eliminator.

**`PosTag0`-`PosTag7` are a fixed on-screen column, not a runtime anchor -
corrected 2026-09-08.** An earlier reading of this page said they resolve to
negative coordinates, off the arcade layout's inner `<Item OffsetX="-40">`
read alone. `<Item>` offsets compose (`Layout::collect`'s `x = offset_x +
OffsetX`), and the outer `<Item OffsetX="445" OffsetY="5">` around it
composes to `(405, 5)` - measured, not argued:
`postag_is_a_fixed_column_not_a_runtime_anchor` in
`crates/game/tests/hud_layout_ground_truth.rs` pins `PosTag0` at `(405, 25)`
through `PosTag7` at `(405, 165)`, one column, twenty pixels a row, both on
screen and evenly spaced by index - which eight independent per-craft
anchors would have no reason to be. `Elimination_HUD.xml` corroborates at
its own `x=460`, needing no composing at all since its `PosTag` block sits
in a single, unnested `<Item OffsetX="460" OffsetY="5">`.

**What the eight rows draw, read 2026-09-30: the Eliminator's kill column,
and nothing at all in a solo race.** `Hud_UpdateKillColumn` (`0x0881af38`,
[hud-kill-column.md](../ghidra/functions/psp-pulse-usa/hud-kill-column.md))
writes `"<name> <kills>"` into the rows, one per craft, ordered by kills, when
the HUD's `0x200` flag is set - which `Elimination_Construct` sets. A live
Venom Eliminator frame reads `KILLS (5)` over `AG Systems 1`, `Feisar 1`,
`Qirex 1`, `AAA 0`, `Triakis 0`, `Goteki 45 0`, `Piranha 0`, `EG-X 0` in the
`Default` face at `x=460`. In a solo single race, Tournament or Time Trial
nothing writes them (`hud+0x40` has `0x40`, whose reader only writes the
`Position`/`PositionOf` digits); a place *list* in the same rows exists for
multiplayer alone (`0x800`), a mode this build does not run. Wired in
[`oag_hud::kill_tags`](../../crates/hud/src/kill_tags.rs), with a new
`Default`-face text bucket, for Pulse only (`oag_title::HudArt::kill_column`; HD authors
its own `KillsText` and `PosTag0`-`PosTag5`, unmeasured). The player's row is drawn at scale
`1.0` and the others at `0.8`. **Chosen, not measured**: the player's row carries
the player's team name where the original prints the profile tag (this build
has none), and the tie order follows this build's grid array. `PosTag` is no
longer skipped by `hud::is_screen_positioned`.

**`PlrTag0`-`PlrTag7` are the genuine runtime anchor** - a separate,
multiplayer-only layout (`MPTag_HUD.xml`), and its eight `<Text>` widgets
carry no `x`/`y` at all, unlike `PosTag`. That is the one case
`hud::is_screen_positioned`'s exemption still describes correctly: nothing
to compose, nothing to check.

## The reader is shared with HD/Fury, and HD is what corrected it

`oag_hud`'s `Layout` reads Wipeout HD / Fury's eighteen layouts as well as
Pulse's five - see [hd-hud](../formats/hd-hud.md), which is where the numbers
are. Two rules on this page were right about Pulse and wrong about the dialect,
and HD is what showed it:

- **`<Item>` offsets compose.** The `collect` walker read an offset as absolute,
  on the grounds that no shipped file nested one. Pulse nests none; HD nests
  them three deep and puts them on widgets as well as on `<Item>`. Composing is
  a strict no-op here - all nine assertions in
  `crates/game/tests/hud_layout_ground_truth.rs` pass unchanged.
- **A nameless widget is drawn.** Both widget readers required a `name` and
  recorded the loss in `Layout::skipped` otherwise. Pulse names all of its;
  HD leaves 106 anonymous, mostly background panels behind a named readout.

Neither could have been caught from Pulse's own data, which is the argument for
pointing a parser at a second title even when no milestone is open on it.

### And the grid is the source's, not the PSP's

A layout carries bare numbers and says nothing about the space they are in, so
the renderer has to be told which one. `oag_game::hud_overlay::Overlay` never told it,
and `crate::render::Renderer` starts at `Space::PSP` - so **every HUD, on every
title, was drawn as if authored at 480x272**. That is right for both Pulse
pressings on PSP and for Pure, silently wrong for the PS2 pressing's 640x448,
and badly wrong for HD's 1920x1080: a factor of four in both axes.

Measured 2026-08-20, on `01_vineta_k` and `talons_junction` at tick 1, before
and after. The symptom nobody read as a HUD bug is the reason this section
exists: `hud_lap_counters.xml` authors the lap number as
`<Text name="Lap" color="0xFFFFFF00" x="100" y="88">` under a parent at
`OffsetX="40" OffsetY="40"`, so `(140, 88+40)` in a 1920x1080 grid is the
top-left panel - and in a 480x272 one it is a quarter of the way into the
picture at four times the size. Drawn there, in the disc's own yellow, over a
circuit's grandstands, **it reads as a piece of scenery**: a solid vertical
slab that was reported as "a tower our renderer does not draw properly". It is
the digit `1`. The same frame put `LapOf`'s grey `3` in the middle of the sky
and pushed the speed bar, the total time and the lap times off the right and
bottom edges entirely, which is why an HD race looked like it drew almost no
HUD at all.

The fix is one value, not a scaling pass: `Assets::space` carries
`Space::of(archives.layout.platform)` - the same call `boot::load_shell`
already made for the front end - and `Overlay::new` sets it on both renderers.
`Space` was built for exactly this when the PS2's grid arrived; see
`crate::frontend::Space` for why the grid and the display aspect are two
numbers.

**What it did not fix, and what did.** HD's sprites came from up to six atlases
in one layout against the one sheet everything downstream bound, so the lap
panel behind that digit was not drawn - the digit sat in the right place with no
hexagon around it. That closed on 2026-08-25: the sheet holds every texture a
layout names, `sprite_draw` offsets each sprite by its own texture's placement,
and which widgets are up is a per-title table rather than Pulse's names. See
[hd-hud](../formats/hd-hud.md#the-hud-draws-and-each-sprite-out-of-its-own-texture).
The PS2's own lap panel is still absent for a reason not yet chased.

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

The fix is in [`oag_game::font`](../../crates/ui/src/font.rs) and
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
shows `Total` where a time trial shows `Record`.

**The rule behind this one pair is now read, confidence 90 for the mapping
itself.** `Hud_UpdateTimeCluster` (`0x0881c9d0`) picks `TotalTimeTxt`'s caption
key from a five-way table keyed on an ordinal "tier" value, re-resolving the
localised string only on a transition rather than every tick:

| Tier | Key |
| ---: | --- |
| `-1`, or `>= 4` (except `5`) | `IG_HUD_TOTAL` (the layout's own default) |
| `0` | `IG_HUD_BRONZE` |
| `1` | `IG_HUD_SILVER` |
| `2` | `IG_HUD_GOLD` |
| `3` | `IG_HUD_RECORD` |
| `5` | caption untouched this tick; only the numeric format differs |

All five string addresses round-trip exactly, and the `IG_HUD_RECORD` load is
the *only* code reference to that string anywhere in the binary - a search over
all 525,049 disassembled instructions found one hit. **What the tier value
itself tracks is now read too** - `PlayerStatus_Update` (`0x0883b3b8`,
confidence 92) - and wired up for the campaign gold/silver/bronze case; see
"Medal targets, closed 2026-09-28" below and
[hud-time-caption-substitution.md](../ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md)
for the full evidence and what still isn't reproduced. This is one widget
pair of the 26 code-only keys; the other 25 are not read.

## Still open after the frame

- **The shield bar is not one colour - wired in 2026-09-05.** Found 2026-08-11
  in the two single-race frames that settled the position anchor, which were
  not looking for it: on the grid the bar is **cyan** at `100%`, and fifty
  seconds later, after wall contact, it is **solid red** at `79%` - same
  widget, same rectangle, same authored `Color`. Two samples could not
  separate a threshold from a gradient, so the colour writer was read
  directly instead: `Hud_UpdateEnergyBar` (`0x0881c638`,
  [shield.md](../ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient))
  forces solid red whenever the pool is at or under **20%** or during a
  one-shot flash right after a hit, and otherwise draws the widget's own
  authored colour (`HudColour3`, `0xFF0DDFDD`, the cyan the 100% frame shows) -
  a threshold plus a transient flash, never a blend between two colours. The
  79% frame lands inside that post-hit flash window, which is why it reads
  solid red well above the 20% floor. Confidence 82. `ShieldBarText` stays
  white in both, which is consistent - only the bar fill is tinted. The speed
  bar in the same frames is mint green (`HudColour2`, `0xFF7DEFC0`) and fills
  from the left, which is what this build already draws.
  **Now implemented**: [`Readout::shield_forced_red`](../../crates/hud/src/lib.rs)
  computes the rule from a shield percentage plus a one-tick-memory flash flag
  the readout carries (`Readout::shield_flashing`), and
  [`oag_hud::draw::draw_list`](../../crates/hud/src/draw.rs) applies
  it to `ShieldBar` alone. ~~leaving the alpha byte untouched. **Not
  reproduced**: the low-shield icon's own separate blink cycle (`hud+0x1dc`,
  scaled by `8.0`) ... and `iVar1`, the external override flag ...~~ -
  **both closed 2026-09-25**: `iVar1` is
  `HullOverlay_AbsorbWindowActive(player)` -
  [`Readout::shield_absorbing`](../../crates/hud/src/lib.rs) - and it
  *suppresses* the forced-red branch rather than being an unread override,
  and `hud+0x1dc`'s blink is now applied to `ShieldBar`'s own alpha through
  [`Readout::shield_blinking`]/[`Readout::shield_blink_phase_on`], driven off
  the pool, the post-hit flash, and absorbing together - see
  [shield.md](../ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-the-absorb-flash-2026-09-25).
  The low-shield **icon** itself (a separate widget from `ShieldBar`,
  `Hud_SetEnergyBar`'s own two-tier blink) is a different gap and still not
  reproduced.
- **The outline colour is approximate.** The default is the layout's own
  `HudBGColour`, `0x40000000` - 25 % black - because that is what every widget that
  *does* name a border points at, which makes it data rather than invention. The
  original's outline reads crisper and darker than 25 % alpha produces. Measuring
  it off the frame is the fix; opaque black is the obvious candidate.
  **Tried 2026-10-06 and refuted**: an opaque outline fills the digits with
  black halos the original does not show; see [hud-skins.md](hud-skins.md#a-why-pulses-hud-text-reads-poorly).
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
| Tags | `PosTag0-7`, `HeadToHeadBar` | 95 anchor / 85 content | a fixed column (`405/460, 25..165`), not runtime-anchored; the rows are the Eliminator's kill column, blank in a solo race - see above |
| Tags | `PlrTag0-7` | 50 | genuinely runtime-anchored, multiplayer only (`MPTag_HUD.xml`) |
| Message lines | `Info1`-`Info4` | 80 | **the four message slots**, read 2026-10-06 (`Hud_UpdateMessages`, [hud-messages.md](../ghidra/functions/psp-pulse-usa/hud-messages.md)); drawn as of the same day - see "Medal messages" |
| Debug | `VersionTextOnHUD`, `Info`, `Info2nd` | 40 | present in shipped layouts; purpose inferred from the names |

### The pickup icon is found by name, and there are thirteen of them

**This row read "14 `*Icon` widgets" with "numeric" ids and no known mapping
until 2026-08-11, and both halves were wrong.** `Arcade_HUD.xml` authors
**thirteen** weapon icons - the fourteenth was `TimeDiffIcon` being counted -
and each is named after its weapon's own `type` string:

```text
TurboIcon ShieldIcon AutopilotIcon RocketIcon MissileIcon QuakeIcon CannonIcon
PlasmaIcon BombIcon MineIcon LeachBeamIcon RepulserIcon ShurikenIcon
```

That is exactly `oag_tables::weapons::Weapon::ALL`, misspellings (`LeachBeam`,
`Repulser`) included, so the lookup is `format!("{}Icon", weapon.as_type())` -
`oag_hud::pickup_icon_name` - and needs nothing recovered. Pinned against
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
icon is invisible inside an opaque white hexagon. The original sets a colour at
runtime, and the *exact* mechanism is still unrecovered, but **the picture is
not**: reference frames taken 2026-09-04 show eleven of the thirteen weapons
sharing one of two colours, green or magenta, corroborated by the disc's own
loading-screen art for the other nine. This build draws those eleven in the
colour their frames show and the remaining two (`Bomb`, `Mine`, no frame taken)
in the layout's own `HudBGColour` still. The measurement, the confidence and
what is and is not claimed are on [pickups](../gameplay/pickups.md).

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
# Single-quote the path so your own shell's quoting reaches `just` intact -
# `wad` now hands its arguments through unmangled (see justfile's own
# `positional-arguments` comment); a *doubled* backslash used to be needed to
# survive an unquoted `{{ARGS}}` interpolation eating one layer, and would
# now reach the archive as a literal double backslash and fail to hash.
just wad cat --expand <image>:PSP_GAME/USRDIR/Data.wad 'Data\XML\Arcade_HUD.xml'
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
**no total time anywhere on the screen**. Two frames, both `POS`, no clock.

**The place was never the reason, and the anchor is not why the clock is
missing.** This section originally read the missing clock as the place winning a
shared anchor and said `Elimination_HUD.xml` "keeps its clock" because it authors no
`Position`. Both halves are wrong. `PlayerStatus_Update` (`0x0883b3b8`) leaves the
clock's target at `-1` in every mode but Time Trial, Speed Lap, Free Play and
Multiplayer Time Trial, and `Hud_UpdateTimeCluster` (`0x0881c9d0`) hides
`TotalTime`/`TotalTimeTxt` on `-1`: the clock is hidden in a single race whether
or not a place is up. A live PPSSPP frame of an **Eliminator** race
(2026-09-30, `g_game_mode` 8) has no `TOTAL` in its top-right corner although its
layout authors no `Position` - the corner holds `KILLS (5)` and the per-craft kill
column - and both widgets' flag words read with the visible bit clear. See
[`race-progress.md`](../ghidra/functions/psp-pulse-usa/race-progress.md#which-modes-hide-the-clock-confirmed-live-2026-09-30)
for the evidence. `Zone_HUD.xml` authors no `TotalTime` at all (only
`TimeTrial_HUD.xml`, `Elimination_HUD.xml` and `Arcade_HUD.xml` do), so there is
nothing to hide there.

The rule this build applies is [`oag_title::HudArt::total_time_timed_modes_only`]
(true for Pulse alone): outside Time Trial and Speed Lap both widgets are hidden.
`oag_hud::place_owns_the_anchor` stays for the titles the flag is `false`
for - 2048 draws `TOTAL` beside `POS` on a live frame, and HD and Pure are
unmeasured - where it asks the *layout*, not just the readout. **The caption pair was measured directly on a single race, 2026-09-30**
(own PPSSPP, Venom single race on Talon's Junction, `g_game_mode` 3, at `0.23.7`):
`PLAYER_HUD+0x30` reads `0xffffffff`; `TotalTime` (`hud+0x200`) and `TotalTimeTxt`
(`hud+0x204`) both have flag word `0xb082`, visible bit `0x4` clear, against `0xf086`
on `CurrentTime`, `BestTime`, `Position` (`hud+0x240`) and `PositionOf`
(`hud+0x244`); the frame reads `pos` over `8 / 8` and no `TOTAL` anywhere. So the
`TOTAL` caption is hidden by the same `-1` gate as its clock, and `POS` is not
hidden by anything: the two never compete for the corner in Pulse. The **~5 px
overlap argument is retired** - nothing in the original is suppressed because of an
overlap. In Time Trial and Speed Lap the clock and its caption show and there is
no field, so no place. `place_owns_the_anchor` is therefore redundant for Pulse
(the mode gate decides first) and remains only for the titles the gate is off for.

**The same two frames settle a second question nobody asked them.** The original
places its own **parked player 8th of 8 on the grid**, before anyone has crossed
the line - not 1st. That is independent corroboration of the lap-1 rule in
`oag_race::Standing::distance`: a craft that has not reached the line is behind the
field, and the arithmetic that read it as almost a lap ahead disagreed with the
original as well as with common sense. See [ai.md](../gameplay/ai.md#the-field-is-placed).

Found by `no_two_live_widgets_share_an_anchor_on_any_shipped_layout`, which is a
new ground-truth check and reported this collision the first time it ran. Its unit
counterpart in `oag_hud` had claimed for months that the shipped layouts were
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

[`hud::Layout`](../../crates/hud/src/lib.rs) is already the seam. It is a plain
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

- **The `<Mode3D>` layer: the countdown only, now.** `Pulse_Ready_Go` and
  `Cockpit_321GO` still need a second pass with its own projection.

  **Two different objects both plausibly answer "the countdown", and they
  must not be conflated - a mistake this project's own notes made for one
  session before catching it.** One is *this* `<Mode3D>` HUD overlay widget -
  the on-screen "3, 2, 1, GO" graphic drawn near the camera. The other is the
  **track-side starting-line gantry**, a physical 3D object standing on the
  circuit itself, authored per track through `TrackStartup.xml`'s billboard
  slot 8 and unrelated to any HUD layout file - see
  [`billboards.md`](../ghidra/functions/psp-pulse-usa/billboards.md), which has an
  open handover thread of its own tracking exactly which mesh a given circuit's
  gantry resolves to. Findings below are about the **HUD overlay only**; the gantry is a separate,
  still-open question, and it is the more likely home for a player's memory of
  Zone looking different, since it is the one actually standing in the world.

  **The HUD overlay does not vary by mode or view, confirmed 2026-09-02 on all
  three titles checked - `oag-wad`/`psarc.py cat` straight off each disc,
  not inferred.** Pulse's `Arcade_HUD.xml`, `TimeTrial_HUD.xml`,
  `Elimination_HUD.xml` and `Zone_HUD.xml` all carry an identical
  `<Model name="ReadyGo">` entry, `Src="Data\HUD\Pulse_Ready_Go.vex"`, same
  `x`/`y`/`z`/`ztest`, plus an identical `Cockpit321Go` entry pointing at
  `Data\HUD\Cockpit_321GO.vex` - Zone's own copy is byte-for-byte the same
  block as Arcade's. Pure's `Arcade_HUD.xml` and `Zone_HUD.xml` agree the same
  way on a single model, `Data\HUD\Ready_GO.vex` (Pure has no separate cockpit
  entry in either file) - measured in more depth in
  [`rendering/start-gantry.md`](../rendering/start-gantry.md#wipeout-pure-no-track-side-gantry-at-all-confidence-85)'s
  Pure section, since it turned out to be the nearest thing to a gantry this
  title ships: 3 static mesh nodes, an all-white varying-alpha palette, and a
  per-node `TEXOFFSET` scroll unrelated to the track-side gantry's per-vertex
  UV-cell walk. **HD/Fury agrees too, and more thoroughly than
  either PSP title**: its `hud_ready_go.xml` fragment - included by every
  mode's own HUD file via `LoadXML SrcRel`, per this page's own "`SrcRel`
  resolves against the including file's directory" section above, which
  raised exactly this possibility - is **byte-identical across every skin
  checked**, including the default (`/data/xml/`), the `2097_hud` and
  `wo3_hud` skins, and even `splitscreenzone_hud`: all four reference the
  same `Data\HUD\Pulse_Ready_Go.vex` (HD's own asset at `/data/hud/
  pulse_ready_go.vex`), no `Cockpit321Go` sibling found alongside it this
  pass. So **the on-screen "GO" graphic itself is shared across every mode on
  every title checked so far** - Pulse, Pure and HD alike. Zone's own HUD
  layout differs only in its *other* widgets (Pure's Zone layout swaps in
  `Zone_Bar_1/3.vex` and `Zone_outline_1.vex` for the speed/shield bars, in
  place of Arcade's `Bar_1/3.vex`/`Bar_outline_1.vex` - a skin change to the
  HUD bars, nothing to do with the countdown).

  **Correction, 2026-09-06: HD's `hud_ready_go.xml` is disabled, not just
  present.** Its `<Mode3D>`/`<Model>` tags are actually spelled `<aMode3D>`/
  `<aModel>` - the same "authored to be skipped without deleting the
  reference" convention this project's own `docs/formats/hd-hud.md` documents
  for `<aLoadXML>` in `DATA05`'s `speedlap_hud.xml`. `LoadXML_Item` keys on
  the literal tag name, so this fragment is never parsed at all on HD. "Byte
  identical across every skin" above is still true of the fragment's *text*;
  it is not evidence the widget draws - see
  `docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md` for the full
  finding, reached while chasing Pulse's own countdown placement.

  **The track-side gantry is a different story, and it is where a real
  difference is confirmed - on HD, from the disc, not a string search this
  time.** `PS3_GAME/USRDIR/DATA00.PSARC` and `DATA02.PSARC` carry, under
  `/data/billboards/hd_adverts/321go/`, four distinct `.vex` meshes with four
  distinct byte sizes - `321go_startfinish.vex` (12,544 B), `321go_zone.vex`
  (8,368 B), `321go_hd_zone_battle.vex` (10,304 B),
  `321go_hd_detonator.vex` (11,568 B) - real geometry differences, not
  aliases of one shared file. **Which one a given race actually instantiates
  is still the open handover thread's own question**, not settled here or by
  this session. 2048 ships the same four plus two of its own
  (`321Go_2048.vex`, `321Go_2048_Combat.vex`), found by string search rather
  than a full archive listing - not yet cross-checked the PSARC-listing way
  HD was here.
  **A player's memory of Zone showing a different countdown most plausibly
  comes from this gantry, on HD/Fury or 2048** - the on-screen graphic is
  ruled out on every title checked, but the physical track object is a real,
  disc-confirmed difference on HD, still open on whether it actually gets
  drawn per mode.

  **The weapon sights came off this list on 2026-08-26** - they are drawn, and as
  2D quads rather than through a 3D pass. The three model names this entry used
  to list are right; what was missing is that they are **nine widgets over three
  models**: `missile_sight_1` ... `missile_sight_4` all instance
  `missile_sight_outer.vex`, `missile_sight_inner` has its own, and
  `leachbeam_sight_1` ... `leachbeam_sight_4` instance `leachbeam_sight.vex`.
  Each model is a single flat textured quad, so that block needs no projection
  of its own - only a per-quad rotation. **The quad's size is a title
  difference, not a constant**: Pulse's three measure `7.9978027` units square
  and Pure's `missile_sight_inner.vex` measures `11.999471`, both read off the
  vertices at load into `oag_hud::sprite::Placed::quad_extent`.
  `hud::sight_draw::SIGHT_SIZE` still draws all of them at Pulse's `8.0`,
  which is a known flattening rather than a reading - `model_draw` already
  takes the per-model extent and `bracket_draws` does not. See
  [lock-sight.md](../ghidra/functions/psp-pulse-usa/lock-sight.md), which also
  identifies the lock tone as `~ROCKLOCK`.
- **Eliminator's HUD.** Its layout parses; nothing drives its own widgets yet.
  **Zone's no longer belongs on this line** - `"Zone"` and `"Score"` in
  [`text_for`](../../crates/hud/src/draw.rs) draw the counter and the
  score, and as of 2026-08-28 so does a voice line at each milestone this
  title's own `speech_zone.bnk` names one for, see
  [`oag_sound::sfx::Announcer`](../../crates/sound/src/sfx/announcer.rs).
  **Wipeout HD's own Zone counter still has nothing to draw it with**, and not
  for the reason Pulse's did: HD's `zone_hud.xml` authors no `"Zone"` text
  widget at all - the counter is `ZonePlus0`-`ZonePlus10`, a row of
  statically-labelled `"1"`-`"10"` text widgets beside `ZonePlusLight0`-`10`
  image widgets (`docs/formats/hd-hud.md`'s `ALWAYS_ON` list), which reads as a
  light-up dial rather than a number readout, alongside `SpeedClass`/
  `NextSpeedClass` text widgets that likely pair with the `MR_*` speed-class
  voice lines `speech_zone.bnk` also carries
  (`docs/formats/psp-audio.md#speech_zonebnk-names-the-zone-announcer-one-ladder-per-title`).
  Unread on HD's own executable and undrawn here rather than forced onto the
  Pulse-shaped `"Zone"` match arm, which would be wrong for what this widget
  set actually is.
- ~~**Medal targets.** `IG_HUD_GOLD`/`SILVER`/`BRONZE`/`RECORD` need
  progression data...~~ **Closed 2026-09-28** - see "Medal targets, closed
  2026-09-28" below.
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

### Medal targets, closed 2026-09-28

**The tier `TotalTimeTxt`'s caption table switches on is read, and it is
honest against data this build already has.**
[`hud-time-caption-substitution.md`](../ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md)
found the selection *mechanism* first; what fed its ordinal tier stayed open
until a PPSSPP write watchpoint on `*(hud+0x3c)+0x30/+0x34/+0x38` (armed live
off `g_hud`, `0x08ab0838`) caught every write inside `PlayerStatus_Update`
(`0x0883b3b8`), across an eight-second drive of a real Venom Time Trial on
Talon's Junction, `pulse-psp-usa.chd`. Full law, the five write PCs and the
two reference frames this closes on:
[race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md#the-target-time-readout-0x780x7c0x80-closed-2026-09-28).

**It is a different tier from `Cell_EvaluateMedal`'s, as this section
suspected, and the two are related by exactly `2 - medal` on the three
tiers they share.** `Cell_EvaluateMedal` (`race-campaign.md`) numbers
`0 = gold, 1 = silver, 2 = bronze`, evaluated once, at the end of a race;
`PlayerStatus_Update`'s own tier - `0 = BRONZE` up to `3 = RECORD`, matching
this page's caption table - is evaluated **every tick**, live, against the
*same* cell's `gold`/`silver`/`bronze` fields for a campaign Time Trial or
Speed Lap cell - **except that `RECORD` can win there too**, when the
player's own stored personal best for the cell already beats gold and the
live pace beats that best as well. Otherwise (no campaign cell) the tier is
always `RECORD`, compared against that same personal-best store narrowed by
the track's own best time - which is what the frame `hud.md`'s own
"structural finding" first noticed (`record` where the layout authors
`Total`) turns out to be: a non-campaign Time Trial, comparing against a
personal-best/track-record pair rather than a medal at all.

**Implemented for the whole target-time block, 2026-09-30.**
[`oag_hud::TimeTrialPace`](../../crates/hud/src/time_trial_pace.rs)
reimplements the live tier as a pure function of the elapsed tick count
(`race_ticks` for Time Trial, `lap_ticks` for Speed Lap). A campaign cell
races `oag_tables::race_campaign::Cell::gold`/`silver`/`bronze`; any other
Time Trial or Speed Lap is `RECORD` throughout, counting down to
`min(stored best, authored time)` - `<RaceTimes>` for Time Trial, `<LapTimes>`
for Speed Lap, both from the circuit's own `stats.xml`
(`oag_hud::RecordTarget`, read at load off `FEData.wad` like the
campaign's AI-skill lookup). A campaign cell whose stored best already beats
gold shows `RECORD` while the run is ahead of it, as the original does.
`RaceStage::draw_hud` and the headless capture path both compute it.
**Checked against the original on the same ship and track**: a Venom Time
Trial on Talon's Junction shows `record 1.33.2` at `0.23.7` on the live PPSSPP
frame and `rekord 1.33.2` at `0.23.8` on ours (German UI), and `1.57.0` at the
start of the run (`117.0 s`). **The stored best is this build's own, keyed by
circuit, mode and class - chosen, not measured.** The original keys its store
by team (`FUN_088091a0`, `Profile_GetBestRaceTime` in
[race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md#the-stored-best-fun_088091a0-and-the-record-branch-closed-2026-09-30)),
which `oag_game::records::Key` has no field for, so a run in one team's ship
races the best of any team's. A track whose `stats.xml` does not read (any source but Pulse on a PSP disc; the load
report says so) draws the plain elapsed clock in a plain race, as the original does when
its track record is null; a campaign cell still races its own ladder.

### Medal messages, 2026-10-06

**The maintainer's report from play: a Zone run or a Speed Lap goes on after
gold, and nothing on screen says a medal was earned.** The question was whether
the original draws something there that this build lacks.

**Measured negative, by static census (confidence 85): the original draws no
mid-race medal.** `Cell_EvaluateMedal` is reached from the end-of-race record
and two menus, never from the race HUD; the HUD's own message lines name eight
events and a medal is none of them; Zone's layout has no medal or target widget.
Evidence, addresses and the one-sided limit of it (no live frame of a campaign
Zone or Speed Lap crossing a threshold was obtained - the fresh profile has
those cells locked) are in
[hud-messages.md](../ghidra/functions/psp-pulse-usa/hud-messages.md). What the
original does show mid-race is the live target tier in `TotalTimeTxt`
(`IG_HUD_GOLD`/`SILVER`/`BRONZE`, "Medal targets" above) - the tier the current
lap is on pace for, which in Speed Lap resets every lap and so is not an award.

**What ships, because the maintainer asked for it: chosen, not measured, no
confidence score.** A campaign **Zone** or **Speed Lap** cell raises
`Gold medal awarded` (`ER_GMA`, `ER_SMA`, `ER_BMA` - the end-of-race screen's own
phrases, resolved from the disc's table) the tick the player's result first
reaches a tier, and again on each better tier. The test is
`RaceStage::campaign_medal` asked mid-race - the same `Cell_EvaluateMedal` law
the results use, so it never promises a medal the results withhold. The race is
**not** ended: the original ends none of these on a medal, and nothing here
adds one.

The presentation is the original's, not invented: the line goes into the
message lines `Info1` to `Info4` the layouts already author
(`oag_hud::messages`), with the slot law decompiled from `Hud_UpdateMessages` -
four seconds, the `sin(pi * (1 - (t/4)^6))` fade, green for a good message, one
line per 0.8 s when several are raised together. That law is itself decompiled
and not yet checked against a live frame.

**Ported to HD and Pure on Pulse's law** (`oag_title::HudArt::message_slots`; a
title with no measured rule of its own takes Pulse's). HD's Zone, Speed Lap and
Time Trial layouts all compose the same `Info1`-`Info4` through
`InfoTextParent` - checked 2026-10-06 on the composed layouts, so the earlier
reading of HD's `Info1` as an elimination-only text box was wrong - and HD's own
English table carries `ER_GMA`/`ER_SMA`/`ER_BMA`: the line draws in HD's font at
the place its layout authors (`medal_message_ground_truth`, and
`data/scratch/hud-medal/shots/hd-zone.png`; legibility on a bright backdrop is
poor, HD authors no backing frame for it). **Pure's table has no `ER_GMA`**, so
nothing draws there - an id a table lacks draws nothing, never the raw id; a
Pure phrase (its `zone_gold`/`zone_silver`/`zone_bronze` voice lines suggest
Pure does announce a Zone medal) is the open piece. 2048 (`RaceMedal` is never
seen in any state reached) and Omega (racing out of scope): **checked, applies,
not wired**.

**A standing line, added the same day (chosen, not measured).** The banner is
four seconds and then the player cannot tell which medal is already safe, so the
last of the four slots (`Info4`) now keeps the best medal earned so far for the
rest of the race, in the same disc phrase (`Gold medal awarded`) and in the
widget's own authored colour and outline - the words name the tier, so no
colour of this build's own is added (a first version tinted it with the campaign
screens' medal swatches; on HD's orange sky the tint disappeared and the
authored white read). It reuses `Info4`, so it lands where no other widget of
any layout does: Pulse's Zone and Speed Lap layouts author `Info1`-`Info4` at
`(240, 80/95/110/125)`, centred and 0.6 scale, with nothing else within
that band (dumped from the composed layouts 2026-10-06); HD's author them at
`(600, 280/365/450/535)` in a 1920x1080 space in both layouts. While a banner
holds that slot the standing line yields, and it is not drawn while a banner
with the same words is up. `RaceStage` raises both through one function,
`oag_game::medal_watch::tick`, which the headless capture calls too, so a
capture of a real run shows exactly what the window would:
`--campaign-cell grid8_3_2` (Pulse Speed Lap, silver at 44 s) or
`--campaign-cell grid0_4_2` (Pulse Zone, or HD's own `grid0_4_2`). Seen on live
frames: Pulse Speed Lap silver at the lap that earned it and still up ten
seconds on, Pulse Zone silver at zone 19, HD Zone bronze at zone 14
(`data/scratch/hud-medal-2/shots/`). On a very bright backdrop (Pulse's cyan
Zone circuit) the line is as hard to read as every other authored Pulse HUD
label there; nothing here adds a backing the layout does not author.

**HD legibility, checked.** HD authors `Info1`-`Info4` as white `Small` text with
a `0,0,0,0.25` border and no backing, in both Zone and Speed Lap, the same
style as every other HD label (`TimeDiffText`, `ZonePlus*`; the ladder tabs are
the only HD text with a backing). So the washed-out banner on a bright backdrop
is HD's own text style over an overbright sky, not a defect of the line, and
nothing was added. The banner's green is Pulse's runtime colour (`0x30ff30`),
which HD inherits under "unmeasured titles take Pulse's law"; HD's own colour
for these lines is unread.

**`MESSAGE` cue, wired.** `Cue::Message` (`"MESSAGE"`, `hud.bnk`, 0.34 s) plays
dry the tick a slot starts showing (`MessageBoard::just_shown`, from
`Race::tick`), as `Hud_UpdateMessages` does. It resolves on every PSP and PS2 disc
and on HD's `weapons.bnk` (6 waveforms; bank presence only, HD's dispatch is
unread). Checked headlessly: two `--dump-audio` runs of the same Zone race, one
with `--force-medal 400:gold`, differ for exactly 0.34 s from the tick the line
shows. Decompiled, not heard against a live original.

**Title status.** 2048: **checked, applies, not wired** - its campaign events
earn `Pass`/`Elite` (`oag_2048::campaign::Tier`), not gold/silver/bronze, its
HUD dialect carries `message_slots: false` and no `Info1`-`Info4`, and
`RaceMedal` was never seen; a medal line there would need its own phrase and
widget first. Omega: **not checkable** (racing out of scope, no HUD to draw
into). Pure: unchanged - no `ER_GMA` in its table, so neither line draws.

**Not done, and open:** the `gold_med`/`silver_med`/`bronze_med` jingles have no
recovered trigger (unplayed even on the end-of-race screen) and stay silent.
Time Trial raises nothing mid-race: its medal is at the finish. Eliminator
(kills against gold) would take the same hook and was not asked for. The
other eight events the original raises in these lines (`IG_HUD_PLAP`,
`NEWLAP_REC`, `FIN_LAP`, `PERF_ZONE`, `NEW_ZONE_RECORD`, ...) have no recovered
trigger and stay unraised. A capture aid, `--force-medal TICK:TIER`, shows the
line on any `--race` capture; the real trigger needs a campaign cell, which a
headless run cannot reach.

## Lap counting was the one real blocker

**Resolved 2026-09-16**: `Hud_UpdateLapCounter` (`0x0881a690`) writes `Lap`
and `LapOf` from the `"PLAYER_HUD"` block's `+0x08`/`+0x0c`, which
`PlayerStatus_Update` fills from `craft+0xacc - 1` and `g_race_laps`; the
counter itself is `Craft_UpdateLapProgress` (`0x08842a18`). See
[race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md). The
paragraphs below are the search as it stood, kept because the widget-name
lead in them is what found it.

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
