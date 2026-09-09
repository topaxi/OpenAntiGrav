# Batch draw state: the PS2 port keeps the PSP's `pass_mask`, bit for bit

Found 2026-08-23, while asking why the PS2 boost plume draws as solid geometry
rather than as a glow. The `pass_mask` findings below are the durable result;
the answer to the plume question that was first drawn from them turned out to
be **wrong**, and the section at the end records how a capture refuted it.

The PSP side of this is
[mesh-draw.md](../psp-pulse-usa/mesh-draw.md); everything below was read
independently on `SCES_547.48` and is a **cross-verification of a finding that
until now rested on one binary**, not a restatement of it.

## `Gfx_BuildBatchStateList` at `0x001e9088`, confidence 88

The per-batch state setup. Both mesh batch walkers call it with the batch's own
header:

```c
Gfx_BuildBatchStateList(renderer,
                        *batch,                  /* pass_mask, the u16 at +0x00 */
                        *(u8 *)((int)batch + 3), /* header_flags               */
                        renderer->field_0x134,
                        renderer->field_0x138,
                        *(u8 *)((int)batch + 0x19));
```

That call shape is what ties the arguments to the fields `oag_vex::vex`
already decodes: argument 2 is `Batch::pass_mask` and argument 3 is
`Batch::header_flags`, read at the offsets that crate reads them at.

Its body, with the wrappers named below:

```c
Gu_PixelMask(0xffffffffff000000);
Gu_...( (pass_mask >> 2) & 1, (pass_mask >> 3) & 1 );   /* texture wrap, see note */
if ((pass_mask & 0x20) == 0) Gu_Enable(5); else Gu_Disable(5);   /* GU_CULL_FACE */

if ((pass_mask & 0x700) == 0) {
    Gu_DepthMask(arg4 == 0x3b2);
    Gu_Disable(4);                                     /* GU_BLEND off          */
    if ((pass_mask & 0x800) == 0) {
        Gu_DepthFunc((header_flags & 0x20) ? 5 : 6);
        Gu_Enable(0);                                  /* GU_ALPHA_TEST         */
        Gu_AlphaFunc(1, 0, 0xff);                      /* GU_ALWAYS             */
    } else {
        Gu_Enable(0);
        /* the alpha reference, and this is where pass_mask & 0x80 is read */
        ...
        Gu_AlphaFunc(6, ref, 0xff);                    /* GU_GREATER            */
    }
    Gu_Disable(0x11);                                  /* GU_COLOR_TEST         */
} else {
    Gu_DepthFunc(6);
    Gu_DepthMask(1);
    if      (pass_mask & 0x100) { Gu_Enable(4); Gu_BlendFunc(0, 2, 3, 0, 0);        Gu_Disable(0x11); }
    else if (pass_mask & 0x200) { Gu_Enable(4); Gu_BlendFunc(0, 2, 10, 0, 0xffffff); Gu_Enable(0x11); }
    else if (pass_mask & 0x400) { Gu_Disable(4); }
}
...
if ((pass_mask & 0xc0) == 0) {
    if ((pass_mask & 0x700) == 0) {
        Gu_StencilFunc(1, renderer->field_0x1178, 0xff);
        Gu_StencilOp(0, 0, 2);                         /* KEEP, KEEP, REPLACE   */
        Gu_Enable(3);                                  /* GU_STENCIL_TEST       */
    } else {
        Gu_StencilOp(0, 0, 0);
        Gu_Disable(3);
    }
} else {
    Gu_StencilFunc(1, 0xff, 0xff);
    Gu_StencilOp(0, 0, 2);
    Gu_Enable(3);
}
```

**Every `pass_mask` bit this project decodes reads the same way here as on the
PSP**, in the same branch priority: `0x20` disables culling when *set* (the
inverted sense `Batch::is_culled` documents), `0x0700` is the transparent class
with `0x100` winning over `0x200` winning over `0x400`, and `0x800` is the
alpha test. That was a PSP-only reading until this page; it now holds on a
second, independently compiled executable for the same game.

## The two batch walkers, confidence 82

`Mesh_DrawBatches` at `0x001dcad0` and `Mesh_DrawAlphaTestedBatches` at
`0x001dce58` are the same loop written twice with the test inverted:
`0x001dcad0` skips a batch when `(*batch & 0x800) != 0`, `0x001dce58` skips one
when `(*batch & 0x800) == 0`. Both take a **list mask** as argument 4 and stop
at the first batch whose `pass_mask` does not carry it - the same list split
`vex::mesh_batches` walks.

`0x001dcad0` also carries the corroboration that the PS2 port kept the PSP's
graphics vocabulary rather than merely its data: it logs a texture-wrap mode by
name, and the names are the literal strings `"SCEGU_REPEAT"` and
`"SCEGU_CLAMP"`, selected on `pass_mask & 4` and `pass_mask & 8`.

## `Gu_BlendFunc` at `0x0010b978`, confidence 90

The strongest single result here, because it is checkable arithmetic rather
than a structural analogy. The function takes sceGu blend-factor codes and
writes the Graphics Synthesizer's `ALPHA_1` register, whose output is
`(A - B) * C + D` with `A`/`B`/`D` in `{Cs=0, Cd=1, 0=2}` and `C` in
`{As=0, Ad=1, FIX=2}`:

