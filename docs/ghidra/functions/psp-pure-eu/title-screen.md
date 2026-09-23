# `/pure/BOOT-psp-pure-eu.BIN` - `Title Screen`'s wordmark, `FE Screen`'s backdrop, and the `<Animation><Key>` reveal

Where `Title Screen->TitleFrame`, `FE Screen->BackgroundController`'s two
children, and Pure's own `<Animation><Key>` timeline get their runtime
behaviour. See [`docs/formats/pure-status.md`](../../../formats/pure-status.md)
for the content-scan history these findings correct and extend.

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure EU, `UCES-00001`), image base `0x08804000`, and Pure USA (`UCUS-98612`), same base |
| **Decompiler constant offset** | Every immediate this decompiler prints for a `lui`/`addiu` pair building a `.rodata` or `.data` address is the real address **minus `0x08804000`** - verified independently against three known strings (`"Title Screen->TitleFrame"`, `"TextureWidth"`, `"Data\FE\Images\FMV_last_frame"`) via `get_xrefs_from` on the exact delay-slot instruction, not assumed from one match. Apply the offset to read any address this page's decompiles print. |

## `TitleFrame`'s wordmark: a literal, region-suffixed name, confirmed live

`Screen_ConstructTitleScreen` (`FUN_08952554` EU / `FUN_08952c00` USA, bodies
identical past relocated immediates) is a per-screen-type constructor: it
finds `Title Screen->TitleFrame` by path
(`Screen_FindElementByPath(screen, "Title Screen->TitleFrame", 1)`), builds a
texture name by concatenating two separately-defined string literals -
`"Data\FE\Images\FMV_last_frame"` (`0x08a76710` EU, identical on USA) and a
**per-pressing** suffix (`"_EU.mip"` at `0x08a76730` on the EU binary,
`"_US.mip"` at `0x08a7cc70` on the USA binary) - then loads it by that
concatenated name and assigns the result to the `TitleFrame` element's own
texture field.

**The hash closes the loop.** `oag_formats::wad::hash_name` on each literal
name lands exactly on a hash this project already had, from the independent
content-scan documented in `pure-status.md`:

| Literal name (from the executable) | `hash_name` | `Data.wad` entry | Content |
| --- | --- | --- | --- |
| `Data\FE\Images\FMV_last_frame_EU.mip` | `0xb6677aab` | 535 | blue "wipEout" over orange "pure" |
| `Data\FE\Images\FMV_last_frame_US.mip` | `0x3af18d90` | 537 | orange "wipEout" over white-outlined "pure" |
| `Data\FE\Images\FMV_last_frame_JAP.mip` | `0x3f313472` | 536 (byte-identical to 535) | not checked against a JAP disc - none is in this project's corpus |

The EU and USA hashes are exactly the two values 2026-09-08's scan already
found and named `oag_pure::hashes::TITLE_LOGO`/`TITLE_LOGO_EU`, closing what
that scan's own confidence cap named as unread: "the runtime mechanism that
assigns this hash to this widget". They are **not the same picture** - a
different colourway per pressing, not the same texture under two names - so a
single hard-coded hash is wrong for one pressing no matter which of the two it
picks. `oag_pure::frontend::title_frame_src` is the fix; see that function's
own doc comment.

**Confirmed live, not just statically read.** A breakpoint on
`0x08952554` fires on a real `pure-psp-eu.chd` boot under PPSSPP v1.20.4
(Xvfb, SDL build, websocket debugger) - the CPU actually stops there during a
real boot, so this is not dead or conditionally-compiled code. The same
session's screenshot of the settled `Title Screen`, at native rect
`x=0 y=76 width=480 height=128`, shows the blue-over-orange colourway -
matching entry 535, not 537 - confirming the EU pressing genuinely draws its
own, different, wordmark. Screenshots:
`~/.cache/oag/drive/reports/pure-title-reveal/eu-title-screen-settled.png`.

**One detail flagged rather than resolved**: the constructor writes the
loaded texture into the `TitleFrame` element's own field at offset `+0xd4`
(with `+0xdc`/`+0xe0` defaulted to `1.0f` and `+0xe4`/`+0xe8` to `0`) - which
field the widget's own *draw* call actually reads is not traced, so the
identification rests on the name/hash/live-capture chain above, not on having
read the draw call too. That chain alone is confidence **90**.

Confidence **90** (EU and USA constructors both). Named
`TitleScreen_AssignWordmarkTexture` in both binaries' `names.tsv`.

## `BackgroundController`'s two children: a declared global, not a hash

