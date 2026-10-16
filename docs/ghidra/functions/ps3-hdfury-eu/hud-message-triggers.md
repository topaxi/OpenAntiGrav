# What raises HD's HUD message lines, and its lap-line time gap

**Binary:** `ps3-hdfury-eu` `EBOOT.elf`, Ghidra `/hdfury/EBOOT-ps3-hdfury-eu.elf`.
Read 2026-10-12 (`hud-messages-port` lane), companion to Pulse's
[hud-message-triggers.md](../psp-pulse-usa/hud-message-triggers.md) and
[hud-messages.md](../psp-pulse-usa/hud-messages.md). Everything here is below
`0x32d5e0`, where Ghidra's own TOC is the right one; every `lwz rN,disp(r2)` was
resolved with `scripts/ps3-toc.py resolve <function> -0x<disp>` and reads
`exact`. Disassembly was cross-checked with `llvm-objdump -d --triple=powerpc64`.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x00091d20` | `Hud_UpdateMessages` | 85 | Drains the flag word at `*(hud+0x40)+0xf4` into four slots; called once a tick from `Hud_Update` (`0x0009e3d0`). The same machine as Pulse's `Hud_UpdateMessages` (`0x0881f148`), with its own bit numbering. |
| `0x000907b0` | `Hud_UpdateTimeDiff` | 85 | The gap display, called from `Hud_Update`. Pulse's `Hud_UpdateTimeDiff` with three differences, below. |
| `0x00093c48` | `Hud_UpdateWrongWay` | 85 | Drives the `WrongWay` widget (`hud+0x20c`) and the `WRONGWAY` cue off `status+0x54`. Named so nobody reads `+0x20c` as `TimeDiffText` again. |
| `0x000e4a30` | `Craft_UpdateLapProgress` | 78 | The per-tick spline progress, line crossing and lap bookkeeping of one craft: the writer of the lap and medal bits. Ghidra's own body for it stops at `0x000e4e7b` (an inline jump table at `0x000e4c3c` defeats the flow); the real function runs to `0x000e6060`, the next OPD entry. Read from the disassembly. |
| `0x000d06e0` | `Craft_RecordSplit` | 80 | `(race manager, split index)`: stamps the split table and raises the gap flags. Called twice from `Craft_UpdateLapProgress`: `0x000e5068` at a split inside the lap and `0x000e5a38` at the line (index `0`). |
| `0x000e7760` | `Ship_ApplyDamage_q` | (existing) | Raises `CONT_ELIM`'s two bits. Not renamed here. |
| `0x001e25c0` | (HD's `Cell_EvaluateMedal`) | (existing) | See [race-campaign.md](race-campaign.md). Called from the Speed Lap medal block below. |

## Which modes: ids 5 and 10

`GameState+0xe0` is the mode ([mode-manager.md](mode-manager.md#the-mode-enum-22-ids-eleven-of-them-named)).
Ids `5` and `10` both construct `SPTimeTrial` there, and the campaign cell's own
ordinals (`cell+0x110`, [race-campaign.md](race-campaign.md)) number `Zone` as `6`,
the lineage's numbering that Pulse's `g_game_mode` uses (Time Trial `5`, Speed Lap
`10`, Zone `6`). So `5` is Time Trial and `10` Speed Lap here too (80): two ids on
one manager is exactly Speed Lap riding the Time Trial class. Every rule below keyed
on `5` or `10` rests on that.

## The flag word's bits: HD numbers them differently from Pulse

`Hud_UpdateMessages` reads `uVar6 = *(*(hud+0x40)+0xf4)` and clears it. The
string table slots are TOC `0x008a7854`-`0x008a789c` (`ps3-toc.py resolve
0x00091d20 -0x5c84` and onward), and each push allocates `strlen+1` bytes
(`FUN_00676988`), which agrees with every id's length.

| Bit | Line (language-table id) | Good byte | Pulse's bit for the same line |
| --- | --- | --- | --- |
| `0x1` | `IG_HUD_PLAP` | 1 | `0x1` |
| `0x2` | `IG_HUD_NEWLAP_REC` | 1 | `0x2` |
| `0x4` | `IG_HUD_FIN_LAP` | 1 | `0x4` |
| `0x8` | `IG_HUD_CONT_ELIM` | 1 | `0x8` |
| `0x10` | `IG_HUD_CONT_ELIM` | 0 | `0x10` |
| `0x20` | `ER_GMA` | 1 | none: Pulse has no medal line |
| `0x40` | `ER_SMA` | 1 | none |
| `0x80` | `ER_BMA` | 1 | none |
| `0x100` | `IG_HUD_PERF_ZONE` | 1 | `0x20` |
| `0x200` | `IG_HUD_PERF_BOOST` | 1 | `0x40` |
| `0x400` | `IG_HUD_NEW_ZONE_RECORD` | 1 | `0x80` |
| `0x800` | `IG_HUD_NEW_SCORE_RECORD` | 1 | `0x100` |
| `0x1000` | `IG_HUD_INVALID_LAP` | 0 | none |
| `0x2000` | a string popped off a queue (`FUN_00677548`..`77578`), its own good byte | queue's | `0x200` |
| `0x4000` | `IG_HUD_TARGET_ATTAINABLE` | 1 | none |

**HD does announce a medal in these lines**, which Pulse's routine does not
(`hud-messages.md`'s "no medal here" is Pulse's alone). `IG_HUD_INVALID_LAP`
and `IG_HUD_TARGET_ATTAINABLE` are HD's own additions; 2048 carries both too.

## The slot law is Pulse's, with four differences

The gate, the sentinel and the curve are the same numbers, read through TOC
slots: gate `0x3f4ccccd` (0.8 s, `-0x1757`), the waiting sentinel `0x42c80000`
(100.0, `-0x17c5`), `pi` (`-0x1787`), `1.0`/`0.0` (`-0x17c7`/`-0x17c8`). Alpha is
`sin(pi * (1 - (t/life)^6))`, the same sixth power. Life is `4.0`, or `8.0` when
`GameState+0xe0 >= 0x10` (Pulse: `>= 0xe`). Green `0x30ff30` for a good line,
red `0xff3030` for a bad one. The differences:

1. **Retro HUD skins.** `hud+0x6a0` (set from the mode in {13, 21, 8, 20, 14};
   [hud-readouts.md](hud-readouts.md)) halves the alpha (`128`, not `255`) and
   draws a bad line white. Not modelled.
2. **A backing frame per line.** `hud_info_text.xml` authors `FrameMiddle1`
   (240 x 95, `HUD_Components.gtf`), `FrameLeft1`, `FrameRight1` under each
   `Info1`-`Info4`, and the routine writes the three frames' vertex colours with
   the line's own alpha byte every tick; when the text is wider than the 240 px
   middle it moves the caps out to `+/-` half the text width. **The previous page
   claimed HD draws its lines with "no backing"; that was wrong** and is the
   reason a bright backdrop washed them out. Wired in
   `oag_hud::messages::backing` (the caps are drawn where the layout puts them;
   the widening is not done). `TimeDiffText` has its own `FrameMiddle`,
   `FrameLeft`, `FrameRight` children (`hud_time_diff.xml`), up with the text.
3. **Voice cues.** A line is accompanied by an announcer cue through
   `Sound_QueueAnnouncerCue`: `PERFECT_LAP` for `0x1`, `NEW_LAP_REC` for `0x2`,
   `cont_elim` for `0x8` only. All are gated on a global byte (`0x00938560`,
   `g_PhysicsHalfStep`) being clear. **Wired 2026-10-16** (`hud-lap-cues`):
   `Cue::PerfectLap` and `Cue::NewLapRecord`, names in
   `oag_title::lap_hud::LapVoices`, both in HD's own `speech.bnk` (`PERFECT_LAP`
   0.87 s, `NEW_LAP_REC` 1.24 s, no authored lead-in), raised on the crossing tick
   with the line (so no record voice in Time Trial, where the line is off). The
   gate byte is not modelled (its meaning is unread). `cont_elim` stays unwired
   (Elimination, the attacker byte).
4. **A voice-only branch.** `hud+0x44 & 0x20` turns the whole drain into
   "`0x1` plays `PERFECT_LAP` and nothing is drawn". What sets that bit is not read
   (a second local player is the obvious reading; 40).

The `MESSAGE` cue plays as a slot starts showing (`FUN_002ffa58` with the id at
TOC `-0x1712`), as on Pulse.

**`IG_HUD_NEWLAP_REC` is skipped in Time Trial.** `0x2` runs only when the
profile byte `0x009384e1` is set **or** `GameState+0xe0 != 5`. The byte is read by
about 80 functions (weapon managers, `RaceManager_Update`, the loading screen)
and its meaning is unread; the repository already records `0` as the ordinary
race ([weapons.md](weapons.md), and `oag_hd::loading`), so in an
ordinary Time Trial the line is not raised. 80 for the branch, 60 for what the
byte is.

## Who sets the bits: a census, and the lap function

A scan of the whole disassembly for `lwz rA,0xf4(rB)` ... `ori rA,rA,N` ...
`stw rA,0xf4(rB)` finds these writers. The ones that go through `lwz r,0x40(hud)`
first are the `PLAYER_HUD` status block's; the rest are other structs that share
the offset.

| Bit | Writer (function, store) | Condition |
| --- | --- | --- |
| `0x1` | `Craft_UpdateLapProgress` `0x000e5250` | local craft (`+0x628c == 0`), `+0x6a64 & 0x20` clear, not finished (`+0x7002` clear); sets craft flag `0x200000` too |
| `0x2` | same, `0x000e5d30` | `0x21198(profile, lap_cs)` true (a stored record to beat), `+0x7024 == +0x7020` (this lap is the race's best), not finished |
| `0x4` | same, `0x000e5b00` | `+0x7810 == GameState+0xc` (the crossing count equals the lap count: the start of the last lap), local craft, not finished |
| `0x20`/`0x40`/`0x80` | same, `0x000e58b8` / `0x000e5d9c` / `0x000e5e24` | the Speed Lap medal block, below |
| `0x8`/`0x10` | `Ship_ApplyDamage_q` `0x000e84d0` / `0x000e8488` | Elimination, a weapon kill; green or red by the attacker |
| `0x1000` | `0x00070178` `0x000703bc` | not read |
| `0x2000` | `0x00368a88` `0x00368c3c` | not read |
| `0x4000` | `0x0003f6d8` `0x0003fb64`, `0x0006d958` `0x0006e0cc` | not read |
| `0x100`-`0x800` | **none found** | no `ori` of `256`/`512`/`1024`/`2048` into `+0xf4` anywhere; the same negative as Pulse's (75, a census and not a live watch) |

Other `+0xf4` read-modify-writes at `0x00411158`, `0x00411d40` (function
`0x00410f70`) and `0x003ff40c`, `0x0040cdf4` (`0x003fdae8`, `0x0040aba0`) sit in the
`0x008bd3c4` TOC region where Ghidra's TOC is wrong and they do not step through
`+0x40`: another struct, not `PLAYER_HUD`'s. Not chased.

The lap time in `+0x7020` is `(int)(seconds * 100.0)` (`0x42c80000` at TOC
`-0x4848`): **centiseconds**, the unit the campaign cells' `Gold`/`Silver`/`Bronze`
targets are authored in. The conditions for `0x1`, `0x2` and `0x4` are
Pulse's, field for field (`+0x860 & 0x20` there, `+0x6a64 & 0x20` here), so the
three lines keep Pulse's law; only the numbers around them differ.

### The Speed Lap medal block (the lines that are measured on HD)

After the new-record test, `Craft_UpdateLapProgress` reaches `0x000e553c`: if
this lap is the race's best (`+0x7024 == lap`), the campaign cell
(`GameState+0x90`) exists and `cell+0x110 == 10` (Speed Lap), it reads the cell's
three targets at `cell + 12 * GameState+0xdc + {0xb0,0xb4,0xb8}` (the difficulty
rung picks the row; exactly [race-campaign.md](race-campaign.md)'s
`Cell_EvaluateMedal` HD analogue, `0x001e25c0`) and picks the tier by
`lap_cs <= gold`, else `<= silver`, else `<= bronze`. It then

- raises `0x20`/`0x40`/`0x80` (`ER_GMA`/`ER_SMA`/`ER_BMA`),
- shows the `SpeedLapGoldMedal`/`SpeedLapSilverMedal`/`SpeedLapBronzeMedal`
  widget (`speedlap_hud.xml` authors them as `<ImageModel>`s, `hd_gold.vex`,
  `hd_silver.vex`, `hd_bronze.vex` from `Data\FE\Trophies\`, at (1600, 200),
  208 x 150, `StartEnabled="false"`), a 3D trophy,
- and plays `gold_med_shrt`/`silver_med_shrt`/`bronze_med_shrt`.

So the original raises the medal line **every time a lap is a new race-best and
inside a tier**, not only when the tier improves. **Wired 2026-10-16**
(`hud-lap-cues`): `oag_title::lap_hud::LapHud::speed_lap_medal` (HD only) selects
the measured rule in `oag_game::medal_watch`; every new race-best lap inside a
tier raises the tier's `ER_*MA` line and `Cue::SpeedLapGold`/`Silver`/`Bronze`
(`gold_med_shrt` etc., authored in `speech_results.bnk`, the boot bank the front
end loads; the voice starts 0.83 s after the trigger, the lead-in the bank
authors). The standing line (the build's own) still follows the best tier. Every
other title keeps the tier-improvement watch (chosen, not measured). Disc-backed:
`medal_message_ground_truth::hd_speed_lap_raises_the_medal_and_its_voice_on_every_new_best_lap`
(two bronze laps, two raises, two voices, WAV compared with and without). **Not
done**: the trophy model (the HUD crate has no `<ImageModel>` and the end-of-race
screen's trophy draw is a different path). Confidence 75: the targets and the tier test are in the disassembly
(`cmplw r22,r0` at `0x000e58a8`), the trophy widgets are named by TOC strings.

A neighbouring branch (`0x000e5c64`-`0x000e5cfc`) compares the lap with a target
for a PS3 trophy ("Beat Zico", the debug string `Comparing %d time with %d for
Beat Zico trophy...`): an achievement, not a HUD line.

## The gap

`Hud_UpdateTimeDiff` (`0x000907b0`) binds `TimeDiffText` at `hud+0x210` and
`TimeDiffIcon` at `+0x214` (`Hud_BindWidgets` stores `WrongWay` at `+0x20c`,
`TimeDiffText` at `+0x210`, `TimeDiffIcon` at `+0x214`; `ps3-toc.py resolve
0x0009bb30 -0x59c8` is `WrongWay`). The status bytes are `PLAYER_HUD+0x64` (the
player led), `+0x60` (the stamp), `+0x65` (a gap was made), `+0x66` (ahead),
`+0x5c` (the gap); the HUD's own `+0xb4` stamp, `+0xb8` leading, `+0xb0` gap,
`+0xb9` holding, `+0xba` ahead, `+0xc4` timer; the format strings
`"- %.1f"`/`"+ %.1f"` at `+0xcc`/`+0xd0` (`Hud_Construct`'s `0x00097be8`
neighbour stores them from TOC `-0x5b7c`/`-0x5b78`). Constants: `30.0`, `3.5`,
`5.0`, the four colours `0xff10c010`/`0xff30ff30` (ahead) and
`0xffc01010`/`0xffff3030` (behind), `0xff60ff60` for the count-up: **all Pulse's**.

The differences from Pulse's `Hud_UpdateTimeDiff` (`0x0881f890`):

1. **Time Trial and Speed Lap keep the count-up with no other craft live.** When
   `GameState+0xe0` is `5` or `10` and the profile byte `0x009384e1` is clear, a
   leading player gets the count-up with no live-craft test. Elsewhere it needs
   `Race_CountLiveCrafts > 1`, as on Pulse. **It can only bite after a stamp**, and
   `Craft_RecordSplit` stamps nothing while one craft is live (its first lines are
   `if (FUN_00055c90(race_manager, 0) == 1) return`, the live counter below), so a
   solo Time Trial shows no count-up on HD: the branch matters once the other
   craft is gone (an opponent wrecked in Speed Lap, say). Whether the counter
   counts a ghost is unread.
2. **No place stop.** Pulse re-reads `PLAYER_HUD+0x10` into its hold flag every
   tick, so a won gap ends the moment the player stops being first. HD's routine
   has no such read: the gap is held the full 3.5 s.
3. **Three gaps a lap.** See below.

### `Craft_RecordSplit`: a lap is split in thirds

It opens with `Race_CountLiveCrafts(race_manager, 0) == 1` and returns: **a lone
craft stamps nothing**. The counter (`0x00055c90`) walks the manager's craft list
(`rm+0xe8`...) and counts those with `craft+0x6a64 & 0x1000` clear and
`FUN_000d19b0(craft) != 7`. Pulse's and Pure's lap functions have no such early-out
(`Race_CountLiveCrafts` is called only from the display, `PlayerStatus_Update`
and the weapon AI on Pulse). 88.

`Craft_UpdateLapProgress` keeps two thresholds in the craft, `+0x71c4` and
`+0x71c8`: the progress at the first line crossing plus `1/3` and `2/3`
(`0x3eaaaaab`, `0x3f2aaaab`, TOC `-0x44a8`/`-0x44a4`, adjacent at `0x008a9030`),
wrapped at `1.0`; passing one bumps a split counter (`+0x71d8`) and calls
`Craft_RecordSplit(race_manager, split)`; the line crossing calls it with `0`. The
table is `race_manager + 0x11c + (crossings * 3 + split) * 0x10`, sixteen-byte
entries valid for crossings `1`-`99` and `split < 3`: `+0` the race clock
truncated to a whole second of the first craft over (`-1` before), `+4` the local
player was first, `+0xc` an AI craft was. The laws are Pulse's: the second craft
over a split, when one of the two is the player and neither is finished, raises
the gap `now - first` (`+0x66 = 1` player ahead, `0` behind) and a player who
crosses first raises `+0x64` and the stamp. **The gap is to the first craft over
that split, not to the craft directly ahead or behind**, the same as Pulse. The
first crossing of a race (`+0x7810 == 1`) does not call it.

A second branch (`g_PhysicsHalfStep` set, and not mode 13) keeps a previous-slot
word at entry `+8` and raises the gap against the craft that was over the split
just before: a split-screen rule, not read further (50).

### Wired

`oag_hd::hud::ART.lap_hud` carries all of it as data (`oag_title::lap_hud`):
`record_line_in_time_trial: false`, `count_up_alone_in_timed_modes: true`,
`stamps_need_two_live: true`, `green_hold_needs_first_place: false`,
`splits_per_lap: 3`, `table_crossings: 100`. `oag_raceplay::split_gap` stamps a craft at
`crossings * 3 + floor(3 * progress / length)` (**chosen, not measured**: the
original's thresholds are fractions of its normalized spline progress, ours is the
course's arc length over its length; the two differ slightly where the spline is
unevenly parameterised). Disc-backed:
`crates/game/tests/lap_messages_ground_truth.rs`
(`hd_skips_the_record_line_in_time_trial_and_a_lone_craft_stamps_nothing`).

## Other titles

- **Pulse**: [hud-message-triggers.md](../psp-pulse-usa/hud-message-triggers.md);
  checked, differs in the five ways above.
- **Pure**: [../psp-pure-usa/hud-message-triggers.md](../psp-pure-usa/hud-message-triggers.md);
  checked, differs (own keys, white, a sentence).
- **2048**: [../vita-2048-eu-v104/hud-message-triggers.md](../vita-2048-eu-v104/hud-message-triggers.md);
  checked, applies for the drain, gap routine unread.
- **Omega**: [../ps4-omega-eu/hud-message-triggers.md](../ps4-omega-eu/hud-message-triggers.md);
  checked, applies for both routines, split table unread.

## Open

- The trophy `<ImageModel>` (above).
- What sets `hud+0x44 & 0x20` and what `0x009384e1` is.
- The writers of `0x1000`, `0x2000` and `0x4000`.
- HD's cap widening for a long line, and the retro skin's halved alpha.
