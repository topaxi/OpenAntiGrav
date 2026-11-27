---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-27
summary: "HD's roster, score pace, AI repeat and one-live exclusions are wired; open: 2048/Omega still grant Repulser and Shuriken and take Pulse's draw"
size: M (40)
difficulty: medium (45)
---

# HD's pickup draw still differs from ours beyond the roster

Evidence: [pickup-draw.md](../../docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md).

## Open

- **2048 and Omega still grant the Repulser and the Shuriken** in Eliminator. Both carry HD's
  11-row weapons screen and no code or tag for either manager; their draw and mask are unread, so
  whether they keep HD's AI repeat and one-live exclusions (wired for HD as `PickupRoster` fields)
  is open too.
- **HD's weapons screen toggles bits 0 to 10** for a custom or online race, with an "at least 4"
  line. This project has no such screen.
- `0x000168b0` copies the masks from an imported record whose source is unread.

## Next Steps

1. Wire 2048 and Omega by lineage (`Sourced::inherited`; the value moves somewhere both can reach,
   title packages do not depend on each other), or read their draw first (static, half a day),
   including whether they keep HD's AI repeat and one-live exclusions
   (`PickupRoster::ai_refuses_repeat`, `PickupRoster::exclusions`).
   In the same change move their `score_pace` to HD's terms (`Sourced::inherited` from HD): they
   keep `ScorePaceTerms::WITH_REPULSER_AND_SHURIKEN` only because our draw for them still grants
   both, and without the swap their score Eliminator slows as HD's did (median 8 to 14 laps).
