---
status: ready-to-wire
titles: [omega, 2048]
updated: 2026-11-09
summary: "Omega's and 2048's 3-2-1-GO start gantry cards: extend HD's existing gantry path, check against shadPS4 and 2048 frames"
categories: [rendering]
---

# Omega's and 2048's start gantry countdown cards

2026-11-09, drive lead, a lane the maintainer asked for. No measurement has been taken yet.

## Open

- Omega's start gantry 3-2-1-GO countdown cards are not drawn and phased the way HD's are. HD's gantry
  board is a card drawn in the scene and phased by the gantry clock
  ([start-gantry.md](../../docs/rendering/start-gantry.md), [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md)).
  The "Omega card route" is listed open in `hd-lit-billboard-specular-and-gantry-leftovers.md` in this directory.
- **Reuse, do not rebuild (maintainer):** HD/Fury definitely wires much of this, and 2048 may. Start from
  the existing HD and 2048 path and extend it to Omega.
- **2048 in the same lane (maintainer):** whatever lands for Omega is wired for 2048 too. The doc page
  records "ported", unless there is evidence 2048 differs.
- Omega's advert/material route (`omega-low-hanging-from-hd.md` in `handover/tooling/`, item 4) may carry
  the card's material. If so, do this after it.

## Next Steps

1. Census: which gantry and card entries Omega and 2048 ship, against HD's. Find what the existing HD
   path already reads for each title.
2. Wire Omega, then 2048, through that path.
3. Evidence: a shadPS4 Omega frame of the countdown (`scripts/shadps4-omega-pair.py`, audio muted),
   and a 2048 frame or 2048's own data as evidence. Name the pair and box whose error must drop.
