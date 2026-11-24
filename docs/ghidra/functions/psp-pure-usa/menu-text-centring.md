# Pure's text layout: where a centred string starts

Where Pure's front end places a centred string, read off the executable and fitted to the seven pause-menu rows measured in PPSSPP. The Pure PSP USA `BOOT.BIN` (Ghidra program `/pure/BOOT-psp-pure-usa.BIN`, image base `0x08804000`). Lanes `pure-centre-rounding` (2026-11-24), a follow-up of `pure-row-centring`, which refuted every closed-form rule of the advance sum and asked for the draw code.

## Functions

Data addresses: this import leaves code immediates unrelocated and rebases the data listing, so Ghidra's decompile shows the flag as `DAT_002b059c` and the layout's state (x, y, alignment, scales, face) as `_DAT_0009a570` to `_DAT_0009a588`; the data sits at the immediate plus `0x08804000` (`0x08ab459c` for the flag, where the cross-references resolve; `docs/ghidra/functions/psp-pure-eu/string-anchors.md`). Scores follow `docs/reverse-engineering/confidence-rubric.md`: decompile and disassembly only, no runtime trace, so the layout and the widget stay under 85; `Math_Ceilf` (the same code as Pulse's libm routine, a second binary) and the measure (the fit below needs it exactly) are 90 and 85.

| Address | Name | Confidence | What |
| --- | --- | --- | --- |
| `0x088d3000` | `Text_Draw` | 75 | The text widget's draw: origin `(int)(x + offset)`, alignment from widget `+0x1f0` (vertical `+0x1f4`), scale from `+0x1dc`/`+0x1e0`, writes the snap flag from widget byte `+0x227`, calls `Text_BuildGlyphQuads` through `0x08947a48`, then leaves the flag at 1 |
| `0x08947ae0` | `Text_BuildGlyphQuads` | 80 | Lays a string out as sprites: the centred start, the per-glyph ceil, the quad edges |
| `0x08947868` | `Text_MeasureString` | 85 | The string's width, used by every alignment; steps each pen up to a whole unit when the flag is set |
| `0x088da5e0` | `Math_Ceilf` | 90 | Byte for byte the libm `ceilf` (the `1.0e30` inexact trick, mantissa mask by exponent); the same code as Pulse `0x0897df24` |
| `0x0894722c` | `Font_FindGlyph` | 80 | Glyph record by codepoint (sorted table, first record for a miss) |
| `0x0894728c` | `Font_GetKerning` | 80 | Kerning of a glyph against the previous one: a `(previous codepoint, s16)` list at record `+0x0e`, ended by `0xffff` |
| `0x08ab459c` | `g_text_snap_pens` | 80 | The flag the measure, the layout and the anchor function read; written only by `Text_Draw` (widget byte `+0x227` while it draws, 1 after) |

## The law

Every constant below is a value in the glyph record (`+0x02` ink width, `+0x0c` advance, `+0x0d` signed bearing) times the widget's scale `s`, in `f32`, in this order. `b0` is the first glyph's bearing.

**Measure** (`Text_MeasureString`, `n` glyphs): `pen = 0 - b0*s`; for each glyph but the last, `pen = pen + kern*s + b*s`, then, with the flag set, `pen = ceil(pen)`, then `pen = (pen - b*s) + adv*s`; the width is `pen + ((b + w) + kern)*s` for the last glyph. So the width starts at the first glyph's ink, steps each pen up to a whole unit as the draw will, and ends on the last glyph's ink, not its advance.

**Layout** (`Text_BuildGlyphQuads`): `pen = x`; centred (alignment 1): `pen = pen - width*0.5` (right, 2: `pen - width`); then `pen = pen - b0*s`. Per glyph: `q = pen + kern*s + b*s`, `q = ceil(q)` with the flag set; the quad's left edge is `q` and its right edge `q + w*s`, `u0 - 0.5`; `pen = (q - b*s) + adv*s`.

So a centred string's first glyph is at `ceil(x - width/2)`, with no other rounding of the start: an odd width at scale 1 is a `.5` tie and rounds up, and at scale 1.2 the width is the sum of per-glyph snapped pens, not the raw advance sum.

## The quad (confidence 80)

The per-glyph sprite is two vertices, built in `Text_BuildGlyphQuads` (`0x08947f88` to `0x08948054`, the face's border pad zero; the padded branch at `0x08947e88` grows the quad and the uv by the pad on every side and draws twice). With `q` the ceiled left edge above, `y` the layout's top (`local_5c`: the origin less the widget's `+0xac` offset and any vertical alignment, ceiled when the flag is set) and `w`, `h` the record's `+0x02`, `+0x03` bytes:

- corner 0: position `(q, y)`, texture `(u0 - 0.5, v0 - 0.5)`;
- corner 1: position `(q + w*s, y + h*s)`, texture `(u1 + 0.5, v1 + 0.5)`;

`u0`, `u1`, `v0`, `v1` are the record's `+0x04`, `+0x06`, `+0x08`, `+0x0a` halfwords (`u1 - u0 = w`: the right and bottom edges are exclusive, so the box is `w` by `h` texels). The two positions go to the GE as floats through a sprite primitive; the rasteriser keeps whole pixels of the quad (inferred from the fit, not read: ours: `floor(w*s)` by `floor(h*s)`, measured on the Stats and menu faces, `docs/ui/menus-original.md`, "Pure's glyph quads are whole pixels") and computes the texture slope from the **fractional** span. So the sprite samples `w + 1` by `h + 1` texels, starting half a texel early on **both** axes, spread over `w*s` by `h*s` pixels and cut to whole pixels: the span of the quad's own pixels is `(w + 1) * floor(w*s) / (w*s)` texels in x and the same in y (`oag_ui::font::snapped_quad`, `u_span`, `v_span`).

Evidence: a texture span of exactly `w` (the previous wiring) is the only part of this that differed from the code. Rendered from `PRESS START BUTTON` (`FX300ANG.fnt`, scale 1.5, `w` 12, `h` 15: 18 by 22 pixels from 18 by 22.5) the span turns the title pair's share of pixels more than 40 grey apart from 0.32 % to 0.01 % (mean grey 0.53 to 0.08), the text's box (x 100 to 420, y 198 to 220) from 5.91 % to 0.00 %. All ten Pure menu pairs improve (whole frame, mean grey: main 0.18 to 0.13 %, 0.55 to 0.27; mode 0.22 to 0.15, 0.58 to 0.27; pause 2.23 to 1.80, 5.52 to 5.09; sr-class 0.14 to 0.08, 0.42 to 0.18; sr-league 0.14 to 0.08, 0.38 to 0.17; sr-track 0.25 to 0.18, 0.73 to 0.33; t-class 0.13 to 0.07, 0.41 to 0.17; t-sel 0.16 to 0.10, 0.45 to 0.20; team 0.18 to 0.12, 0.65 to 0.26). Variants tried on the same pairs: `w + 1` texels over the whole-pixel width (title 0.01 %, but sr-track 0.25 % to 0.49 %: the Stats face at 0.8 has a 0.6 pixel fraction), `h + 1` in y alone (title 0.28 %), a shift without the wider span (title 0.17 % to 0.11 %, every other pair worse). Confidence **80**: the decompile and the disassembly read, ten of ten pairs closer to the original; no live GE list or vertex read.

Pure's menu skin names no `borderExtendPixels` for any face, so the unpadded branch is the one read and the one wired; a padded face would be the other branch.

## Evidence (confidence 80)

1. **The seven measured rows fit, with no free parameter.** `FX300ANG.fnt` (the `<Menu>` face) at x 240, first quad column against the original's, in float32 (`crates/ui/tests/pure_centre_ground_truth.rs`):

   | row | scale | width | `240 - width/2` | first column | measured |
   | --- | --- | --- | --- | --- | --- |
   | Continue | 1.2 | 107.8 | 186.1 | 187 | 187 |
   | Restart | 1.2 | 103.8 | 188.1 | 189 | 189 |
   | Quit | 1.2 | 50.0 | 215.0 | 215 | 215 |
   | Continue | 1.0 | 87 | 196.5 | 197 | 197 |
   | Restart | 1.0 | 84 | 198 | 198 | 198 |
   | End Session | 1.0 | 118 | 181 | 181 | 181 |
   | Quit | 1.0 | 41 | 219.5 | 220 | 220 |

   The two `.5` ties (Continue and Quit at 1.0) round up, the two 1.2 rows ours put 1 px left now agree, and Quit at 1.2 is unchanged. The `pure-row-centring` search over 400 closed forms of the advance sum found none that fits seven; this one comes from the code and has none to choose.
2. **Kerning is zero.** Every one of `FX300ANG.fnt`'s 202 records is 18 bytes with `ff ff ff ff` at `+0x0e`: the pair list is empty, so `Font_GetKerning` returns 0 on every pair of the seven rows.
3. **The call chain.** `Text_BuildGlyphQuads` (`0x08947ae0`) has two callers: `0x08947a48` (a wrapper that resolves the string) and `0x088d15a4` (a single-character selector). `0x08947a48` is called from `Text_Draw` (`0x088d3000`) and from `0x088bd0d0` and `0x088d4838`, not read. `Text_Draw` is the only writer of the flag (`0x088d344c`, `0x088d3478`, `0x088d3490`, `0x088d34c4`, `0x088d34d0`) and leaves it 1, so any other layout caller runs snapped. `0x089481a0` (it returns the first glyph's ink box) reads the same flag and does the same centring.
4. **Pulse corroborates the structure** (`0x08919cf8` `Text_BuildGlyphQuads`, `0x089199b0` its measure, `0x0897df24` ceilf, flag `0x08abf584` set from widget byte `+0x173` bit 1 by `Text_Draw` `0x088ce668`): the same centring and the same per-glyph ceil. Cited as corroboration; no Pulse row is added here.
5. **The footer is unchanged.** The pause's footer `Press X to Confirm` is a `<Text>` centred at x 240 and drawn by the same function; ours renders it pixel for pixel the same before and after the change, and before the change it already started where the original does (column 144, `docs/tools/compare-page.md`, "The band's words and the HUD top, measured again").

What this is not: a live breakpoint. The addresses are read from the decompile and the disassembly of `0x088da5e0`; no PPSSPP watch was set on `0x08947ae0` or on the flag. The fit of seven frames with no free parameter is the confirmation, and it is why the confidence is 80 and not 90.

## Open

- ~~**The quad's right edge.**~~ Closed: the code builds the corner fractional and the rasteriser keeps `floor(w*s)` pixels of it; what was missing was the texture span (above, "The quad").
- **The quad's left edge for a glyph with a bearing.** The code reads `ceil(pen + b*s)`; ours uses `ceil(pen)` where `pen` is before the bearing. Identical for bearing 0, which is every glyph of the seven rows and every first glyph of Pure's menu strings; they differ for a string led by a glyph with a bearing (punctuation).
- **The left-aligned first glyph.** The code starts the pen at `x - b0*s` and so the first quad at `ceil(x)`; ours starts at `ceil(x)` and adds the bearing before the ceil. Not touched.
- **Which widget draws a `<Menu>` row.** The pause rows are `<Menu name="Option">` entries; the 7/7 fit says they lay out through `Text_BuildGlyphQuads`, but the widget between the menu and `0x08947a48` (`0x088bd0d0` or `0x088d4838`) was not read.
- **A glyph the face lacks.** The original draws the face's first glyph for a missing codepoint (`Font_FindGlyph`); ours leaves it out.

## Cross-title (CLAUDE.md, "A 2048 finding is checked against Omega")

Pure only. Pulse (PSP, PS2) has the same code but its front end is not snapped (`MenuSkin::snapped_pens` is Pure's), so nothing changes there: a Pulse PSP, a Pulse PS2 and two HD pairs re-render pixel for pixel equal. 2048, Omega: no snapped front end; not applicable.
