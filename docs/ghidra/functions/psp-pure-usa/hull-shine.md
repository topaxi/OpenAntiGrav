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

**The material colour, checked against Pulse (2026-11-10, `pulse-hull-sheen`): applies in
structure, not wired.** On Pulse the `0xffffff` in the table above is not a constant: it is the
light the hull stands in, white only where the circuit's lights sum past 255
([mesh-draw.md](../psp-pulse-usa/mesh-draw.md), "The extra pass's colour is the light the hull
stands in"; Metropia's grid reads `0x72a6b7`). Pure's `FUN_08840cac` is Pulse's
`SceneLight_Rebuild` statement for statement (the builder's return to `list+0xcc`, `0xffffffff`
when `model+0x228 == 0`, then `0x55`/`0x58`/`0x56`/`0x57`), so the same is expected here; the
accumulator inside Pure's builder was not read and all four lists above saturate, so
`oag_pure`'s `hull_shine_lit` stays unread. What settles it: one GE list on a Pure circuit or
tunnel whose light sum falls short of 255.

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

## The hull's texture level

Lane `pure-modesto-hull` (2026-11-24). The residual of the Modesto grid pair's hull box (`x 180..300, y 176..272`, grey MAD 5.83 on
the stored pair) was **texture level selection**, not lighting, texture content, geometry or timing. Confidence **90**.

**What the GE is given.** One `gpu.record.dump` of the pair's own save state, read twice (two dumps of one load; the original's
frame is bit-identical across two loads of the state, MAD 0.0): the hull is the three PRIMs at command chunks 1128, 1138 and 1148
(ordinary pass, lighting on, `BLENDMODE 0x2a`) preceded by 1104, 1111 and 1118 (the `TEXMAPMODE` 2 environment pass). Registers at
those PRIMs:

| Word | Ordinary pass | Extra pass |
| --- | --- | --- |
| `TEXLEVEL` `0xc8` | `0x350002`, `0x420002`, `0x3b0002` (mode 2, slope; bias 53, 66, 59 sixteenths = 3.31, 4.13, 3.69) | `0x2a0002` (bias 2.63) |
| `TEXLODSLOPE` `0xd0` | `0x3b8000` = `1/256`, as on **all 430** PRIMs of the frame | same |
| `TEXMODE` `0xc2` | `0x40001`: swizzled, levels 0 to 4 (128, 64, 32, 16, 8 px) | `0x1`: level 0 only |
| `TEXFILTER` `0xc6` | `0x107`: minify linear-mipmap-linear, magnify linear | same |
| Lights | ambient `(73, 63, 53)`, L0 `(-0.6797, 0.3998, -0.6149)` diffuse `(249, 249, 249)`, L1 `(-19.24, 7.19, 1.42)` diffuse `(208, 47, 1)` (the point light, above), no specular | |

