---
status: measure
titles: [pulse]
updated: 2026-10-19
summary: "Frame a two-tile advert quad on 13 and the second billboard8 quad on 14; PS2 FINAL LAP card on PCSX2"
categories: [rendering]
---

# Pulse's gantry card: sampler measured; leads left

2026-10-18, `gantry-pulse` then `pulse-card-verify`: slot 8 on Pulse (PSP and PS2) draws through the model's own
camera into the 128 x 128 target the track's `billboard8` quad samples, wrapping in U and V. The wrap is **measured**
(`TEXWRAP 0`, filter `0x107`, one level, on ten PSP circuit loads and two boots) and nine circuits were laid
against ours with no defect found. Account in `docs/rendering/start-gantry.md`, "Pulse's gantry card against the
original".

## Open

- **A framed shot of a two-tile advert quad** (raw V 0..2, e.g. `13`'s 32-vertex prim) on the original and ours: the
  register says it tiles, the picture was not framed (the countdown camera does not reach it).
- **The second `billboard8` quads** (`14`: 81 indices at 378.8, 90.5, -273.9; `11`: 75 indices) are decided from the asset
  as repeater panels (`docs/rendering/start-gantry.md`, 2026-10-18 `pulse-mount-11`); a framed shot beside one on the
  original is still open (`place` respawns the craft off the track).
- **PS2 advert quads past 0..1**: PS2 gantry card measured on PCSX2 (2026-10-19; offscreen 128 x 128, CLAMP, linear, decal, no defect), but the sampler is clamp and the PS2 advert quads' UV extents were not surveyed.
- **Final lap / chequered** on a second circuit.

## Next Steps

1. A framed shot of a second quad on `14`: a `place` on a spline point (`Spline::nearest` gives one) or a save state mid-lap.
2. PCSX2 (`OAG_PCSX2_DISPLAY`, `gsdump`): FINAL LAP/chequered on PS2, and a circuit with advert quads to read their UV and clamp.
