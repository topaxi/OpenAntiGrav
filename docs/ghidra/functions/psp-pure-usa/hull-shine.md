# Pure's hull extra pass: Pulse's environment map, weighted by the hull texture's alpha

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-10-26 (lane `pure-render-fidelity-2`): decompiles,
plus four recorded GE lists of the original (PPSSPP v1.20.4, software renderer,
`scripts/psp-ge-dump.py`) taken on the four Pure compare pairs' own save states, and
replays of edited copies of one of them. Companion to Pulse's
[mesh-draw.md](../psp-pulse-usa/mesh-draw.md), "The hull's extra pass", which this page
follows step by step.

## What ours drew

Feisar's canopy read near white in the original and dark teal in ours. Pure's hull authors the
`0x2000` extra pass: `feisarShape`'s mesh word and all three batches read `pass_mask 0x3001`,
and each material's `+0x08` names texture 1, `Data\Tex\envtest4.tga` (64x64, CLUT8). Every Pure
hull does (the loader line reads 5 to 10 batches per team). An earlier pass concluded "no
`0x2000` batch" because `oag_pure::effects::LOOKS.hull_shine` was scoped to the PSP and the
roster asked `applies_everywhere()`, which a scoped rule never passes. The roster now asks
`applies(platform)`, as it does for `hull_wreck`.

## The code is Pulse's, call for call

| Address | Name | Pulse twin | Confidence | Evidence |
| --- | --- | --- | ---: | --- |
| `0x088f0234` | `Gu_TexMapMode` | `0x08811508` | 88 | The only `lui 0xc100` in the binary; the same 21 instructions writing `0xc0`/`0xc1` and the shadow at `ctx+0xf4` |
| `0x088f08dc` | `Gu_BlendFunc` | `0x0881197c` | 85 | 26 callers, every one passing `(op, src, dst, fixa, fixb)` with GE factor values; `FUN_0886e1c8` and `FUN_08866ecc` below program through it exactly what Pulse's twins do |
| `0x0886e144` | `Mesh_BeginTransparentPass` | `0x0890d904` | 85 | `Gu_Disable(10)`, the specular coefficient, `Gu_TexMapMode(2, 0, 1)`, `Gu_Enable(4)`, the model's material list, `Gu_AlphaFunc(1, 0, 0xff)`, `Gu_Disable(0)`, `Gu_Disable(0x11)`: Pulse's sequence. Its caller `FUN_0886de30` is the three-line bracket (begin, `FUN_0886e374` over the `0x2000` batches, restore) |
| `0x0886e1c8` | `Mesh_SetBatchDrawState` | `0x0890d994` | 85 | Per extra-pass batch: list A or B light list by `pass_mask & 0x8000`, cull by `& 0x20`, then `Gu_BlendFunc(ADD, FIX, FIX, white, header_flags & 0x10 ? black : white)` |
| `0x08866ecc` | `Gfx_BuildBatchStateList` | `0x0891f890` | 85 | The same nest over `pass_mask & 0x700/0x800/0x80` and `header_flags & 0x10/0x20`; inside `header_flags & 0x10`, `Gu_BlendFunc(ADD, FIX, header_flags & 0x02 ? SRC_ALPHA : FIX, white)`, exactly as Pulse's |

