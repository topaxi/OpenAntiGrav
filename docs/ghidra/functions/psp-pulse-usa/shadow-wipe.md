# The shadow pass's wipe: how dark a stencil shadow is

[`shadow-occluder.md`](shadow-occluder.md) reads the function that extrudes a
craft's stencil shadow volume. A stencil volume only marks pixels; this page
reads the pass that darkens what it marked, which is where
`oag_render::shadow::HULL_DARKNESS` comes from.

2026-11-17. Static reading of `psp-pulse-usa`'s `BOOT.BIN`, cross-checked
against the PS2 executable (`SCES_547.48`) and against a GE dump of a running
frame taken on 2026-10-21 ([`shadows.md`](../../../rendering/shadows.md), "The
darkness is the original's").

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08922ea4` | `Shadow_DrawWipe` | 88 |
| `0x08922e18` | `Shadow_SubmitWipe` | 80 |
| `0x0892342c` | `ShadowNode_Read` | 75 |
| `0x08a88680` | `g_shadow_darkness` | 90 |
| `0x08b62550` | `g_shadow_darkness_override` | 78 |

## The law

```text
darkness = g_shadow_node_count != 0 ? g_shadow_darkness_override : g_shadow_darkness   # 0.38
d        = trunc(darkness * 255.0)                                                    # 96 = 0x60
if sort_key == 0x49000000: d >>= 1                                                    # 48
fix      = d | d << 8 | d << 16
blend    = ADD, src factor FIX(fix), dst factor FIX(fix ^ 0xffffff)
colour   = 0xff000000 (black)
out      = dst * (255 - d) / 255
```

A marked pixel keeps `159/255` of its colour, so the shadow is `96/255` =
`0.3765` dark. **Confidence 90**: the float, the truncation and the blend are
read in one function on PSP, the same law is in the PS2 build, and the GE dump
of a live frame shows the three words it produces (`BLENDMODE` `0xaa`,
`BLENDFIXEDA` `0x606060`, `BLENDFIXEDB` `0x9f9f9f`).

## `Shadow_DrawWipe` (`0x08922ea4`)

`(self, display, sort_key)`. No direct caller: its address sits in a method
table at `0x08ad1bf8` and the render queue calls it with the key it was
enqueued under. In order:

1. Loads the float at `0x08a88680` (bytes `5c 8f c2 3e` = `0.38`), or the one
   at `0x08b62550` when `g_shadow_node_count` (`0x08abffd0`) is non-zero,
   multiplies by `255.0` and truncates to an integer.
2. `if (sort_key == 0x49000000) d >>= 1` (the `lui a0, 0x4900` at
   `0x08922ea8`).
3. Pushes identity on the view and model matrix stacks and sets the
   projection to identity too (`Gu_SetMatrix(0|1|2, 0x08a907a0)`), so the quad
   is in clip space.
4. State: `Gu_DepthMask(1)` (writes off), stencil test on, the bloom's glow
   mask made read-only (`Bloom_SetGlowMaskWritable(g_bloom, 0)`), alpha test
   `ALWAYS`, texture, lighting, fog and culling off, blending on.
5. `Gu_BlendFunc(0, 10, 10, fix, fix ^ 0xffffff)`: op `ADD`, both factors
   `GU_FIX`.
6. `Gu_StencilOp(2, 2, 2)` (`REPLACE` on every outcome), `Gu_PixelMask(0)`,
   `Gu_StencilFunc(6, *(u8 *)(display + 0x1178), 0xff)`: the test passes where
   the stencil differs from the display's resting value (`4`, the value the
   glow mask reads as "no glow", [`ghost.md`](ghost.md)) in the direction the
   volume's increments leave it, and every pixel the quad touches is written
   back to that value. So the wipe both darkens the marked pixels and clears
   the marks: the GE dump's test word is `0xff0406`.
7. `Gu_Color(0xff000000)` and `Gu_DrawArray(4, 0x183, 4, 0, 0x08abff70)`: a
   four-vertex triangle strip whose positions are the clip-space corners
   `(-1, -1) (-1, 1) (1, -1) (1, 1)` (read out of `0x08abff70`).
8. Restores culling, depth test, blending and stencil state and pops the two
   matrix stacks.

## `Shadow_SubmitWipe` (`0x08922e18`)

`(self)`, in the same method table at `0x08ad1be8`. If the frame has shadow
volumes (`0 < *0x08abffc0` and the byte at `0x08abffc4` set) it enqueues the
object on the render queue at key `0x47000000`, and, when `*0x08ab22f8 > 0`,
a second time at `0x49000000`. The second wipe is the half-strength one.

`0x08ab22f8` is a counter written by `MeshNode_Ghost_ConvertTree` and
`FUN_08902ca0` and read nowhere else. **Hypothesis, confidence 55**: it counts
ghost craft, so a time-trial ghost's shadow is wiped in its own later layer at
half darkness (`48/255` = `0.188`). Not verified in a running frame, and this
project draws no shadow for a ghost.

## `ShadowNode_Read` (`0x0892342c`)

The reader of vex class `0x3cb` (`shadow`), which `shadow-occluder.md` left
unnamed. It reads one float from the node's payload into
`g_shadow_darkness_override` (`0x08b62550`), then normalises the negated
second row of the node's world matrix into `g_shadow_direction_override`
(`0x08b62530`). So a `shadow` node authors both the direction and the
darkness of every shadow while it is alive. No `.vex` on the disc authors one
(`shadows.md`), so the constants are what every shipped frame uses.

## PS2 cross-check

`FUN_001ec498` in `/pulse/SCES_547.48-ps2-pulse-eu` is the same function:
the float `0.38` is at `0x002bd9b0` (the only `5c 8f c2 3e` in the image,
read at `0x001ec530`), the override is taken when the word at `0x00282bc0` is
non-zero, the product with `255.0` (`lui at, 0x437f`) goes through a
float-to-int helper, and the result is shifted right by
`sort_key == 0x79000000` (`lui v0, 0x7900`, `xor`, `sltiu`, `srlv`) before it
is packed as `d | d << 8 | d << 16`. The key's top byte differs (`0x79`
against `0x49`), the law does not. PS2 confidence 80 (disassembly only).

## What this project draws

`HULL_DARKNESS` is `1 - 159/255` = `96/255`, the value this law produces, so
the constant did not move; its comment now cites this page. The `original`
tier draws the volume's ground cap as a polygon at that alpha, not a stencil
wipe, so overlapping shadows of two craft darken twice where the original's
wipe darkens once. Open: the half-strength ghost wipe.
