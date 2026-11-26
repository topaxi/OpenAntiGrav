---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-26
summary: "HD's roster is wired; open: score pace, AI repeats, one-live exclusions, and 2048/Omega still grant Repulser and Shuriken"
size: M (45)
difficulty: medium (45)
---

# HD's pickup draw still differs from ours beyond the roster

Evidence: [pickup-draw.md](../../docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md).

## Open

- **HD's score Eliminator runs slow on the right roster.** The Novice leader's median is 14 laps
  against the original's 9, because `oag_raceplay::score_pace` was tuned with the Repulser and the
  Shuriken in the draw. `a_novice_score_eliminator_reaches_its_target_at_about_the_originals_pace`
  runs only with `OAG_REVIEW_FINDINGS=1` until then
  ([race-modes.md](../../docs/gameplay/race-modes.md), "The pace of a score Eliminator").
- **HD's AI draw has no repeat refusal** (85, read). Ours refuses a repeat for every driver.
- **Both HD draws exclude** the LeachBeam and the Quake while any craft holds or runs one, and the
  Cannon while two craft hold one (85, read; "One-live exclusions" on the page). Ours applies none.
- **2048 and Omega still grant the Repulser and the Shuriken** in Eliminator. Both carry HD's
  11-row weapons screen and no code or tag for either manager; their draw and mask are unread.
- **HD's weapons screen toggles bits 0 to 10** for a custom or online race, with an "at least 4"
  line. This project has no such screen.
- `0x000168b0` copies the masks from an imported record whose source is unread.

## Next Steps

1. Wire the AI repeat as a `PickupRoster` field (about an hour; moves HD race draws).
2. Wire the three exclusions as `PickupRoster` data: count the craft holding or running each in
   `oag_raceplay::pads` and pass them out of the draw (a few hours).
3. Wire 2048 and Omega by lineage (`Sourced::inherited`; the value moves somewhere both can reach,
   title packages do not depend on each other), or read their draw first (static, half a day).
4. Retune `score_pace` for the score Eliminator after 1 to 3, since each moves the pace; then drop
   the test's guard (an afternoon of 24-seed sweeps; 2048 and Omega take the pace unmeasured).