`BackgroundController_UpdateImages` (`FUN_088bb004` EU / `FUN_088bb770` USA)
runs every frame against `FE Screen->BackgroundController`'s own object. Its
own constructor, `FUN_088baf48`, resolves three names to global-variable
handles at construction time - `"BackgroundColor"`, `"BackgroundTexture"` and
`"BackgroundTopRightTexture"` (via `FUN_088afe94`, stored at offsets `+0xb0`,
`+0xb4`, `+0xb8`) - the same `FEGlobals->Name` indirection
`FrameLineColor`/`TitleColor` already go through for colours
(`docs/formats/race-setup.md`), just carrying a WAD path string instead of an
ARGB one for the latter two.

Each frame, `BackgroundController_UpdateImages` resolves `+0xb4`
(`BackgroundTexture`) and `+0xb8` (`BackgroundTopRightTexture`) to their
current string value via `FUN_088aff90`, and if the resolved string changed
since last frame, reloads the corresponding element's texture (`+0x9c` =
`BackgroundImage`, `+0xa0` = `BackgroundTopRightImage`) by that name through
`FUN_08a3af94` - **or leaves it untextured if the resolved string is empty or
null**, an explicit branch (`if (pcVar4 != 0 && *pcVar4 != '\0')`), not a
side effect of a failed lookup.

**`Data\Skins\Default\Skin.xml` (the second, activated style skin -
`docs/formats/race-setup.md`) declares both**, identically on both pressings:

```xml
<Variable global="BackgroundColor">
    <Values String="0xFFFFFFFF"></Values>
</Variable>
<Variable global="BackgroundTexture">
    <Values String=""></Values>
</Variable>
<Variable global="BackgroundTopRightTexture">
    <Values String="Data\Skins\Default\Images\default_texture.mip"></Values>
</Variable>
```

`hash_name("Data\Skins\Default\Images\default_texture.mip")` is
`0x7ba78aca` - exactly `oag_pure::hashes::MENU_TOPRIGHT_LOGO`, the value the
2026-09-08 content scan already found and matched against a real `Main Menu`
capture. **`BackgroundImage`'s own empty string is the same kind of
confirmation for the negative**: `Main Menu`'s flat white background was
already measured off a capture; this is the disc's own declared reason for
it, not merely a corroborating absence.

Confidence **80**: the field layout, the three resolved names and the
empty-string branch are read directly from the decompile, and the hash and
capture both agree with the resolved literal - short of 90 because
`FUN_088aff90`/`FUN_08a3af94`/`FUN_088b0118`'s own exact signatures (return
types, what a failed lookup returns) are inferred from call-site behaviour
rather than independently decompiled and confirmed.

Named `BackgroundController_UpdateImages` in both binaries' `names.tsv`.

## `<Animation><Key>`'s own struct, read but not its consumer

`Animation_ParseValuesOrKey` (`FUN_088b9f28` EU / `FUN_088ba694` USA) is the
child-tag handler an `<Animation>` element calls for each of its own
`<Values>` and `<Key>` children (found via `get_xrefs_to` on the `"Values"`/
`"Key"`/`"TextureWidth"` strings landing in the same function, then verifying
this function's own address appears as a table entry reached from the
`"Animation"`-tag registration call, `FUN_088ba4bc`/`FUN_08952c00`'s sibling -
`FUN_088b3448(0x2b50f8, "Animation")` on EU).

**`<Values>`'s five attributes**, matched against the string table at
`0x08a4ec10` (all five present verbatim: `IncButton`, `DecButton`,
`FixedFrames`, `Loop`, `Active`) and cross-checked against `Data\Plugins\PI001\
GUI\Skin.xml`'s own `<Values Incbutton="none" Decbutton="none" Loop="false">`
on every `<Animation>` `Title Screen` authors.

**`<Key>`'s own struct is 0x20 bytes, allocated per key and linked into the
`Animation`'s own `+0x9c` list** (`FUN_088943cc(0x20, ...)`; each new key is
appended via a `+0x1c` `next` pointer, and the `Animation`'s own `+0xd8` is
incremented as a running count). Seven fields, matched the same way against
the same string table:

