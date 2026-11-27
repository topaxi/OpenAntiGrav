# Pure's texture level selection: Pulse's per-texture slope bias, call for call

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra `/pure/BOOT-psp-pure-usa.BIN`.
Read 2026-11-27 (lane `farfield-lod-titles`), against Pulse's
[mesh-draw.md](../psp-pulse-usa/mesh-draw.md) ("`Texture_BuildBindList`") and checked against two
recorded GE lists of the original on Modesto Heights (PPSSPP v1.20.4, software renderer,
`scripts/psp-ge-dump.py`, the save state of the Pure Modesto compare pair; the lists
[hull-shine.md](hull-shine.md) read). Per-title summary:
[texture-level-per-title.md](../../../rendering/texture-level-per-title.md).

## The law

Every textured draw is in GE slope mode or automatic mode, chosen per texture:

- **Per frame**, the draw setup (`FUN_08866bec`) calls `Gu_TexLevelMode(2, 1.0)` and
  `Gu_TexLodSlope(1 / powf(2.0, 8.0))`, `TEXLODSLOPE` `1/256`.
- **Per texture**, the bind list `Texture_BuildBindList` emits its own `TEXLEVEL`:
  - header byte `+0x06`, bits `0x18`, value 0: slope mode `0xc8000002` with bias byte
    `trunc(16 * Texture_LodBiasFromDensity(f32 at header +0x18)) & 0xff`;
  - value 1: `0xc8000000`, automatic, bias 0;
  - otherwise: no `TEXLEVEL` word.
- **`Texture_LodBiasFromDensity(x)`** is `log2(sqrt(x)) + K + 0.2 + 8.0`, clamped to `[-8, 7.5]`, and 0 for
  `x == 0`. Here `K = *(float *)(g_display + 0x5df4)`.
- **`Camera_StoreTextureLodTerm`** writes K. It projects `(-0.5, 0, 1, 1)` and `(0.5, 0, 1, 1)` by the
  current projection, takes `d = |x0/w0 - x1/w1| * 480` and stores
  `K = log2(1/d) - log2(|w0|)`. That is `-log2(480 * m00)` for the GE's projection.
- **Stored K.** A texture keeps the K it was built with at its object's `+0x2a0`.
  `Texture_Bind` (`FUN_0887e900`) rebuilds the bind list (`Texture_RebuildBindList`) only when
  `|stored K - current K| > 0.2`. Pulse keeps its stored K at `+0xb0` with the same 0.2 threshold
  (relayed from lane `pulse-farfield-lod`).

The GE then selects `level = TexLog2(2 * w * slope) + bias` (PPSSPP's software rasteriser, the reading
`pulse-farfield-lod` measured).

## Functions