Ours reads the same values (a scratch run of `HullLights::from_track` on `03_Modesto_Heights`: ambient `(73, 63, 53)`, light 0 the same
vector and `249`; light 1 is `HullPointLights`' slot 0, pinned by `modestos_grid_hull_takes_the_second_light_the_ge_list_carries`).
The slope rule `level = log2(|z| * slope) + bias` is `mesh-draw.md`'s (`Gu_TexLodSlope`, `0x3b800000`). The hull's view depth is 6.2
to 16.8 units (the dump's view matrix over the batches' bounds), so the level is **-0.6 to -2.1: level 0 over the whole hull**, at any
bias up to 3.9. Replaying the dump with `TEXLEVEL` forced to const level 0 moves the original's hull by 0.16 grey MAD: it is already
there. (The same replays: `TEXFILTER` nearest 0.003, ambient zeroed 9.1, light 1 off 3.1, extra pass removed 22.2.)

**What ours did.** `vex::textures` dropped every level of a version-4 file ("handing them on would switch Pure onto the slope level
rule, which nothing here measured"), so Pure's hull took the sampler's own derivative-driven level over a box-filtered chain, 2 to 3
levels coarser than the original's level 0. The residual was almost all high-frequency (6 % of the squared error low-pass), the
original carried 15 % more local contrast (high-pass gain 1.154), and the interior shading already matched to 1 to 3 levels. Forcing
the hull's bias to -4 (level 0) as a scratch experiment read the box at 2.77; -2: 3.06, -1: 3.75, +1: 6.82, +2: 8.41.

**The fix** reads the levels: a flagged version-4 texture authors them like a version-5 one (each swizzled at its own padded stride).
The hull's levels 0 to 4 on all three textures equal the recorded list's byte for byte (both dumps); `envtest4`'s level 0 does (its
`TEXMODE` allows no other). The base level of 387 flagged textures (the eight hulls, Modesto and Vineta K) is unchanged. Test:
`oag-vex::it pure_texture_levels_ground_truth` (every texture of the eight hulls and Modesto's track reads `mip_count - 1` levels of
the right size; Feisar's three hull textures are chained; red with the version gate restored).

| Modesto grid, box `180..300 x 176..272` | before | after |
| --- | ---: | ---: |
| grey MAD, stored pair (no world clock) | 5.28 | 2.94 |
| grey MAD, re-captured with the world clock (10.04 s) | 5.09 | **2.74** |
| edge correlation at (0, 0) (`psp-pure-pair.py`'s `craft_offset`; 0.408 as first stored) | 0.465 | 0.479 |
| high-pass correlation / gain | 0.846 / 1.154 | 0.965 / 1.017 |
| teal body, MAD (signed original - ours) | 6.61 (-5.2) | 1.83 (-1.2) |
| orange livery | 5.72 (-0.1) | 1.63 (-1.0) |
| canopy | 16.9 (+11.7) | 5.21 (+0.5) |
| hull silhouette edge | 8.62 | 3.93 |

Regions are masks off the two frames' hull hues; the box splits into hull (4,600 px, 51 % of the box's error after, 68 % before),
plume glow (386 px, 8 %) and ground, road and background (6,500 px, 41 %). Pairs, stored ours against re-render, hull box
`190..290 x 175..272` / sky / track / frame, grey MAD:

| Pair | Hull | Sky | Track | Frame |
| --- | --- | --- | --- | --- |
| Vineta K | 12.17 -> 9.02 | 6.01 -> 5.99 | 8.17 -> 7.63 | 7.35 -> 7.07 |
| Chenghou | 10.49 -> 8.36 | 6.57 -> 6.54 | 7.08 -> 6.67 | 7.66 -> 7.46 |
| Blue Ridge | 12.52 -> 12.13 | 6.06 -> 6.05 | 6.35 -> 6.24 | 7.13 -> 7.08 |
| Modesto | 5.71 -> 3.31 | 3.74 -> 3.70 | 4.95 -> 4.52 | 6.18 -> 5.96 |
| Sinucit | 14.14 -> 13.02 | 15.71 -> 15.85 | 14.40 -> 14.19 | 15.33 -> 15.31 |
| Citta Nuova | 10.13 -> 6.90 | 6.87 -> 6.78 | 7.23 -> 6.62 | 8.64 -> 8.35 |
| Sebenco Climb | 23.84 -> 22.77 | 12.10 -> 12.38 | 16.93 -> 16.71 | 15.46 -> 15.48 |
| Sol 2 | 15.55 -> 13.96 | 11.30 -> 11.28 | 6.46 -> 6.14 | 10.27 -> 10.13 |
| Karbonis | 14.63 -> 13.12 | 1.32 -> 1.32 | 2.84 -> 2.54 | 4.27 -> 4.16 |
| Sagarmartha | 31.54 -> 31.76 | 4.41 -> 4.41 | 6.84 -> 6.88 | 8.74 -> 8.76 |
| Manor Top | 11.63 -> 10.02 | 3.41 -> 3.40 | 3.17 -> 2.85 | 6.28 -> 6.15 |
| Mandrashee | 11.67 -> 9.98 | 1.85 -> 1.85 | 3.00 -> 2.67 | 6.82 -> 6.69 |
| Zone z8 | 59.31 -> 61.06 | 59.54 -> 59.54 | 42.15 -> 42.46 | 52.21 -> 52.34 |
| Zone z9 | 63.38 -> 64.53 | 69.21 -> 69.21 | 56.93 -> 57.12 | 64.22 -> 64.31 |

Ten of the twelve race pairs' hulls improve; Sagarmartha's hull is the 4 px shifted one (`compare-page.md`) and moves by +0.22. The
Zone pairs read 52 to 64 grey (the camera and palette gap of `pures-render-what-the-pairs-do-not-show.md`, item 9) and move by +0.1 to
+0.2 on the frame, not attributed to the level rule; z8's track band is the one sky or track reading that moves by more than 0.3 (+0.31). Two Pulse PSP pairs (Amphiseum White, De Konstruct Black) re-render
pixel-identical. Pulse's own version-4 textures (13 nodes, the Zone wreck set) take the slope rule now: their flags byte `0x61` is the one every Pure texture of the frame carries (Feisar's five, Modesto's 156) and the builder is the twin (confidence 75; no Pulse wreck frame was read).

**Open.**

- The per-texture bias. Ours uses `PSP_TEXLOD_BIAS` (1.0) on every chained texture. The original's is per texture (hull 3.3 to 4.1,
  the extra pass 2.6, scenery `0x00` to `0xff` in the same frame); `mesh-draw.md` gives its source (`Texture_BuildBindList`,
  `log2(sqrt(x)) + display term + 8.2` of a header float) and no Pure read of it exists. It changes nothing within about 26 units; a
  distant opponent's hull and mid-distance scenery can step a level or two earlier in the original.
- The environment pass's `TEXMODE` allows level 0 only; ours lets the slope rule step `envtest4` at depth (a far hull).
- What the box still holds: the plume glow (ours brighter by 5 to 11, blue most), the ground band 2.0 MAD (the shadow's shape,
  `compare-page.md`) and the canopy's edge (5.2).

## The circuits

Pure draws its circuits' chrome-map pass too: the four lists carry 57, 36, 31 and 28 mode-2 PRIMs
away from the craft (Vineta K, Chenghou, Blue Ridge, Modesto), all `BLENDMODE 0xaa`. The circuit batches carry the
alpha-weight bit as the hull's do. **Wired for Pure** (lane `pure-chrome-map`): `Looks::track_shine` draws
`oag_render::shine::build_track` and `Looks::track_shine_weighted` keeps each weighted batch's pairing of chrome
map and its own texture, so `shade.wesl` multiplies the chrome map by that texture's alpha (the hull's
`PROGRAM_SHINE_MASK` path). Both are Pure `Measured` on the PSP.

