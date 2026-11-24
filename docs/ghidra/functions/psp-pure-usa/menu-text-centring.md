# Pure's text layout: where a centred string starts

Where Pure's front end places a centred string, read off the executable and fitted to the seven pause-menu rows measured in PPSSPP. The Pure PSP USA `BOOT.BIN` (Ghidra program `/pure/BOOT-psp-pure-usa.BIN`, image base `0x08804000`). Lane `pure-centre-rounding` (2026-11-24), a follow-up of `pure-row-centring`, which refuted every closed-form rule of the advance sum and asked for the draw code.

## Functions

| Address | Name | Confidence | What |
| --- | --- | --- | --- |
| `0x088d3000` | `Text_Draw` | 75 | The text widget's draw: origin `(int)(x + offset)`, alignment from widget `+0x1f0` (vertical `+0x1f4`), scale from `+0x1dc`/`+0x1e0`, writes the snap flag from widget byte `+0x227`, calls `Text_BuildGlyphQuads` through `0x08947a48`, then leaves the flag at 1 |
| `0x08947ae0` | `Text_BuildGlyphQuads` | 80 | Lays a string out as sprites: the centred start, the per-glyph ceil, the quad edges |
| `0x08947868` | `Text_MeasureString` | 85 | The string's width, used by every alignment; steps each pen up to a whole unit when the flag is set |
| `0x088da5e0` | `Math_Ceilf` | 95 | Byte for byte the libm `ceilf` (the `1.0e30` inexact trick, mantissa mask by exponent); the same code as Pulse `0x0897df24` |
| `0x0894722c` | `Font_FindGlyph` | 80 | Glyph record by codepoint (sorted table, first record for a miss) |
| `0x0894728c` | `Font_GetKerning` | 80 | Kerning of a glyph against the previous one: a `(previous codepoint, s16)` list at record `+0x0e`, ended by `0xffff` |
| `0x08ab459c` | `g_text_snap_pens` | 80 | The flag the measure, the layout and the anchor function read; written only by `Text_Draw` (widget byte `+0x227` while it draws, 1 after) |

## The law

Every constant below is a value in the glyph record (`+0x02` ink width, `+0x0c` advance, `+0x0d` signed bearing) times the widget's scale `s`, in `f32`, in this order. `b0` is the first glyph's bearing.

**Measure** (`Text_MeasureString`, `n` glyphs): `pen = 0 - b0*s`; for each glyph but the last, `pen = pen + kern*s + b*s`, then, with the flag set, `pen = ceil(pen)`, then `pen = (pen - b*s) + adv*s`; the width is `pen + ((b + w) + kern)*s` for the last glyph. So the width starts at the first glyph's ink, steps each pen up to a whole unit as the draw will, and ends on the last glyph's ink, not its advance.

**Layout** (`Text_BuildGlyphQuads`): `pen = x`; centred (alignment 1): `pen = pen - width*0.5` (right, 2: `pen - width`); then `pen = pen - b0*s`. Per glyph: `q = pen + kern*s + b*s`, `q = ceil(q)` with the flag set; the quad's left edge is `q` and its right edge `q + w*s`, `u0 - 0.5`; `pen = (q - b*s) + adv*s`.

So a centred string's first glyph is at `ceil(x - width/2)`, with no other rounding of the start: an odd width at scale 1 is a `.5` tie and rounds up, and at scale 1.2 the width is the sum of per-glyph snapped pens, not the raw advance sum.

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

- **The quad's right edge.** The layout builds it as `q + w*s`, fractional, where ours (`oag_ui::font::snapped_quad`, measured on the Stats and menu faces) uses `floor(w*s)`. The measured law and the code disagree here; not changed by this lane (it would move every Pure pair).
- **The quad's left edge for a glyph with a bearing.** The code reads `ceil(pen + b*s)`; ours uses `ceil(pen)` where `pen` is before the bearing. Identical for bearing 0, which is every glyph of the seven rows and every first glyph of Pure's menu strings; they differ for a string led by a glyph with a bearing (punctuation).
- **The left-aligned first glyph.** The code starts the pen at `x - b0*s` and so the first quad at `ceil(x)`; ours starts at `ceil(x)` and adds the bearing before the ceil. Not touched.
- **Which widget draws a `<Menu>` row.** The pause rows are `<Menu name="Option">` entries; the 7/7 fit says they lay out through `Text_BuildGlyphQuads`, but the widget between the menu and `0x08947a48` (`0x088bd0d0` or `0x088d4838`) was not read.
- **A glyph the face lacks.** The original draws the face's first glyph for a missing codepoint (`Font_FindGlyph`); ours leaves it out.

## Cross-title (CLAUDE.md, "A 2048 finding is checked against Omega")

Pure only. Pulse (PSP, PS2) has the same code but its front end is not snapped (`MenuSkin::snapped_pens` is Pure's), so nothing changes there: a Pulse PSP, a Pulse PS2 and two HD pairs re-render pixel for pixel equal. 2048, Omega: no snapped front end; not applicable.
