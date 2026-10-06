# The HUD's message lines raise one chosen event; the original's eight have no trigger

2026-10-06, branch `hud-medal`. Evidence:
[hud-messages.md](../../docs/ghidra/functions/psp-pulse-usa/hud-messages.md),
[hud.md "Medal messages"](../../docs/ui/hud.md#medal-messages-2026-10-06).

**State.** Pulse's four message lines (`Info1`-`Info4`, `oag_hud::messages`) are
drawn with the original's decompiled slot law; a campaign Zone or Speed Lap
raises "<Tier> medal awarded" through them the first time a tier is reached
(chosen, not measured - the original shows no mid-race medal).

## Open

- **A live frame of the slot law.** The four-second fade, the 0.8 s gate and the
  colours are decompiled only. Capture any `IG_HUD_PLAP` / `NEWLAP_REC` /
  `FIN_LAP` line (a Speed Lap lap, a personal-best lap) on PPSSPP and compare.
- **The negative is static.** No live frame of a campaign Zone or Speed Lap
  crossing a medal threshold: a fresh profile has those cells locked. Unlock one
  (write the cell record) and watch `hud+0xe0` and the slot busy bytes.
- **The `MESSAGE` cue** the original plays as a line appears, and the
  `gold_med`/`silver_med`/`bronze_med` jingles. `MessageBoard::just_shown` is
  the hook; the cue is not routed to a mixer. The jingles are unplayed even on
  the end-of-race screen.
- **The eight events with no trigger**: `IG_HUD_PLAP`, `NEWLAP_REC`, `FIN_LAP`,
  `CONT_ELIM`, `PERF_ZONE`, `PERF_BOOST`, `NEW_ZONE_RECORD`, `NEW_SCORE_RECORD`,
  and the `0x200` queue at `DAT_08b32d40`. The flag word writers
  (`*(hud+0x3c)+0xe0`) are the thing to find.
- **Pure's phrase.** Pure authors the same four widgets and ships `HUD_Perfect
  Zone!`/`HUD_New Zone Record` and `zone_bronze/silver/gold` voice lines: the
  one lead that Pure announces a medal in Zone. `message_slots` is on (Pulse's
  law) but Pure's table has no `ER_GMA`, so nothing draws; find Pure's own phrase
  and its executable's routine. Whether Pure launches a campaign cell at all was
  not checked.
- **HD** composes the same slots in every race layout and draws the phrase from
  its own table; the HD campaign trigger (`campaign_cell` is title-blind) was not
  exercised end to end. Its line is hard to read on a bright backdrop.
- **2048** (`RaceMedal` was never seen in any state reached, `2048-hud.md`) and
  Omega (racing out of scope): checked, applies, not wired.
- **Elimination** (kills against gold) would raise through the same hook.

## Next Steps

1. Find the writers of `*(hud+0x3c)+0xe0` (xrefs on `+0xe0` stores in
   `PlayerStatus_Update` and the lap code) and wire `PLAP`/`NEWLAP_REC`/`FIN_LAP`.
2. Route `just_shown` to the `MESSAGE` cue.
3. Read Pure's HUD message routine and turn `message_slots` on there.