Region MAD against the original (grey, 480x272), pass off (`--no-track-shine`) to on:

| Pair | Frame | Sky band | Track band | Box |
| --- | --- | --- | --- | --- |
| Vineta K | 11.39 -> 8.82 | 12.16 -> 6.16 | 9.84 -> 8.45 | windows (320,45,160x105): 20.7 -> 6.5 |
| Chenghou | 9.36 -> 8.72 | 8.83 -> 6.94 | 7.42 -> 7.29 | |
| Blue Ridge | 8.57 -> 8.12 | 7.01 -> 6.11 | 7.09 -> 6.65 | |
| Modesto | 9.19 -> 8.96 | 10.79 -> 10.16 | 6.02 -> 5.94 | |

Vineta's window box mean reads 89.9 against the original's 89.2 (74.4 before). The weighted result was not
separated from the unweighted one on these pairs (only the pass on/off was scored). Whether the original
weights it was read from the batch bit and the hull's law, not from a circuit-pass-only replay.

**Pulse: weighted, measured 2026-10-26** (lane `pulse-circuit-weighting`): two Pulse PSP GE lists draw the weight-bit
batches' ordinary pass `0x2a` after a replace-form environment pass, as Pure's; `track_shine_weighted` is on for Pulse
PSP. Evidence and the before/after on five Pulse pairs: [circuit-shine.md](../psp-pulse-usa/circuit-shine.md).
**Checked against Omega/2048:** not applicable, a PSP `.vex` rule; HD/2048/Omega circuits author no `TEXMAPMODE` pass.

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
list also carries a second light, `(208, 47, 1)` along `(-19.24, 7.19, 1.42)`, unnormalised: a `wopoint`, see
"Modesto's second light" below (all four light types read directional in a GE list).

