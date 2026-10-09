---
categories: [rendering, gameplay]
---

# HD's Zone, Zone Battle and Detonator start boards are measured and not drawn

2026-10-08, `hd-gantry`. HD's slot 8 is now a 512 x 256 card (`Adverts::gantry_card`), which
fixed the halo that read as a blurry countdown. The per-mode boards are the part left.

## Open

- **The mode picks the file, measured:** `GetMode()` 6 (Zone), 13 (Zone Battle), 14
  (Detonator) each show their own board before the release; 3 and 8 show `3 2 1 GO`. Same ids as the
  PS4 allowlist. Evidence and pictures: `docs/rendering/start-gantry.md`, "the gantry is a card".
- **`321Go_Zone.vex` as a card draws a blank white panel** (both archive copies). The picture to match is a
  dark panel with a loop icon assembling from light pieces, 210 ticks before the release. Not found: which of `DATA00` (8.4 KB, bars) and `DATA02` (2.6 KB,
  `cf_321_zone` + `321_go_64_zone2.gtf`) the game loads, and why ours draws the model empty (the card's clear is
  transparent and the model's own dark background panel may be a draw ours drops).
- **`oag_race::Mode` has Zone only.** Zone Battle and Detonator need modes before their boards can be chosen.
- HD's own code site (which function fills the name) is unread; the only references to the four names
  are the hash registration loop at `0x003f1300`.
- ~~Ours is about 1.6x brighter than the original on every surface~~ - answered 2026-10-08 by `hd-exposure`: the
  display `pow(1/2.2)` is the ROP's encode and the original's exposure is unity, so it is not an extra gamma step; the
  gap is per surface (ours 0.84x at Talon's grid, 1.54x in the corridor) and sits in the lightmap term. See
  `hds-frame-was-too-bright-and-too-bloomy.md`.

## Next Steps

1. Dump the slot-8 card of a Zone race (`rpcs3-drive.py place --hook scripts/rpcs3_draw_hook.py` after
   `--nav "Single Player=right,right,right,right,right,cross"`) and name its textures against the three
   `321go` `.gtf` (the head samples alone are ambiguous: dump 5 offsets and compare all).
2. Make a Zone card draw something in ours, then add `zone_gantry` back to `Adverts` (it was written and removed).
