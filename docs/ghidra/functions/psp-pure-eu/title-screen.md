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
located this pass): `+0x3c` is the object's own vtable pointer (see the
2026-09-28 section below for the correction - it is not merely a parse-time
tag-dispatch table); `+0x9c` the key list head;
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

Confidence **80** for the `<Key>`/`<Values>` struct layout and the authoring
convention (both read directly and cross-checked against the shipped XML).
Named `Animation_ParseValuesOrKey` (EU and USA) and `Animation_ConstructFromNode`
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
| `0x088b9a80` | `Animation_Update` | 90 | Pure EU |
| `0x088b9dd0` | `Animation_InterpolateKeys` | 90 | Pure EU |
| `0x088b9d0c` | `Animation_JumpToTime` | 80 | Pure EU |
| `0x088b9ca8` | `Animation_ComputeRect` | 85 | Pure EU |
| `0x0889300c` | `Element_UpdateTree` | 85 | Pure EU |
| `0x088b38b4` | `Element_UpdateFade` | 85 | Pure EU |
| `0x088b73c4` | `Widget_CreateFromElement` | 80 | Pure EU (USA twin not located this pass) |
| `0x088b5010` | `Element_ComputeRect` | 80 | Pure EU |

## 2026-09-26: the reveal's `TextureWidth` is write-once, not re-read - a live watchpoint, not a guess

**The consumer is still not found, and this pass adds a specific, evidence-backed
reason why the obvious next moves both failed**, rather than another round of the same
static search.

**`Animation` base-class chain confirmed to write `0` at offset `+0x00`, never a vtable
pointer.** `Animation_ConstructFromNode` (`0x088b9854`) calls `FUN_088b350c`, which
calls `FUN_08892e6c`; decompiling all three top to bottom (the full chain, not just the
outermost frame) shows `+0x3c` overwritten three times, once per level, with three
*different* table pointers (`0x2b4b08`, then `0x2b50f8` from the derived level) - a
reused "current parse-tag-dispatch table" slot, not a vtable, matching what the
2026-09-23 pass already flagged. **`FUN_08892e6c` (the root-most constructor reached)
explicitly zeroes `+0x00`** (`*param_1 = 0;`) and nothing later in the chain writes it
again. So the "find the real vtable" plan in the old Next Steps here rests on a field
that is deliberately zero at construction and never assigned by any constructor this or
the prior pass decompiled.

**This section's own reading of `+0x3c` as "not a vtable" is corrected below (2026-09-28):
`+0x3c`, not `+0x00`, is the real vtable pointer.** The observation here (three levels,
three table pointers) is still accurate; the *interpretation* - that this rules `+0x3c`
out as a vtable - does not survive a live read. Left in place rather than deleted, since
the arithmetic slip that caused the miss (below) is itself worth keeping visible.

**`FUN_088b9ca8`, the one candidate the 2026-09-23 pass flagged from the `+0x3c` table's
neighbourhood, was retracted here as "an unrelated position += velocity integrator" - that
retraction is itself wrong, and is corrected in the 2026-09-28 section below as
`Animation_ComputeRect`.** The decompile quoted in that retraction is accurate; what was
wrong was the claim that its offsets don't line up with `Animation`'s own layout, which
rested on applying the `+0x08804000` correction to the *wrong* neighbouring table
(`0x08ab50f8`, an arithmetic error - the correct sum is `0x08ab90f8`).

