# Lap counting, lap timing and race positions

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence.
Nothing here is under 70, so nothing carries a `_q`.

[lap-counting.md](../../../gameplay/lap-counting.md) opened with "nothing
in the original says where a lap starts or how one is counted", and built
this project's own convention on a constant measured from one capture at
confidence 65. **Both halves are now read.** The original counts laps by
**unwrapping the craft's arc length along the spline** and watching an
integer crossing count; the line is at a **fixed 154 units ahead of the
authored `Start Position` along the spline's tangent**, re-projected onto
the track; and a lap time is interpolated to the fraction of the tick in
which the crossing happened. What made it findable was the HUD: the `Lap`
widget's string is written from a field, and that field's writer is the
counter.

## Summary

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x08842a18` | `Craft_UpdateLapProgress` | function | 92 |
| `0x08827f0c` | `Race_UpdatePositions` | function | 90 |
| `0x08826b18` | `Craft_TotalRaceTime` | function | 88 |
| `0x08826dc8` | `Race_CountLiveCrafts` | function | 85 |
| `0x08829124` | `RaceManager_Construct` | function | 88 |
| `0x0882821c` | `Race_CreatePlayer` | function | 80 |
| `0x0883b044` | `PlayerStatus_Construct` | function | 82 |
| `0x0883b3b8` | `PlayerStatus_Update` | function | 82 |
| `0x0881a690` | `Hud_UpdateLapCounter` | function | 88 |
| `0x08819f30` | `Hud_BindPlayerStatus` | function | 80 |
| `0x0887dba0` | `AiTrack_ComputeLength` | function | 88 |
| `0x0893c804` | `Registry_Register` | function | 82 |
| `0x0893c9d4` | `Registry_Lookup` | function | 85 |
| `0x08b317b4` | `g_race_manager` | data | 90 |
| `0x08b317b8` | `g_named_registry` | data | 85 |
| `0x08b30f9c` | `g_race_laps` | data | 90 |
| `0x08ab0838` | `g_hud` | data | 85 |
| `0x08ab0ba0` | `g_player_status` | data | 85 |

`AiTrack_LocatePosition` (`0x0887ce78`) and the `SplinePt` record are on
[track.md](../../../formats/track.md); `Hud_BindWidgets` and `Hud_Update`
on [hud.md](../../../ui/hud.md). One existing row moves: `0x08ab0ba0` had
been `g_tournament_points_by_position` on [tournament.md](tournament.md),
whose own text says "index 0 unused"; `Race_CreatePlayer` stores a pointer
there, so index 0 is a different variable and the points table proper
starts at `0x08ab0ba4`.

## From the HUD to the counter

`Hud_BindWidgets` (`0x0881fbec`) looks up the `Lap` and `LapOf` widgets and
stores them at `hud+0x220`/`+0x224`. `Hud_UpdateLapCounter` (`0x0881a690`)
formats `*(hud+0x3c)->+0x08` into `Lap` and `->+0x0c` into `LapOf`, hiding
`LapOf` when the latter is below 1 (a mode with no lap target). `hud+0x3c`
is set by `Hud_BindPlayerStatus` (`0x08819f30`), which resolves the name
`"PLAYER_HUD"` through the **named-object registry**: `g_named_registry`
(`0x08b317b8`), 256 slots of `{pointer at +0x000, hash at +0x400, hits at
+0x800}` with the count at `+0xc00`, keyed by `Libc_HashString(name) ^
class_tag`; `Registry_Register` (`0x0893c804`) and `Registry_Lookup`
(`0x0893c9d4`, which also bumps the hit counter). The same registry carries
`"Race manager"`, `"AI track data"`, `"start position 1"` and `"FE_ModelSkin"`,
so it is how subsystems find each other without globals.

`"PLAYER_HUD"` is registered by `PlayerStatus_Construct` (`0x0883b044`) as
`self+0x48`, a 0xfc-byte block the HUD reads directly. `PlayerStatus_Update`
(`0x0883b3b8`, the object's pre-update virtual) refreshes it every tick
from the player's craft:

| `PlayerStatus` field | Source | Meaning |
| --- | --- | --- |
| `+0x48` | `\|body velocity\| * 3.6` | speed for the HUD, km/h |
| `+0x4c` | `craft->+0x94->+0x2b8` | |
| `+0x50` (`"PLAYER_HUD"+0x08`) | `craft+0xacc - 1` | **current lap number** |
| `+0x54` (`+0x0c`) | `g_race_laps` | laps in the race |
| `+0x58` | `craft+0xbd4` | position index |
| `+0x5c` | `craft+0x918` | |
| `+0x6c` | `craft+0x920` | **current lap time, seconds** |
| `+0x70` | `craft+0x92c` | last lap time, centiseconds |
| `+0x74` | `craft+0x930` | best lap, centiseconds |
| `+0x78` / `+0x7c` / `+0x80` | see "The target-time readout" below | the target-time readout (Time Trial, Speed Lap, Free Play) |
| `+0x84` | `Ship_Shield(craft)` | |
| `+0x8c` | `dot(craft forward, craft+0xb00) < -0.4` | **wrong way**; in Zone it also charges `Ship_Damage(dt * 15)` after 5 s of it |
| `+0x90` | `craft->+0x4c->+0x1bc` remapped | the held weapon's HUD icon index |

`g_race_laps` (`0x08b30f9c`) is `*(0x08b30f90 + 0x0c)`, set by
`Race_ReadSetupOptions` from the XML `Laps` value or the per-mode default
([state-machine.md](state-machine.md)).

`Race_CreatePlayer` (`0x0882821c`) is where the player's craft, input,
status object and HUD binding are built together: `Craft_Construct_q` into
the race manager's grid slot (`manager+0x78+4n`, and `manager+0x2c0` for
the player), the `FE_ModelSkin` livery, `PlayerInput_Construct`, then
`PlayerStatus_Construct(craft, input)` stored in `g_player_status`
(`0x08ab0ba0`), then `Hud_BindPlayerStatus(g_hud, "PLAYER_HUD")`. The
`sceKernelGetGPI() & 4` devkit switch swaps the `player_input` binding for
`autopilot_input` - a hardware DIP that makes the player's craft drive
itself.

## The target-time readout: `+0x78`/`+0x7c`/`+0x80`, closed 2026-09-28

**Confidence 92** for `PlayerStatus_Update`'s own write of these three
fields, up from 82: a PPSSPP write watchpoint on all three, armed live off
`g_hud` (`0x08ab0838`) -> `+0x3c` (the registered `"PLAYER_HUD"` pointer,
`self+0x48`) -> `+0x30`/`+0x34`/`+0x38`, logged every write's own PC across
an eight-second drive of a real Venom Time Trial on Talon's Junction
(`pulse-psp-usa.chd`). **Checked against a full re-scan of the raw PPSSPP
log, not a `tail` sample** (an earlier revision of this page read only the
tail and undercounted, both the number of hits and the number of distinct
PCs) - `grep -a 'CHK Write' ppsspp.log | grep -oE 'at 09a4a7(28|2c|30)
\(\(09a4a7(28|2c|30)\)\), PC=[0-9a-f]+' | sed -E 's/ \(\(.*\)\),/ /' | sort
| uniq -c`, keyed by address as well as PC:

```text
   8373 at 09a4a728  PC=0883b800
   8374 at 09a4a728  PC=0883baf0
   8374 at 09a4a72c  PC=0883ba38
   8374 at 09a4a730  PC=0883b808
   4153 at 09a4a730  PC=0883bad8
