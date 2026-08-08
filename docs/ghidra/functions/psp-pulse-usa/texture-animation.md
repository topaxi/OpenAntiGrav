# Texture animation: the GU transform primitives

Where the engine can move a texture under fixed geometry, and what is known
about who does it.

The renderer's side of this - which surfaces animate and why - is in
[`docs/formats/vex.md`](../../../formats/vex.md), "Tracks animate too". This
page is the code half: the two GE state setters the effect must go through, and
a bounded statement of who calls them.

## The two primitives

| Address | Name | Conf | Signature |
| --- | --- | ---: | --- |
| `0x08811630` | `Gu_TexOffset` | 88 | `void (float u, float v)` |
| `0x08810ab4` | `Gu_TexScale` | 88 | `void (float u, float v)` |

Both are thin two-float wrappers in the same address neighbourhood as the
already-named `Gu_CallList` (`0x08810598`) and `Gu_DrawArray` (`0x08810e98`),
which is what a statically linked `libgu` looks like.

**Evidence.** `Trail_DrawRibbon` (`0x0892acc8`) advances a per-layer scroll and
immediately passes it to these two, in this order:

```c
*(float *)(iVar24 + 8)   += *(float *)(iVar24 + 0x10) * _DAT_08a889d0 * 3.0;
*(float *)(iVar24 + 0xc) += *(float *)(iVar24 + 0x14) * _DAT_08a889d0;
/* each wrapped into [0,1] */
FUN_08811630(*(undefined4 *)(iVar24 + 8),  *(undefined4 *)(iVar24 + 0xc));   /* offset */
FUN_08810ab4(*(undefined4 *)(iVar24 + 0x18), *(undefined4 *)(iVar24 + 0x1c)); /* scale  */
```

