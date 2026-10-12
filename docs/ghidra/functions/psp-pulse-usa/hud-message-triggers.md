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

- **Pure**: checked, differs. The table has none of the three ids (Pure keeps
  its strings under `HUD_`-prefixed keys), so the lines stay off there rather
  than raise a `MESSAGE` cue for words nothing draws; the gap runs under
  Pulse's law (Pure's own routine unread, the rule inherited as for every
  unmeasured title) and draws the text without the icon, which its layout does
  not author.
- **HD**: checked, applies, wired by inheritance. Its table carries
  `IG_HUD_PLAP`/`NEWLAP_REC`/`FIN_LAP`, so they raise under Pulse's law; its own
  routine is unread. 2048 has no message slots: not wired. Omega: not
  checkable (racing out of scope).