The pass's fixed light pair is built from the same literals: `-1.0` and `0.3` sit together at
`0x08a8e8ec`, read once, by `FUN_08864018` (Pure's model loader), which builds `Rx(-1.0)` and
`Ry(0.3)` through library rotation builders (a radian polynomial: `cos` at `0x08a2eae4` is
`1 - 0.49999 x^2 + ...`) and multiplies them through `FUN_088634b0`, whose `vmmul.q
E500, E600, E700` takes the `Ry` operand where Pulse's inline `vmmul.q E000, E100, E200` does.
Confidence **85**, carried from Pulse's operand-order derivation; the GE lists below measure the
result.

## Measured: what the GE is given

On every pair's grid, Feisar's three `feisarShape` batches draw twice. Registers at the PRIM,
from the recorded lists:

| | Extra pass (`TEXMAPMODE` 2) | Ordinary pass |
| --- | --- | --- |
| Lighting | off | on |
| Lights 0/1 position | `(0.9553, -0.2484, -0.1595)`, `(0.0, 0.5404, -0.8412)` on all four circuits | the circuit's directionals |
| Material ambient | `0xffffff` | `0xffffff` |
| `BLENDMODE` | `0xaa` (`FIX`, `FIX`), `FIXA` white, `FIXB` black: replace | **`0x2a`** (`FIX`, `SRC_ALPHA`), `FIXA` white |
| Depth | test function 6, writes on | function 2 (equal), writes off |
| Fog colour | `0` | the circuit's |
| Texture | `envtest4`, CLUT8, swizzled | the hull texture, CLUT4 RGBA8888, alpha 10 to 191 |
| `TEXFUNC` | `0` | `0` |

The lights are `oag_render::texgen::ENV_BASIS_0`/`ENV_BASIS_1` to four places, so the
coordinates are Pulse's. `envtest4`'s texels in the list equal our decode of the file. What
differs from Pulse is one word: Pulse's ordinary pass reads `0xaa` with `FIXB` white
(`src + dst`), Pure's reads `0x2a`, destination times **source alpha**.

## Measured: the weight is the hull texture's alpha

The Blue Ridge list replayed in PPSSPP reproduces the pair's original frame (MAE 7.7 grey, the
difference the HUD clock and the beam). Edited copies, replayed the same way:

1. `FIXA` 0 on the ordinary passes (the frame shows only what the extra pass left, as weighted):
   the hull shows its panel lines, grille and stripes in grey. The environment map has none of
   them, so the weight is texture-shaped.
2. The same, with the CLUT bound at those passes made opaque: the panel detail goes, a smooth
   radial highlight is left.
3. The CLUT made opaque alone, or `BLENDMODE 0x2a` swapped for Pulse's `0xaa`: the full frame
   turns into the broad pale wash ours drew with the pass unweighted.

So the pixel is `lit * texture + env * texture.a`. The hull textures are named `*_shinemap`, and
their alpha is the mask. The bit is `header_flags & 0x02`, read by both executables in the same
branch (`oag_vex::vex::Batch::weights_extra_pass_by_alpha`). Confidence **90**: the decompile,
the recorded state and the three replays agree; `TEXFUNC` reads `0` (`TCC` RGB) and the alpha
reaches the blend anyway, which this page records and does not explain.