`249` is `(255 * 250) >> 8` and reproduces every channel: ambient `72 -> 70`, `78 -> 76`, `84 -> 82`,
`113 -> 110`, `137 -> 133`, `171 -> 166`. So each grid's scale is `250`, Pulse's own live grid value
([scene-light.md](../psp-pulse-usa/scene-light.md), "The scales"): `Spline_AccumulateLightScale`'s double
truncation of a blend of `255` points.
Pure's copy onto the hull (its twin of `Craft_ApplyHullLightScale`) was not located.

**Wired (`pure-light-scale`).** The premise "ours takes the nearest point's `255`" was wrong at the course level: the
ring points already carried the double truncation (`251` at a segment's start, `250` at its other three samples), and
the four grids read `250, 250, 251, 251` in ours before this lane. `oag_race::Course::light_scale_at` now runs the
integer blend (`oag_vex::track::blend_light_bytes`) over the segment's four control points at the craft's parameter,
taken as the spline's nearest point to the craft (found on the spline, not a chord), instead of a lerp of two
already-blended samples. A run of `255` points reads `249..=251` by `t` (`251` at `t` 0 and 1, `249` from about 0.3 to
0.7, `250` between). Grids: Vineta `250`, Chenghou `249`, Blue Ridge `250` (was `251`), Modesto `250` (was `251`); the
original reads `250` on all four, so three are exact and one is a byte off: the original tracks its own parameter, which
is not the nearest point to a pose (De Konstruct, `scene-light.md`, "Blend at the craft"). Hull box (190,175,100,97)
grey MAD, before -> after: Vineta 12.200 unchanged, Chenghou 10.512 -> 10.487, Blue Ridge 12.524 -> 12.523, Modesto
5.751 -> 5.738: a one-byte scale is below what a frame shows. Pure and Pulse only: HD, 2048 and Omega light their hulls
by another path (`ship-sun-occlusion.md`). Test:
`oag-raceplay::it_h_l hull_light_scale_ground_truth::a_pure_grid_craft_reads_the_original_s_250_not_a_full_255`.
Modesto's second light is wired: see the next section.

## Modesto's second light

It is `wopointShape2` of `03_Modesto_Heights\track.vex`, at `(-329.143, 205.185, -428.569)`: colour `(0.84, 0.192, 0.011)`,
full strength inside 22.398 units, none beyond 33.779. The grid craft at `(-309.904, 197.997, -429.991)` is 20.59 units
away, so the builder (`SceneLight_BuildLightingList`, `0x08841648`, Pulse's point branch: [`scene-light.md`](../psp-pulse-usa/scene-light.md),
"Selection and colour") fills a free slot with it as a type-0 light at the unnormalised vector `light - craft` =
`(-19.240, 7.188, 1.421)`, which is why the GE list shows a directional. Its colour byte is `trunc(0.84 * 255) = 214`, times the
grid's scale `250 >> 8`: `214 * 250 >> 8 = 208`; green `0.1922 * 255 = 49 -> 47`, blue `0.0109 * 255 = 2 -> 1`. The twin
`wopointShape3` is 47.64 units away and reaches nothing, so the second point slot is empty. Modesto authors one
directional, which leaves three free slots; ours caps the hull at two point or spot slots (`HULL_POINT_SLOTS`), which
this list does not exceed. Confidence **90**: colour bytes to the byte and the vector to four decimals from the disc's
own nodes, the builder's point branch read statically.

**Already wired.** `pure-render-fidelity` (2026-10-26) put Pure's `wopoint`/`wospot` nodes through the same slots, and the
stored Modesto pair already drew it; the earlier line "ours does not draw it" was a misreading. Taking it out
(`HullPointLights` left empty) moves the pair's hull box (190..290, 175..272) grey MAD from 5.83 to 7.69 and the track
band (0..480, 140..240) from 4.96 to 5.26; sky is unchanged at 3.76. Test:
`oag-raceplay::it_h_l hull_light_scale_ground_truth::modestos_grid_hull_takes_the_second_light_the_ge_list_carries`
(through Pure's own load; red with the loader's `hull_points` assignment removed). Open: the remaining hull MAD of 5.83
on this pair is not this light.