```

Five distinct write sites, not four as an earlier revision of this page
said. Two of them share `0x09a4a730` (`+0x38`, the redden bool):
`0x0883b808` at the same per-tick rate as every other write (the
unconditional `redden = false` reset at the top of the block), and
`0x0883bad8` at roughly half that rate - consistent with the conditional
`redden = true` store inside the tier comparison (`bVar1`'s own branch in
the decompile below), which only fires on a subset of ticks, though this
particular 8-second window's own field reads (taken before and after, not
continuously through it) never confirmed `redden` actually flipped during
it - an address-level correlation, not a behaviourally confirmed one. All
five write sites fall inside `PlayerStatus_Update`'s real body,
`0x0883b3b8`-`0x0883c0cb` (`get_function_by_address`); no address outside
that range ever wrote any of the three fields in this run. This is what
closes
[`hud-time-caption-substitution.md`](hud-time-caption-substitution.md)'s own
"what the tier itself is" gap - see that page for the reader
(`Hud_UpdateTimeCluster`) and the caption table.

This is the one block of `PlayerStatus_Update`'s own decompile that reads
`g_game_mode` rather than `craft`, gated on it reading `5`, `0x11` or `10` -
which [`state-machine.md`](state-machine.md#the-game-mode-enum-g_game_mode-0x08b31048)'s
enum names as `Time Trial`, `Multiplayer Time Trial` and `Speed Lap` (a
different ordinal space from [`race-campaign.md`](race-campaign.md)'s
`PI_Cell.mode`, which also uses `5` for `Time Trial` and `10` for `Speed Lap`,
because both index the same nine-name front-end vocabulary). A fourth,
separate `if` below it handles `g_game_mode == 7` (`Free Play`, tier `5`). An
earlier revision of this page guessed `0x11` was "a front-end-only Free Play
variant"; the enum settles it. **Every other mode leaves the target at `-1`**,
because the function writes `param_2+0x78 = 0xffffffff` unconditionally a few
lines above this block, every tick - which is what hides the clock on Zone,
Eliminator, single race, Head2Head and Tournament. See "Which modes hide the
clock" below for the live check.

```c
uVar14 = craft->0x920 * 100.0;              // current lap time, centiseconds
if (mode != SpeedLap) {                     // Time Trial / "Free Play": sum every
    for (i in 0..20)                        // completed lap split at craft+0x934+4i
        uVar14 += craft->(0x900 + i*4 + 0x34);
}
target[0x78] = -1;                          // hidden unless a target exists
redden[0x80] = false;
if DAT_08ab0de0 != 0 {                      // a global gate, unread past this
    ghost = Profile_GetBestRaceTime(DAT_08b31774); // per-team, per-track
                                             // record store, see "The
                                             // stored best" below. Read
                                             // unconditionally, campaign
                                             // cell or not.
    if DAT_08b310b4 == 0: /* no loaded track record: the block ends here,
                             target keeps uVar14 = the plain clock */
    if campaign_cell := DAT_08b30ffc; campaign_cell != 0 {
        gold   = campaign_cell->0xa0;       // == Cell::gold, `<Gold Target=>`
        silver = campaign_cell->0xa4;       // == Cell::silver
        bronze = campaign_cell->0xa8;       // == Cell::bronze
        // gold < silver < bronze, all in centiseconds, exactly
        // `oag_tables::race_campaign::Cell`'s own three fields -
        // race-campaign.md's own Target0..2 table confirms the offsets
        // independently, off the XML parser rather than off this function.
    } else {
        gold = silver = bronze = 0;         // forces the branch below
        // ghost is replaced with min(ghost, authored), non-campaign only,
        // where authored = (uint)(float * 100.0) of
        //   DAT_08b310b4 + DAT_08b31040*4 + 0xb0   (`<LapTimes>`, Speed Lap)
        //   DAT_08b310b4 + DAT_08b31040*4 + 0xa0   (`<RaceTimes>`, otherwise)
        // and DAT_08b31040 is the class ordinal, 0 = Venom. A ghost of 0 or
        // 0xffffffff means "none stored" and is replaced outright.
    }
    if      gold == 0:                  tier[0x7c] = 3            // RECORD - always true, non-campaign
    elif    ghost < gold && uVar14 < ghost: tier[0x7c] = 3         // RECORD - a campaign cell too, when
                                                                    // the player's own stored best already
                                                                    // beats gold and current pace beats it.
                                                                    // oag_game::hud::TimeTrialPace does not
                                                                    // reproduce this branch - see its own doc.
    elif    uVar14 <= gold:   tier[0x7c] = 2  // GOLD
    elif    uVar14 <= silver: tier[0x7c] = 1  // SILVER
    elif    uVar14 <= bronze: tier[0x7c] = 0  // BRONZE
    else: /* tier left at its previous value - see below */
    target[0x78] = the winning branch's own threshold - uVar14 (ghost for
                    RECORD, gold/silver/bronze otherwise), or 0 + redden=true
                    once uVar14 exceeds it
}
```

**The original's own tier field is stateful - it is simply not written once
`uVar14` exceeds every target**, so it keeps showing whichever tier was last
in reach. Because `uVar14` only grows across a race, that is provably the
same tier a fresh per-tick evaluation against the current `uVar14` alone
would show, past the point every target is missed: the last real write
before that point necessarily left it at `0` (bronze), which is exactly what
a stateless re-evaluation also produces once `uVar14 > bronze`. See
[`oag_game::hud::TimeTrialPace::evaluate`](../../../../crates/game/src/hud/time_trial_pace.rs)'s
own doc comment for the branch-by-branch argument this reimplementation is
built on - it needed no persistent per-race state as a result, only
`race_ticks`/`lap_ticks`, already on `Readout`.

**What this closes for the implementation**: a campaign Time Trial or Speed
Lap cell's own `gold`/`silver`/`bronze` fields
(`oag_tables::race_campaign::Cell`, already parsed, already the exact
numbers `campaign_cell->0xa0/0xa4/0xa8` read) are what the original compares
the live elapsed time against - the mapping this project needed to draw the
HUD tier is honest, not invented.
[`oag_game::hud::Readout::time_trial_pace`](../../../../crates/game/src/hud.rs)
carries it, computed at `RaceStage::draw_hud` (where the campaign cell
lives) and consumed by `oag_game::hud::draw`'s `TotalTime`/`TotalTimeTxt`
arms - see [hud.md](../../../ui/hud.md#medal-targets-closed-2026-09-28).
`RECORD` followed on 2026-09-30; see "The stored best" below.

**What stays open**: `DAT_08ab0de0`'s own meaning (confirmed non-zero on a
fresh profile, gating the whole block; `0` forces tier `4`, the layout's own
default "Total" caption). The `ghost` branch is closed - see below.

### The stored best (`FUN_088091a0`) and the `RECORD` branch, closed 2026-09-30

`Profile_GetBestRaceTime` (`0x088091a0`), confidence **75** (decompile read
directly; the value it returns was not read live, and the store's own layout
past the two rows below was not chased). It takes the profile object
(`DAT_08b31774`) and returns a time in centiseconds, or `0` when
`profile+0x464 == -1` (no row selected):

- **`g_game_mode` 5 (Time Trial) and `0x11`**: the sum of five `ushort`s at
  `row + profile+0x460*0x50 + 0x144 + 4 + 8*i`, skipping `0xffff`. A whole
  race's total, from per-lap splits.
- **`g_game_mode` 10 (Speed Lap)**: one `ushort` at `row + profile+0x460*0x30 +
  0x408`. One lap.
- Race, Head2Head, Tournament and the 14-16 family read a different row of the
  same store; the HUD never asks for those.

`profile+0x460` selects the row, so the store is **per team**, which
`oag_game::records::Key` (circuit, mode, class) has no field for. This build
maps the two rows to `Record::best_total_ticks` and `Record::best_lap_ticks`
and says so: **chosen, not measured**.

**The authored half, measured live 2026-09-30** (own PPSSPP, `pulse-psp-usa.chd`,
a plain Venom Time Trial on Talon's Junction, `g_game_mode` 5, no campaign
cell): `DAT_08b310b4` read `0x08d0aed0`, `DAT_08b31040` (the class ordinal) `0`,
`DAT_08b30ffc` (campaign cell) `0`, `DAT_08ab0de0` `1`. `DAT_08b310b4+0xa0` read
`(117.0, 138.0, 119.0, 128.0)` and `+0xb0` `(38.0, 33.0, 29.0, 25.0)`, and the
frame at `0.23.7` read `record 1.33.2`: `117.0 - 23.7 = 93.3`, with `PLAYER_HUD+0x30`
reading `8265` cs and tier `3`. `TrackStats_ParseElement` (`0x088c46f8`) puts
`Venom` at `+0xa0`, `Flash` `+0xa4`, `Rapier` `+0xa8`, `Phantom` `+0xac` **by
attribute name**, and the file's own `<code>` dictionary spells them `p`, `n`,
`o`, `m` - so the class is the dictionary's word, not its position. Both figures
are seconds.

What `oag_game::hud` does now: `RecordTarget` carries
`min(stored best, authored)`, `TimeTrialPace::evaluate` applies the branch
structure above (a campaign cell's `RECORD` needs `best < gold` and the run
ahead of it; a plain race is `RECORD` throughout), and a track whose `stats.xml`
did not read draws the plain clock in a plain race, the same as the original's null
`DAT_08b310b4`; a campaign cell still races its ladder there, which the original would not
(chosen: the cells' targets are their own data, and PS2 Pulse's per-track files are unread). See [hud.md](../../../ui/hud.md#medal-targets-closed-2026-09-28).
Not measured: a live frame with a **stored** best, since the profile used holds
none; the min and the campaign `RECORD` branch rest on the decompile plus unit
tests.

### Which modes hide the clock, confirmed live 2026-09-30

Confidence **95** (was an inference from the decompile alone, no frame taken).

- **The gate.** `PlayerStatus_Update` (above) writes `target = -1` and `redden =
  false` on every tick and overwrites the target only for `g_game_mode` 5, `0x11`,
  10 and 7. `Hud_UpdateTimeCluster` (`0x0881c9d0`) reads `-1` and clears bit `0x4`
  of `+0x2c` on the two widgets it bound at `hud+0x200` and `hud+0x204`.
  `Hud_BindWidgets` (`0x0881fbec`) binds exactly these by name: `hud+0x200` is
  `"TotalTime"` and `hud+0x204` is `"TotalTimeTxt"` (the decompile passes
  `s_TotalTime_08a79fd4` and `s_TotalTimeTxt_08a79fe0` to the widget lookup
  immediately before each store), which closes the "not confirmed by name" note
  on [`hud-time-caption-substitution.md`](hud-time-caption-substitution.md).
  Bit `0x4` is the visible bit: it is set on `CurrentTime` and the widgets that draw.
- **Eliminator (`g_game_mode` 8), read off a running PPSSPP**
  (`pulse-psp-usa.chd`, own profile, Xvfb, single race type 6): `PLAYER_HUD+0x30`
  (`PlayerStatus+0x78`) reads `0xffffffff`, `+0x34` reads `4`, `+0x38` reads `0`;
  `TotalTime` and `TotalTimeTxt` flag words (`+0x2c`) both read `0xb082` (bit `0x4`
  clear) while `CurrentTime`, `CurrentTimeTxt`, `BestTime` and `BestTimeTxt` read
  `0xf086` (bit `0x4` set). The frame agrees: `KILLS (5)` and the per-craft kill
  column fill the top-right corner, `best` and `current` sit bottom-left, and there
  is no `TOTAL` caption and no total time anywhere on screen. The mode object
  is `Elimination_Construct` (`Elimination_HUD.xml`); the HUD's flag word
  `hud+0x40` read `0x229f` and `hud+0x284` (the mode it bound for) read `8`.
- **Single race (`g_game_mode` 3), read off a running PPSSPP the same day**
  (same build): `PLAYER_HUD+0x30` reads `0xffffffff`, `+0x34` reads `4`; `TotalTime`
  and `TotalTimeTxt` flag words (`+0x2c`) both read `0xb082` (bit `0x4` clear) while
  `CurrentTime`, `CurrentTimeTxt`, `BestTime`, `BestTimeTxt`, `Position` (`hud+0x240`)
  and `PositionOf` (`hud+0x244`) read `0xf086`. `hud+0x40` read `0x20df` (the `0x40`
  place-readout bit set, `0x200` and `0x800` clear). The frame reads `pos 8/8` and
  no `TOTAL`. This is the direct measurement of the caption pair: `TOTAL` is hidden
  by the `-1` gate with its clock, `POS` is drawn, and neither is hidden by an
  overlap. Confidence **95**.
- **Zone (`g_game_mode` 6) has nothing to hide.** `Zone_HUD.xml` authors no
  `TotalTime`, `TotalTimeTxt`, `CurrentTime` or `BestTime` at all (its widgets
  are `Lap`/`LapOf`/`Lap Outof`, `Zone`, `Score`, `SpeedClass`/`SpeedClassTxt`
  and `TimeDiffText` - `just wad cat --expand` on
  `Data\XML\Zone_HUD.xml`), and `Hud_BindWidgets` skips the whole
  `+0x1f0`-`+0x204` bind when the HUD's `0x20` flag is set, the same flag its
  `Zone`/`Score`/`SpeedClass` bind is under. The revision of this page that said
  both layouts author `TotalTime` was wrong for Zone: only `Elimination_HUD.xml`
  does. No Zone frame was taken (Zone is greyed on a fresh profile; a forced
  `g_game_mode = 6` hung the loader, see [`countdown-voice.md`](countdown-voice.md)),
  so the Zone half is the layout read plus the bind gate, not a live frame.
- **What this build does now.** `oag_title::HudArt::total_time_timed_modes_only`
  is `true` for Pulse only, and `oag_game::hud::time_trial_pace::mode_hides_total_time`
  hides both widgets outside Time Trial and Speed Lap - the modes this build races
  from the original's `{5, 10}`; Free Play (7) and Multiplayer Time Trial (`0x11`)
  are not modes it runs. That also hides the clock in a single race, Tournament and
  Head2Head **when no place is on screen yet**, which the older
  place-owns-the-anchor rule missed; where a place is up it was already hidden.
  2048's live frame draws `TOTAL` beside `POS`, so the rule is not applied to
  any other title.

## The counter: `Craft_UpdateLapProgress` (`0x08842a18`)

Confidence **92**. Called for every craft in the grid by
`Race_UpdatePositions`, and it does four things in order: locate the craft
on the spline, unwrap its progress, detect a crossing, and on a crossing
record the lap. The craft fields it owns:

| Field | Meaning |
| --- | --- |
| `craft+0x8c0` | the AI track object (`*` -> float `units_per_t`, below) |
| `craft+0x8c4` | the race manager; `+0x7ac` on it is the **line** |
| `craft+0x900` | body position copied in for the locate |
| `craft+0xaf0` | the located `SplinePt` sample; `+0x40` of it (`craft+0xb30`) is `t`, `+0x10` (`craft+0xb00`) the tangent |
| `craft+0x91c` | previous arc length |
| `craft+0xad0` / `+0xad4` | **unwrapped progress** along the circuit and its rate |
| `craft+0xbdc` | wrap count (laps of arc length, signed) |
| `craft+0xbe0` | past-the-line flag this tick |
| `craft+0xac8` | **crossing count**: `wraps + past_line` |
| `craft+0xacc` | the crossing count the next lap completes on; **`HUD lap = this - 1`**; `2` at construction |
| `craft+0x910` / `+0x911` / `+0x912` | race started (first crossing seen) / crossed this tick / finished |
| `craft+0x920` | lap clock, seconds |
| `craft+0x928` / `+0x92c` / `+0x930` | last lap (s), last lap (cs), best lap (cs) |
| `craft+0x934[20]` | lap times, centiseconds, by lap index |
| `craft+0x8dc[20]` | per-lap "perfect" bytes |
| `craft+0x988[20]`, stride 0x10 | the twenty fastest laps: `{time, perfect, lap, +0x8d0}` |
| `craft+0x860 & 0x20` | hit a wall this lap (set on the collision-damage path, [shield-pickup.md](shield-pickup.md)); cleared at each crossing |

### Progress is arc length, unwrapped

```c
arc      = sample.t * track->units_per_t;          // craft+0xb30 * *(craft+0x8c0)
past     = manager->line_arc < arc;                // craft+0xbe0
if (prev_arc == 0) {                               // first tick: seed
    wraps    = past ? -1 : 0;                      // craft+0xbdc
    progress = wraps * L + arc;                    // craft+0xad0
} else {
    cand = wraps * L + arc;                        // pick the wrap that moves least
    if (|prev - (cand + L)| < |prev - cand| || |prev - (cand - L)| < |prev - cand|)
        wraps += (|prev - (cand + L)| < |prev - (cand - L)|) ? +1 : -1;
    progress = wraps * L + arc;
    rate     = (progress - prev_progress) / dt;    // craft+0xad4
}
crossings = wraps + past;                          // craft+0xac8
```

`L` is the track object's first word, written by `AiTrack_ComputeLength`
(`0x0887dba0`) at load as `sum of control-point distances / sum of the
control points' +0x40 deltas + 0.001` - **`SplinePt+0x40` is the authored
normalised arc position, `0..1` round the circuit**, which is what the
file field [track.md](../../../formats/track.md) had as `unk_0x40` holds
(checked on `16_Track`: 862 points, `t` rises from `0.0` at path 1's first
point to `0.9988` at path 0's last, and `dt * L` matches the point spacing
to a mean 0.16 units). The same function recomputes each path's
`max_spacing` from the data, which is why that field was "exactly the
longest gap" (the reset of that field to `0.5` when it reads outside
`0.01..20` is dead code: the measured maximum overwrites it unconditionally).
If the file's `t` deltas do not sum to something in `0.6..10`, every `+0x40`
is zeroed and the divisor becomes `1.0`, so `L` is the perimeter and `arc`
is `0` everywhere - a track without the field would count no laps at all.

