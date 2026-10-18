---
categories: [rendering]
---

# Pulse's gantry is a card now; its wrap mode and fifteen circuits are not checked against the original

2026-10-18, `gantry-pulse`: slot 8 on Pulse (PSP and PS2) draws through the model's own camera into the 128 x 128
target the track's `billboard8` quad samples, and Pulse's cards wrap in U and V. Account and evidence in
`docs/rendering/start-gantry.md`, "Pulse: the gantry is a card too".

## Open

- **The original's wrap mode is inferred.** Repeat in both axes follows from the authored UVs (U 0.875..2.0, V
  0.5..2.0 on 12 of the 16 circuits, V negated) but no GE dump read `TEXWRAP`. Dump one of those circuits (not
  Talon's Junction, whose quad is 0..1) and read it.
- **Talon's Junction and Moa Therma White (`03_Track`) were laid against the original.** The other 14 read
  `3 2 1` on an opaque panel in ours; none was captured on PPSSPP.
- **Every Pulse advert's sampling changed** (the card sampler wraps): slot 1's quad (V -2..0) now draws two
  tiles of its card. Not compared against the original; a framed shot of slot 1 was not obtained.
- **PS2 is by analogy**: not captured on PCSX2.
- **A second `billboard8` quad** on circuit `11` (75 vertices) and `14` (81 vertices, U 0..2) is drawn with the
  same card; whether it is a second board or authored debris is unread.
- **The card's alpha is 0 everywhere** (clear 0, blend writes no alpha) and the quad ignores it; if a later
  title's quad alpha-blends, that is the first place to look.

## Next Steps

1. `scripts/psp-ge-dump.py` on a `01_Track` countdown frame: read `TEXWRAP` on the prims that sample the card.
2. Capture each remaining circuit's `3` frame on PPSSPP software and lay it against `--ticks 150`.
