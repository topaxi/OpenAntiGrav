# What raises the HUD's message lines, and the lap-line time gap

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read in Ghidra
2026-10-12 (`pulse-hud-messages` lane). Companion to
[hud-messages.md](hud-messages.md), which has the slot law this page feeds.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x08842a18` | `Craft_UpdateLapProgress` | 92 | Already named ([race-progress.md](race-progress.md)). Three of the flag word's writers live here: bits `1`, `2`, `4`. |
| `0x08819f30` | `Hud_BindPlayerStatus` | 80 | Already named. `*(hud+0x3c) = Registry_Lookup("PLAYER_HUD")` and a refcount bump; called once from `Race_CreatePlayer` (`0x0882821c`). This is the writer of the "view" pointer: the status block, not a per-frame object. |
| `0x08819b64` | `Hud_Construct` | 80 | Zeroes `hud+0x3c`/`+0x40`, `+0xa4`-`+0xa6`, `+0x124`, the four slots' busy bytes, `+0xe0`; sets `+0xb8 = "- %.1f"` and `+0xbc = "+ %.1f"` (strings at `0x08a79c10`/`0x08a79c18`). |
| `0x0881f890` | `Hud_UpdateTimeDiff` | 85 | The lap-line time gap: binds `TimeDiffText` (`hud+0x20c`) and `TimeDiffIcon` (`+0x210`, bound in `Hud_BindWidgets` when `hud+0x40 & 0x200` is clear). Decompile and disassembly read. |
| `0x08808bec` | `Profile_IsNewLapRecord` | 80 | `time_cs < Profile_GetStoredLapRecord() && time_cs != 0`. |
| `0x088090a0` | `Profile_GetStoredLapRecord` | 75 | The profile's stored best lap in centiseconds for the track and class (`profile+0x464`/`+0x460`), at a per-mode offset: modes `3`, `4`, `14`-`16` read `+0x348`, `5` and `17` (Time Trial) `+0x288`, `10` (Speed Lap) `+0x408`; every other mode, and a track with no row, `0`. |

## The flag word's writers: the whole census

A store census of the program (every `sw`/`lw` against offset `0xe0` that is
followed by `ori`) finds exactly these writers of `*(hud+0x3c)+0xe0`, and no
others in code that runs in a race:

| Bit | Line | Writer | Condition |
| --- | --- | --- | --- |
| `0x1` | `IG_HUD_PLAP` | `Craft_UpdateLapProgress` `0x08842ff0` | The **player's** craft completes a lap, `craft+0x860 & 0x20` is clear (no wall hit since the last completed lap), and the craft has not finished (`+0x912`). |

Which contacts spoil a lap (2026-10-14): the bit is armed in `FUN_088418e0`'s loop for **every** contact record with positive combined friction while no shield absorbs it ([contact-response.md](contact-response.md), reaction #2; [shield-pickup.md](shield-pickup.md)), with no inbound-speed or impulse-size test. Ours feeds `oag_raceplay`'s lap watch from `WallResponse::resolved` (any reacting contact), not `impact` (inbound edge only), so a sustained scrape counts. In a network race the watch follows `Race::player_slot` and skips reconciliation replays (chosen, not measured). Confidence 80.

**Craft-to-craft contact spoils a lap too (2026-10-14, static read of the pair path, confidence 82).** `Collision_BoxAgainstBox` (`0x0881702c`) writes the pair contact's `+0x34` friction as colliderA's `+0x64` (`0.02` on a racing craft, `Ship_SetColliderFriction`) unless either collider's is negative, so it is positive for two racing craft (`0x08817a84`-`0x08817b10`). `Body_ResolveContactPair` (`0x0884ef30`), once its separating gate passes, calls `Body_RecordPairContact` (`0x0884dc60`) for **both** bodies with that friction in `f12`, appending to the same eight-entry ring (`body+0x370`) at `+0x1a0` with a kind word of `1` at `+0x170`. `FUN_088418e0`'s loop does not read the kind word for the gate: `0.0 < record+0x1a0`, craft state 1 and no shield (`entity+0x1b8 & 0x10`) arm `0x20 | 0x400020` on `craft+0x860`, passing `kind != 0` as `Ship_Damage`'s third argument. So bumping a rival spoils **both** craft's laps, shield permitting. The record is written in the physics step and read by the next tick's craft update, which is how ours feeds it (`RaceView::pair_spoil`, set by `Race::resolve_craft_pairs`, read by `Race::observe_lap`). No live leg (no watchpoint on `craft+0x860` during a bump): a static single-binary read. The racing-craft friction branch (`0.0` friction for class `6` without the `0x08ab07e3` byte) is unmeasured; with `0.0` friction the gate would not fire. Pure and HD: **not checked**, listed open (their `mp-time-trial-collision.md` pages cover the pair path, not this bit).

| `0x2` | `IG_HUD_NEWLAP_REC` | same, `0x08842f74` | Player, not finished, `Profile_IsNewLapRecord(last_lap_cs)` and `craft+0x930 == craft+0x92c` (this lap is the race's best so far). |
| `0x4` | `IG_HUD_FIN_LAP` | same, `0x08843224` | Player, not finished, after a lap completes `craft+0xac8 == g_race_laps`: the start of the final lap. |
| `0x8`/`0x10` | `IG_HUD_CONT_ELIM` green/red | `Ship_Damage` `0x088439ac`, at `0x08843ef8`/`0x08843ee4` | Mode `8` (Elimination) only: a craft is destroyed by a weapon (`param_3 == 2`) of a different craft; the attacker's byte `+0x48` clear gives the green line, set the red. **Not wired**: what `+0x48` means is read at confidence 60 (a guess: the AI flag), below the bar; the address is recorded for a lane that reads it. |
| `0x20`-`0x100`, `0x200` | `PERF_ZONE`, `PERF_BOOST`, `NEW_ZONE_RECORD`, `NEW_SCORE_RECORD`, the queue | **none found** | Drained by `Hud_UpdateMessages` and never set by any code in Pulse's race path (no `lw`/`ori`/`sw` on `0xe0` in `Zone_Update`, the Zone mode functions, `Race_*` or the boost code). Pulse announces a perfect zone through the voice and the end screen, not these lines. `0x200` pops `DAT_08b32d40`, whose pushers (`FUN_08966d90`/`FUN_08966ddc`) are `Ship_Damage`'s multiplayer kill feed, modes `>= 14`. Confidence 75 (a census, not a live watch). |

Facts that are easy to get wrong, all read off the decompile:

- **None of the three fires on the finishing crossing.** `+0x912` is set at
  `crossings == g_race_laps + 1` before any of the tests.
- **The "hit a wall" bit is cleared only when a lap completes**, not on the
  first crossing, so lap 1's verdict includes the drive from the grid to the
  line. It is set on the collision-damage path and not on the shield pickup's
  (`ShipShield_Hit`, [shield-pickup.md](shield-pickup.md)).
- **A record needs a record to beat.** A stored `0` makes
  `Profile_IsNewLapRecord` false, so the first lap ever driven on a row never
  announces one, and neither does any mode with no stored lap (Zone, Free Play,
  Elimination, Head2Head).
- The three are drained in bit order, `PLAP` then `NEWLAP_REC` then `FIN_LAP`,
  each through the 0.8 s gate of the slot law.

## The announcer voices (2026-10-16, `hud-lap-cues` lane)

**Pulse plays `PERFECT_LAP` and nothing else beside the lap lines.** `FUN_08829778`
(`0x08829778`, per-frame race-manager update, unnamed: its role is read, not
named) plays `Sound_PlayNamedInSlot(DAT_08ac1e10, DAT_08ac1dfc, "PERFECT_LAP",
0x400, 0, 0, 0)` at `0x088299c0` while `*(craft+0x860) & 0x200000` is set and the
race state `+0x7c8 == 2`. `Craft_UpdateLapProgress` clears that bit at its first
line every tick (`& 0xffdfffff`) and sets it at `0x08842ff0` together with the
HUD's bit `0x1`, so the voice is **one tick, the crossing tick**, with the line.
`DAT_08ac1dfc` is the mode's speech slot (`speech.bnk`, `speech_elim.bnk` or
`speech_zone.bnk`: the slot `ready`/`go` use, [countdown-voice.md](countdown-voice.md)).
Confidence 85. The cue is 0.955 s of speech after an authored 0.697 s lead-in
(timeline delay, `oag-wad sounds`: two waveforms, left and right).

- **`NEW_LAP_REC` is in all three Pulse speech banks and no code names it**: the
  string census finds `IG_HUD_NEWLAP_REC` and `IG_HUD_LAP_REC` and no
  `NEW_LAP_REC` in `BOOT.BIN`. Pulse's new-lap-record line is silent. 80 (a
  string census, not a live watch).
- **`FINAL_LAP` is voiced too, outside this lane**: the same function plays
  `"FINAL_LAP"` (once, flag `+0x1986`) on `speech_results.bnk`'s slot
  `DAT_08ac1e00` when the lap count reaches the last; so do `RACE_COMPLETE`,
  `TOURN_COMPLETE`, `SESS_COMPLETE` at the finish. Open, not wired.
- Wired as `oag_title::lap_hud::LapVoices` (`perfect_lap: Some("PERFECT_LAP")`,
  `new_lap_record: None`), `Cue::PerfectLap`, raised in
  `oag_raceplay::Race::raise_lap_messages` on the crossing tick. Disc-backed:
  `lap_messages_ground_truth::pulse_voices_a_perfect_lap_on_the_crossing_tick_and_never_a_record`
  renders the WAV with and without the voice and finds speech only where the bank
  authors it (0.70 s after the cue). The seat a split-screen voice rides on is the
  seat whose line it is (chosen, not measured).

## The Elimination `CONT_ELIM` line: the attacker byte (2026-10-16)

`Ship_Damage` (`0x088439ac`), on a craft's shield reaching zero with `param_3 == 2`
(a weapon) in mode `8`: `attacker = (&DAT_08b31788)[victim->node->0x13c]`, the race
manager's craft table indexed by the victim node's last-attacker slot; if it is not
the victim itself (`!= victim+0x360`) it adds one to `attacker+0x8d8` (the kill
count) and raises HUD bit `0x8` (green) when `attacker+0x48 == 0`, else `0x10`
(red). **The line is raised on the player's HUD for any kill by any craft**, not
only the player's.

`craft+0x48` has one writer, `Craft_Construct_q` (`0x08840c74`), from its role
argument (the value stored at `+0x368`): `0x08841098` / `0x088410d0` store `0` for
roles `0` and `1`, `0x088410f0` stores `1` for roles `2` and `3`, nothing for `4`
and above (confidence 90, the disassembly). So **green is "the attacker is not
role 2 or 3"**. Whether roles 2 and 3 are the AI-driven craft and role 1 a second
human or ghost is the open half (60: `+0x368 == 0` is the local player on seven
uses, [autopilot.md](autopilot.md); the roles above `0` are unlabelled). The
line is **left unwired**: wiring green for the player's kill and red for an AI's
is the likely reading and not settled to 70. HD: `Ship_ApplyDamage_q`
(`0x000e84d0`/`0x000e8488`) not read in this pass.

## The lap-line time gap (`Hud_UpdateTimeDiff`), confidence 85

The maintainer saw it as "a periodic gap to the racer behind, green when you
lead, or in front, red". It is `TimeDiffText` plus `TimeDiffIcon`, **not** an
`Info` line, and it is once per lap.

The table: `g_race_manager + crossings * 0x18 + 0xc0` holds, per crossing
count, the race clock **truncated to a whole second** (`cvt.s.w` reads it back,
`0x08842dcc`) of the first craft over, `-1` before, with a player and an
opponent byte ([race-progress.md](race-progress.md#the-split-time-table)). The
second craft over a line, when one of the two is the player and neither
crossing is a finishing one, raises the gap `now - first` (`now` the float
clock): `+0x55 = 1`, `+0x4c` the gap, `+0x56` `0` when the player crossed
second, `1` when the player crossed first. The player crossing first raises
`+0x54` and the stamp.

The display (`Hud_UpdateTimeDiff`):

- the player leads (`+0x54`) and two or more craft are live
  (`Race_CountLiveCrafts`): `"%.1f"` of `clock - stamp`, colour `0xff60ff60`,
  text only, while under 30 s - a count-up from the player's own crossing, the
  argument read from the disassembly (`sub.s f12, f13, f12` at `0x0881f984`);
- the gap event: `"- %.1f"` in green (player ahead) or `"+ %.1f"` in red
  (player behind) held for 3.5 s (`> 3.5` ends it), the icon tinted as the text,
  flashing at 5 Hz (`(int)(t * 5) & 1`) between `0xff10c010`/`0xff30ff30`
  (ahead) and `0xffc01010`/`0xffff3030` (behind); drawn only for `|gap| < 30`;
  the green one also ends when `PLAYER_HUD+0x10` (the place, as
  [hud-kill-column.md](hud-kill-column.md) reads it) is no longer `1`.

**Which racer.** The gap is to **the first craft over that crossing**, not the
one directly ahead or behind; behind the leader it is the leader, and the gap
to a car one place ahead is not shown. That differs from the maintainer's
description and is the code's rule. The leader's stamp is a whole second, so a
gap reads up to a second long.

## Where it is built

`oag_hud::time_gap` (the display machine, unit-tested), `oag_raceplay::split_gap`
(the table, fed by every craft's `Standing`), `oag_raceplay::lap_messages` (the
three lines), switched on by `Race::watch_laps` from both game loops.
Disc-backed: `crates/game/tests/lap_messages_ground_truth.rs`.

## Other titles

Each is now read in its own executable and wired as data
(`oag_title::lap_hud::LapHud`, 2026-10-12, `hud-messages-port` lane):

- **Pure**: [../psp-pure-usa/hud-message-triggers.md](../psp-pure-usa/hud-message-triggers.md).
  Checked, differs: `HUD_<English>` keys (the earlier "its table lacks the ids" was
  a spelling), white text, the gap as a sentence, no icon, no place stop, six
  crossings. Its lines are on.
- **HD**: [../ps3-hdfury-eu/hud-message-triggers.md](../ps3-hdfury-eu/hud-message-triggers.md).
  Checked, differs: its own bit numbering with medal bits, a backing frame per line,
  the new-lap-record line off in Time Trial, the count-up alone in Time Trial and
  Speed Lap, no place stop, **three gaps a lap**.
- **2048** and **Omega**: [../vita-2048-eu-v104/hud-message-triggers.md](../vita-2048-eu-v104/hud-message-triggers.md),
  [../ps4-omega-eu/hud-message-triggers.md](../ps4-omega-eu/hud-message-triggers.md).
  Their drains read as HD's: ported, with the gap routine (Omega read, 2048 unread)
  and the split table (both unread) chosen from HD at one split a lap.

What the maintainer described as a gap "to the racer directly ahead or behind" is,
on every title read, the gap to the **first craft over that split**.