That block is already independently documented in
[`exhaust.md`](exhaust.md#trail): `+0x10`/`+0x14` are the authored `u`/`v`
scroll **rates**, `+0x08`/`+0x0c` the animated offsets, `+0x18` the `u` texture
scale, and `_DAT_08a889d0` is `0x3c888889` = `0.016666668`, one 60 Hz tick. So
the argument roles are fixed by a caller whose semantics were recovered
separately, which is why this scores 88 rather than being a guess from position.

88 and not higher because no import NID names them - `sceGu*` is statically
linked, not imported (see [imports.md](imports.md)) - and no runtime trace has
confirmed the GE commands they emit.

`DAT_08ab0628` gates the advance: when it is non-zero the offsets are submitted
but not stepped, which is the shape of a global pause flag. Not traced to what
sets it; **confidence 50**, recorded rather than named.

## Who calls them

`Gu_TexOffset` has **12** callers, `Gu_TexScale` **16**. Only one is
identified.

| Caller | Status |
| --- | --- |
| `Trail_DrawRibbon` `0x0892acc8` | Identified. The exhaust ribbon, documented in [`exhaust.md`](exhaust.md). |
| `FUN_089271cc` | A two-line helper: applies scale from `+0x18`/`+0x1c` and offset from `+0x20`/`+0x24` of a struct. Called only from `FUN_089307b4`. **Its gate is now read - see below.** |
| `FUN_089307b4` | Large; also calls both primitives directly. Reached from `FUN_0892f35c` (4 sites) and `FUN_0893021c`. |
| 9 others | Not examined. |

`FUN_0893021c` calls both `FUN_089307b4` and the VRAM texture upload
`FUN_08928550`, which is the shape of a bind-texture-plus-material-state path.
That makes `FUN_089307b4` the most likely home of a general per-material texture
transform, and `+0x20`/`+0x24` of its struct the most likely place a track
surface's UV offset would be written.

**This is a hypothesis, not a finding.** Nothing traces a track material to that
struct, and no name is proposed for either function - **confidence 45**, below
the [rubric](../../../reverse-engineering/confidence-rubric.md)'s rename
threshold. Written down instead, per that rubric's rule.

### The gate is a per-material bit, read 2026-08-08

The "most likely home" reading above is right about the shape and can be
tightened: `FUN_089307b4` calls the pair at three sites, each guarded by the
same test on the **runtime material** (`*(mesh+0x5c) + material_index * 0x14`,
the stride-`0x14` repacked array), and the branch picks between the immediate
form and a prebuilt list:

```c
if ((*(u16 *)(*(int *)(mesh + 0x5c) + material_index * 0x14) & 0x10) != 0) {
    if (mode == 1) Gu_CallList(*(int *)(mesh + 0x64) + material_index * 0x18);
    else if (mode == 0) FUN_089271cc(*(int *)(mesh + 0x60) + material_index * 0x40);
}
```

So `& 0x10` on the material's first `u16` is **the per-material "this surface
has a texture transform" bit**, and `mesh+0x60 + index * 0x40` is the block
whose `+0x18`/`+0x1c` and `+0x20`/`+0x24` `FUN_089271cc` submits.
Corroborated from the data side: `Data\Ships\<Team>\shipboost.vex`'s single
material carries flags `0x212`, which has the bit, and that model's authored
`u` spans only `[0.000, 0.008]` - one step of an 8-bit texcoord on its 64x16
texture - which is geometry that *needs* a scale to make sense of. Confidence
**75** for the gate (the test is read at instruction level at three sites and
the one model checked agrees); still **0** for where the block's values come
from, since the on-disc material's `+0x0c..0x14` is zero on that ship exactly
as it is on every track.

This does **not** contradict the live negative below. That measured
`Gu_TexOffset`, and found only the ribbon submitting a non-zero one;
`Gu_TexScale` was never sampled, and a scale with a zero offset would have
looked identical to it.

Still no rename: the gate is read but the data path into the block is not, and
`FUN_089307b4` is a large function whose name would have to cover much more
than this.

## Measured in a live race: only the trail scrolls

**Confidence 88, and it is a negative.** A breakpoint on `Gu_TexOffset` through
a running Time Trial on Talon's Junction, 60 hits:

| Return address | Hits | Offsets passed |
| --- | ---: | --- |
| `0x0892b088` in `Trail_DrawRibbon` | 9 | **9 distinct**, advancing every frame - `u` 0.100 to 0.800, `v` 0.067 to 1.933 |
| `0x088a7b74` | 26 | `(0, 0)` every time |
| `0x0891e8ec` | 25 | `(0, 0)` every time |

So the **only** non-zero texture offset the engine submits during a race is the
exhaust ribbon's, whose scroll rates were already recovered independently
([`exhaust.md`](exhaust.md#trail)). The other two sites reset the offset and are
called about three times a frame each, which is pipeline setup rather than
per-batch state. **No track surface receives a texture-coordinate offset.**

The read is on-target rather than incidental: Talon's Junction is `16_Track`,
which carries two of the surfaces
`oag_render::mesh::ANIMATED_TEXTURES` lists (`col_display7_GLOW` and
`col_display7_BLEND_GLOW`), and `Gfx_BindTexture` was hit 80 times over the same
window with many distinct texture objects, so the track was plainly drawing.

**A CLUT scroll is not the explanation either.** Five palette regions reached
through bound texture objects were byte-identical over a 3.5-second window with
the CPU running. That does not cover every palette on the disc, so it is a
bounded negative - **confidence 70** - but it removes the obvious second
candidate.

### What this does and does not overturn

It does **not** touch the measurement that a ship's shoulder lights pulse: that
came from 120 frame-accurate screenshots with pixels sampled at a real light,
and it stands. What it contradicts is the *mechanism* inferred from it -
[`vex.md`](../../../formats/vex.md) reads that pulse as the engine scrolling the
shared texture's V globally, and no such offset is ever submitted, for the ship
or for anything else. The pulse is real and its cause is **not** a texture-
coordinate offset.

The untested candidate that fits every observation is an animated **colour**
rather than an animated coordinate: `Trail_DrawRibbon` already sets a per-draw
colour through `FUN_0881125c`, and a per-object colour advanced on a clock would
pulse a light without moving a UV or rewriting a palette. Not investigated.

**Consequence for the renderer**: `[graphics] animated_textures` now defaults
**off**. The eight track surfaces were selected on real geometry - narrow
authored V bands against full-tile static art - and that measurement is kept,
but making them scroll is a departure from the original rather than a
reproduction of it.

## What was ruled out

**`Gfx_BindTexture` (`0x08928460`) does not animate anything.** It compares the
texture's `+0xb0` against a global at `g_display + 0x5df4` and re-runs
`FUN_08928550` when they differ by more than `0.2`, which reads like a global
animation phase against a per-texture copy of it. It is not: `FUN_08928550`
walks the mip chain into VRAM (`FUN_089287c0` per level), sets a
resident flag, and *then* stores the global into `+0xb0`. So `+0xb0` is a
last-uploaded timestamp and the `0.2` is a texture-cache re-upload heuristic.
Recorded because the shape is genuinely misleading and cost a read.

## Open

- **The global V-scroll clock has not been found.** `vex.md` establishes the
  mechanism and measures its period (~30 ticks per authored cycle) from a
  capture, but no code has been read that advances it. It must be reachable from
  `Game_UpdateFrame` (`0x08804978`) - see [main-loop.md](main-loop.md) - or from
  a texture-manager tick under it. Finding it would pin the rate this project
  currently reuses as a guess across every animated track surface.
- **The nine unexamined `Gu_TexOffset` callers.** A caller inside track drawing
  would move the whole trackside-animation reading from inferred (65) to
  evidenced.
- **What sets `DAT_08ab0628`.**
