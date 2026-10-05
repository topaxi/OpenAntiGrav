# The countdown's two `<Mode3D>` widgets: bound the same way as the sights, updated for fade only

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the placement question is closed with a negative result. Read
[lock-sight.md](lock-sight.md) first - this page follows the same route (the
`"HUD->"` name-lookup idiom, the `+0x08804000` immediate correction) and it
transfers in *method*, not in *result*: the sights get a per-frame position
writer because they track a moving 3D target; the countdown widgets do not,
and no analogous writer exists for them anywhere reachable from their two
known consumers. **`ReadyGo`'s own drawing question is closed too, also with
a negative result** - see "`ReadyGo`'s gate" below: it is confirmed not to be
a digit display (a render off the disc settles that), but its entire visible
envelope is code-supplied and that code's trigger was not found.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0881fbec` | `Hud_BindWidgets` | 80 |
| `0x0881f624` | `Hud_UpdateCountdownFade` | 70 |

## The bind: `"HUD->ReadyGo"` and `"HUD->Cockpit321Go"` resolve exactly like the sights

`Hud_BindWidgets` (`0x0881fbec`) is a much larger relative of `HudSight_Bind`
(`0x0881b604`) - same `"HUD->"` prefix, same `func_0x0008cf30` name-resolver,
same one-call-per-widget shape, but resolving on the order of 40 named
widgets rather than nine. Two of them are this page's subject:

```c
func_0x00142d54(auStack_a50, uVar8, 6);       // "HUD->"
func_0x0016f390(auStack_a50, 0x276060);       // + "ReadyGo"
uVar2 = func_0x0008cf30(*(undefined4 *)(iVar4 + 0x7c), auStack_a50, 0);
*(undefined4 *)(iVar4 + 0x24c) = uVar2;       // hud+0x24c = ReadyGo widget

func_0x00142d54(auStack_9d0, uVar8, 6);       // "HUD->"
func_0x0016f390(auStack_9d0, 0x276068);       // + "Cockpit321Go"
uVar2 = func_0x0008cf30(*(undefined4 *)(iVar4 + 0x7c), auStack_9d0, 0);
*(undefined4 *)(iVar4 + 0x250) = uVar2;       // hud+0x250 = Cockpit321Go widget
```

### The `+0x08804000` correction, verified against real bytes rather than assumed

Every literal string reference in both `HudSight_Bind` and `Hud_BindWidgets`
is a `lui`/`addiu` pair whose immediate is short of the real address by the
image base - the same defect this project's `workflow.md` already documents
for `jal` targets, confirmed here to generalise to `lui`/`addiu` data
references too (not just the `lui`/`lw` pair form mentioned in the standing
note). Read directly off the disassembly of `HudSight_Bind`:

```
0881b618: lui a0,0x27
0881b624: addiu s2,a0,0x5c28      ; raw 0x275c28
0881b64c: lui a1,0x27
0881b658: addiu a1,a1,0x5cd4      ; raw 0x275cd4
```

`inspect_memory_content` at `0x275c28 + 0x08804000 = 0x08a79c28` reads
`"HUD->\0"` verbatim, and at `0x275cd4 + 0x08804000 = 0x08a79cd4` reads
`"missile_sight_1\0"` - both exactly where the correction predicts, not
approximately. The same correction applied to `Hud_BindWidgets`'s own two
immediates, `0x276060` and `0x276068`, lands on `0x08a7a060` (`"ReadyGo"`)
and `0x08a7a068` (`"Cockpit321Go"`), the two strings `search_strings`
independently finds in `.rodata`. Confidence 90 on the correction itself -
two independent addresses, both checked against real memory content, not one
lucky match.

**`get_xrefs_to` on `func_0x0008cf30` (the name-resolver, a real function)
works fine and returns 50+ real `jal` callers**, including every one of
`Hud_BindWidgets`'s ~40 lookups. The relocation defect this project's brief
warns about is specific to *data* references (the `lui`/`addiu` pairs above);
a direct `jal` call target is absolute in the instruction encoding and needs
no relocation to resolve. Worth recording since it is easy to over-generalise
"xrefs don't work on this binary" into "no xref call ever works here."

