# The HUD's message lines: lap lines and the split gap run on five titles; the rest is open

2026-10-06 `hud-medal`, 2026-10-12 `pulse-hud-messages` and `hud-messages-port`.
Evidence, one page per executable:
[Pulse](../../docs/ghidra/functions/psp-pulse-usa/hud-message-triggers.md),
[Pure](../../docs/ghidra/functions/psp-pure-usa/hud-message-triggers.md),
[HD](../../docs/ghidra/functions/ps3-hdfury-eu/hud-message-triggers.md),
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/hud-message-triggers.md),
[Omega](../../docs/ghidra/functions/ps4-omega-eu/hud-message-triggers.md);
the slot law is [hud-messages.md](../../docs/ghidra/functions/psp-pulse-usa/hud-messages.md),
[hud.md "Medal messages"](../../docs/ui/hud.md#medal-messages-2026-10-06).

**State.** Every title's lap-crossing lines (a perfect lap, a new lap record, the
final lap) and the time gap are **data**: `oag_title::lap_hud::LapHud` on
`HudArt::lap_hud`, each field read in that title's own executable.
`oag_raceplay::{lap_messages, split_gap}` and `oag_hud::{messages, time_gap}` run
the machine; a title with no `lap_hud` raises nothing. What differs: Pulse and HD
`IG_HUD_*` ids green or red, **Pure `HUD_<English>` keys drawn white** with a gap
worded as a sentence and a six-crossing table, **HD a backing frame per line, no
record line in Time Trial, the count-up alone in Time Trial and Speed Lap, no place
stop and three gaps a lap**, 2048 and Omega HD's with one split a lap (chosen). The
gap is to the **first craft over the split**, not the craft directly ahead or
behind, on every title read. The chosen medal line (`oag_game::medal_watch`) stays
for Zone and Speed Lap.

## Open

- **Per-seat lap lines landed 2026-10-14** (one board and watch per local slot; every seat
  compares against the primary's stored best, chosen). Still open from it: the PS2 split's own
  message drain per view is unread; the split gap and pace stay primary-only; a craft bump
  spoiling a lap is a static read on Pulse PSP (`Body_RecordPairContact`, 82) with no live
  watchpoint on `craft+0x860`, and Pure and HD are unchecked.
- **A live frame of the slot law and of a lap line** on PPSSPP (Pulse, Pure) and
  RPCS3 (HD): the flash phases, the 3.5 s hold, the 0.8 s gate, Pure's outline
  staying at full alpha while the text fades (read off one decompile, 70).
- **HD's Speed Lap medal** (measured, `Craft_UpdateLapProgress` `0x000e553c`): the
  line is raised on every lap that is a new race-best and inside a tier, with a 3D
  trophy (`SpeedLapGoldMedal`/`Silver`/`Bronze` `<ImageModel>`s, `hd_gold.vex`,
  208 x 150 at (1600, 200)) and the voice `gold_med_shrt`. `medal_watch` raises on a
  tier improvement and draws no trophy; the HUD crate has no `<ImageModel>`.
- **Announcer cues** the original plays with the lines: `PERFECT_LAP`,
  `NEW_LAP_REC` (HD `0x91d20`; 2048 `RACE_Perfect_lap`/`RACE_New_lap_record`;
  Omega `RACE_Perfect_lap`) and `cont_elim` on a kill. `Cue::ContElim` exists for a
  wrecked opponent; the other two have no `Cue`.
- **`CONT_ELIM`**: Pulse `Ship_Damage` (`0x088439ac`, attacker byte `+0x48` read at
  60), HD `Ship_ApplyDamage_q` (`0x000e84d0`/`0x000e8488`), 2048 unread. Wire it in
  Elimination once the attacker byte is settled.
- **HD**: the cap widening of a line wider than 240 px and the retro skin's halved
  alpha (`hud+0x6a0`); what `hud+0x44 & 0x20` is (voice-only) and what the profile
  byte `0x009384e1` is (its clear state is the ordinary race); the writers of
  `IG_HUD_INVALID_LAP` (`0x1000`, `0x00070178`), the queue (`0x2000`,
  `0x00368a88`) and `IG_HUD_TARGET_ATTAINABLE` (`0x4000`, `0x0003f6d8`,
  `0x0006d958`), three lines Pulse does not have.
- **2048 and Omega**: 2048's gap routine and both titles' split tables (are the
  laps split in thirds as on HD?); 2048's `MSC_RACE_ENDS` notice and
  `HUD_Message_Ship_Destroyed`; their lap-function writers are assumed HD's.
- **Unraised bits** (`PERF_ZONE`, `PERF_BOOST`, `NEW_ZONE_RECORD`,
  `NEW_SCORE_RECORD`): no setter on Pulse, Pure or HD (a census, 75). Pure's
  `HUD_Perfect Zone!` and `HUD_New Zone Record` are strings it ships and never
  raises.
- **Elimination** (kills against gold) would raise through the same hook.

## Next Steps

1. Capture a lap line on PPSSPP (Pulse and Pure) and RPCS3 (HD) and compare.
2. Read 2048's gap routine and the split tables of 2048 and Omega.
3. Wire the HD Speed Lap medal as measured (per-lap trigger), then the trophy model.
4. Read `Ship_ApplyDamage_q`'s attacker byte and wire `CONT_ELIM` in Elimination.
5. Add the `PERFECT_LAP`/`NEW_LAP_REC` announcer cues through the `Cue` table.
