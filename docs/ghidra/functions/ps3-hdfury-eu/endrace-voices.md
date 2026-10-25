# HD / Fury end-of-race voices: the place line and the medal line

2026-10-11, lane `hd-results-voice`. `/hdfury/EBOOT-ps3-hdfury-eu.elf`, decompiled headless from a scratch copy of the
project (the bridge was not used); string sites found with `scripts/ps3-toc.py`'s TOC-correct scan (all functions here
read TOC `0x008ad4d8`, `exact`). Pulse's side is [menu-sounds.md](../psp-pulse-usa/menu-sounds.md#the-end-of-race-voices).

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x0022e3d0` | `EndRaceResults_OnEnter` | 78 | The `EndRace Results` screen's entry method. Plays `FIRST_PLACE`/`SECOND_PLACE`/`THIRD_PLACE` |
| `0x0021ea10` | `EndRacePhoto_ShowMedal` | 80 | Unhides the earned `EndRacePhotoMedal*` widget and plays `gold_med`/`silver_med`/`bronze_med` |
| `0x0021ec60` | `EndRacePhoto_OnEnter` | 72 | The `EndRace Photo` screen's entry method; calls `EndRacePhoto_ShowMedal` |

`0x0022e3d0` sits in `EndRaceResults_Screen.cpp`'s vtable (`0x00868ae8`, beside `0x0022e680`, the per-frame method; the
`.cpp` attribution is `scripts/ps3-toc.py map`) and nothing calls it directly. It runs `0x0016cd60` first, which
increments a counter at `+0xc4` that `0x0022e680` tests (`+0xc4 == 0` falls back to the base), points `+0xb34` at
`RaceManager + 0x1980` (the result block) and builds the grid and the loyalty ticker: one-shot set-up, hence "entry".
The 78 is that reading; it was not seen running. `0x0021ea10` is in `EndRacePhoto_Screen.cpp` (between its constructor
`0x0021e790` and `EndRacePodium`'s `0x0021eed8`).

## The place line

All three plays are `Sound_QueueAnnouncerCue(*0x008b08d8, *0x008b08dc, name, 1)`: manager `0x00b6e300`, bank slot
`0x00b6cc18`. The front-end bank is `0x00b6cc14` and the boot order is `frontend`, `speech_results`
([sound-bank-loads.md](sound-bank-loads.md)), so `0x00b6cc18` is `speech_results`. The last argument is a priority class
(`0` or `1`; above that the cue is not queued): the place line is the higher.

It plays, once, on entry, when **all** of these hold (confidence 85, a direct read):

1. `g_GameState + 0x90 == 0`: no campaign cell in play (the medal's side tests `!= 0`, below).
2. `*(RaceManager + 0x13e8) + 0x7002 != 0`: a byte of the local craft, **meaning not read**.
3. `*0x009384e1 == 0`: the byte the loading screen and `AI_UpdateCraft` also read, **meaning not read**; `0` on every
   capture ([loading-screen.md](loading-screen.md)).
4. `g_GameState + 0xe0` (the mode) is in `{3, 4, 8, 9, 11, 16, 20}` (the mask `0x110b18`). Named by
   [mode-manager.md](mode-manager.md): `3`/`9` `SPArcade`, `4` `SPTournament`, `8` `SPElimination`, `16` `MPArcade`,
   `20` `MPElimination`; `11` has no manager class. 
5. `byte(g_GameState + 0xe4) < 2`: one local player.

The place is `word 0` of the result block, `1` plays `FIRST_PLACE`, `2` `SECOND_PLACE`, `3` `THIRD_PLACE`, anything else
nothing. **Unlike Pulse, there is no standings flip**: `0x0022e680` (the Update) handles Tournament paging (`mode 4`
or `0x11`) and calls no announcer cue; `Sound_QueueAnnouncerCue`'s callers are `0x0022e628`, `0x0022e64c`, `0x0022e670`
here and none in `0x0022e680`. So a Tournament leg speaks the leg's place on entry.

## The medal line

`EndRacePhoto_ShowMedal`, called by `EndRacePhoto_OnEnter` and nothing else. Hides all three `EndRacePhotoMedal*` widgets,
then, **only under `g_GameState + 0x90 != 0`** (a campaign cell), reads the medal ordinal at `result + 0x1350`: `0`
gold, `1` silver, `2` bronze, else nothing. It shows the widget and plays `gold_med`/`silver_med`/`bronze_med` with
priority `0`, once per race: the guard is the byte `result + 0x135c`, set after the play, in the RaceManager's result
block rather than in the widget, so entering the screen again does not repeat it. Confidence 85.

**The place and the medal line are mutually exclusive on HD**: `+0x90 == 0` against `+0x90 != 0`. Pulse speaks both in a
cell.

## Which waveform

None is chosen. Each cue is one timeline of two grains: `FIRST_PLACE` cmd 13..14 keys the pair at tick 200 (angle 30) and
205 (angle 330) of the PS3 bank's 240 Hz clock; `gold_med` the same; `gold_med_shrt` 200 and 206. The two waveforms of a
cue have the same length (24224/24224 for `FIRST_PLACE`), so they are the left and right of one recording, not
alternates (a one-off read of the `FIRST_PLACE` and `gold_med` cues). The line therefore starts
**200 ticks (0.83 s) after the trigger**, authored in the bank. Confidence 90 (a direct read of the command list).

**The Fury style**: neither site reads `FrontEnd_IsFuryStyle`, so there is no Fury spelling and no style choice. 85.

## Not recovered here

- `EndRacePhoto`'s place in the flow. HD's redirects are data-driven and the port's HD flow has no photo screen
  ([hd-endrace-screens.md](../../../formats/hd-endrace-screens.md)), so the port plays the medal on the first screen it
  shows, `EndRace Results`: **chosen, not measured**.
- Which mode id a free Racebox Single Race and a campaign cell carry (a cell logged `3`), the meaning of the two bytes
  above, and what `result word 0` is in an Eliminator (the port uses the finishing place; the Pulse reading is the
  player's rank in the kill table).
- Other cues in the same bank with call sites elsewhere: `gold_med_shrt`/`silver_med_shrt`/`bronze_med_shrt` from
  `0x000e4a30`; `RACE_COMPLETE`, `TOURN_COMPLETE`, `SESS_COMPLETE`, `FINAL_LAP` from `RaceManager_Update`
  (`0x0005e948`). Not read.
- A live run. Everything above is static: confidence 85 at best and no PPSSPP-style breakpoint backs it.

## What the port plays

`oag_title::ResultsLaw` carries the difference (`TournamentPlace::OnEntry`, `place_in_campaign: false`,
`MedalScreen::Results`) as HD's data, Pulse's being `StandingsFlip`, `true`, `Rewards`. The place needs a Single Race,
an Eliminator or a Tournament; the port's modes carry no id `9`, `11`, `16` or `20`. The mix is dry and on the speech
bus, as Pulse's (**chosen, not measured**); the original queues the line behind any announcer line still playing, the
port starts it at once. `endrace_voice_ground_truth.rs` renders the line and asserts silence for the authored lead and
energy after it.

## Omega

**Checked, differs / applies, not wired.** `eboot.bin` (base and patch) carries `gold_med`, `gold_med_shrt` and
`gold_medium` as strings and **no** `FIRST_PLACE`; the front end plays Wwise events with no name path
([game-boot.md](../ps4-omega-eu/game-boot.md)), so nothing is wired. The Fury style: same answer as HD, no style choice.