The seed is the subtle part: a craft that starts *past* the line seeds
`wraps = -1` so its progress starts one lap negative and its crossing
count at `0`. Every craft therefore begins at `crossings = 0`, and the
first forward crossing takes it to `1` regardless of which side of the
line the grid slot is on.

### A crossing is `crossings` rising by exactly one

Falling (reversing over the line) does nothing but move the count back,
and a jump by two cannot happen from one tick's motion. When it rises:

```c
frac = dt * (prev_arc - line_arc) / (prev_arc - arc);   // time from tick start to the crossing
if (crossings == 1 && !started) {                       // the start line, first time
    next_lap_on = 2; started = crossed_this_tick = 1;   // no lap recorded, clock NOT restarted
} else if (crossings == next_lap_on) {
    if (g_race_laps && crossings == g_race_laps + 1) finished = 1;
    ... split table, see below ...
    next_lap_on = crossings + 1;
    last_lap        = lap_clock + frac;                 // craft+0x928
    lap_clock       = dt - frac;                        // the new lap has already run the rest of this tick
    last_lap_cs     = (int)(last_lap * 100);            // craft+0x92c
    lap_times[crossings - 2] = last_lap_cs;             // craft+0x934[], 20 max
    if (best == 0 || last < best) && Ship_State == 1: best = last;   // craft+0x930
    if new record (FUN_08808bec(g_profile, last_lap_cs)) and last == best and !finished: HUD flag |= 2
    perfect[crossings - 2] = !(flags & 0x20);           // craft+0x8dc[]
    if perfect && !finished: flags |= 0x200000; HUD flag |= 1;
    ... profile statistics: laps driven, per class, per team ...
    replace the slowest of the twenty fastest laps if this one beats it
    flags &= ~0x20;                                     // a fresh lap has hit nothing yet
    if crossings == g_race_laps && !finished: HUD flag |= 4;   // final lap
}
```

