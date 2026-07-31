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
| `FUN_089271cc` | A two-line helper: applies scale from `+0x18`/`+0x1c` and offset from `+0x20`/`+0x24` of a struct. Called only from `FUN_089307b4`. |
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
