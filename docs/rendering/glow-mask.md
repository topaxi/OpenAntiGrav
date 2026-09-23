# Pulse's glow mask: what the bloom actually reads, measured

The bloom's four passes are recovered on
[`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md) and ported in
`oag_render::post::bloom`, every constant read. What was not known until
2026-09-23 is what the framebuffer's alpha channel holds when the bright pass
reads it, and whether the pass runs at all. This page is that measurement,
and the rules it gives for writing the mask.

## The bloom runs every race frame

A logged, non-halting read watchpoint on `g_bloom_scratch_a` (`0x08a84cf8`),
whose only per-frame reader is `Bloom_Draw` (its list builders read it once,
at construction), counted **121** reads at each of `0x08907630` and
`0x08907724` in two seconds of a single race. The positive control, a write
watchpoint on the player's clock `craft+0x830`, counted **121**. So
`Bloom_Draw` runs once per race frame, and there is no switch: the
constructor is called unconditionally from `FUN_0888adcc`, on
`Game_MainLoop`'s path. Confidence 90.

## Alpha is the stencil, and blending never writes it

The GE keeps the stencil in the framebuffer's alpha. PPSSPP's software
rasteriser (`GPU/Software/DrawPixel.cpp`, `DrawSinglePixel`) writes
`new_color |= stencil << 24` on both the blended and the unblended path: the
blend equation produces RGB only, and the alpha byte is the stencil after the
stencil op. With the stencil test off, the stencil is the old value, so
alpha is kept. The hardware backends say the same in words
(`GPU/Common/GPUStateUtils.cpp`, `ConvertBlendState`): "we shouldn't do any
blending in the alpha channel as that doesn't seem to happen on PSP", and
"Retain the existing value when stencil testing is off."

So on the original a surface reaches the mask **only through a stencil op**,
and what it writes is the stencil reference, not its own alpha. That settles
the question `Graphics::bloom`'s doc comment left open: an additive
`SrcAlpha`/`One` alpha blend accumulates a mask the original never
accumulates.

## The mask, read out of EDRAM

PPSSPP's software renderer writes emulated EDRAM, so `memory.read` at
`0x04000000` returns the real framebuffers. Talon's Junction, single race,
Assegai stationary on the grid after GO, the displayed buffer (`fb1`, 480 x
272 at stride 512):

| Alpha | Pixels | Where |
| ---: | ---: | --- |
| 4 | 125,008 | everything else, sky included |
| 92 | 2,028 | the lit panels overhead, `walls002_sb_GLOW` |
| 254 | 1,680 | the `nuricom` banner, `hub_banner_GLOW` |
| 255 | 852 | the neon strips along the track edges and wall lights |
| 0 | 830 | behind the nozzle |
| 247 | 126 | the hull's blink lights, `colours_flashing_GLOW` |
| 175 | 20 | the start-line laser, `startline_laser_ADD_GLOW` |
| 63 | 16 | an advert, `Harimau2_Glow` |

The frames are `~/.cache/oag/drive/reports/pulse-bloom/vram-grid-fb1-*.png`.
The buffer being drawn (`fb0`) had only the track in it and read **4** on
every one of its 130,560 pixels, so the frame clear leaves 4 too.

## The three rules that reproduce it

Read in `Gfx_BuildBatchStateList` (`0x0891f890`, the per-batch state list)
and `FUN_089307b4` (the batch set's recorder), and each checked against the
table above:

1. **An opaque or alpha-tested batch stamps `g_display+0x1178`.** For
   `pass_mask & 0x700 == 0` without `& 0xc0` the state list sets
   `StencilFunc(ALWAYS, *(g_display + 0x1178), 0xff)`,
   `StencilOp(KEEP, KEEP, REPLACE)` and enables the test. The byte read live
   is **4**. Its writer is not read. Confidence 85.
2. **A batch with `pass_mask & 0xc0` stamps its texture's glow byte.** The
   state list sets ref `0xff` and REPLACE for it whatever its blend class,
   and `FUN_089307b4` then sets the ref to `*(texture + 0x9c) + 0x1e`, which
   is **byte `+0x1e` of the `Texture` node's own payload**. Every value in the
   table matches that byte: `walls002_sb_GLOW` `0x5c`, `hub_banner_GLOW`
   `0xfe`, `rp_tunnel_lights2_GLOW` `0xff`, `colours_flashing_GLOW` `0xf7`,
   `startline_laser_ADD_GLOW` `0xaf`, `Harimau2_Glow` `0x3f`. The laser's two
   batches are transparent, so a transparent `0x80` batch stamps too.
   Confidence 88.
3. **A transparent batch without `0xc0` leaves the mask alone.** The state
   list disables the stencil test for it, and a disabled test keeps alpha.
   Confidence 85.

On `16_Track`, `pass_mask & 0x80` is set on 214 of 2,079 batches over 21
textures, and every one of the 21 is named `_GLOW`; no batch sets `0x40`
(`crates/vex/examples/light_rig_probe.rs`). On Assegai's hull it is one of
14 batches, the blink lights. The 2026-08-10 census on `bloom.md` counted `0x40` alone and found
none, which is why the mask looked unused.

Two effect writers sit outside the mesh path: the exhaust ribbon stamps its
ramp (`Trail_BuildStateList`, REPLACE), which is the 0 behind an idle
nozzle, and the absorb overlay stamps `0xff` (`Gfx_BuildBatchStateList(0x282)`).
The PSP boost plume's batches are `0x1232`: transparent, no `0xc0`, so the
plume **does not** write the mask.

## What this changes in the port

See `oag_render::mesh_render::GlowMask` and `Graphics::bloom`. What the port
reproduces, and what it chose, is recorded there and in
[`bloom.md`](../ghidra/functions/psp-pulse-usa/bloom.md).