Three consequences for a reimplementation:

- **Lap times are sub-tick, and one frame long.** `frac` is the linear
  interpolation of the crossing within the tick, so a lap time is not a
  multiple of the frame period - which is why
  [lap-counting.md](../../../gameplay/lap-counting.md)'s captured laps of
  3,069 and 3,087 ticks read as `0.50.25` and `1.11.08`. But read the
  arithmetic exactly: the clock had **already** received this tick's whole
  `dt` at the top of the update (`0x08842acc`, `add.s f12,f12,f20`), and
  the record then adds the pre-crossing fraction to it again
  (`0x08842e88`) while the new lap starts at `dt - frac` (`0x08842ca8`,
  `sub.s f20,f20,f22`; stored at `0x08842e94`). The two laps around a
  crossing therefore share `2 * dt` of clock where only `dt` elapsed: **every
  recorded lap is exactly one frame longer than the time driven.** Static
  reading only, confidence 85; a capture that logs `craft+0x928` against a
  frame count is the check - `psp-trace.py` can read `craft+0x928` and
  `craft+0x920` directly, so one capture over two laps settles it - and the
  excess is one frame time, whatever the variable timestep made that frame.
- **The first crossing starts the race but not the clock.** `craft+0x920`
  accumulates `dt` from the first update and is only reset by a *lap*
  completion, so lap 1 is timed from whenever `Race_UpdatePositions` starts
  stepping crafts - not from the line. **That is the thrust release** (measured
  2026-09-29: `0.0` through the countdown, first `dt` on the release tick -
  [race-modes.md](../../../gameplay/race-modes.md#the-race-clock-starts-at-the-release)).
- **Lap numbering is off-by-one by design.** `craft+0xacc` is the crossing
  the next lap completes on, `2` from construction, so the HUD shows lap
  `1` on the grid, `1` after the first crossing, `2` after the second.

Multiplayer (`g_game_mode >= 0xe`) times laps off a shared clock
(`FUN_0895faa0`) against `craft+0x924` instead of accumulating `dt`, and
mirrors each lap time into the session (`FUN_0894da78`).

### The split-time table

`g_race_manager + crossings * 0x18 + 0xc0` is a per-crossing record shared
by the whole grid: `+0x00` the race time (`manager+0x2b8`, seconds) the
first craft reached this crossing at, `-1` until then; `+0x04` "the player
set it", `+0x05` "an opponent set it". The player crossing first writes
the time and shows it (`"PLAYER_HUD"+0x54 = 1`, `+0x50 = time`); a player
crossing after an opponent shows the **delta** (`+0x55 = 1`, `+0x4c = now -
their time`, `+0x56 = 0`); an opponent crossing after the player shows the
delta the other way (`+0x56 = 1`). That is the HUD's split readout, and it
is keyed on crossing count, so every craft's second crossing is compared
whichever lap they are on.

## Where the line is: `RaceManager_Construct` (`0x08829124`)

Confidence **88**. The manager registers itself as `"Race manager"`, then:

```c
start = Registry_Lookup("start position 1");           // the authored Start Position node
track = Registry_Lookup("AI track data");
AiTrack_LocatePosition(9984.0, track, &sample, start->pos /*+0x30*/, &cursor, -1, 0);   // 0x461c4000
line_point = sample.pos + 154.0 * sample.tangent;        // 0x431a0000
if (AiTrack_LocatePosition(100.0, track, &sample, &line_point, 0, -1, 0))
    manager->line_arc /*+0x7ac*/ = sample.t * track->units_per_t;
```

So the line is not authored and not at `t = 0`: it is **the grid slot
projected onto the spline, advanced 154 units along that point's tangent
in a straight line, projected onto the spline again**, expressed as an arc
length. On `16_Track`, off the file alone with control-point resolution:
the slot's nearest point is `t = 0.4509`, the advanced point lands at
`t = 0.4796`, and the difference is **146 arc units** (control points are
about 6 units apart, so read that as `146 +/- 6`). The 137.9 units
[lap-counting.md](../../../gameplay/lap-counting.md) measured is a
different thing: it is where the captured craft *spawns* relative to the
slot, and it is **16.5 units short of the line** as the ring measures it
(`the_captured_run_begins_just_behind_our_start_line`,
`crates/trace/tests/lap_capture_ground_truth.rs`). So the original places
its craft just behind the line and lets the first crossing start the race -
which is exactly the "spawns behind the line, crosses it seconds in" that
`RaceState::lap_gate` was built to tolerate, and the `started` flag above is
the original's version of the same gate.

The rest of the constructor: `AiStats_LoadAll`, `WeaponAiStats_Load`, the
skybox object (`DAT_08b323e0`) unless Zone, the `InGame` state's
`InGameInfoText`/`InGameMusicText`/`InGameMusicBG` widgets, the track name
into `InGameTrackDescriptionScreen`, grid size `manager+0x1a0c =
grid_count - 1`, eight `-1` slots at `+0x58`, and a `0x750`-byte child
built by `FUN_0883170c`.

## Positions: `Race_UpdatePositions` (`0x08827f0c`)

Confidence **90**. Once per tick over the grid (`g_race_manager + 0x78 +
4n`, `DAT_08b30f90` crafts): update every craft's lap progress, then give
each a sort key and bubble-sort the `+0x98` array (twenty passes at most)
ascending on it:

| Craft | Key |
| --- | --- |
| finished (`Craft_TotalRaceTime` non-zero) | its **total race time** in centiseconds |
| racing | `1,000,000 - unwrapped progress` |
| destroyed / respawning (`Ship_State` 5 or 6) | `2,000,000` |
| retired (`Ship_State` 7) | `3,000,000` |

Then `craft+0xbd8 = rank` (1-based, equal keys share a rank) and
`craft+0xbd4 = index + 1`. `+0xbd8` is the field
[tournament.md](tournament.md) and `Race_RecordResult` already read as
"finishing position", and this is its writer. `Craft_TotalRaceTime`
(`0x08826b18`) is the sum of `craft+0x934[0..laps)`, `0` if any lap is
still unrecorded - so a craft that has finished sorts ahead of every
craft still racing, by its time, and finishing order is exactly the order
of total times. `Race_CountLiveCrafts` (`0x08826dc8`) counts crafts not
destroyed (`+0x860 & 0x1000`) and not retired, optionally listing them.

## Consequences for this project

- `Course::START_LINE_OFFSET` (137.9, measured) is replaced by the rule
  above: nearest ring point to the slot, 154 units along its tangent,
  nearest ring point again. `docs/gameplay/lap-counting.md` is rewritten
  accordingly.
- `RaceState::lap_gate`'s "near half then far half" is a stricter version
  of the original's unwrap-plus-first-crossing; the original allows a
  lap to be re-earned after reversing exactly as the gate does (the count
  falls and must rise again), so the behaviours agree except that the
  original never needs the half-circuit test.
- Sub-tick lap times and the lap-1 clock origin are recorded here and
  **not** implemented on this pass; `oag_race` still times from the line
  in whole ticks. The doc comment on `Standing` says which.

## Open questions

- ~~When `Race_UpdatePositions` starts running relative to the countdown -
  it decides what lap 1's clock measures.~~ **Answered 2026-09-29, measured:**
  the lap clock (`racer+0x920`) and the manager's race time (`manager+0x2b8`)
  are exactly `0.0` through the countdown and take their first `dt` on the tick
  `throttleState` leaves zero, so lap 1 is timed from the thrust release. See
  [race-modes.md](../../../gameplay/race-modes.md#the-race-clock-starts-at-the-release).
- `craft+0x8d0`, stored per fastest lap and zeroed per lap, has no
  incrementer in the binary (only the two zero stores); the fourth field
  of the top-twenty record is dead.
- What `FUN_0883170c`'s `0x750`-byte object under the manager is.

## History

- 2026-09-16: first reading. Static, plus one file measurement
  (`16_Track`'s `+0x40` column). No runtime leg; the HUD-to-writer chain
  and the `+0x40` arithmetic closing are what put the counter at 92.