**A live, non-halting memory watchpoint answers a narrower but real question: nothing
reads `Key.TextureWidth` through a normal CPU load, ever, across a full boot-to-reveal
window.** Method: PPSSPP v1.20.4 SDL build, Xvfb `:97`, debugger port `45001`,
`memory.breakpoint.add` with `enabled=False, log=True, read=True, write=True` (the
non-halting form `docs/reverse-engineering/ppsspp-debugger.md`'s watchpoint section
documents) armed on twelve `Key.TextureWidth` addresses *before* any navigation input,
covering the four wide frame-line-shaped keys (`-448`, `-444`, `-438`, `-256`/`-176`
two-and-three-key chains) plus one textured-patch chain (`-347`). These addresses are
**static and reproducible**: identical byte-for-byte across two independent cold boots,
both while `Language Selection` was still showing and again once `Title Screen` itself
was on screen and fully settled - the front end's screen-definition parse tree lives at
a fixed location (`0x08b32460`-`0x08b3722c` this build) regardless of which screen is
currently active, not a per-navigation heap allocation the way the `TitleFrame`/
`BackgroundController` texture assignments are. **Confirmed still true, live, in the
2026-09-28 pass**: a real `Animation`'s own `+0x9c` key-list head resolves inside this
same range, so the watchpoints were armed on the right memory.

Scripted the whole way from a cold boot - `Language Selection` (cross) ->
`Developer Publisher Screen` (auto) -> `MemoryStickWarning` (cross) -> `FMV Intro`
(start, skipping the movie) -> `Title Screen`, then a further 6 s sitting on the
settled title screen - with the twelve watchpoints armed for the entire span. Result,
read via `memory.breakpoint.list` and cross-checked against the emulator's own log
lines:

- **Every one of the twelve fired exactly once, a `Write32`, all at the same PC**
  (`0x08899318`, PPSSPP's own auto-symbol `z_un_088992f4`) - the parse-time write that
  populates the `Key` from the authored XML attribute, matching
  `Animation_ParseValuesOrKey`'s already-documented behaviour.
- **Zero reads, on any of the twelve, across the whole window** - including the roughly
  0.65-1.5 s the authored `Key` data itself says the reveal should be actively
  interpolating in. The watchpoint mechanism itself is demonstrably alive for this
  exact window (it caught all twelve writes, correctly timestamped and PC-tagged), so
  this is not the "watchpoint never armed" failure `ppsspp-debugger.md` warns about.
  **Resolved in the 2026-09-28 section below**: `Animation_InterpolateKeys` does read
  each `Key` field, once per `Animation_Update` call - this window's own zero-hits
  result is not explained by this pass, only superseded by a direct read of the
  consuming function.

**The VFPU hypothesis this pass raised is retired (2026-09-28): the actual consumer
reads every `Key` field with plain scalar `f32` loads.** Recorded here for the history;
see below for the live-confirmed function.

Also checked and ruled out: writing a new value directly into an already-settled
`Key.TextureWidth` (post-reveal, `Title Screen` sitting on "PRESS START") and letting
the CPU run has **no visible effect on the next rendered frame** - consistent with (and
independent evidence for) "write-once, not re-read": whatever renders the settled
widget is not deriving its width from this field every frame either, scalar or VFPU.

Confidence **75** for "the raw `Key.TextureWidth` field is not read by any normal CPU
load during construction, the reveal window, or afterward" (the watchpoint result,
positive-signal-checked) - this narrow claim stands; the render-mapping conclusions
drawn from it (VFPU, no consumer) do not, see below.

## 2026-09-28: the consumer, found - `+0x3c` is the real vtable, not just a parse table

**The 2026-09-26 dead end was a self-inflicted arithmetic error.** That pass computed
`0x2b50f8 + 0x08804000` by hand and got `0x08ab50f8` - off by `0x00040000`. The correct
sum is `0x08ab90f8`. Reading the wrong address is exactly why the table there looked
unrelated (mesh/vertex-sort code): it *was* unrelated, because it was the wrong table.
`FUN_088b9ca8`'s retraction inherited the same error and is itself wrong - see below.

**`Element_UpdateTree`, found from a live breakpoint's own return address, is the generic
per-frame update walker every element on a screen goes through.** Breaking on
`Animation_Update` (below) during a real reveal and reading `ra` lands inside a function
at `0x0889300c` whose decompile is exactly a tree-walking dispatcher:

