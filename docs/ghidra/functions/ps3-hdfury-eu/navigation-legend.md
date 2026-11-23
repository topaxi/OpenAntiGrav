# The footer legend: how HD lays out its button prompts

Wipeout HD / Fury PS3 EU (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), 2026-11-23, lane `hd-legend-re`. Static reading
through the Ghidra bridge, checked against the `hd/menu-cell-select` pair (RPCS3, 1920x1080, cold boot, Safe Area
96 % read off the save file, captured 2026-11-19). Omega's side is
[../ps4-omega-eu/navigation-legend.md](../ps4-omega-eu/navigation-legend.md).

The footer row on every HD menu (`ΔΓΒΑ NAVIGATION  ⓧ CONFIRM  Ⓞ BACK  □ AI DIFFICULTY (NOVICE)`) is not drawn at the
x that `skin.xml` authors. `skin.xml`'s `<NavigationController>` (`BodgeScreenContainingNavigationController`) gives
`ControlTextConfirmButton` x 490, `ControlTextConfirm` 534, `ControlTextBackButton` 720, `ControlTextBack` 764, all
`scale="0.8"`, the glyphs in `font="buttons"` (`PS_BUTTONS.fnt`), the words in `Default` (`helv.fnt`). The
`NavigationController` widget **overwrites every one of those x** each update with a flow: each prompt starts 20
units after the previous word's measured ink, and only `ControlTextNavigation`'s own x (260) survives as the anchor.

## The law

All values are authored 1920x1080 units; `w(t)` is the `Text` widget's width virtual (below).

1. **Start**, in `NavigationController_Init` (`0x001da048`), once per controller:
   `cursor = (int)(nav.x + 20.0 + w(nav))`, where `nav` is `ControlTextNavigation` (x at `Text + 0xb0`) and 20.0 is
   the TOC float at `0x008ae554` (`0x41a00000`, `lfs f0,0x107c(r2)` at `0x001da258`; `fadds`, then `fctiwz` truncates
   at `0x001da288`). Stored at `controller + 0xd0`.
2. **Each prompt**, in `NavigationController_Layout` (`0x001dab20`), for a pair (glyph Text `g`, word Text `t`):
   - `g.x = cursor`
   - `t.x = cursor + 44`, or `cursor + 84` when `strcmp("ther1button", g.font) == 0`. The branch-free form is
     `((((s>>31) - ((s>>31) ^ s)) >> 31) & 0xffffffd8) + 0x54`: `-40 + 84 = 44` for any nonzero `strcmp` result, 84
     for zero. `g + 0x78` is the font name; the footer's glyphs are `font="buttons"`, so they take **44**. No path in
     the seven archives' file lists contains `ther1`, and neither `skin.xml` nor `cellmode_definition.xml` names it;
     the other XMLs were not searched.
   - `cursor = (int)w(t) + t.x + 20` (`+ 0x14`).
3. **Order**: Confirm when the controller's flag word `+0xc0` has bit 0, Back when it has bit 1, then up to **five**
   extra pairs a screen registered, then Invite (`ControlTextInvite`/`ControlTextInviteButton`, held in the shared
   table) last. The pairs live in a shared table at `0x009a7fe8`: `+0x00` the count, `+0x04` five `(glyph, word)`
   pointer pairs, `+0x2c` the Invite word, `+0x30` the cursor, `+0x34` the Invite glyph.
4. **Registering an extra prompt**: `NavigationController_AddPrompt` (`0x001d9ee0`, `(glyph, word)`, refuses when the
   count is already above 4 or either pointer is null). **Cell Selection's Difficulty prompt flows**:
   `CellSelection_OnEnter` (`0x002197d0`) looks up `DifficultyButtonIcon` and `DifficultyButton` (TOC `+0x2b8c`/`+0x2b90`
   = `0x00793008`/`0x00793020`) and passes them to `0x001d9ee0`. Their authored x (900, 944 in
   `cellmode_definition.xml`) is overwritten too. Four other screens call it (`0x001a7588`, `0x001f0b50`, `0x002587a0`,
   `0x0069dc98`; not read).