| Offset | Field | Default | Authored on `Title Screen`? |
| --- | --- | --- | --- |
| `+0x00` | `Time` | `0` | always |
| `+0x04` | `X` | `0` | never (this screen) - `hud.rs`'s own `<Animation><Key>` reading is this field, on Pulse's HUD icons |
| `+0x08` | `Y` | `0` | never (this screen) |
| `+0x0c` | `TextureWidth` | `0` | always - the field this screen actually uses |
| `+0x10` | `TextureHeight` | `0` | never (this screen) |
| `+0x14` | `ScaleX` | `1.0f` | never (this screen) |
| `+0x18` | `Scaley` (sic - the disc's own spelling, not a transcription error) | `1.0f` | never (this screen) |
| `+0x1c` | `next` | `0` | (linked-list pointer, not authored) |

**The `Animation` object's own field layout**, read off its constructor
(`Animation_ConstructFromNode`, `FUN_088b9854` EU only - the USA twin was not
located this pass): `+0x3c` is the parse-time tag-dispatch table pointer
(`0x2b50f8`, the table `Animation_ParseValuesOrKey` and
`Animation_ConstructFromNode` are both entries of); `+0x9c` the key list head;
`+0xa0`/`+0xa4` the `IncButton`/`DecButton` names resolved to ids; `+0xa8`/
`+0xa9`/`+0xaa` the three `<Values>` booleans (`FixedFrames`, `Loop`,
`Active` - `Active` defaults **true**, the other two **false**); `+0xac` a
`1.0f`-defaulted scale; `+0xb0` the **last key's own `Time`** (copied out on
every new key parsed, so it always holds the final key's `Time` once parsing
finishes - the animation's own total duration); `+0xd8` the key count;
`+0xdc`/`+0xe0` both default `0` and are plausibly the playback clock and the
last-evaluated output, but neither is confirmed by a read of the code that
writes them during playback.

**What this settles about the authoring, independent of the consumer.**
Every `<Animation>` on `Title Screen` wraps exactly one child widget and
brackets `TextureWidth` between `-<width>` (the wrapped widget's own `width`/
`TxtrWidth`, negated) at an early `Time` and `0` at the final `Time` -
confirmed for all thirteen: the four frame lines (`TitleAnim1`/`2`/`4`/`5`),
the eight bracket/box segments (`TitleAnim9`-`11`, `13`-`21`) and the four
textured patches (`TitleAnim3` barcode, `TitleAnim6` tiny text, `TitleAnim7`
double arrows, `TitleAnim12` Japanese characters). Several repeat the same
negative value at an intermediate `Time` before the final `Time` snaps it to
`0` (e.g. `TitleAnim13`: `-3` at `0`, `-3` at `0.63`, `0` at `0.65`) - a hold
then a fast transition, not a `Time`-`0`-to-final linear ramp throughout.

**What is not settled: how `TextureWidth` maps to a rendered pixel.** Whether
the widget's own displayed width is `authored_width + TextureWidth` (reading
`0` at the negative end, growing to the full width by the final key), whether
it crops a texture-space sample instead of the render rect, and which screen
edge stays anchored while the other moves, are none of them read from a draw
call - only the authoring convention above is read. The consuming
update/draw pair was searched for (the parse-time table at `+0x3c` is
confirmed to hold `Animation_ParseValuesOrKey` as a *parse*-time entry, not a
runtime behaviour vtable - it mixes function pointers with what look like
further nested tables, e.g. `FUN_088b9ca8` alongside data-shaped entries in
the 0x234000-0x237000 range) but not found: the object's own real leading
vtable (C++ ABI offset `+0x00`, never written by either constructor this page
decompiled) is the more likely location for `Update`/`Draw` and was not
reached this pass. **Do not implement a reveal animation from the negative
convention alone** - it is a strong authoring pattern, not a read of what the
renderer does with it.

Confidence **80** for the `<Key>`/`<Values>` struct layout and the authoring
convention (both read directly and cross-checked against the shipped XML);
**no confidence claimed** for the render mapping, which is unread. Named
`Animation_ParseValuesOrKey` (EU and USA) and `Animation_ConstructFromNode`
(EU only) in `names.tsv`.

## Applied names

| Address | Name | Confidence | Binary |
| --- | --- | --- | --- |
| `0x08952554` | `TitleScreen_AssignWordmarkTexture` | 90 | Pure EU |
| `0x08952c00` | `TitleScreen_AssignWordmarkTexture` | 90 | Pure USA |
| `0x088bb004` | `BackgroundController_UpdateImages` | 80 | Pure EU |
| `0x088bb770` | `BackgroundController_UpdateImages` | 80 | Pure USA |
| `0x088b9f28` | `Animation_ParseValuesOrKey` | 80 | Pure EU |
| `0x088ba694` | `Animation_ParseValuesOrKey` | 80 | Pure USA |
| `0x088b9854` | `Animation_ConstructFromNode` | 80 | Pure EU (USA twin not located this pass) |

## Next steps

- Find the object's real leading vtable (offset `+0x00`) for the `Animation`
  class, and read its `Update`/`Draw` slots - this is what would answer the
  render-mapping question above. A breakpoint on `Animation_
  ParseValuesOrKey`'s own caller (the generic "walk an element's children by
  tag" loop, not yet identified either) would find where the allocated
  `Key`/`Animation` objects go next.
- `Data.wad` entry 536 (`Data\FE\Images\FMV_last_frame_JAP.mip`) names no
  disc in this project's corpus. Leave unwired rather than guessed at.
- `FUN_088aff90`/`FUN_08a3af94`/`FUN_088b0118` (global-to-string resolution,
  name-based texture load, and whatever the third does with a colour) are
  none of them independently decompiled past the call-site behaviour this
  page infers.