```c
void Element_UpdateTree(dt, element) {
    if (!(element->flags & 8)) {                      // not hidden
        if ((*(*(element+0x3c)+0x24))(dt, element + *(short*)(*(element+0x3c)+0x20))) {
            for (child = first_child(element); child; child = next_sibling(child)) {
                if (child->flags & 2) Element_UpdateTree(dt, child);
            }
            if (!(element->flags & 0x8000)) {
                (*(*(element+0x3c)+0x2c))(element + *(short*)(*(element+0x3c)+0x28), 1);
            }
        }
    }
}
```

`element+0x3c` is the object's own **real vtable pointer** - not merely "the parse-time
tag-dispatch table" the 2026-09-23/26 passes characterised it as. That was not wrong about
what the table *contains* (it does hold `Animation_ParseValuesOrKey` as one entry), only
incomplete about what else lives in the same table: `+0x24` is the per-frame **Update**
slot and `+0x2c` a second slot called after every child has updated (a post-order
finalize; for `BackgroundController` this slot is a four-instruction "set the drawn flag"
stub, `0x08a2e678` - not a draw call, so what actually rasterises a widget is still not
this page's own finding). Both calls pass an *adjusted* `this` (`element + a 16-bit
offset read from the table itself`), the standard C++ multi-base thunk shape -
confirming `+0x3c` is a real vtable, once you look at the right address.

**`Animation_Update` (`0x088b9a80`) is that `+0x24` slot for an `Animation` object,
confirmed two independent ways:**

- **Static**: `Animation`'s own vtable sits at `0x08ab90f8` (raw `0x2b50f8`, corrected).
  `0x08ab90f8 + 0x24 = 0x08ab911c`, and the four bytes there are `80 5a 0b 00` -
  unrelocated `0x000b5a80`, i.e. `0x088b9a80` once the base is added back. Found by
  literal byte search for the function's own address as data (`search_byte_patterns`),
  the same method that found `BackgroundController_UpdateImages` sitting at its own
  vtable's `+0x24` (`0x08ab9298 + 0x24 = 0x08ab92bc`) - two classes, the same slot.
- **Live**: a breakpoint on `0x088b9a80` fires repeatedly during a real reveal
  (`pure-psp-eu.chd`, PPSSPP v1.20.4, Xvfb `:94`), once per `Animation` object per frame,
  stepping through thirteen ~0x2a0-byte-apart heap objects - matching "all thirteen"
  `<Animation>`s the 2026-09-23 pass already counted on `Title Screen`. Reading `ra` at
  the breakpoint gives `0x08893064`, squarely inside `Element_UpdateTree`'s own body -
  the actual running call site, not an inference from the vtable shape alone.

`Animation_Update(dt, animation)` advances a running clock at `animation+0xb8` by
`dt * animation+0xac` (the `IncButton`/`DecButton` branches at `+0xa4`/`+0xa0` are for a
different, button-driven mode nothing on `Title Screen` authors), clamps it to
`[animation+0xb4, key list's own duration]`, then unconditionally calls
`Animation_InterpolateKeys`.

**`Animation_InterpolateKeys` (`0x088b9dd0`) is the interpolator the whole thread was
looking for.** It walks the `Key` list at `animation+0x9c`, finds the two keys bracketing
`animation+0xb8` (or holds the nearest boundary key's value outside the list's own span),
and linearly interpolates all six fields - `X`/`Y`/`TextureWidth`/`TextureHeight`/
`ScaleX`/`ScaleY` - into `animation+0xbc`/`+0xc0`/`+0xc4`/`+0xc8`/`+0xcc`/`+0xd0`. Plain
scalar `f32` arithmetic (`pfVar1[N] + t * (pfVar2[N] - pfVar1[N])`) - the 2026-09-26
pass's VFPU hypothesis is **retired**: nothing here needs `lv.q`. A live read this pass
of a real `Animation`'s own `+0x9c` list head resolves inside the documented static
parse-tree range `0x08b32460-0x08b3722c`, not a per-instance copy, so the 2026-09-26
watchpoint (zero scalar reads of `Key.TextureWidth`) was watching the right memory - the
resolution is that `Animation_InterpolateKeys` reads each `Key` field once per `Update`
call, not on every query, and the watchpoint's own capture window may simply not have
caught the specific write-then-read pairing rather than there being no scalar read at all.

**`Animation_ComputeRect` (`0x088b9ca8`) is un-retracted.** The 2026-09-26 pass decompiled
it correctly:

```c
void Animation_ComputeRect(int animation) {
  Element_ComputeRect(animation);
  *(animation+0x48) += *(animation+0xbc);   // + interpolated X
  *(animation+0x4c) += *(animation+0xc0);   // + interpolated Y
  *(animation+0x50) += *(animation+0xc4);   // + interpolated TextureWidth
  *(animation+0x54) += *(animation+0xc8);   // + interpolated TextureHeight
}
```

and retracted it as "a four-field `position += velocity` integrator... a different class
entirely" because applying the project's own `+0x08804000` correction to the
*neighbouring* raw table entries landed on unrelated mesh/vertex-sort code - the same
arithmetic slip as above, on a table this function has nothing to do with.
`Animation_ComputeRect` is exactly what its call to `Element_ComputeRect` (the shared
base - see next) says: `Animation`'s own override of the generic rect-layout method,
confirmed live by the vtable slot it actually occupies (`0x08ab90f8+0x7c`, matching
`Element_ComputeRect`'s own slot in `BackgroundController`'s vtable,
`0x08ab9298+0x7c = 0x088b5010`).

**`Element_ComputeRect` (`0x088b5010`) is the shared base every class's `+0x7c` slot
either uses directly (`BackgroundController`) or calls before adding its own delta
(`Animation`).** It walks up to the parent's own `+0x3c` vtable, calls the parent's own
`+0x7c` slot (recursive up the tree) for the parent's rect, adds the parent's own
`+0x48..+0x54` into the child's own `+0x48..+0x54`, then adds the child's own local
offset (`+0x40`/`+0x44`). So `animation+0x50` (width) starts as the parent chain's own
accumulated width, and `Animation_ComputeRect` adds the interpolated `TextureWidth`
(authored `-width` to `0`) on top - **the widget's own rendered width literally shrinks
to `0` and grows back to its authored size**, not a texture-space crop computed
elsewhere. `oag_ui::screen::interpolate_reveal`/`oag_ui::frontend::draw`'s `reveal_delta`
mirror this formula directly.

**One rendering detail this static chain does not settle, resolved from a live capture
instead: crop, not stretch.** `Element_ComputeRect`'s own accumulation only touches the
render-space width (`+0x50`); nothing in this chain touches a widget's separately-authored
`TxtrWidth`/sample-space attribute, so the decompile alone is consistent with either
"stretch the full texture into a narrower box" or "sample proportionally less of it". A
free-running capture of `Title Screen`'s own barcode patch mid-reveal
(`reveal_150ms.png`, `pure-psp-eu.chd`, PPSSPP v1.20.4) shows a
crisp, correctly-proportioned partial pattern - bars at their own native width, growing
from the left - not a squeezed one. `crates/ui/src/frontend/draw.rs` shrinks the sampled
UV width by the same amount as the render width, which is what reproduces that: a real
capture breaking the tie a static read alone could not.

**`Element_UpdateFade` (`0x088b38b4`) is a second, separate reveal - the answer to what
actually happens to `TitleFrame` itself, since `TitleFrame` is not one of the thirteen
`<Animation>`-wrapped elements (confirmed live: none of nine live `Animation` objects'
own `+0x18` child pointer equals `TitleFrame`'s own address).** `Animation_Update` calls
it unconditionally at its own top (every `Animation`-wrapped widget gets this too, on top
of its own `TextureWidth` wipe), and it is also `TitleFrame`'s own update path directly.
A non-halting write watch on a live `TitleFrame` object (`pure-psp-eu.chd`, address
`0x08ed6880` that boot) caught it firing every frame, writing `element+0x6c` (`0.0099..`
rising to `1.0`) and `element+0x94`. The decompile is a three-mode fade controller keyed
on `element+0x70`/`+0x74` (fade-in/fade-out durations) and a global "entering vs. leaving"
flag at `*(element+0x68)+0xc8`:

- `+0x70 == 0`: an optional delay (`+0x98`) then an instant snap to `0` or `1`.
- `+0x70 != 0`, leaving: `+0x94` counts down from `+0x74`, `alpha = +0x94 / +0x74`.
- `+0x70 != 0`, entering: `+0x94` counts up to `+0x70`, `alpha = +0x94 / +0x70`.

`element+0x2c`'s bit `0x4` (the "still needs updating" flag `Element_UpdateTree` checks
on a child) clears once `alpha` settles within `0.0001` of its target. Live-read off
`TitleFrame` post-settle: `+0x70 = +0x74 = 0.1` seconds - **`TitleFrame`'s own reveal is a
0.1-second linear alpha fade-in, not a width wipe, over in six frames at 60 Hz.** Where
`0.1` itself is authored (a per-widget attribute, a per-skin default, or a constructor
constant) is not read; the base chain (`FUN_08892e6c`, `FUN_088b350c`) zeroes every one of
`+0x6c`/`+0x70`/`+0x74`/`+0x94`/`+0x98` at construction, so whatever sets `0.1` runs later
and was not traced this pass.

**Confidence.** `Animation_Update`/`Animation_InterpolateKeys`: **90** - the vtable slot
match and the live `ra`-at-breakpoint both agree, and the interpolation formula is a
direct decompile. `Animation_ComputeRect`/`Element_ComputeRect`: **85** - decompiled
directly and the vtable-slot correspondence confirmed, short of 90 only because the
rasteriser that reads `+0x48..+0x54` is still not located. `Element_UpdateTree`: **85**
(decompiled, and the `ra` hit proves it runs); the crop-vs-stretch render detail is
**corroborated by one live capture**, not read from a draw call. `Element_UpdateFade`:
**85** for the mechanism. **Where `0.1` itself comes from is resolved in the
2026-09-28 section below this one**: a class-wide `Widget_CreateFromElement`
default, confidence 85, not an XML attribute.

## 2026-09-28: where `0.1` comes from - a class-wide default, not an attribute, and `TitleFrame`'s fade is now wired

**Not an XML attribute anywhere.** A full-text search of both expanded XMLs
`TitleFrame` could plausibly read from - `Data\Plugins\PI001\GUI\Skin.xml`
(the front-end root) and `Data\Skins\Default\Skin.xml` (the activated style
skin, `docs/formats/race-setup.md`) - for any `Fade*`/`Transition*` attribute
or `<Variable global="...">` comes back empty in both files (`oag-wad cat
--expand`, `pure-psp-eu.chd`). `TitleFrame`'s own node is exactly
`<Image name="TitleFrame" StartEnabled="false"><Values width="480" x="0"
y="76" height="128" .../></Image>` - no `Transition`, `EnableTransition` or
`DisableTransition` of its own, and nothing wraps it in a container that
authors one either (`Title Screen->Viewport` authors `enabletransition="0.7"`,
but see below - that does not reach `TitleFrame`).

**The generic node-to-widget constructor, found and fully decompiled.**
`FUN_088b73c4` (renamed `Widget_CreateFromElement`, matching the name
Pulse's own independently-RE'd twin already carries -
`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`'s own
`Widget_CreateFromElement`, `0x088920fc`) is the function every front-end
XML element passes through, `Image`/`Text`/`Animation`/`Viewport` alike -
found via `get_xrefs_to` on the `"Transition"`/`"Delay"` string literals,
both landing in this one function. It reads a fixed attribute set including
`Transition`, `EnableTransition` (`"DisableTransition"` at `0x08a4e2ec`,
confirmed by direct memory read) and `Delay`, matching Pulse's own
documented set closely enough that the two are almost certainly the same
generic UI toolkit shared across titles, not independently written twice.
The four relevant lines (`iVar6` is the widget just allocated,
`local_748`/`local_744` are `EnableTransition`/`DisableTransition`,
`local_74c` is `Transition`, `1.1754944e-38` (`FLT_MIN`) is this reader's
own "not authored" sentinel - the same idiom Pulse's own reader uses):

```c
if (local_748 == FLT_MIN) {                    // EnableTransition
    fVar9 = local_74c;                          // falls back to Transition
    if (local_74c == FLT_MIN) { fVar9 = *(float *)(iVar6 + 0x70); }  // inherited
    *(float *)(iVar6 + 0x70) = fVar9;
} else {
    *(float *)(iVar6 + 0x70) = local_748;
}
// symmetric for local_744 (DisableTransition) -> `iVar6 + 0x74`
```

**"Inherited" is not "cascaded from the parent's current state" - it is
whatever `iVar6` (the freshly-allocated widget) already held before this
attribute read ran**, confirmed by direct observation rather than inferred:
`TitleScreen_AssignWordmarkTexture` runs *after* `Widget_CreateFromElement`
for `TitleFrame` and never writes `+0x70`/`+0x74` at all (its own five
writes are `+0xd4`/`+0xdc`/`+0xe0`/`+0xe4`/`+0xe8`, all texture-assignment
fields), so whatever value a live breakpoint there reads is already the
class-wide default the allocator itself supplied.

**Confirmed live, not inferred from the decompile alone.** A breakpoint on
`0x08952694` (`TitleScreen_AssignWordmarkTexture`'s own last write to
`TitleFrame`'s `+0x2c` flags, right after the constructor above has already
run) on a real `pure-psp-eu.chd` boot (PPSSPP v1.20.4, Xvfb `:94`, websocket
debugger, scripted the same `Language Selection -> ... -> Title Screen` path
the 2026-09-26 pass used) reads `TitleFrame+0x70 = TitleFrame+0x74 = 0.1`
exactly, at address `0x08ed6880` - the same address a pass two sessions
earlier already recorded for this same widget on this same boot script, so
the address itself is reproducible, not a heap coincidence. Walking
`Title Screen->Viewport`'s own child list (`+0x10` head, `+0xc` sibling
pointer) at the same breakpoint:

| Widget | vtable | `+0x70` | `+0x74` |
| --- | --- | --- | --- |
| White Background (`Fill`) | `0x8ab9c58` | `0.1` | `0.1` |
| `TitleFrame` | `0x8ab9c58` | `0.1` | `0.1` |
| `PRESS START` (`Text`) | `0x8abb040` | `0.1` | `0.1` |
| `StartCursor` (`Text`) | `0x8abb040` | `0.1` | `0.1` |
| `HOLD ON!` (`Text`) | `0x8abb040` | `0.1` | `0.1` |
| every `TitleAnimN` (`Animation`) | `0x8ab90f8` | `0.0` | `0.0` |
| `Title Screen->Viewport` itself | `0x8abb380` | `0.7` (authored `enabletransition`) | `0.1` |

**`0.1` is a genuine class-wide default, not `TitleFrame`-specific, and not
even `Image`-specific** - three different vtables (`Fill`, `Image`, `Text`)
all land on the identical `0.1`/`0.1` when neither authors a transition
attribute nor sits under a container that does. `Animation` objects
themselves are the one exception, reading `0.0`/`0.0` - `Animation_ConstructFromNode`
overwrites the inherited default rather than never receiving one, so an
`<Animation>`-wrapped widget's own reveal is the `TextureWidth` wipe alone,
confirming the 2026-09-28 (earlier, same-day) finding above that
`Element_UpdateFade` runs on every `Animation`-wrapped widget "unconditionally
at its own top" but produces no visible second fade. **The Viewport's own
`enabletransition="0.7"` does not cascade to its children** - `TitleFrame`
reads `0.1`, not `0.7`, settling that `Widget_CreateFromElement`'s own
"inherited" fallback is a per-widget construction-time default, not a
parent-to-child cascade (unlike `LeftLayer`'s own confirmed cascading
behaviour on Pulse, which is a different, deliberate mechanism - see
`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`).

**Implemented.** `oag_ui::screen::resolve_fade_in`/`MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS`
and `oag_ui::frontend::draw`'s `fade_alpha` reproduce
`Element_UpdateFade`'s entering-branch formula (`elapsed / duration`,
clamped, confirmed linear the same way Pulse's own
`Widget_UpdateTransitionFraction` already is) for `Image` widgets, gated on
`StartEnabled="false"` rather than applied unconditionally - see that
function's own doc comment for exactly why the scope is narrower here than
what the executable itself does. Pinned against the real disc in
`crates/game/tests/pure_boot_ground_truth.rs`'s
`title_screens_own_wordmark_gets_the_measured_texture`, and checked by eye
against `--screen "Title Screen" --screen-seconds 0.03/0.06/0.1` on both
pressings: the wordmark is invisible at `0.0`, faintly visible partway
through, and fully settled at `0.1`, in each pressing's own colourway.

Confidence **85** for `0.1` as the measured class-wide default (three
independent widget types, one reproducible address, matches a second pass's
own earlier capture) - short of 90 only because `Widget_CreateFromElement`'s
own allocator (`FUN_08a3a928`, which resolves a tag name to its class's own
prototype/vtable before this function runs) was read only at the call site,
not independently decompiled, so exactly *where* the `0.1` itself is first
written (the prototype object's own construction, unlocated) stays open.

## Next steps

- **Implemented**: `oag_ui::screen::RevealKey`/`interpolate_reveal` and
  `oag_ui::frontend::draw`'s `reveal_delta` reproduce `Animation_InterpolateKeys`/
  `Animation_ComputeRect`'s width formula for every `<Animation>`-wrapped `Image`/`Fill`,
  pinned against the real disc's own per-widget key data in
  `crates/game/tests/pure_boot_ground_truth.rs`'s
  `title_screens_frame_lines_keep_their_own_rects_and_share_one_colour`.
  **Also implemented, 2026-09-28**: `TitleFrame`'s own `0.1`s alpha fade -
  see the section immediately above. **Not implemented**: the same measured
  default for `Text`/`Fill` widgets that start enabled (`PRESS START` etc.
  above all measure `0.1`/`0.1` too, live) - scoped out on purpose to avoid
  a blanket rendering change across every screen of every title on the
  strength of one Pure EU capture; see `oag_ui::screen::resolve_fade_in`'s
  own doc for the reasoning and the one theoretical gap it leaves open
  (an inherited literal `0` is indistinguishable from true absence).
  `Widget_CreateFromElement`'s own allocator (`FUN_08a3a928`) is unlocated
  past the call site - finding it would show exactly where `0.1` is first
  written into a fresh widget, closing the one open point above.
- The actual **rasteriser** - whatever reads `element+0x48..+0x54` and a widget's own
  `TxtrWidth`/`U`/`V` to produce pixels - is still not located. Finding it would upgrade
  the crop-vs-stretch reading above from "one live capture" to "read off the draw call
  itself", and would settle whether a `Centred="true"` widget (none currently authored
  with a reveal) keeps its own centre fixed or drifts as its width changes.
- `Data.wad` entry 536 (`Data\FE\Images\FMV_last_frame_JAP.mip`) names no
  disc in this project's corpus. Leave unwired rather than guessed at.
- `FUN_088aff90`/`FUN_08a3af94`/`FUN_088b0118` (global-to-string resolution,
  name-based texture load, and whatever the third does with a colour) are
  none of them independently decompiled past the call-site behaviour this
  page infers.
- None of this pass's new names have a confirmed USA-binary twin - only EU was worked.