| Address | Name | Pulse twin | Confidence | Evidence |
| --- | --- | --- | ---: | --- |
| `0x0887f058` | `Texture_BuildBindList` | `0x08928d4c` | 85 | One of two `lui 0xc800` sites in the binary. Pulse's body: the `+0x06 & 0x18` switch, the `0xc8000002`/`0xc8000000` words, `TEXMODE` with `levels - 1`, the per-level loop, the CLUT words and `TEXFLUSH`. Only the context offsets differ (`+0x180` list, `+0x148` per-level addresses) |
| `0x0887d610` | `Texture_LodBiasFromDensity` | `0x08926f6c` | 85 | The same expression and constants: `/0.6931472`, `+0.2`, `+8.0`, clamp `7.5`/`-8.0`, read of `g_display + 0x5df4` |
| `0x08a30f24` | `Camera_StoreTextureLodTerm` | `0x0887840c` (tail) | 90 | The same two projected points (`-0.5`/`0.5`, `z = 1`, `w = 1`), the `* 480.0` and the difference of two logs, stored at `+0x5df4` (`0x08a3111c` seen live). Three callers (`0x0883fc04`, `0x0883ff34`, `0x0885fdf8`), below |
| `0x0883ff34` | `Camera_SubmitScene_q` | `0x08878874` | 65 | Builds the view and projection from a camera object and the field of view, then calls `0x08a30f24`. Live: the flyby's `K` and, once racing, the race camera's |
| `0x0883fc04` | `Camera_ApplyViewProjection` | `0x0887840c` | 70 | Copies the stored view and projection into the display slots, then calls `0x08a30f24`. Live: the race camera's `K` |
| `0x0887e900` | `Texture_Bind` | (Pulse's bind, read by `pulse-farfield-lod`) | 70 | `|*(+0x2a0) - K| > 0.2` calls `Texture_RebuildBindList`, then hands `+0x180` (the bind list) to the GE call list |
| `0x0887e9e8` | `Texture_RebuildBindList` | | 75 | Lays out the per-level addresses, calls `Texture_BuildBindList`, stores K at `+0x2a0` |
| `0x088f01d4` | `Gu_TexLevelMode` | `0x088114b0` | 85 | The only other `lui 0xc800`: `(clamp(bias * scale, -0x80, 0x7f) << 16 | mode) | 0xc8000000` |
| `0x088f04e4` | `Gu_TexLodSlope` | `0x08811694` | 85 | The only `lui 0xd000`: `0xd0000000 | bits >> 8`; one caller, `FUN_08866bec`, passing `1 / powf(2, 8)` |

## Checked against the original

**Register census** of the two dumps (one state load, dumped twice): every textured `PRIM` with the
`TEXLEVEL` and `TEXLODSLOPE` in force, walked with `scripts/psp-ge-dump.py`'s reader. Results, the
same in both dumps:

- one `TEXLODSLOPE` write, `0.00390625` (1/256);
- 56 distinct `TEXLEVEL` values;
- mode 2 biases from `-0.0625` to `+6.25`;
- 15 mode-0 (automatic) words and 4 mode-2 words with bias 0.

The `TEXMODE` max level reaches 3, so Pure's track textures carry levels.

**The bias against the header float.** Each textured `PRIM` was matched to its Modesto `track.vex`
texture by texel bytes:

- **Texture set.** 156 textures were dumped from `Data\Environments\03_Modesto_Heights\track.vex` in
  `PSP_GAME/USRDIR/Data.wad`. Every one carries header flags `0x61`, so every one is mode 2.
- **Matching.** 88 slope binds matched 40 distinct textures.
- **Prediction from the drawn camera.** The K of the drawn projection is `-log2(480 * 0.665039)` =
  `-8.3184` (the GE matrix's `m00`). The observed bias is lower than that prediction by 11 or 12
  sixteenths on 72 of the 88 binds.
- **Fit.** One K in `[-9.0507, -9.0486]` reproduces the byte exactly for 37 of the 40 textures,
  over `x18` from 0.007 to 1,584 (five decades). The three misses are `TempTexBlue.tga` binds,
  several textures under one name.
- **Reading.** The `0.5 * log2(x18)` slope holds, and the `K` in the textures is about -9.05.
  It is a stored value from an earlier camera, below.

**Confidence 85** for the law on Pure: static identity with Pulse's measured law, plus 37 of 40
textures exact in a recorded list.

## The `K` of -9.05 is the pre-race flyby's

**Confidence 85** (2026-11-27, PPSSPP v1.20.4, software renderer, the Modesto compare pair's save
state). One state and one circuit, and the flyby watched is a RESTART's, not the first load's.

**In a race frame** (write watch on `g_display+0x5df4`, `g_display` at `0x08a8e960`):

| `K` | `m00` | Through |
| ---: | ---: | --- |
| -8.3184 | 0.66504 | `0x0885fdf8`, once a frame |
| -8.8799 | 0.98149 | `0x0883fc04`, the rest |

The world meshes bind in `FUN_08882e5c` (the 0.2 test at `0x088830c0`). Over 4 frames 300 of 306
checks, and on a reload 174 of 180, read `K = -8.8799` with the projection slot's `m00` 0.98149;
the other 6 read -8.3184. Of the 70 textures checked, 63 hold `-9.0506`, 4 `-9.0491` and 3
`-8.3184`. -9.05 is 0.17 from -8.88, so the test keeps it. An earlier reading on this page took
the -8.32 projection for the one the textures bind under, and concluded the value could not be a
stale build; it can.

**Through a restart** (pause menu, RESTART, twice from the same state; halting watch on value
changes): the first `-9.0506` store is `Camera_StoreTextureLodTerm` called from `0x0883ff34`
(under `0x08893394`), `m00` 1.10471, about 0.85 s after the confirming press. A frame taken then
shows the flyby camera with the HUD hidden. Pulse's flyby camera writes the same value
([mesh-draw.md](../psp-pulse-usa/mesh-draw.md), "`K = -9.0506` is the pre-race flyby camera's").

Not read: where the 4 textures' `-9.0491` comes from, the camera of `m00` 1.45836 seen in one
restart, Pure's first load, and whether Pure has Pulse's residency rebuild.

**Consequence for wiring.** Pure takes Pulse's rule: a stored `K` per texture, replaced past 0.2,
with the flyby's draws counted. Current-camera `K` alone (-8.88 here) is 2 to 3 sixteenths coarser
than the original on the textures the flyby drew.

## Ours

`oag_vex` reads a flagged version-4 texture's authored levels (commit `9213011dd`), so Pure's track
textures are a `Texels::Chain`. They take the shared slope path with
`mesh_render::PSP_TEXLOD_SLOPE` and Pulse's constant `PSP_TEXLOD_BIAS = 1.0`. The per-texture bias
and the mode-0 textures are what lane `pulse-farfield-lod` wires for Pulse in the shared
`oag-mesh` path. Pure takes the same law with Pure's own header floats and K
(**checked, applies, not wired**).

## Reproducing

```sh
# the bind list and the bias helper
ghidra: decompile 0x0887f058, 0x0887d610, 0x0887e900, 0x0887e9e8, 0x08a30f24 in /pure/BOOT-psp-pure-usa.BIN
# the texture set
cargo run -p oag-tools --bin oag-wad -- cat data/images/pure-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    'Data\Environments\03_Modesto_Heights\track.vex' > modesto.vex
```

Then dump each texture's header `+0x18` float with `oag_vex::vex::stored_textures` and compare with
the `TEXLEVEL` byte in force at each `PRIM` that samples it.
