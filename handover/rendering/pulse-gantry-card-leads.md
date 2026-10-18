---
categories: [rendering]
---

# Pulse's gantry card: sampler measured; three leads left

2026-10-18, `gantry-pulse` then `pulse-card-verify`: slot 8 on Pulse (PSP and PS2) draws through the model's own
camera into the 128 x 128 target the track's `billboard8` quad samples, wrapping in U and V. The wrap is **measured**
(`TEXWRAP 0`, filter `0x107`, one level, on ten PSP circuit loads and two boots) and nine circuits were laid
against ours with no defect found. Account in `docs/rendering/start-gantry.md`, "Pulse's gantry card against the
original".

## Open

- **A framed shot of a two-tile advert quad** (raw V 0..2, e.g. `13`'s 32-vertex prim) on the original and ours: the
  register says it tiles, the picture was not framed (the countdown camera does not reach it).
- **The second `billboard8` draw on `14`** (81 indices at 378.8, 90.5, -273.9): a dump taken with the craft near it
  decides board versus debris; `psp-drive.py place` did not hold the pose on `14` (back at the grid). Circuit `11` is
  not offered by the original's Track Select.
- **`11` in ours** loads with a "44.6 x 643.7" panel extent and the board hangs over the grid; unreachable on the
  original, so a look-only item.
- **PS2 is by analogy**: not captured on PCSX2, its `TEXWRAP` equivalent unread.
- **Final lap / chequered** on a second circuit.

## Next Steps

1. `14`: reach a pose near the second quad (a `place` along `14`'s own forward spline, or a save state mid-lap) and `census` the dump.
2. PCSX2 on one circuit, reading the GS sampler state of the card draw.
