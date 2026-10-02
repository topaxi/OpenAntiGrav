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
| Pulse PS2 | none: the glow mask is not measured there (`scene.rs`: `measured_mask`); the original does bloom, see `docs/rendering/ps2-bloom.md` | n/a | 7-tap, 320x224, half-strength add |
| Pure | none, same reason | n/a | unread |
| HD / Fury | `post::hd_bloom`, five stages, per-circuit `HDR and Bloom` values | yes, as `Glow::Drawn` or `Suppressed` (the chain still runs; only the glow term goes) | read, not measured as a player option |
| Omega | none: the patch carries no `HDR and Bloom` block | n/a | n/a |
| 2048 | none | n/a | unread |

The original exposes no bloom option in any title. A level is therefore ours by
definition, and the only faithful value is the title's own.

## Zone (2026-10-02, `zone-bloom` lane)

The maintainer found the bloom "quite obvious" in Zone. Measured on a native
Zone race (`bloom.md`, "Is ours stronger than the original in Zone?"): **ours
is 0.89-0.90x the original racing** (zone 2 and zone 3), 1.2x on the start grid
from a bottom-of-frame strip that ours stamps `214` and the original leaves at
`4` (cause not identified). Zone's bloom is the original's own look: the same
pose adds 1.5 luma on the ordinary circuit and 13 on Zone's, and the original's
mask is 13-18 % at >= 200. No code change.

## Open

- Why ours is still 1.2-1.6x on a racing straight (candidates: the flare and
  plume's drawn size and mask ramp at 85 km/h; not varied here).
- A matched boost-pad on/off pair in the original (1 of 4 placed approaches
  triggered the pad in this lane).
- HD's own strength at matched poses: this lane measured Pulse PSP only.

- ~~Does Pulse PS2's original bloom at all?~~ **Yes** (2026-10-02, confidence
  90): a GS dump of the grid and of a racing frame shows five passes per field
  (320x224 downsample masked by frame alpha, 7-tap H and V blur, weights
  16,32,32,64,32,32,16, additive composite at half strength). The frame-rate
  hypothesis is falsified. See `docs/rendering/ps2-bloom.md`. Open: the PS2
  glow mask (frame alpha) is still unmeasured, so ours still draws none there.

## Next Steps

0. Zone grid strip: find which `zone_track.vex` batch stamps `214` across rows
   231-243 at the countdown (`oag-view --draws` plus node isolation) and why
   the original's mask is `4` there (depth tie of a coplanar `GEQUAL` decal is
   the candidate). Only the 1.2x grid residual depends on it.

1. Explain the residual 1.2-1.6x on a racing straight (flare and plume mask
   size at speed) before touching any constant.
2. Measure the PS2 glow mask (read frame alpha out of a GS dump VRAM) so the PS2 path can bloom; the original does. Circuit: Talon's Junction too.
3. Measure HD's bloom strength at matched poses on RPCS3.