## The update: fade and visibility only, never position

`Hud_UpdateCountdownFade` (`0x0881f624`) is the only other function that
touches `hud+0x24c` or `hud+0x250` anywhere `search_instructions` finds in
the whole binary (both offsets searched exhaustively; every other hit is an
unrelated stack slot or an unrelated struct at the same offset in a
different function). Full decompile in the plate comment at its address;
summarised:

- A timer at `hud+0x16c` accumulates `dt` (the function's own `param_1`)
  while a gate byte pair (`*(hud->0x3c)+0x58`/`+0x59`, on the same
  largely-unidentified `hud->view` struct [lock-sight.md](lock-sight.md)
  already reads at `+0x48`/`+0xe4`) is set, and is reset to `0` and both
  widgets hidden while it is clear.
- **Below `1.0`, both widgets are shown, but their colour differs, confirmed
  by direct decompile 2026-09-06.** With `t = timer * 2`, `Cockpit321Go`
  always gets flat `0xffffffff` from `func_0x0010e2b4` - the fade timer
  contributes nothing to it, only the hidden bit does. `ReadyGo` gets
  `(alpha << 24) | 0x050517` where `alpha = clamp(0, 255, (1 - t*t*t) * 255)`
  - a cubic ease that starts fully opaque at `timer = 0` and reaches `0` by
    `timer = 0.5`, then stays clamped at `0` (still "shown", just invisible)
    until `timer` passes `1.0`. **This is a fade-*out*, not a fade-in**, and
  it is `ReadyGo`'s entire visible envelope - it has no baked animation of
  its own beyond the continuous 0.983 s material loop below. Confidence 85 -
  the constants and the branch structure are read directly off the
  decompile, not inferred; what dt's real units are (and so what real-time
  span `timer: 0->1` actually spans) is not traced further, since nothing
  needed it to answer the drawing question this page exists for.
- Past `1.0`, or while a second, separate gate (mode `!= 6` and a global
  byte) fails, it sets the hidden bit instead - for both widgets, regardless
  of the fade timer's own state.

### `ReadyGo`'s gate: not a digit-sync problem, but a genuinely unrecovered one

2026-09-06: whether `Pulse_Ready_Go.vex`'s own 0.983 s material loop needs some
sync to the countdown's length before it can be drawn.

**Rendered standalone off the disc** (`oag-view --mesh
'Data\HUD\Pulse_Ready_Go.vex' --screenshot`), the model's one mesh
(`thingieShape`, 285 vertices, radius 16 units) is a **nested ring/swoosh
shape, not digit glyphs** - two open bracket-like loops, the same family
`start-gantry.md` already documents for Pure's own `Ready_GO.vex` ("nested
ring/loop shapes, not digit outlines"). Confidence 90 - a render off the
disc, the same class of evidence this project's `321go_zone.vex` comparison
used. This retires the original worry (that looping every 0.983 s would
"blink it roughly 4.6 times through" the 272-tick countdown) as aimed at the
wrong problem: there are no digit states for a free-running loop to
misalign against. The loop's own authored period is also short relative to
`Cockpit_321GO.vex`'s four tracks (each a one-shot burst dressed in a
600-6000 s nominal loop that never wraps in any real race), which is
consistent with `ReadyGo`'s loop being an intentionally-repeating ambient
shimmer rather than an unfinished digit animation.

