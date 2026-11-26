---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-26
summary: "HD's roster is wired; open: AI repeats, one-live exclusions, weapons toggles, and 2048/Omega still grant Repulser and Shuriken"
size: S (45)
difficulty: easy (40)
---

# HD's pickup draw still differs from ours beyond the roster

Evidence: [pickup-draw.md](../../docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md).

## Open

- **2048 and Omega still grant the Repulser and the Shuriken** in Eliminator. Both carry HD's
  11-row weapons screen and no code or tag for either manager; their draw and mask are unread.
  Ready to wire by lineage: `pickup_roster: Some(Sourced::inherited(oag_hd::pickups::ROSTER, "HD"))`
  (the value would move to a shared place, title packages do not depend on each other).
- **HD's AI draw has no repeat refusal** (85). Ours refuses a repeat for every driver.
- **Both HD draws drop LeachBeam, Quake and Cannon while enough are live** (counts at
  `*(TOC-0x30e0)`, `-0x30dc`, `-0x30d8`). What the counts are is unread.
- **HD's weapons screen toggles bits 0 to 10** for a custom or online race, with an "at least 4"
  line. This project has no such screen.
- `0x000168b0` copies the masks from an imported record whose source is unread.

## Next Steps

1. Wire 2048 and Omega (about an hour), or read their draw first (static, half a day).
2. Wire the AI repeat for HD as a `PickupRoster` field (about an hour; moves HD race draws).
3. Read the three counts' writers, then apply the exclusions (static, a few hours).
