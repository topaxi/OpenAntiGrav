# The opponent driver on Pure: a sibling of Pulse's, piece by piece

**Binary:** `pure-psp-usa` `BOOT.BIN` (`UCUS98612`), image base `0x08804000`. Read
2026-10-12 by the `orig-ai-pure` lane, which recovers Pure's original opponent the
way `orig-ai-pulse` recovered Pulse's
([psp-pulse-usa/ai-controls.md](../psp-pulse-usa/ai-controls.md)) and `orig-ai-hd`
mapped HD's onto it ([ps3-hdfury-eu/ai-controls.md](../ps3-hdfury-eu/ai-controls.md)).
The EU pressing (`/pure/BOOT-psp-pure-eu.BIN`) was checked for the one question that
decides everything else, [which block is live](#which-of-the-two-blocks-is-live);
everything else is the USA binary.

**Read the import trap first.** This import leaves `lui`/`addiu` immediates
unrelocated ([psp-pure-eu/string-anchors.md](../psp-pure-eu/string-anchors.md#the-trap-code-immediates-are-unrelocated-data-addresses-are-rebased)),
so `get_xrefs_to` and Ghidra's call graph say "no references" for everything on this
page. Every reference below was found with `scripts/psp-reloc.py refs`, and every
call with a `jal` search for the unrelocated target (`address - 0x08804000`).
Globals the decompiler prints as `_DAT_000aeef0` and the like are **segment-1
addends**: the loaded address is `0x08ac3eb0 + addend` (`psp-reloc.py pair` at
`0x0891c3b0`: `HI16 addr_base 1, adds 0x2bfeb0`).

**The answer in one paragraph.** Pure's opponent is not Pulse's with branches; it is
the earlier sibling Pulse grew out of. The steering controller, the lookahead, the
two lane-point helpers, the AI-stats header constants, the nine-candidate lane bumps,
the fork coin and the rail's gates are Pulse's to the constant. Around them, five
things are different: **the thrust is a per-slot cruise speed drawn at load**, scaled
by one field-wide multiplier read off the `ZoningStats` curves at the nearest
opponent's gap to the player, by a fatigue factor, and blended toward a launch speed;
**there is no lane ring** (one current lane with a bang-bang lateral velocity, steered
toward by both the controls and the rail); **the planner scores the other craft and
track objects**, which Pulse's does not; **the rail moves position only** (it computes
a velocity turn and never stores it); and **every opponent updates its thrust and lane every frame** (the round-robin byte is
set on all racers by the snapshot), where Pulse's run one driver a frame. Of the two per-class blocks in `AIRaceStats.xml`, the engine parses
both into disjoint halves of one record and reads **the legacy half**: the switch that
selects `<NewStats>` is initialised to `0` in `.data` and has no writer on either
pressing. So `PosBalancing` (`AIThrust`, `SpreadDist`), `ShipSpacingVariation`,
`TailZone` and the `<NewStats>` start rows are parsed and never reach a craft.

Confidence is per row. Static reading unless a row says measured.

## The cast

| Address | Name | Confidence | Pulse counterpart | Verdict |
| --- | --- | --- | --- | --- |
| `0x0891c314` | `AiStats_LoadAll` | 85 | `AiStats_LoadAll` | sets defaults, parses `AIControlStats.xml` then `AIRaceStats.xml` (string refs `0x0891c370`, `0x0891c398`) into `RM+0x140`, then draws each class's cruise speeds, [below](#the-record) |
| `0x0891c1c8` | `AiStats_SetDefaults` | 85 | `AiStats_SetDefaults` | the header is Pulse's to the bit (`950`, `80`, `0.002`, `23000`, `10`); per class the record defaults, [below](#the-record) |
| `0x0891c504` | `AiStats_ParseFile` | 85 | `AiStats_ParseFile` | root `AIStats`; `MasterSpeedTweak` is iterated attribute by attribute and **discarded**; `Vector`/`Venom`/`Flash`/`Rapier`/`PhantomStats` pick the class record (stride `0x240`) |
| `0x0891c738` | `AiStats_ParseClass` | 85 | `AiStats_ParseClass` | `StartStats`, `ZoningStats`, `FatigueStats`, `Controller`, `GridSpread`, `NewStats` (exact `strcasecmp`, `0x08899fd0`) |
| `0x0891cabc` | `AiStats_ParseNewStats` | 85 | - | `StartStats`, `ZoningStats`, `FatigueStats`, `GridSpread`, `ShipSpacingVariation`, `PosBalancing` into the record's upper half |
| `0x0891d4f4` | `AiStats_ParseStartStats` | 85 | `AiStats_ParseStartStats` | `GridPlaceN`: `NormalRaceSpeedMin` `+0x28`, `NormalRaceSpeedMax` `+0x48`, `StartSpeed` `+0x88` (`[N-1]`) |
| `0x0891d6b0` | `AiStats_ParseZoning` | 85 | - | `ZoneN` `Position` `+0xa8`, `SpeedPercent * 0.01` `+0xc0`; `LeadZoneN` `+0xd8`/`+0xf0` (`[N-1]`, six each). **No `TailZone` branch** |
| `0x0891d914` | `AiStats_ParseFatigue` | 85 | - | `FatigueStartTime` `+0x108`, `FatigueDecayTime` `+0x10c`, `FatigueMultiplier` `+0x110`, `RestoreRate` `+0x114`, `FatigueZoneFront` `+0x118`, `FatigueZoneBack` `+0x11c` |
| `0x0891d38c` | `AiStats_ParseController` | 85 | `AiStats_ParseController` | Pulse's six attributes at Pulse's offsets `+0x00..+0x14`. Only the element named exactly `Controller` reaches it: **`ControllerPO` is never parsed** (it is not a string in the binary, and the compare is `strcasecmp == 0`) |
| `0x0891d2f0` | `AiStats_ParseGridSpread` | 85 | - | `distance` to `+0x18` |
| `0x0891ccb4` | `AiStats_ParseNewStartStats` | 85 | - | `BaseStartThrust` `+0x130`, `StartBoostSpeed` `+0x150` |
| `0x0891ce44` | `AiStats_ParseNewZoning` | 85 | - | `Zone` `+0x1b0`/`+0x1c8`, `LeadZone` `+0x1e0`/`+0x1f8`, `TailZone` `+0x210`/`+0x228` |
| `0x0891d160` | `AiStats_ParseNewFatigue` | 85 | - | **an empty function**: `<NewStats>`' fatigue rows are never stored |
| `0x0891c930` | `AiStats_ParseNewGridSpread` | 85 | - | `distance` to `+0x120` |
| `0x0891c9cc` | `AiStats_ParseShipSpacing` | 85 | - | `min` `+0x124`, `max` `+0x128`, `trailingFieldMultiplier` `+0x12c` |
| `0x0891d168` | `AiStats_ParsePosBalancing` | 85 | `AiStats_ParsePosBalancing` | `AIThrust` `+0x170`, `SpreadDist` `+0x190` |
| `0x08919024` | `Ai_Construct` | 80 | `Ai_Construct` | registers `driver+0x08` as `"AI input %d"` (`0x08919170`) or `"autopilot input"` when `craft+0x488 == 0`; grid slot to `+0x60` (`8` for the player) |
| `0x0891b8d0` | `AiManager_Update` | 80 | `AiManager_Update` | snapshot, the round-robin byte, `Ai_Update` on every driver, the weapon AIs; **no shuffle**, see [the round robin](#the-round-robin) |
| `0x0891bee4` | `AiManager_SnapshotRacers` | 90 | `AiManager_SnapshotRacers` | racer table at addend `0xae0f0` (`0x08b71fa0`), stride `0x1c0`; count `0x08b72da0`, player craft `0x08b72da8` (from `RM+0x13c`), player racer `0x08b72da4` |
| `0x08919394` | `Ai_Update` | 80 | `Ai_Update` | per driver per tick, [below](#ai_update-0x08919394-in-order) |
| `0x08919918` | `Ai_UpdateFatigue` | 75 | - | the launch blend and the fatigue factor, [below](#launch-and-fatigue) |
| `0x0891ac04` | `Ai_ComputeControls` | 80 | `Ai_ComputeControls` | Pulse's steering law; the legacy thrust, [below](#the-thrust-legacy-block) |
| `0x0891a73c` | `Ai_ComputeNewStatsThrust` | 70 | `AI_ComputeOpponentThrust` | the `<NewStats>` thrust (`AIThrust` by the player's place, a field-mean formation term, `BaseStartThrust` blend), called only behind the switch: dead on the shipped executable |
| `0x08919b6c` | `Ai_PlanLane` | 75 | `Ai_PlanLane` | different design, [below](#the-planner-no-ring) |
| `0x08918ed8` | `Ai_AddLaneBump` | 85 | `Ai_AddLaneBump` | the same trapezoid over the nine candidates (`i * 0.25 - 1`, the `0.95`/`0.05` shoulder, half width floored at `0.05`), accumulating into `driver+0x1e0` |
| `0x08918570` | `Ai_ScoreOtherCraft` | 70 | - | a negative bump at each other racer `-15..100` units ahead, [below](#the-planner-no-ring) |
| `0x08918888` | `Ai_SampleLookahead` | 85 | `Ai_SampleLookahead` | identical: `d = max(t * smoothed_speed, 15)`, `p = lap fraction + d / lap_length * 1.5`, wrapped past `1` |
| `0x0891849c` | `Ai_LanePoint` | 85 | `Ai_LanePoint` | identical, racing line included |
| `0x08917224` | `Ai_LanePointFromCentre` | 85 | `Ai_LanePointFromCentre` | identical, no racing-line term |
| `0x08917b80` | `Ai_SnapToLane` | 85 | `Ai_SnapToLane` | Pulse's gates; position only, [below](#the-rail-position-only) |
| `0x0891892c` | `Ai_ComputePitch` | 80 | `Ai_ComputePitch` | Pulse's law times a distance fade, [below](#pitch) |
| `0x08917f70` | `Ai_ChooseBranch` | 75 | `Ai_ChooseBranch` | the `rand() & 0x100` coin, [below](#forks) |
| `0x08810b30` | `RaceManager_UpdateZoneSpeed` | 88 | - | writes `RM+0x134` from the zone curves, [below](#the-zone-multiplier) |
| `0x08810ddc` | `RaceManager_UpdateLaunchPhase` | 70 | - | clears the launch flag `RM+0x1e34` once the field is strung out, [below](#launch-and-fatigue) |
| `0x08923a1c` | `Craft_GapAlongTrack` | 75 | - | `wrap(b.progress - a.progress)` over the lap (`craft+0xa38`), [below](#the-zone-multiplier) |
| `0x089336f0` | `Body_SetPosition` | 85 | `Body_SetPosition` | writes `body+0x30..+0x3c` and nothing else |
| `0x088d9390` | `TrackStats_ParseElement` | 80 | `TrackStats_ParseElement` | a track `stats.xml`'s elements into the settings object (pointer at `0x08b17430`): `RaceTimes` to `+0xa8..`, `ZoneMedals` to `+0xc4..`, and `<AIStats><<Class>Tweak Min Max>` to `+0xd0 + 4 * class` (`Min`) and `+0xe4 + 4 * class` (`Max`), the speed tweak's range ([the record](#the-record)) |
| `0x08ab3b6c` (data) | `g_ai_use_new_stats` | 94 | - | the block switch, [below](#which-of-the-two-blocks-is-live) |

`FUN_0891a3d0` reads the `<NewStats>` spacing rows (`+0x124`, `+0x128`, `+0x12c`) and
the `AIThrust` row and has no `jal` caller: an unused `<NewStats>` thrust variant.
No name. `FUN_08918d30` adds a positive bump toward entries of a per-track object list
(addend `0xaef00`, stride `0x30`, built by `FUN_0891bb74` from `"AI track data"`) that
lie up to 3.5 s ahead; what the objects are (pads, by the shape) is not read, so no
name. `FUN_08917870` returns `0` and `FUN_08918924` is empty; both are called by
`Ai_Update`.

## Which of the two blocks is live

`AiStats_ParseClass` hands `<NewStats>` to its own parser, and the two parsers write
**disjoint** fields of one record: the legacy rows `+0x18..+0x11c`, the `<NewStats>`
rows `+0x120..+0x23c`. So both are always parsed, whatever order the file lists them
in, and the question is which half a reader reads. Every reader that could take
either half tests one byte first:

| Site | Legacy (byte `0`) | `<NewStats>` (byte non-zero) |
| --- | --- | --- |
| `Ai_ComputeControls` `0x0891b330` | cruise-speed thrust | `Ai_ComputeNewStatsThrust` |
| `Ai_UpdateFatigue` `0x089199f4` | `StartSpeed[slot]` | `StartBoostSpeed[slot]` |
| `RaceManager_UpdateZoneSpeed` `0x08810c4c` | `Zone`/`LeadZone` | `Zone`/`LeadZone`/`TailZone` |
| `RaceManager_UpdateLaunchPhase` `0x08810e34` | `GridSpread` (legacy) | `GridSpread` (`<NewStats>`) |
| `FUN_08918144` `0x0891814c` | returns `0` | a track-object search |

The byte is `0x08ab3b6c` (`.data`, file offset `0x2afbdc`), **initialised to `0`**,
and `psp-reloc.py refs` finds exactly those five sites, all `lbu`: no store and no
address-forming `addiu` anywhere in `.text`. The EU pressing agrees: its byte is
`0x08aad344`, initialised to `0`, referenced by five `lbu` and nothing else
(`0x08810a64`, `0x08810c4c`, `0x08917a00`, `0x089192a8`, `0x0891abe4`). Two binaries,
static: **the legacy block is live.** A write through a computed pointer would hide
from that sweep; the live leg read the byte as `0` in every sample of five races, so
confidence 94.

So in the shipped game: `PosBalancing` (`AIThrust`, `SpreadDist`) never reaches a
craft, nor do `ShipSpacingVariation`, `BaseStartThrust`, `StartBoostSpeed`,
`TailZone` or the `<NewStats>` `Zone`/`LeadZone`/`GridSpread` rows; `<NewStats>`'
`FatigueStats` is not even stored; `MasterSpeedTweak` is parsed and dropped; and
`ControllerPO` is never parsed. The legacy block's `ZoningStats`, `FatigueStats`,
`StartStats` and `GridSpread`, and `AIControlStats.xml`'s `Controller`, are what the
opponents run on.

## The record

`AiStats_LoadAll` is called with `RM + 0x140`; the header is five words, then five
class records of `0x240` from `RM + 0x154` (`R` below). Driver `+0x138` holds `R` for
the race's class (index addend `0x53530`, `0x08b173e0`).

| Offset | Field | Default (`AiStats_SetDefaults`) |
| --- | --- | --- |
| `+0x00..+0x14` | `SteerMul`, `SteerDamp`, `LookAheadSecs`, `xtrackMul`, `xtrackMax`, `xtrackDamp` | Pulse's |
| `+0x18` | `GridSpread distance` | `100` |
| `+0x1c`, `+0x20` | the speed-tweak range | `1`, `1`; overwritten at load from the race's settings object (pointer at `0x08b17430`, `+0xd0 + 4 * class` and `+0xe4 + 4 * class`) when one exists, and set to `1` if zero. Those two come from **the track's own `stats.xml`** in `FEData.wad` (`Data\Environments\<n>_<Name>\stats.xml`), `<AIStats><VectorTweak Min=".." Max=".."/>` and one row per class, parsed by `TrackStats_ParseElement`. Read live 2026-10-12: the object's ten words equal Vineta K's file, and its name field reads `Vineta K` |
| `+0x24` | the speed tweak, drawn `U[+0x1c, +0x20]` once at load | - |
| `+0x28[8]`, `+0x48[8]` | `NormalRaceSpeedMin`, `NormalRaceSpeedMax` | `70`, `95` |
| `+0x68[8]` | the cruise speed per grid slot, drawn `U[min, max]` once at load | - |
| `+0x88[8]` | `StartSpeed` | `0` |
| `+0xa8[6]`, `+0xc0[6]` | `Zone` `Position`, `SpeedPercent / 100` | `0`, `1` |
| `+0xd8[6]`, `+0xf0[6]` | `LeadZone` the same | `0`, `1` |
| `+0x108..+0x11c` | fatigue: start, decay, multiplier, restore, front, back | `10`, `10`, `1`, `4`, `10`, `-10` |
| `+0x120..+0x23c` | the `<NewStats>` half | - |

The draws use `rand() * 2^-31` (`4.656613e-10`), the generator at `FUN_088e0568`.

## `Ai_Update` (`0x08919394`), in order

Skipped (controls zeroed) when the racer is not found, its point is null, the craft's
`+200 -> +0x1c0` has bit `2`, or `craft+0xb95` is set.

1. `driver+0x80 = (craft+0xd0 -> +0x134) != 0` (the `x 0.7` flag below; what the
   craft state is, is not read).
2. `dt * 60` into `0x08ab3b64`.
3. First call: `driver+0x38` = the race manager, `driver+0x138 = R`, `+0x140 =
   LookAheadSecs`, `+0x144 = SteerMul`, `+0x13c = 1`.
4. `driver+0x190 = R+0x24` (the speed tweak), every tick; `driver+0xa0 = 1` for the
   player's craft; `driver+0x64 = RM+0x12c` (race time).
5. The excluded fork path to `craft+0xa10` (`-1` for the player unless `+0x20`).
6. `+0x13c`/`+0x140`/`+0x144` rescaled by `1 - 0.5 * FUN_08917870()`, which returns
   `0`: a no-op.
7. `+0x50 = 1 + 0 * ...` (a dead wobble term, multiplied by `0.0`).
8. `+0x174 = 0.95 * +0x174 + 0.05 * |velocity|` (smoothed speed).
9. `Ai_PlanLane`, `Ai_ChooseBranch`, `Ai_UpdateFatigue`, then
   `Ai_ComputeControls(LookAheadSecs, driver, out, driver+0x178)`.
10. Steer and both airbrakes every tick, times the authority `driver+0x70`.
11. **Thrust on the racer's round-robin tick only** (`racer+0x1b0`): `out.thrust`,
    `x 0.7` when `driver+0x80`; then, for a craft that is not the player
    (`*(0x08b17798) + 0x13c`), Pulse's per-point speed floor on `SplinePt+0x66 < 35`
    with Pulse's quadratic, **and a floor of `35`** on the result (Pulse has none).
12. A debug byte (`0x08ab3b5c`) zeroes thrust and airbrakes.
13. `Ai_SnapToLane(dt, driver)`, `Ai_ComputePitch(driver)`.
14. The 20-tick steer average into `driver+0x130`.

There is no second-controller block (Pulse's step 14).

## The round robin

`AiManager_Update` (`0x0891b8d0`) **increments** `mgr+0xa8` first, wraps it at the
racer count (`0x08b72da0`), and sets that racer's `+0x1b0`. Pulse sets
`racer[mgr+0xa8]` and then counts, wrapping at `8` whatever the field. So Pure's
period is **the number of racers**, and its first thrust tick goes to racer `1`
(HD's phase). **But it gates nothing**: `AiManager_SnapshotRacers`, called just before,
sets `racer+0x1b0 = 1` on every racer every frame, so every driver's thrust, lane
choice and fork choice run on every frame. Measured, [below](#measured-2026-10-12): 90.

The update is skipped while a network condition holds (`0x08b173e4 >= 9` and a byte
at `*(0x08b17780) + 0xb0`, unless `mgr+0xac > 99`); what it gates (an ad hoc session, by the shape) is not read.

There is **no pack shuffle and no spread wander** on Pure: no `AIPackSwapping`
string, no `SkillScale` string, nothing in the manager after the snapshot but the
round robin.

## The thrust (legacy block)

`Ai_ComputeControls` (`0x0891ac04`), on the round-robin tick, for an opponent
(`driver+0xa0 == 0`) with the switch `0`:

```text
slot   = driver+0x60                                 // grid slot, 1-based
cruise = R[+0x64 + 4 * slot]                         // the drawn cruise speed, [slot - 1]
x      = cruise * driver+0x190 * driver+0x54         // speed tweak, zone x fatigue
thrust = x + (R[+0x84 + 4 * slot] - x) * driver+0x1c0   // toward StartSpeed[slot - 1] by the launch blend
```

The player's driver (the autopilot) keeps the output default `100` on this path.
`Ai_Update` then applies the `x 0.7` flag, the speed floor and the floor of `35`.

So **the legacy thrust reads the player twice, both indirectly**: through the zone
multiplier inside `driver+0x54` (the nearest opponent's gap to the player) and
through the fatigue window (this craft's own gap to the player). It does not read the
player's place, and nothing on this path reads `AIThrust` or `SpreadDist`.

## The zone multiplier

`RaceManager_UpdateZoneSpeed` (`0x08810b30`, argument `RM`) writes `RM+0x134`, one
number for the whole field:

```text
if the player's craft (RM+0x13c) has no track point: return
nearest = the racer (not eliminated, craft+0xb95 == 0, not the player) minimising |g|,
          g = Craft_GapAlongTrack(player, racer)
if none: RM+0x134 = 1; return
curve = player's place (craft+0xb44) == 1 ? LeadZone : Zone       // legacy rows
RM+0x134 = piecewise-linear SpeedPercent at g over Position[0..5], held flat past either end
```

`Craft_GapAlongTrack(a, b)` (`0x08923a1c`) is `b.progress - a.progress` (`craft+0xa38`)
wrapped into `+/- half` the lap (`FUN_08923388`). So **`Position` is the nearest
opponent's distance ahead of the player: negative is an opponent behind the player.**
That is the sign `gameplay/ai.md` hypothesised at 55; it now rests on the code
(80, static).

`Ai_UpdateFatigue` copies `RM+0x134` into `driver+0x54` every tick before it scales it.

## Launch and fatigue

`Ai_UpdateFatigue` (`0x08919918`), every tick, skipped for the player's driver:

```text
launch = driver+0x1c0
launch = RM+0x1e34 ? min(launch + 0.5, 1) : max(launch - 0.002, 0)
driver+0xb4 = launch * StartSpeed[slot - 1]                // a reader was not looked for
g = Craft_GapAlongTrack(player, own craft)
inside = FatigueZoneBack < g < FatigueZoneFront and race time (driver+0x64) >= 10
t = driver+0xb0
t = inside ? min(t + 1/60, StartTime + DecayTime) : max(t - RestoreRate / 60, 0)
k = t <= 0 ? 1
  : t < StartTime ? 1
  : 1 + (FatigueMultiplier - 1) * min((t - StartTime) / DecayTime, 1)
driver+0x54 = RM+0x134 * k
craft+0xcd  = t != 0
```

The `1/60` is a literal `0.016666668`, not `dt`: fatigue is per call. With the
shipped `FatigueMultiplier` the factor is `1` throughout, so on the shipped data
**fatigue changes nothing** but `craft+0xcd`.

`RaceManager_UpdateLaunchPhase` (`0x08810ddc`) clears `RM+0x1e34` once the spread of
every craft's `+0xaac` (max minus min) exceeds `GridSpread distance * R+0x24`. Two
debug keys (`0x38`, `0x39`) step `R+0x24` by `0.002` inside its range. So the launch
blend is `1` from the grid until the field has strung out to `GridSpread`, then
decays over 500 ticks to the cruise speed.

## The planner: no ring

`Ai_PlanLane` (`0x08919b6c`):

- **Authority** `driver+0x70` moves `0.025` a tick toward `1` (toward `0` for the
  player's driver while its controls are not consumed), Pulse's ramp.
- **First call**: project the craft (radius `200`) and set the current lane
  (`driver+0x1c4`), target (`+0x1c8`) and the two copies at `+0x154`/`+0x158` to the
  craft's lane coordinate, the current lane clamped to `[-3, 3]` and the target to
  `[-1, 1]`; lane velocity `+0x1cc = 0`; `+0x14c` race time, `+0x150` that plus `13`.
- **The choice, on the round-robin tick**: scores zeroed, then `Ai_AddLaneBump(0, 1.0,
  0.05)` and `(preferred, 0.15, 0.5)` (Pulse's two), then `FUN_08918d30`'s object
  bumps and `Ai_ScoreOtherCraft`'s (`-2 * w` at each other racer's lane, width
  `6 / max(its corridor width, 0.5)`, `w` falling with distance past `100` units and
  weighted by the bearing: `x 1.5` abeam, `x 0.5` behind). Smoothed `0.9 / 0.1` into
  `+0x204`, best of nine as Pulse's, then the same local search from the craft's
  current lane coordinate. No contact push.
- **Preferred lane**: `rand() % 400 == 125`, `u = 2 * rand01 - 1`,
  `u >= 0 ? 1 - u^2 : u^2 - 1`, every tick (Pulse's law).
- **The lane moves as a second-order bang-bang**: with `e = target - lane` and lane
  velocity `v`, an acceleration of `+/-0.0008` (and `+/-0.0004` in the position
  term) per frame, braking when `v` would overshoot, all times
  `dt * 60 * s` where `s = 0.9 s + 0.1 clamp((|v_xz . fwd| - 10) * 0.02, 0, 1)`:
  `v += a * k`, `lane += v * k + a/2 * k^2`. The result is `driver+0x44`, the lane the
  controls and the rail steer to, and the planner's own `+0x1c4`.
- When `driver+0x80` is set the lane wanders: `+0x15c += t * rand01 * 6.5` and
  `driver+0x44 += sin(...) * 0.6`, clamped to `[-1, 1]`.

So Pure plans **one lane for the whole lookahead**: the steering's ahead target and the
rail's target are the same `driver+0x44`, where Pulse reads an 80-slot ring at each
point's own progress. Confidence 75: read whole, the bang-bang's branch structure read
from the decompile only.

## The steering controller

`Ai_ComputeControls`' steering is Pulse's: the airborne fraction `(|h| - 15) * 0.025`,
the cross-track error from `Ai_LanePointFromCentre` times `1 - air`, the lookahead of
`LookAheadSecs + air`, the `0.7`/`0.3` aim shift times `xtrackMul * x * dist`, the
`0.0676` velocity lead, the `-0.0206` yaw prediction, `err = -dot(cross(fwd', dir),
up)`, `derr = (err - last) / (dt * 60)`, `steer = (err + derr * SteerDamp) * 950 *
SteerMul`, and the saturation into the airbrakes with `80` and `10`. The lane used for
both the own point and the ahead point is `driver+0x44` (above). Confidence 80.

## The rail: position only

`Ai_SnapToLane` (`0x08917b80`): Pulse's gates exactly (`(|v| - 50) * 0.01`, `1 - (|h|
- 7) / 7` via `* 0.14285715`, zero when the controls are not consumed or when a
non-player craft has contacts at `*(craft+0x8b4) + 0x3b0`, the blend toward `w` at
`dt * 0.6`, or `1.5` in ship state `2`, `f = blend^2 * 0.7`, `f >= 1e-6`), with the
lane point at `driver+0x44`. Then:

```text
v' = v - lateral * dot(v, lateral) * f        // no 0.25: Pulse has v - lateral * dot * f * 0.25
...the angle between v' and v through asinf (jal 0x0015902c), discarded
Body_SetPosition(pos - lateral * dot(pos - P, lateral) * f)      // jal 0x0012f6f0 at 0x08917f40
```

The disassembly (`0x08917dcc..0x08917f44`) stores `v'` only to the stack and passes
nothing to the body but the three position floats; `Body_SetPosition` (`0x089336f0`)
writes `body+0x30..` alone. **Pure's rail moves the craft's position and never its
velocity.** Confidence 85, static.

## Pitch

`Ai_ComputePitch` (`0x0891892c`) is Pulse's below the `0.9` ground fraction
(`craft+200 -> +0x268`), `clamp(-2500 * (err + 40 * rate), -100, 100)`, multiplied
by `FUN_08917744(|pos - own.pos|, 1030, 1180, 1, 0)`: a linear fade from `1` at `1030`
units from the own track sample to `0` at `1180`. Confidence 75.

## Forks

`Ai_ChooseBranch` (`0x08917f70`): Pulse's coin, `rand() & 0x100` picks the fork's
second or first path, decided on the round-robin tick once the fork is under a second
ahead at the current speed (`(fork progress - lap fraction) * lap_length / speed < 1`),
and **held for 2 s** (`driver+0x6c = 2.0`, counted down by `dt`, then `-1`), where
Pulse holds it to the merge. Confidence 75.

## Measured (2026-10-12)

PPSSPP 1.20.4 (software renderer), `pure-psp-usa.chd`, Single Race, Alpha league,
Vineta K, Feisar, eight craft, the player parked on the grid in eighth (no input).
Pure's Single Race has no difficulty setting (the walk goes Single Race, class,
league, track, team), so the two "levels" are **two speed classes, Venom and
Vector** - a deviation from the brief, named here. The tool is
`scripts/psp-pure-ai-thrust.py`: it stops at `0x08919728`, right after
`Ai_ComputeControls` returns inside `Ai_Update` (`s0` the driver), reads the law's
output at `sp+0x14` and every input the law reads (the class record, `driver+0x54`,
`+0x190`, `+0x1c0`, `+0x60`, `RM+0x134`, every racer's progress, place and lap), and
`--predict` recomputes each sample in `float32`.

| Run | Class | Race time | Samples (opponents) | Exact (`< 1e-3`) | Exercised |
| --- | --- | --- | ---: | ---: | --- |
| `trial` | Venom | 56 s | 9 | **9** | the cruise law under a zone multiplier of `0.72` |
| `venom-start` | Venom | 0 to 0.7 s | 280 | **280** | the launch: blend `1`, every opponent at its `StartSpeed` |
| `venom-decay` | Venom | 0 to 63 s | 420 | **420** | the blend ramping in (`0.5` on the first frame) and decaying over 500 frames, the zone plateaus and mid values, fatigue time accumulating while a lapping craft passes the player |
| `vector-start` | Vector | 0 to 63 s | 420 | **419** | the same on Vector's record |
| `vector-burst` | Vector | 2 to 80 s | 560 | **560** | two consecutive frames per sample, for the zone multiplier |

**1,689 opponent samples, 1,688 reproduced with worst residual `0.00000`.** The one
miss (Vector, 45.1 s, grid slot 5) logged `100` where the law gives `36.7`: the
output record's default, which `Ai_ComputeControls` returns untouched when
`Ai_SampleLookahead` or `Ai_LanePoint` fails (static reading; one sample, not
chased). That is the legacy thrust law above, measured: the per-slot cruise draw, the
speed tweak, `driver+0x54`, the launch blend toward `StartSpeed`. Confidence on the
thrust law: **94** (runtime trace on one binary; the EU pressing agrees statically
on the switch only).

The rest, from the same runs:

- **The switch reads `0` in a race** (`g_ai_use_new_stats`, every sample and the
  run header), so the legacy block is live: **94**, runtime plus the static sweep on
  both pressings.
- **The zone multiplier.** `RaceManager_UpdateZoneSpeed`'s law, recomputed from the
  logged racers (nearest non-player racer by wrapped progress gap, `Zone` or
  `LeadZone` by the player's place, piecewise linear), equals the `RM+0x134` the
  **next** frame logs on all 633 next-frame pairs (`trial` 8, `venom-start` 305,
  `vector-burst` 320), worst residual `0`: the manager writes it once a frame from
  the positions it sees then. The parked player never led, so the `LeadZone` branch
  is unexercised; the `Position` sign is exercised (the plateaus at the far ends and
  the mid values only come out with `gap = opponent - player`). **88** for the zone
  law, `Position` sign included.
- **No round robin in practice.** `AiManager_SnapshotRacers` sets `racer+0x1b0 = 1`
  on every racer every frame (`*(piVar7 + 0x6c) = 1`), so the manager's own
  round-robin byte is redundant: every stop on every frame had the byte set, eight
  drivers a frame (`ticks` in every capture). **Every opponent runs its thrust law,
  its lane choice and its fork choice every frame.** The round-robin paragraph above
  is the code as written; this is what it does. **90.**
- **Fatigue runs and changes nothing**: `driver+0xb0` rose while a lapping craft was
  inside the window, and `driver+0x54` stayed equal to `RM+0x134` (the shipped
  multiplier is unity).
- **The rail fires**: a stop on `Body_SetPosition`'s `jal` (`0x08917f40`) hit five
  distinct bodies in 60 stops (`venom-start`) and moved them `0.0` to `0.21` units in
  80 stops (`vector-burst`, median `0.008`). That the velocity is not written stays
  static (85).
- **The launch**: on the first race frame the blend was already `0.5`
  (`+0.5` per frame from `0`), `1` from the second; the drop to `0` took the logged
  500 frames once the field had strung out past `GridSpread`.

To reproduce, with a PPSSPP of your own on the USA disc, parked in a Single Race:

```sh
uv run --with websocket-client python3 scripts/psp-pure-ai-thrust.py --port <port> \
    --out data/scratch/<lane>/venom.json --samples 480 --gap-seconds 1.0
python3 scripts/psp-pure-ai-thrust.py --predict data/scratch/<lane>/venom.json
```

Not measured: the steering's output, the planner's choices (the lane and its
bang-bang), the pitch fade, the fork hold, the speed floor and the `35` floor, the
`x 0.7` flag, and the `LeadZone` branch.

## What is built, and what is not

Built (2026-10-12): `oag_tables::pure_ai_stats` reads the three files as the
executable does, and `oag_ai::original::cruise` carries the measured throttle
(`CruiseDraws`, `zone_multiplier`, `CruiseState`, `cruise_thrust`,
`finish_thrust`), with a unit test that reproduces a live sample to the bit and
`crates/game/tests/pure_original_ai_ground_truth.rs` on the disc. The two
`rand()` draws (the tweak and the eight cruise speeds) are drawn from the seeded
generator instead: **chosen, not measured** (the captures show they are redrawn every
race: two Venom races drew different values in the same bands).

Not built, by name: the steering (Pulse's law, already in `oag_ai::original`), the
ringless planner with its other-craft and track-object scoring (`FUN_08918d30`'s
object list is unread), the position-only rail, the pitch fade, the 2 s fork hold,
the every-frame update, and the field that would fly them. `original_opponent`
stays `false` for Pure: this project has no Pure campaign, and the original's
opponent is the campaign's. See
[gameplay/ai.md](../../../gameplay/ai.md#pure-a-sibling-driver-2026-10-12).