**What still blocks drawing it: the fade-out envelope above *is* the
widget's whole visible shape, and its trigger is unrecovered.** Unlike
`Cockpit321Go` (flat colour, fade contributes nothing), `ReadyGo`'s only
source of on-screen alpha is the cubic ease driven by the `hud->view+0x58`/
`+0x59` gate byte pair. No writer of that byte was found this session. One
candidate was checked and probably ruled out: a global at `0x08ab2120`,
cleared to `0` at `+0x58` on local-player ship death (`Ship_SetState` case
4, [zone-mode.md](zone-mode.md#not-determined)) - but that object is
allocated at exactly `0x68` (104) bytes in `InGame_Update`
(`search_instructions` on the `0x1ee0` operand pair that both references
share, five hits total), too small to also be `hud->view`, which needs at
least `0xe8` bytes to hold the `+0xe4` field [lock-sight.md](lock-sight.md)
already reads on it. That argument rests on `lock-sight.md`'s own `+0xe4`
read, itself confidence 50, and nothing here confirms what writes
`hud->0x3c` in the first place (checked `Hud_BindWidgets` directly; it reads
`hud+0x3c` nowhere and writes it nowhere) - so call this **confidence 60,
"probably a different object," not a confirmed dead end** the way
`0x08a7a1e8` is elsewhere in this thread.

**Net result: `ReadyGo`'s gate is not recovered.** Drawing the mesh without
its envelope would show a near-opaque shape for the model's entire visible
window rather than the original's brief eased flash - the kind of
plausible-looking stand-in this project's own rule against inventing a
gate exists to prevent. The widget stays loaded and undrawn.
`crates/game/src/hud_countdown.rs`'s module doc carries the same account.

**No store to a screen-position field appears anywhere in this function.**
Contrast `HudSight_Update` (`0x0881dbcc`, see `lock-sight.md`), which writes
`widget+0x98`/`+0x9c` every frame from a live 3D projection - the direct
reason the sights need, and get, a runtime placement law. The countdown
widgets get the *same shape of per-frame updater* (bound the same way,
updated once a frame by name) and it does nothing but fade them in and hide
them. That asymmetry, not an absence of searching, is the finding: the
sights have a placement writer because a widget doesn't stay near a moving
target on its own; the countdown widgets apparently don't need one, which
only makes sense if their position is supposed to be static - and the only
static position on record is the authored `x="0.0" y="0.0"`.

## The mesh itself doesn't carry the missing coordinate either

Before accepting "no writer" as the end of it, the other place a real
position could hide is the `.vex` geometry: if `Cockpit_321GO.vex`'s four
glyph nodes carried an absolute on-screen rest position baked into their
vertices or their `Anim Transform`'s translation track, `x="0.0"` on the
widget would be correct - the mesh would already put the glyph in the middle
of the screen, and the widget offset would just add nothing extra.

Checked directly, off `crates/render/examples/ready_go_census.rs`'s
groundwork plus `oag_vex::vex::mesh_batches` read for both batch lists
(`0` and `1` - these glyphs are transparent-only, list `0` alone returns
nothing):

- **Every `Anim Transform` node's translation is ~0.** `Jons321go:x3`/`x2`/
  `x1`/`GO` all report `translation base [5.877472e-39, 5.877472e-39, 1.0]`
  with `quantum [1.8e-43, 1.8e-43, 0.0]` and a single keyframe value
  `[-32768.0, -32768.0, 0.0]`. `quantum * value` cancels `base` to within
  float noise for both X and Y (`1.8e-43 * -32768 ≈ -5.9e-39`, against a base
  of `5.877e-39`), so the resolved translation is `(≈0, ≈0, 1.0)` for all
  four nodes - no baked offset, and the four are stacked on each other by
  design, which fits the sequential 3/2/1/GO reveal already measured (one
  glyph scale-bursts up while the others fade to hidden, not four digits
  side by side).
- **Every glyph mesh's vertex bounding box is centred on its own local
  origin.** `Jons321go:xShape3/2/1` each span roughly `±3` model units in X
  and `±1.5` in Y; `GOShape` (two letters, wider) spans `±6.2`-`±6.3` in X
  and `±1.5`-`±1.6` in Y. All symmetric about `(0, 0)`, the same convention
  `lock-sight.md` already documents for the sight brackets ("a single
  textured quad ... centred on the origin").

**Both checks agree: the mesh supplies no absolute screen placement.**
Exactly like the sight quads, this asset is authored to be centred on
wherever its widget puts it - the difference is that the sights get that
placement written every frame by `HudSight_Update`, and the countdown
widgets, per the section above, get no equivalent. Confidence 85 that the
geometry itself carries no answer: two independent measurements (baked
translation, vertex bounds), both symmetric about zero, on data read
directly rather than inferred.

## Conclusion: the placement law does not exist as a runtime write on this title

Between the code search (no position writer reachable from either of the two
functions that touch the widget slots) and the asset search (no baked offset
in the mesh), there is no third place left to look on Pulse's own binary for
where `Cockpit_321GO.vex` is meant to land on screen. **`x="0.0" y="0.0"`,
composed with the enclosing `<Mode3D>` block's `OriginX="0.0" OriginY="35.0"`,
is what the disc actually says, not a placeholder overwritten elsewhere** -
unlike the sights, where an identical-looking placeholder position turned out
to have exactly that reason. Confidence 75 for the negative, scoped
precisely: "no writer reachable from the two `jal`-xref-complete consumers of
`hud+0x24c`/`hud+0x250`, and no baked offset in the mesh's own translation or
vertex data." A third mechanism outside both of those (a different HUD
subsystem entirely, keyed off a widget name this page didn't search for)
cannot be ruled out at 100, but nothing found this session points at one.

## `OriginX`/`OriginY` is a real screen anchor, cross-title corroborated - it just isn't a centred one here

`docs/ui/hud.md` lists `OriginX`/`Y` under "attributes not acted on" and
defers it. It is not meaningless: HD/Fury's front end authors
`OriginX="960" OriginY="540"` on a `<BackgroundAnim>` in a confirmed
1920x1080 space - the exact screen centre - and `OriginX="1220" OriginY="412"`
on `Team Selection`'s `ShipModel`, an intentionally off-centre camera anchor
for a preview turntable (see `docs/formats/race-setup.md` and
`docs/formats/hd-frontend.md`). Both read as "where this `<Mode3D>` block's
local origin lands on the real screen," which is exactly what
`crates/game/src/hud_countdown.rs`'s own `ORIGIN_Y` constant already assumes.
That raises the *semantics* of the attribute to confidence 80 - it does not
change the conclusion above, since Pulse's own value for this block,
`(0, 35)`, is simply not a centred one.

## HD/Fury's copy of this widget is disabled, not repositioned - a dead end, not a second data point

`data/xml/hud_ready_go.xml` on `hdfury-ps3-eu-dec.iso`'s `DATA02.PSARC`
reads:

```xml
<aMode3D>
  <Values FirstPass="yes" OriginX="0.0" OriginY="140.0"></Values>
  <aModel name="ReadyGo" Enabletransition="0" Delay="0">
    <Values Src="Data\HUD\Pulse_Ready_Go.vex" x="0.0" y="0.0" z="-70.0" ztest="0"></Values>
  </aModel>
</aMode3D>
```

The `<aMode3D>`/`<aModel>` tag prefix is the same "authored to be skipped
without deleting the reference" convention `docs/formats/hd-hud.md` already
documents for `<aLoadXML>` in `DATA05`'s `speedlap_hud.xml` - `LoadXML_Item`'s
reader keys on the literal tag name, so a renamed tag is never looked for at
all. **HD disabled this exact widget rather than moving it** (consistent with
the thread's separate finding that HD's countdown display lives on the
track-side gantry instead), so its stale `OriginY="140.0"` - neither Pulse's
`35` nor HD's own `1920x1080` screen centre - is dead data from whenever the
widget was last live, not a second measurement of where the widget belongs.
No Cockpit321Go sibling appears in this fragment at all, matching
`docs/ui/hud.md`'s existing note. Confidence 90 - the tag rename is read
directly off the extracted XML, the same mechanism already confirmed
elsewhere on this disc.

## 2026-10-04: the `Cockpit321Go` clock rides the gantry's writes

`Hud_SetCountdownWidgetTime` (`0x0881a3b8`, 75) sets the animation time of
`*(hud+0x250)+0x94`, this page's `Cockpit321Go` widget, and is called with the
gantry's own set-time each time: 0 at `ready` (`RaceMode_UpdateIntro_q`), 3.0 on
the release (`RaceMode_UpdateCountdown`) and 3.2 whenever the race manager resets
the gantry's pre-lap window (`RaceManager_Update`, `n == 0` branch only). So
`Cockpit_321GO.vex`'s clock is the gantry's until the first line crossing and runs
free after it. Recorded, not rewired: the overlay draws only where a circuit has no
gantry, and it still runs off `world.tick / 60`. Evidence:
[gantry-clock.md](gantry-clock.md#5-the-cockpit-countdown-widget-rides-the-same-writes).