| sceGu `(src, dst)` | `ALPHA_1` written | Decodes to | Equals |
| --- | --- | --- | --- |
| `(2, 3)` - `SRC_ALPHA`, `ONE_MINUS_SRC_ALPHA` | `0x44` | `(Cs - Cd) * As + Cd` | `mesh_render::TRANSPARENT_BLEND` |
| `(2, 10)` - `SRC_ALPHA`, `FIX` | `0x48` | `(Cs - 0) * As + Cd` | `mesh_render::ADDITIVE_BLEND` |
| `(10, 10)` - `FIX`, `FIX` | `0x68` + fix | `Cs * FIX + Cd` | the plain-additive `dst + src` |
| `(4, 10)` - `DST_ALPHA`, `FIX` | `0x58` | `Cs * Ad + Cd` | - |
| `(10, 2)` | `0x09` | `(Cd - 0) * As + Cs` | - |

So the two equations this project recovered from the PSP's `Gu_BlendFunc`
calls are the same two the PS2 build programs into GS registers. Nothing about
either constant changes; they gain a second, independent source.

## `Gu_Enable` at `0x0010b1e8` and `Gu_Disable` at `0x0010b230`, confidence 85

A one-line pair over a state bitmask: `Gu_Enable` sets `1 << state` in
`0x002db9bc` and marks it dirty in `0x002db9c0`, `Gu_Disable` clears it and
marks the same bit dirty. State `5` - `GU_CULL_FACE` - is special-cased to a
byte of its own at `0x0027a850` in both. The sense is fixed by
`Gfx_BuildBatchStateList`'s cull branch reproducing the PSP's documented
`(pass_mask & 0x20) == 0 -> enable` exactly.

## What this says about the boost plume, and what is still open

The PS2's `Data\Ships\<Team>\shipboost.vex` carries `pass_mask 0x10b2` on all
four of its batches, and `0x10b2 & 0x700 == 0`. Reading only the function above,
that says the original disables blending for the plume and draws it as opaque
geometry.

**That conclusion was drawn here and is wrong, and how it was refuted is the
most useful thing on this page.** On 2026-08-23 the project took its first
PCSX2 capture of a PS2 boost (`~/.config/PCSX2/snaps/`, not committed - it is
game content). In it each nozzle is a soft violet plume with a white core, no
geometry edge anywhere, and the hull plainly visible *through* it. Drawn opaque,
the same model puts a hard-edged hexagon over each nozzle that occludes the
hull - which is what this project shipped for a few hours, and what the capture
does not show. So the original blends this model, and the only open question is
where it says so.

**Where it says so is not on this path.** `Gfx_BuildBatchStateList` is reached
through `Mesh_DrawBatches`, which `Mesh_Draw` (`0x001dd6f0`) calls for a sort
key of layer `0x750`. The plume's own object is built by `FUN_001d4310` - the
`%s\%sboost.vex` loader - as a model instance with sort word `0x7d000000`, and
queues at layer `0x7d0`. Its vtable is `0x0029a3a0`, which contains neither the
layer-`0x7d` queue `FUN_001de438` nor the forced-mask draw `FUN_001de648`, so
that pair belongs to some other object type; the object reaches its draw through
second-base thunks (`obj+0x38` chained via a `+0x70`/`+0x78` offset pair) and
**that chain has not been followed**. Following it is what would turn
`oag_game::race::Drawable::draw_additive`'s model-scoped override into a decode.

The end branch is a separate, still-standing result: `0x10b2 & 0xc0` is non-zero
(`0x80` is set), so the batch takes the stencil branch that stamps `0xff`
through `KEEP, KEEP, REPLACE` - the same alpha-channel stamp
`Trail_BuildStateList` uses on the PSP to feed the bloom's bright pass, see
`oag_render::exhaust::TRAIL_BLEND`. So the plume is a bloom-contributing
surface whichever way it blends, which is why `race::Scene` giving it
`GlowMask::Written` is right on both discs.

**The lesson worth carrying**: a state builder that is provably correct about a
`pass_mask` still says nothing until you have shown the object in question
reaches it. Two of this page's readings survived the capture and one did not,
and the one that did not was the one no frame had ever been compared against.

**Open, and deliberately not acted on:**

- `pass_mask & 0xc0` is decoded here as "stamp the glow mask" from one
  function's use of it. `0x40` has not been seen set on any batch read so far,
  so whether the two bits mean different things is unmeasured. Nothing in this
  workspace consumes the decode yet; `race::Scene` already gives the plume
  `GlowMask::Written` for an independently recovered PSP reason.
- `Gfx_BuildBatchStateList`'s argument 4 is compared against `0x3b2` to decide
  the depth mask, and what that value identifies is unread.
- Arguments 3 and 6 (`header_flags`, and the byte at `batch + 0x19`) reach
  branches this page does not follow.
- `Gu_AlphaFunc` `0x0010ba68`, `Gu_StencilFunc` `0x0010b968`, `Gu_StencilOp`
  `0x0010b970`, `Gu_DepthMask` `0x0010b8c0` and `Gu_DepthFunc` `0x0010ba10`
  are named in the listing above **as hypotheses from their argument shapes
  only** and are deliberately *not* in `names.tsv`: none has been read at
  instruction level, which puts them under the 50 the rubric requires before a
  name goes on anything.
