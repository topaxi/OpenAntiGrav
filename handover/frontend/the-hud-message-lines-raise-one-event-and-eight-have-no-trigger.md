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
for Zone and every Speed Lap but HD's, which raises per best lap with its voice (2026-10-16).
Announcer voices: `oag_title::lap_hud::LapVoices`.

## Open

- **Per-seat lap lines landed 2026-10-14** (one board and watch per local slot; every seat
  compares against the primary's stored best, chosen). Still open from it: the PS2 split's own
  message drain per view is unread; the split gap and pace stay primary-only; a craft bump
  spoiling a lap is a static read on Pulse PSP (`Body_RecordPairContact`, 82) with no live
  watchpoint on `craft+0x860`, and Pure and HD are unchecked.
- **A live frame of the slot law and of a lap line** on PPSSPP (Pulse, Pure) and
  RPCS3 (HD): the flash phases, the 3.5 s hold, the 0.8 s gate, Pure's outline
  staying at full alpha while the text fades (read off one decompile, 70).
- **HD's Speed Lap trophy**: the medal line and its `gold_med_shrt`/`silver_med_shrt`/
  `bronze_med_shrt` voice are wired as measured (2026-10-16, `LapHud::speed_lap_medal`,
  every new race-best lap inside a tier). Still open: the 3D trophy
  (`SpeedLapGoldMedal`/`Silver`/`Bronze` `<ImageModel>`s, `hd_gold.vex`, 208 x 150 at
  (1600, 200)); the HUD crate has no `<ImageModel>`.
- **Announcer voices landed 2026-10-16**: `PERFECT_LAP` on Pulse (`FUN_08829778`,
  `craft+0x860 & 0x200000`) and HD, `NEW_LAP_REC` on HD only (Pulse's bank carries it and
  no code names it); Pure's bank has neither. Open: 2048 (bank carries both, hashed; which
  spelling is chosen by the unread `FUN_81000930(&DAT_8153fc40,-1)` global), Omega (Wwise
  event names, no `SBlk` path), HD's gate byte `0x00938560`, Pulse's `FINAL_LAP` /
  `RACE_COMPLETE` voices from the same function (`speech_results.bnk` slot).
- **`CONT_ELIM`**: Pulse `Ship_Damage` (`0x088439ac`) credits `(&DAT_08b31788)[victim
  node+0x13c]` and raises green when its `+0x48` is `0`, red otherwise; `+0x48` is written
  once, in `Craft_Construct_q`, as `0` for roles 0-1 and `1` for roles 2-3 (90). What roles
  2 and 3 are (AI-driven) is 60, so it stays **unwired**; HD `Ship_ApplyDamage_q`
  (`0x000e84d0`/`0x000e8488`) and 2048 unread. The line goes to the player's HUD for any
  kill by any craft. See Pulse's `hud-message-triggers.md`.
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
3. Draw HD's Speed Lap trophy model (needs an `<ImageModel>` in `oag-hud`).
4. Settle what `Craft_Construct_q`'s roles 2 and 3 are (60 today), then wire `CONT_ELIM` in
   Elimination: green for a human attacker, red otherwise.
5. 2048's perfect-lap / record voices: read the `FUN_81000930` global, then hashed lookup.
