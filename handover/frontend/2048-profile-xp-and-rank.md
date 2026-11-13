---
status: ready-to-wire
titles: [2048, omega]
updated: 2026-11-13
summary: "2048 XP: tally pays 46 of 78 awards (completion on improvement); Vita 2048 cannot return to its map after a race; Omega map open"
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
- **Landed on 2048**: the end-of-race XP block (`RaceXP`, `PassBonusXP`, `TotalXP`, rank badge and
  bar, the count-up and the rank-up flip) draws off the race's banked tally and `XPRanks.xml`
  (`docs/ghidra/functions/vita-2048-eu-v104/endrace-summary.md`, "The XP block";
  `a_finished_event_shows_what_it_paid_and_the_rank_bar`). Omega's own `EndRace Results` authors the
  same widgets in HD's layout and is not wired (`endrace_entry` is `None`; `FUN_0149f910` unread).
- **Completion pays on improvement** (landed): `bank_event_xp` takes the best before the race;
  Pass pays when no pass stood, Elite over a lesser best, both for an elite first pass
  (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`, "Completion pays on improvement"). The
  improvement test itself is the page's branch (70); the award call sites were not re-read for it.
- `FUN_81131100` (`xprank`/`xpbartext`/`xpbar`, bar `* 300`) and `FUN_810c8286` (rank card with
  `CO_RANK`/`CO_NEXT_RANK`/`CO_TOTAL_XP`) are not the end-race page; their host screens are not
  found (no `xprank` in the seven `NEWGUI` definitions read). Wire them once a host screen is.
- The two rank trophies (rank 25 and 50, fired from `Profile_BankRaceXP`) are not awarded.
- 2048 still pays inherited HD loyalty and Omega loyalty only; neither is replaced by XP.

- **Reveal wiring**: landed; `oag_game::boot::reveals2048::refresh` asks `Store::rank_reveals` every
  time a map is built from the live profile (boot, and each opening of the hosted map), so XP banked by
  a race reveals on the next build. **Vita 2048 has no way back to its map after a race** (see
  `handover/frontend/2048s-end-race-pages-draw-the-rest-is-named.md`, "The way back to the campaign
  map"), so there it shows at the next launch until that lands.
- **Unread, labelled chosen, not measured**: whether `Finished Nth` pays in every mode; the original's mode to column map (we use race for race and
  Speed Lap, elim, zone by `EventKind`). `FUN_812b4872` (the race-end hook) does not show them:
  the awards are issued before it, from the event objects.
- **Hidden-event promotion, read (80)**: `Campaign_UnlockEvents` runs at race end (after the XP is
  banked) and when the map builds its buttons; a non-chain event at progress 0 whose rank threshold
  and required event pass goes to 1 if hidden, else 2 (`docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`,
  "When a hidden event is promoted"). Ours derives it at each map build, which gives the same result
  wherever a map can be reached after a race. Live confirmation open: write the XP total
  before the first map entry of a Vita3K boot and read a side challenge's `+0x2d8` (expect 1).
- **Omega events bank** (`oag_title::Campaign::event_track_dir`, `omega_event_xp_ground_truth`):
  `load_event` opens any title that ships `SP.xml` events (2048, Omega), so an Omega `--event`
  carries Omega's `XPAwards.xml` and banks into the shared total; ranks use Omega's own
  `XPRanks.xml` (half thresholds). Paying XP there is chosen, not measured (dispatch unread).
  Open: Omega's front end cannot launch an event (its 2048 map `Campaign2048_Definition.xml` is
  unread, `RACE CAMPAIGN` stays on the chooser), so no Omega map gates a hidden event by rank and
  only `--event` reaches this path.
- **In-race awards** (`oag_raceplay::xp_tally`): kills, zone / perfect zone, Hit Zone N, held 1st,
  the race-end awards and now Perfect lap (5), Weapon pad (8), Boost start (11), Hit with
  Rocket/Missile/Plasma/Cannon/Mine/Quake/Bomb (12-18), Taken 1st-3rd (21-23), Shielded against the
  same weapons except Quake (32-36, 38) and Full health at the line (44) pay through one tally.
  Hit with Leach Beam (19) and Shielded against Leach Beam (39) are unwired (the link drains every
  tick; whether the original pays per tick is unread); Quake's shield award (37) cannot fire (a wave
  meeting a shield reports nothing); Light Barrier (20, 40) is not a weapon this engine has; 45-47 need
  split checkpoints (three a lap, positions unread). The table on `xp-award.md` ("In-race awards")
  has every row. Unread: the gate `FUN_8100d2a6`'s mode ids (`DAT_8153fd24 == 2`,
  `DAT_8151d438`); whether `GameMode`'s vfunc `+0x80` (XP enabled) is true for every campaign event;
  whether our `zone_dirty` is the original's `0x60a4 & 0x1000000`; the elimination test for held 1st
  is our column, not `IsEliminatorFamily`; whether the lap's damage flag is a health drop in every
  case (ours) and where in the tick the original orders the lap test against damage.
- **Omega**: the in-race feeds hang off the shared simulation, so an Omega `--event` race pays them
  (ported; only award 8 was checked against Omega's own call sites). Omega's map and rank UI stay open.

## Next Steps


1. In-race awards still unwired (address and recipe on `xp-award.md`): N seconds ahead (45-47)
   needs split checkpoints (positions unread: read `Race_RecordSplit`'s split table first); Leach
   Beam hit and shield (19, 39) once the per-tick question is read; barrel roll (4), speed pad (7,
   `+0x6128`), air time (58), Successful Turbo (71), cannon clip (72), three rockets (73), evades
   (74-76), mine and bomb destroyed (63-64), no wreck for the entire event (62). Never paid in v1.04,
   do not wire: 59-61, 65-70, 77. Trophies at rank 25 and 50: this engine has no trophy mechanism.
2b. The in-race `AwardXP` `+N` popup (HUD, `Hud_UpdateXpLines`), and the two readouts above once a host screen is found.
2. Omega: read the map `Campaign2048_Definition.xml` (`FE3DCanvas`/`linkedevent`) into the campaign
   map, launch its events through `load_event`, and call `Store::rank_reveals` against Omega's ranks;
   separately, xref the rank-table global the loader fills to find which Results block pays XP.
