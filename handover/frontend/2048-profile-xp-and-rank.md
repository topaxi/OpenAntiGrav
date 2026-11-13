---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-13
summary: "2048 XP: tally banks 26 of 78 awards, rank gates reveals; Omega --event banks too; Omega map, 52 awards, rank UI open"
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
- **Omega events bank** (`oag_title::Campaign::event_track_dir`, `omega_event_xp_ground_truth`):
  `load_event` opens any title that ships `SP.xml` events (2048, Omega), so an Omega `--event`
  carries Omega's `XPAwards.xml` and banks into the shared total; ranks use Omega's own
  `XPRanks.xml` (half thresholds). Paying XP there is chosen, not measured (dispatch unread).
  Open: Omega's front end cannot launch an event (its 2048 map `Campaign2048_Definition.xml` is
  unread, `RACE CAMPAIGN` stays on the chooser), so no Omega map gates a hidden event by rank and
  only `--event` reaches this path.
- **In-race awards** (`oag_raceplay::xp_tally`): kills, zone / perfect zone, Hit Zone N, held 1st
  and the race-end awards pay through one tally with the exact chain rule. The other awards stay
  unwired, each with its address and the unread condition, in the table on
  `docs/ghidra/functions/vita-2048-eu-v104/xp-award.md` ("In-race awards"). Unread: the gate
  `FUN_8100d2a6`'s mode ids (`DAT_8153fd24 == 2`, `DAT_8151d438`); whether `GameMode`'s vfunc
  `+0x80` (XP enabled) is true for every campaign event; whether our `zone_dirty` is the original's
  `0x60a4 & 0x1000000`; the elimination test for held 1st is our column, not `IsEliminatorFamily`.
- The EndRace screens' `CO_RANK` / `CO_TOTAL_XP` widgets and the rank-up overlay.

## Next Steps


1. In-race awards: read the next unwired trigger (pilot flags `+0x60a4` bits `0x20` and
   `0x1000000`, `+0x6128`, `+0x6f3c`'s initial value, the weapon-type case map of `0x811c42d2` /
   `0x811bd214`) and add it to `XpTally`; Speed pad and Perfect lap are the cheapest.
2b. The `RaceXP` popups and rank bar (HUD), `CO_RANK` / `CO_TOTAL_XP` on the EndRace screens.
2. Omega: read the map `Campaign2048_Definition.xml` (`FE3DCanvas`/`linkedevent`) into the campaign
   map, launch its events through `load_event`, and call `Store::rank_reveals` against Omega's ranks;
   separately, xref the rank-table global the loader fills to find which Results block pays XP.