5. `NavigationController_Update` (`0x001da6e8`) re-runs the same placement for the extras and Invite when its bit 2 is
   set, and slides `+0xb4`/`+0xb8` (the Confirm and Back groups' y) by `dt * 0x008ae570` when their bits toggle.
   `NavigationController_PlacePrompt` (`0x001d9f30`) is the same placement for one pair; no direct caller.

Confidence **88** for steps 1 and 2: unambiguous decompile and disassembly, and the prediction below lands within
1 px on all seven prompts of the pair. 85 for 3 and 4 (the Cell call site read, the other four callers not). 70 for 5.

## What measures a word: `Text_GetWidth` and `Font_MeasureString`

`Text`'s vtable is `0x00867e58` (stored by `Text_Construct` `0x00201118`, the `Text_Item.cpp` constructor); slot
`+0x5c` is the descriptor `0x0087db68` -> **`Text_GetWidth` (`0x00202748`)**. It loads the font by id (`Text + 0xe0`),
publishes the Text's x scale (`+0xb8`, 1.0 by default, `scale="0.8"` here) and y scale (`+0xbc`) into the font
context at `0x00ae9080`, resolves the string (idstring through the string table, or the literal) and returns
`Font_MeasureText(s)` (`0x002cd268`: UTF-8 to UTF-16, then `Font_MeasureString` `0x002cd048`). No safe area, no
`onlyScaleYSD` term enters the width.

`Font_MeasureString(glyphs, n)` with scale `S` (context `+4`), per glyph record `r` (`Font_FindGlyph`, `0x002ac950`,
a binary search of the codepoint table; record `+0x02` width, `+0x0c` advance, `+0x0d` signed left bearing, `+0x0e`
a kerning list `(previous codepoint u16, amount i16)` ending `0xffff`, read by `Font_GetKerning` `0x002ac9f8`):

```text
x = -bearing(r0) * S                      ; fneg at 0x002cd0d8
for k in 0 .. n-2:
    p = f32((kern(rk, prev) + bearing(rk)) * S + x)   ; fmadds 0x002cd180
    if snap: p = ceilf(p)                 ; bl 0x006785d8 -> ceilf 0x00420ed0
    x = f32((advance(rk) - bearing(rk)) * S + p)      ; fmadds 0x002cd118 / 0x002cd1b8
return (kern + bearing(last) + width(last)) * S + x
```

So the width is the ink extent from the first glyph's left edge to the last glyph's right edge, with each glyph's
pen **rounded up to a whole unit** when `snap` is set. `snap` is the byte at `0x008c267c` (TOC slot `0x008b3bec`).
Text's render (`0x00203208`) writes it from the Text's `+0x110` bit 30 before drawing and **sets it to 1 after every
draw**; `Text_Construct` sets bit 30 (`param_1[0x44] |= 0x40000000`). So every measure made after any text has drawn,
which includes every legend layout, takes the `ceilf` path. HD's `helv.fnt` glyphs used here carry no kerning pairs
(every `+0x0e` is `0xffff`). Confidence **85** for the measure, 75 for the snap flag's lifetime (the writes are read,
the boot order before the first draw is not).

**The draw agrees.** `Font_DrawString` (`0x002cfb28`, reached through `0x002d0f18`) subtracts the first visible glyph's
bearing before its loop (`local_2bc = -(bearing * S - x)`), so a string's first ink column is its x. That is why the
table below compares each x straight against the frame's first ink column. Confidence 72 (the rest of the draw not read).

**The snap is in the measure only, not the draw.** `Font_DrawString` makes no call to `ceilf` (a scan of its 1,275
instructions for `bl 0x006785d8` finds none) and advances its pen unrounded. The frame agrees: `NAVIGATION`'s ink is
150 px wide (288 to 437), the unsnapped 156.0 x 0.96 = 149.8, not the snapped 159.8 x 0.96 = 153.4; `CONFIRM`'s is 115
(502 to 616) against 115.2 unsnapped and 118.5 snapped. So a word is drawn at its natural width and the layout's gap
after it is `20 + (snapped - unsnapped width)`: (20 + 3.8) x 0.96 = 22.8 px after `NAVIGATION`, (20 + 3.4) x 0.96 =
22.5 after `CONFIRM`, which is the "about 24 px" a pixel-only reading saw. A port snaps `w()` for the layout and
draws glyphs unsnapped.

## Prediction for `hd/menu-cell-select`

Metrics from `DATA02`'s `/data/fe/fonts/helv.fnt` and `ps_buttons.fnt` (big-endian `.fnt`, [fnt.md](../../../formats/fnt.md)),
emulated in f32 with the `ceilf` path. Screen x is `0.96 * x + 38.4`: the 96 % Safe Area scaling about the screen centre
(below). Original x is the first column with luminance above 90 in rows 975 to 1010 of `theirs.png`.

| Prompt | Font metrics used | Layout x (authored) | Predicted screen x | Original x | Ours today |
| --- | --- | ---: | ---: | ---: | ---: |
| `NAVIGATION` (anchor) | `w` = 159.8 | 260 | 288.0 | 288 | |
| Cross glyph | | 439 | 459.8 | 460 | 511 |
| `CONFIRM` | `w` = 123.4 | 483 | 502.1 | 502 | |
| Circle glyph | | 626 | 639.4 | 639 | 732 |
| `BACK` | `w` = 73.6 | 670 | 681.6 | 682 | |
| Square glyph | | 763 | 770.9 | 771 | 904 |
| `AI DIFFICULTY (NOVICE)` | | 807 | 813.1 | 814 | |

Every prompt within 1 px. Without `ceilf` the same table drifts 3 to 8 px left by the fourth prompt (CONFIRM 499,
BACK 676, AI 806 against 502, 682, 814), so the snap is what closes it. Word widths for reference: `NAVIGATION` 156.0
unsnapped, `CONFIRM` 120.0, `BACK` 72.8. Glyph-to-word offset in the frame: 42 to 43 px (44 x 0.96 = 42.2). The
1 px residuals are an antialiased edge against the threshold.

