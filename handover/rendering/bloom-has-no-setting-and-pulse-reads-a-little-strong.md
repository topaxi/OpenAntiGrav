---
categories: [rendering]
---

# Bloom has no setting; Pulse PSP's racing strength is read, and the 1.2-1.6x is retracted

2026-10-02, `bloom-setting` lane and the maintainer's ruling the same day. The
maintainer, playing Pulse: bloom "was VERY strong". Ours was 4.4-5.7x the
original over a racing frame, fixed by per-tap byte truncation in `bloom.wgsl`
(`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours stronger than the
original?").

**Decided 2026-10-02: no bloom setting at all, only the original's.** A level
(`Off/Low/Original/High`) was proposed and declined: the original offers no
bloom option in any title, so the title's own strength is the only faithful
one. `Graphics::bloom` is removed; a settings file that still says
`bloom = false` is ignored on read (serde skips the unknown key) and the key
disappears on the next save.

## 2026-10-08, `bloom-racing`: the 1.2-1.6x is retracted

Read off the original's own scratch buffers (`scripts/psp-trace.py
--edram-every`, composite poked off, `scripts/psp-bloom-chain.py`), full account in
`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Racing strength against the
original's own scratch buffers". **Done**: the chain is exact (the model on the
original's framebuffer lands on its A; ours is the model to 1 % on 13 frames);
the old figure was a posing artefact (`--pose-boost 10` saturates a low
intensity) plus a poke-and-control subtraction on a near-zero frame. Landed:
the blur tap is `(v * (w + 1)) >> 8` and the composite truncates (both
measured), the ribbon stamps one byte per segment (`trail_stencil`, `114/102`
measured). Talon's Junction ours/original `0.96-1.08` over seven poses, Moa
Therma `1.03-1.37`, the latter following the scene's `rgb * mask` energy.
**No colour grade in Pulse PSP** (the composite is a recorded frame's last
prim). HD: Pulse's finding does not generalise; ours is weaker than the stored
matched captures.

**Open.** (1) The bright pass sampling phase: the model lands within 0.2-5 % of
the original's A and the leftover is at lit edges and the craft. (2) The
original's bright pass reads the HUD's pixels; ours does not. (3) Moa Therma
needs a pose that starts its animated emissives at the trace's own clock, or a
run that is not posed, before `1.15` can be called scene rather than a term.
(4) The ribbon shows segment 2 in ours at the chase framing and the original
only segments 0 and 1: the screen length of the ribbon. (5) Whether a racing
frame on a second circuit has any full-screen pass after the composite (one
rest frame was read). (6) The same readout on HD (RPCS3) at a racing intensity.

## Measured first

Ours was 4.4-5.7 times the original over a matched racing frame and 1.4 times
at rest, and that is fixed on this branch (per-tap byte truncation in
`bloom.wgsl`; `docs/ghidra/functions/psp-pulse-usa/bloom.md`, "Is ours stronger
than the original?"). Whole-frame mean luma the bloom adds, racing straight:
original 1.0-1.3, ours before 5.6-5.7, ours after 1.55-1.63; at rest on the grid
original 8.35, ours before 11.8, ours after 8.3. It was a wash over the whole
picture, not the exhaust, and it is resolution independent already. Ours is
still about 1.2-1.6x the original on a racing straight (1.3x around the craft),
and that residual is a fidelity gap, not a setting. The boost pad's glow is the
original's own look (same size in both). The glow-mask value of the pad is a texture byte, not the stamp.

## What runs today, per title

| Title | Bloom path | Gated by the removed `graphics.bloom` | Original reference |
| --- | --- | --- | --- |
| Pulse PSP | `post::bloom`, four passes, constants from `BOOT.BIN` | yes | always runs (121 of 121 frames) |
| Pulse PS2 | `post::ps2_bloom`, its own three passes (7 integer taps, 320x224, `127/128 * 64/128` add) over its own mask (`GlowMask::StampedByTexel`), both read off replayed GS dumps, `docs/rendering/ps2-bloom.md` | n/a | always runs (every race field) |
| Pure | none, same reason | n/a | unread |
| HD / Fury | `post::hd_bloom`, five stages, per-circuit `HDR and Bloom` values | yes, as `Glow::Drawn` or `Suppressed` (the chain still runs; only the glow term goes) | read, not measured as a player option |
| Omega | none: the patch carries no `HDR and Bloom` block | n/a | n/a |
| 2048 | none | n/a | unread |

The original exposes no bloom option in any title. A level is therefore ours by
definition, and the only faithful value is the title's own.

## Zone (2026-10-02, `zone-bloom` lane)

The maintainer found the bloom "quite obvious" in Zone. Measured on a native
Zone race (`bloom.md`, "Is ours stronger than the original in Zone?"): **ours
is 0.88-0.90x the original racing** (zones 2, 3 and 5, two boots), 1.2x on the start grid
(boot 1 and 2) from a bottom-of-frame strip that ours stamps `214` and the original leaves at
`4` (cause not identified). Zone's bloom is the original's own look: the same
pose adds 1.5 luma on the ordinary circuit and 13 on Zone's, and the original's
mask is 13-18 % at >= 200. No code change.

## The bloom draws over the HUD (2026-10-04, `pulse-bloom-roll`)

Closed. The original's queue puts `Bloom_Draw` (`0x70`) after every HUD widget
(`0x52`..`0x6d`). With the composite poked off, the HUD's glyphs over a glowing
panel lose about 50 luma per channel. Ours now prepares the bloom after the
scene and composites it after the HUD (`Scene::composite_bloom`). Evidence:
`docs/ghidra/functions/psp-pulse-usa/bloom.md`, "The bloom draws over the HUD".
Not reproduced: over a glow surface, the original blooms the HUD widget's own
colour (its bright pass reads the HUD). **Pulse PS2 inherits the order
(2026-10-04, maintainer's call, under the rule that a source with no measured
rule takes Pulse PSP's)**: `Ps2Bloom` splits into prepare and composite the same
way, unmeasured on the PS2 disc (`ps2-bloom.md`). **HD keeps its old order,
pending the maintainer**: its read resolve (`downsamplescaleaddfeedback_fp`)
adds the bloom to the exposure-scaled scene *before* the encode, so a HUD under
the bloom is not a reorder but a choice - the HUD exposure-scaled in the linear
target, or the bloom added after a nonlinear encode - and neither is the
executable's.

## Open

- Why ours is still 1.2-1.6x on a racing straight (candidates: the flare and
  plume's drawn size and mask ramp at 85 km/h; not varied here).
- A matched boost-pad on/off pair in the original (1 of 4 placed approaches
  triggered the pad in this lane).
- HD's own strength at matched poses: this lane measured Pulse PSP only.

- ~~Does Pulse PS2's original bloom at all?~~ **Yes**, and **ported** (2026-10-02):
  the mask (a glow batch's own alpha, zero elsewhere) was read back off replayed GS
  dumps and the chain (320x224, 7 integer taps, half-strength add) is
  `post::ps2_bloom`. Matched on Moa Therma's grid at slot 8, ours adds 1.06x the
  original's mean luma (1.62 against 1.53). Racing craft box (pose not matched): ours 0.84 against 0.40 luma, 2.1x, mask matching. Open: **one circuit and a grid frame**
  - Talon's Junction was not reached and no racing frame has a recoverable pose; the
  ghost craft stamps nothing on PS2; `ATST NOTEQUAL 6` and whether the HUD groups
  before the downsample take the composite are read from registers only. See
  `docs/rendering/ps2-bloom.md`.

## Next Steps

0. Zone grid strip: find which `zone_track.vex` batch stamps `214` across rows
   231-243 at the countdown (`oag-view --draws` plus node isolation) and why
   the original's mask is `4` there (depth tie of a coplanar `GEQUAL` decal is
   the candidate). Only the 1.2x grid residual depends on it.

1. Explain the residual 1.2-1.6x on a racing straight (flare and plume mask
   size at speed) before touching any constant. Re-measure it first: since
   2026-10-04 the composite comes after motion blur and the HUD, so a number
   taken before then mixed in the old order.
2. PS2 bloom: a second circuit (Talon's Junction) and a racing frame at a pose read out of the emulator, for the racing-straight strength the PSP chain still carries 1.2-1.6x on.
3. Measure HD's bloom strength at matched poses on RPCS3.
