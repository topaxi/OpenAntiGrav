---
categories: [rendering]
---

# Bloom has no setting, and Pulse PSP still reads 1.2-1.6x the original on a straight

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

1. Explain the residual 1.2-1.6x on a racing straight (flare and plume mask
   size at speed) before touching any constant.
2. PS2 bloom: a second circuit (Talon's Junction) and a racing frame at a pose read out of the emulator, for the racing-straight strength the PSP chain still carries 1.2-1.6x on.
3. Measure HD's bloom strength at matched poses on RPCS3.