Reproduce: extract the two fonts and the pair's `theirs.png`, read each glyph record (18 bytes, [fnt.md](../../../formats/fnt.md),
"Layout"), and run the law above with `S = 0.8`, starting at `cursor = (int)(260 + 20 + w("NAVIGATION"))`.

## The Safe Area mapping

The pair's own Safe Area is 96 % (`pair.json`, `original.safe_area`, from the save file). The original draws the
whole 1920x1080 layout scaled 0.96 about the centre: `screen = 0.96 * x + 38.4` puts the authored `ControlTextNavigationButton`
(x 160) at 192.0 and `ControlTextNavigation` (x 260) at 288.0, the frame's first ink columns exactly, and the rendered
word `NAVIGATION` is 150 px wide for an authored 156 (0.96). This replaces, for this pair, the older reading of a
0.904 scale with offset (+150, -8) noted in the HD front-end thread. Confidence 80: fit across every prompt, the
transform's code not read.

## The wordmark (`flyerlogo`)

`cellmode_definition.xml` authors two `flyerlogo` Images at x 188, y 800 inside a `Viewport` at X 178, Y 708 (420x168),
one in the Grid unlock box and one on Cell Selection. The Image's x and y are screen-absolute; the Viewport only
clips (the sibling `Text`s author `OffsetX 189` beside it). Measured on the committed pairs, ours already draws it
where the original does: a shift search over a 280x80 window around the logo finds the best match at **(0, 0)** on
both `hd/menu-cell-select` (mean difference 0.86) and `hd/menu-grid-select` (4.39). The original's ink starts at x 271
(Cell, `impact`) and 221 (Grid, `uplift`); the authored x maps to 218.9 and the image's transparent left margin
makes up the rest. The "6 pixels right" note in the HD front-end thread predates the 2026-11-19 re-render of both
pairs: it was written in `1d8e7a954` (2026-11-18 21:14), and both committed `ours.png` files were rendered on
2026-11-19 (mtimes 04:13 Cell, 16:47 Grid). A fresh `--menu-page` still from current `main` would confirm it on today's
code. The Image draw is `Image_Render` (`0x001b6398`, [image-widget.md](image-widget.md)); how a `Viewport` parent
reaches a child's position is not read. Read recipe: `Text_Render` (`0x00203208`) adds the widget's `+0x4c`/`+0x50` to
its x/y; find what writes `+0x4c` on a `Viewport`'s children (the original's pixels rule out adding the Viewport's X:
188 + 178 would put the ink near x 390, not 219 to 271). Confidence 75.

## Omega

**Checked, differs.** Omega's `skin.xml` (`data09.psarc`) authors the same `NavigationController` x and scale, and its
seed is the same (`nav.x + 20.0 + w(nav)`), but its layout is a different law: the word sits at the glyph's measured
width plus 14, not a fixed 44, and there is a right-to-left variant. See
[../ps4-omega-eu/navigation-legend.md](../ps4-omega-eu/navigation-legend.md).

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x001da048` | `NavigationController_Init` | 85 |
| `0x001dab20` | `NavigationController_Layout` | 88 |
| `0x001da6e8` | `NavigationController_Update` | 70 |
| `0x001da470` | `NavigationController_Construct` | 80 |
| `0x001d9ee0` | `NavigationController_AddPrompt` | 85 |
| `0x001d9f30` | `NavigationController_PlacePrompt` | 72 |
| `0x009a7fe8` | `g_NavigationPrompts` (data) | 80 |
| `0x00201118` | `Text_Construct` | 80 |
| `0x00202748` | `Text_GetWidth` | 85 |
| `0x002003d0` | `Text_ShrinkToWidth` | 65 |
| `0x00203208` | `Text_Render` | 70 |
| `0x002cd268` | `Font_MeasureText` | 80 |
| `0x002cd048` | `Font_MeasureString` | 85 |
| `0x002ac950` | `Font_FindGlyph` | 80 |
| `0x002ac9f8` | `Font_GetKerning` | 75 |
| `0x002cfb28` | `Font_DrawString` | 72 |
| `0x008c267c` | `g_FontPenSnap` (data) | 75 |

`Text_ShrinkToWidth` (`0x002003d0`, `(text, max)`): when `w(text) > max` it scales `+0xb8` by `max / w`. Below 70,
so `_q` in Ghidra.

## Still open

- The four other `AddPrompt` callers (`0x001a7588`, `0x001f0b50`, `0x002587a0`, `0x0069dc98`): which screens add which
  prompt. Read recipe: decompile each and resolve its two TOC string slots, as for Cell Selection above.
- What sets the controller's `+0xc0` bits per screen (which screens show Confirm and Back): the writes to `+0xc0`.
- The Safe Area transform's code (the 0.96 is fitted, not read).
