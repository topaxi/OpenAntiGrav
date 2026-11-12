---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-12
summary: "2048 profile XP total is kept and a finished event banks its race-end awards; the hidden-event reveal reads the rank; Omega events and the ~30 in-race awards are open"
---

# 2048 profile XP and rank

Landed: `oag_tables::xp` parses `XPAwards.xml` / `XPRanks.xml` (`Awards`, `Ranks`: `rank` is
`Profile_GetXPRank`, `threshold_xp` is `Profile_GetRankThresholdXP`, `meets` is
`GameModeBase_IsUnlockable`'s rank test). `oag_game::records::Store` keeps one `xp` total per
profile (shared by 2048 and Omega's 2048 mode, maintainer ruling; an old `records.toml` without
it reads 0). `Store::bank_event_xp` is called at the 2048 finish site in `Session::frame` and pays
the completion award plus `Finished Nth`, in the event kind's column. `Store::rank_reveals` is
the query the campaign map calls.

## Decided

- **Rank under a shared total**: each title ranks the shared XP total by its own `XPRanks.xml` (Omega's is half 2048's), so the same total is a higher rank in Omega (maintainer, 2026-11-12; built that way).

## Open

- **Omega's dispatch** between its loyalty block and its XP block on `EndRace Results` is not traced (50): `FUN_0153ebf0` builds the loyalty ticker, `FUN_0149f910` the XP blocks; no caller found for either (`docs/ghidra/functions/ps4-omega-eu/xp-and-loyalty.md`).
- The rank bar and `RaceXP` / `TotalXP` widgets are not drawn on 2048 or Omega.
- The two rank trophies (rank 25 and 50, fired from `Profile_BankRaceXP`) are not awarded.
- 2048 still pays inherited HD loyalty and Omega loyalty only; neither is replaced by XP.

- **Reveal wiring**: landed; `oag_game::boot::reveals2048` asks `Store::rank_reveals` at boot (the
  map is only ever the boot's front end, so XP banked by a race shows at the next launch).
- **Unread, labelled chosen, not measured**: whether `Event complete` pays on every finish or only
  the first; whether Elite pays instead of Pass or as well (we pay Elite instead); whether
  `Finished Nth` pays in every mode; the original's mode to column map (we use race for race and
  Speed Lap, elim, zone by `EventKind`). `FUN_812b4872` (the race-end hook) does not show them:
  the awards are issued before it, from the event objects.
- **When the original promotes a hidden event to progress 1** is unread (`Campaign_UnlockEvents`
  promoted none on map entry at rank 50).
- **Omega events do not bank**: `race::load_event` opens 2048 only, so Omega's campaign has no
  `Campaign2048Progress`. The total is shared and Omega's tables parse (ground truth), so wiring
  is the Omega event launch carrying the same progress fields. Omega ranks use Omega's own
  `XPRanks.xml` (half thresholds) against the shared total.
- The ~30 in-race award sites (barrel roll, perfect lap, kills, pads, zones, chains, air time,
  evading, held 1st) pay nothing yet.
- The EndRace screens' `CO_RANK` / `CO_TOTAL_XP` widgets and the rank-up overlay.

## Next Steps

1. In-race awards: add trigger sites in the tally, summed into `bank_event_xp`'s award.
2. Omega: find the end-of-race function that banks the XP total (xref the rank-table global the loader fills) and diff its caller against `Race_ComputeLoyaltyAward`'s callers; then carry `xp_awards` / `xp_column` in its event launch.