Census (`crates/vex/tests/alpha_test_reference_ground_truth.rs`): extra-pass batches by list and
by the bit are Pure `(A, 0) 406, (A, 0x02) 578, (B, 0) 58`; Pulse PSP `2,003 / 3,446 / 363`;
Pulse PS2 `4,419 / 4,383 / 802`. Every Pure hull batch of list A carries the bit; no Pulse hull
batch does (all eight teams' loader lines read `0 weighted`), so the weighted Pulse batches are
its circuits' chrome maps. Checked against every Pulse source ours draws a hull from, with the loader line
on one pair per circuit: all 12 PSP circuits and the PS2 pair (Moa Therma) read `0 weighted`, and the PS2 pair
plus two PSP Black layouts render pixel-identical before and after. The weight is read in the PSP executables
only; no PS2 hull carries the bit, so the PS2 executable's reading of it is moot for now.

## What ours does

`oag_mesh::mesh` gives a weighted extra-pass batch its own texture slot: a copy of the
environment map's slot whose `lightmaps` entry is the batch's texture
(`shine_pass::masked_slot`). `shine_pass::build` carries each such batch's authored coordinate
in `lightmap_texcoord` and tags its vertices with a glow-table entry of mode
`PROGRAM_SHINE_MASK`, under which `shade.wesl` multiplies the environment texel by that
texture's alpha. Ours draws the pass after the hull, additively; the original draws it first and
blends the hull over it. The two agree wherever the 8-bit sum does not clip.

Region MAD against the original on the four pairs (grey, 480x272; hull `190..290 x 175..272`):

| Pair | Hull before | Pass unweighted | Weighted | Hull bias after |
| --- | ---: | ---: | ---: | ---: |
| Vineta K | 33.79 | 18.81 | 20.57 | -17.20 |
| Chenghou | 27.17 | 19.29 | 14.51 | -9.76 |
| Blue Ridge | 20.49 | 19.49 | 12.65 | +0.54 |
| Modesto | 15.73 | 15.45 | 5.63 | +1.77 |

Vineta K's and Chenghou's remaining bias is the scene-wide darkness of those two circuits (the
track band reads -22 and -8 too), not the hull.

## The circuits

Pure draws its circuits' chrome-map pass too: the four lists carry 57, 36, 31 and 28 mode-2 PRIMs
away from the craft (Vineta K, Chenghou, Blue Ridge, Modesto), all `BLENDMODE 0xaa`. Ours draws
the circuit pass on Pulse's PSP only (`oag_raceplay::load`, `pulse_psp && track_shine`) and
never weights it. **Checked, applies, not wired**, on both titles: the circuit batches carry the
bit as well (Pulse's 3,446 weighted batches), and wiring the weight there would move Pulse's
pairs.

## The sky

Chenghou Project's sky cube draws last among the scene, six faces of 81 vertices with neither colours
nor normals (vertex type `0x101`), lighting off and `MATERIALAMBIENT` `0xffffff`. So the texel is drawn
as it is. Ours took `mesh::build`'s defaults for a colourless vertex, the stand-in rig and a placeholder
grey `(0.75, 0.78, 0.82)` from the first model render. The sky band read 72 grey where the original
reads 108 to 128. `oag_raceplay::load::environment::unlit_sky` draws a colourless sky vertex unlit at
white (loader line `sky: 486 vertices author no colour, ...`). Frame MAD on Chenghou 20.64 to 13.12, top band 49.75 to
15.47. Vineta's, Blue Ridge's and Modesto's skies author colours and do not move. No Pulse sky has a colourless
vertex: the loader line is absent on all 12 PSP circuits and the PS2 pair.

The GE's material colour is not always white. Vineta's layered water plane at `y = -50` draws with material
ambient `(107, 216, 255)` and `(128, 128, 128)`, and where that colour comes from is not read. A texture node's
`+0x1c` is a per-texture average, not it. So only the sky is wired, where the list says white.

## The scene darkness on Vineta K and Chenghou

Vineta K and Chenghou each draw one more camera-space additive PRIM just before the post chain: 60 vertices,
`BLENDMODE 0xa2` (`src * SRC_ALPHA + dst`), depth test off, a 64x64 CLUT8 texture. Vineta adds a 4-vertex
one beside it. Blue Ridge and Modesto draw none. With those PRIMs removed, the replay drops to ours' level.
Vineta: sky 153.9 to 128.8 (ours 126.6), near road 150.5 to 125.3 (ours 124.2), hull 118.6 to 98.4 (ours 101.1).
Chenghou: sky, track and hull the same way. It is the sun's lens flare, wired by its own lane
(`docs/rendering/lens-flare.md`). Before that, four other terms were checked and ruled
out: vertex colours (405 of 497 joined PRIMs carry the file's bytes), CLUTs (the circuit's found byte for
byte in the file), fog (ranges as ours) and the post composite (`FIXA 175` over a 256x256 target, which moves
the sky by 0.1).

## Light scale

The ordinary pass's lights are the circuit's nodes times `249/255` on all four grids: Vineta K's
ambient `(70, 76, 82)` against the nodes' `(72, 78, 84)`, its first directional `249` against
`255`; Blue Ridge `(110, 133, 166)` against `(113, 137, 171)`; Chenghou's directionals
`(249, 231, 178)` and `(249, 204, 174)` against `(255, 237, 183)` and `(255, 209, 179)`. Modesto's
list also carries a second directional, `(208, 47, 1)` along `(-19.24, 7.19, 1.42)`, unnormalised,
which ours does not draw (all four light types read directional).

`249` is `(255 * 250) >> 8` and reproduces every channel: ambient `72 -> 70`, `78 -> 76`, `84 -> 82`,
`113 -> 110`, `137 -> 133`, `171 -> 166`. So each grid's scale is `250`, Pulse's own live grid value
([scene-light.md](../psp-pulse-usa/scene-light.md), "The scales"): `Spline_AccumulateLightScale`'s double
truncation of a blend of `255` points. Ours takes the nearest point's `255`, a 2 % gap shared with Pulse.
Pure's copy onto the hull (its twin of `Craft_ApplyHullLightScale`) was not located. Neither this nor
Modesto's light is wired.
