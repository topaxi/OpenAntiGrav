# The opponent driver on HD: Pulse's, piece by piece, and what HD adds

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (PowerPC, TOC `r2 = 0x8ad4d8`; every function
below uses that TOC exactly, `scripts/ps3-toc.py toc`). Read 2026-10-12 by the
`orig-ai-hd` lane, which flies HD's original opponent in HD's campaign. The map is
Pulse's [psp-pulse-usa/ai-controls.md](../psp-pulse-usa/ai-controls.md): each Pulse
function was found on HD and compared instruction by instruction (capstone with the
Cell `lvlx`/`lvrx` forms decoded, because Ghidra stops at the first of them), constant
by constant and offset by offset. The thrust law and the parser are on
[ai-stats.md](ai-stats.md), which this page does not repeat.

**The answer in one paragraph.** HD's opponent is Pulse's opponent. The driver object
is Pulse's with every field `0x30` further in (Pulse `driver+0x48` rail blend is HD
`+0x78`, `+0x170` smoothed speed is `+0x1a0`, the lane ring `+0x220` is `+0x250`, and
so on), the racer table is Pulse's with a `0x9c0` stride, and the rail, the pitch
controller, the steering controller, the lookahead, the lane planner and ring, the
wander and the shuffle carry Pulse's constants bit for bit. What HD adds is five
things, each gated so that a single-player race that is not on two named circuits and
not a Duel runs Pulse's law exactly: the four thrust-law changes already on
[ai-stats.md](ai-stats.md#the-thrust-law-pulses-with-four-changes), a **halved
lookahead on two named circuits** (below), a **Duel-only** override and rail gap, an
**Eliminator** branch for one craft state, and a split-screen thrust multiplier.

Confidence is per row. Static reading unless a row says measured.

## The cast

| Address | Name | Confidence | Pulse counterpart | Verdict |
| --- | --- | --- | --- | --- |
| `0x000fee30` | `Ai_Construct` | 85 | `Ai_Construct` | Pulse's, plus two circuit flags (below). Registers `driver+0x34` as `"AI input %d"` or `"autopilot input %d"` (TOC slots `0x8a9618`/`0x8a9614`); `spread = 8 - grid` (`8.0` at `0x8a95a8`), wander phase `1e6` (`0x49742400`), segments `1.0`, amplitude `0` |
| `0x000ba9e0` | `AiManager_AddDriver` | 80 | `AiManager_AddDriver` | allocates `0x8c0` bytes, `Ai_Construct`, appends to the racer table (`+0x00` craft, `+0x04` driver, driver `+0x00` = racer record) and the driver list (`mgr+0x44` count, `+0x48` array), sets `mgr+0xb4` |
| `0x000ba750` | `AiManager_SnapshotRacers` | 80 | `AiManager_SnapshotRacers` | racer `i` at `table + 0x10 + i * 0x9c0`: `+0x10` position, `+0x20` up, `+0x30` forward, `+0x40` left, `+0x50` velocity, `+0x60` angular velocity (body `+0x200/+0x1e0/+0x1f0/+0x1d0/+0x190/+0x1a0`), then `0x931` bytes of `craft+0x6ff0` at `+0x70`; clears the round-robin byte `+0x9b0` |
| `0x000bb960` | `AIManager_Update` | 80 (was 70, [ai-stats.md](ai-stats.md)) | `AiManager_Update` | snapshot, the once-a-race shuffle, the round-robin byte on one racer, `AI_UpdateCraft` on every driver, then the weapon AIs. Same law; see [the round robin](#the-round-robin) |
| `0x00102d70` | `AI_UpdateCraft` | 82 (was 75) | `Ai_Update` | Pulse's order exactly; HD's additions are the Duel and Eliminator blocks below |
| `0x00100658` | `AI_ComputeControls` | 85 (was 65) | `Ai_ComputeControls` | identical steering law; adds the split-screen multiplier and the halved lookahead |
| `0x000fe688` | `Ai_SampleLookahead` | 85 | `Ai_SampleLookahead` | identical: `d = max(t * smoothed_speed, 15)`, `p = own + d / lap_length * 1.5`, wrapped, walked along the excluded-fork path (`driver+0x90`). HD passes the cross-track error in `f2` and never reads it |
| `0x000ff850` | `Ai_LanePointOnRing` | 85 | `AiLane_At` + `Ai_LanePoint` | the two inlined: nearest ring slot `(int)(p * 4000) - head`, wrapped `+102000 % 4000 - 2000`, clamped to `[0, count - 1]`, then `pos + lateral * (racing_line + span * l)`; always returns `1` |
| `0x000ff9b0` | `Ai_SnapToLane` | 88 | `Ai_SnapToLane` | identical, see [the rail](#the-rail) |
| `0x000fe510` | `Ai_ComputePitch` | 80 | `Ai_ComputePitch` | identical: `0` while the ground fraction (`craft+0x5fac -> +0x304`) is above `0.9`; else `clamp(-2500 * (err + 40 * rate), -100, 100)` with `last_fwd` at `driver+0x1c0` |
| `0x00101b50` | `Ai_PlanLane` | 78 | `Ai_PlanLane` | Pulse's, with both lane bumps inlined, see [the planner](#the-planner) |
| `0x000fdd18` | `AI_UpdateSpreadWander` | 85 (was 60) | `Ai_UpdateSpreadWander` | Pulse's plus the finished branch, see [the wander](#the-wander) |

`0x000fe818`, called between the planner and the controls with `dt`, is in Pulse's
`Ai_ChooseBranch` slot (path walks through `0x000a5278`, `rand()` draws, the
`0x9384e1` flag) and was not read: no name. `0x000ff340` writes the same fields as
`Ai_Construct` and has no caller; not read.

## The driver object: Pulse's plus `0x30`

Every field the recovered functions touch sits `0x30` further into HD's driver than
into Pulse's, which is what made the counterparts findable:

| Pulse | HD | Field |
| --- | --- | --- |
| `+0x08..+0x18` | `+0x34..+0x44` | the control record: steer, thrust, left, right, pitch |
| `+0x38` | `+0x68` | the race manager |
| `+0x48`, `+0x4c` | `+0x78`, `+0x7c` | rail blend, time on the rail |
| `+0x60` | `+0x90` | excluded fork path |
| `+0x64` | `+0x94` | steering authority |
| `+0x74..+0x98` | `+0xa4..+0xc8` | `spread`, start, end, phase, segments, amplitude, prev, next |
| `+0xa8` | `+0xd8` | "this is the player's driver" |
| `+0x130` | `+0x160` | 20-tick steer average |
| `+0x138`, `+0x140`, `+0x144` | `+0x168`, `+0x170`, `+0x174` | class record, `LookAheadSecs`, `SteerMul` |
| `+0x170`, `+0x174` | `+0x1a0`, `+0x1a4` | smoothed speed, steering state |
| `+0x190` | `+0x1c0` | pitch's last forward |
| `+0x1c8`, `+0x1f0` | `+0x1f8`, `+0x220` | preferred lane, smoothed scores |
| `+0x214..+0x220` | `+0x244..+0x250` | ring head slot, head index, count, slots |
| `+0x3b4`, `+0x3b8` | `+0x3e4`, `+0x3e8` | lane velocity, contact push |
| `+0x880` | `+0x8b0` | the dead second controller's gate |

The one exception is the "controls consumed" byte, Pulse `driver+0x21`, which is HD
`+0x52`: HD's control record is laid out with more fields after the axes.

The racer record: Pulse copies `craft+0x900..` to `racer+0x70`, HD copies
`craft+0x6ff0..`; so `racer+0x82` (finished) is `craft+0x7002`, `racer+0x898` (race
progress) is `craft+0x7818`, `racer+0x8ac` the track point, `racer+0x8b0` the own
sample (`+0x8f0` its lap fraction), `racer+0x994` the place.

## `AI_UpdateCraft` (`0x00102d70`), in order

Skipped (controls zeroed, `return`) when the racer or its point is null, the craft's
`+0x5fac -> +0x264` has bit `2`, `craft+0x6a64` has bit `0x1000`, or `Ship_State`
(`0x000d19b0`) is `1` while `craft+0x628c` is neither `0` nor `2`. Then, matching
Pulse's list item for item:

1. The ring advance (`(int)(racer+0x8f0 * 4000)`, the same wrap and the same reset to
   one slot holding `0` when everything was passed), inlined.
2. `dt * 60` into `0x8c1790`.
3. First call: the race manager and the class record (`RM + 0x1400 + class * 0x118`).
4. `driver+0x1bc = 1.0`; `driver+0xd8 = 1` for the player.
5. The excluded fork path to `craft+0x6fe8` (`-1` for the player unless `+0x52`).
6. `LookAheadSecs` to `+0x170`, `SteerMul` to `+0x174`, and **in a Duel** the zone
   rows' override ([ai-stats.md](ai-stats.md#the-duel-override-fury-and-a-clean-negative)).
7. `+0x1a0 = 0.95 * +0x1a0 + 0.05 * |velocity|` (`0x00103610`, `fmadds` with
   `0.95`/`0.05` at `0x8a955c`/`0x8a9550`).
8. `Ai_PlanLane` (`0x00103618`), `0x000fe818` (`0x00103624`), `AI_ComputeControls`
   (`0x0010363c`).
9. Steer and both airbrakes every tick, times the authority `+0x94`
   (`0x00103640..0x00103680`).
10. Thrust only on the racer's `+0x9b0` tick (`0x0010426c`, and again at `0x00104308`
    after the speed floor): **the same per-point speed floor**, `SplinePt+0x66 < 35`
    (`35.0` at `0x8a973c`) forces `100` when `|v|^2 < (-0.002715 x^2 - 0.869027 x +
    95.11588)^2` (`0x001045bc`), for any craft that is not `RM+0x13e8`.
11. **Eliminator only** (mode `8` or `0x14`, `craft+0x628c == 2`, `0x00103688`): a
    block that zeroes the controls and spins the body about its up axis at `5 pi`
    rad/s (`15.708`), written to the body orientation. Not ported, below.
12. `Ai_SnapToLane` (`0x001042c0`), **skipped in a Duel while `driver+0x30 > 0`**
    (below), then `Ai_ComputePitch` (`0x00103ac8`).
13. The 20-tick steer average into `+0x160`.
14. The second controller, gated on `+0x8b0`. Its only writers are `Ai_Construct`'s
    zero and the uncalled `0x000ff340`: dead, as on Pulse.

There is no counterpart of Pulse's debug byte that zeroes thrust (Pulse step 11).

## The round robin

`AIManager_Update` (`0x000bb9e8..0x000bba0c`) **increments** `mgr+0xac`, wraps it at
`8`, and then sets that racer's `+0x9b0`; Pulse sets `racer[mgr+0xa8]` and then counts.
Same period, same law, one tick of phase: HD's first lane tick goes to racer `1`.
Confidence 85 (read).

## The shuffle

`AIManager_Update` (`0x000bbadc..`), once, when `mgr+0xb4` is set and `+0xb5` clear:
the opponents (driver `+0x04` not the player's craft, table `+0x08`) in table order,
`numAIPositionSwaps` of `AIPackSwapping<level+1>` (`RM + 0x1400 + class * 0x118 + 0xdc
+ level * 0x18`, `level = g_GameState+0xdc`) adjacent swaps by `rand() % (n - 1)`,
segments `max((int) numIntermediatePositions, 1)`, amplitude `intermediateOffsetSize`,
phase `0`, `mgr+0x40 = -10000`. Pulse's exactly; the decompile shows the count read
after the loop, the instructions (`0x000bbbe0`) read it before. Confidence 88.

## The rail

`Ai_SnapToLane` (`0x000ff9b0`), against Pulse's `0x088542e4`:

```text
P = Ai_LanePointOnRing(own sample)                       // 0x000ffa14; bail if it fails
a = clamp((|v| - 50) * 0.01, 0, 1)                         // 50.0 0x8a9654, 0.01 0x8a963c
b = clamp((|dot(pos - P, down_own)| - 7) * -0.142857 + 1, 0, 1)   // 7.0 0x8a9650, -1/7 0x8a9638
w = driver+0x52 ? a * b : 0
if not the player (table+0x08) and body+0x4a0 (contacts) != 0: w = 0
driver+0x7c = w != 0 ? driver+0x7c + dt : 0
blend = w < blend ? w : min(blend + dt * (Ship_State == 2 ? 1.5 : 0.6), w)
f = blend^2 * 0.7                                          // 0.7 0x8a9644
if f >= 1e-6:                                              // 1e-6 0x8a9648
    v' = v - lateral * dot(v, lateral) * f * 0.25          // 0.25 0x8a9578
    body+0x190 = normalize(v') * |v|                       // stvx at 0x000ffd50..0x000ffd88
    Body_SetPosition(pos - lateral * dot(pos - P, lateral) * f)   // bl 0x000f5d58
```

Identical to Pulse's, including the unused-orientation shape (no orientation write).
The one difference is rounding: HD multiplies by `-1/7` in a fused multiply-add where
Pulse divides. Confidence 88.

**The Duel gap.** In mode `0xd` with the `0x9384e1` byte clear, `AI_UpdateCraft`
draws from the skill row's `Input` table (`RM + 0x2f2c`, stride `0x20` per skill):
when `driver+0x2c` reaches the row's period, `rand() * 2^-30 <= ` the row's chance
sets `driver+0x30` to the row's duration (`0x00104668..0x001046a4`); while `+0x30 > 0`
it counts down by `dt` and the rail is skipped (`r27 = 0`, `0x00103218..0x00103230`).
Only a Duel. Confidence 70 for the reading; not ported (no Duel opponent is wired).

## The steering controller

`AI_ComputeControls` (`0x00100658..0x00100e4c`), against Pulse's `0x08855dc4`:

- Output defaults `{byte 0, thrust 100, steer 0, left 0, right 0}` (`0x00100700..0x0010072c`).
- Height and airborne fraction: `air = clamp((|dot(pos - own.pos, up)| - 15) * 0.025, 0, 1)`
  (`15.0` at `0x8a95f8`, `0.025` at `0x8a9664`).
- Own lane at the own sample's lap fraction, through the same inlined ring read, and the
  cross-track error **from the centre-shifted point, without `racing_line`**
  (`0x0010084c..0x0010092c`: `l * (bound - racing_line)`, then `pos + lateral * that`,
  no add of `+0x54`). That is a second binary agreeing with Pulse's
  `Ai_LanePointFromCentre` reading. `x = dot(lateral_own, pos - point) * (1 - air)`,
  stored to the steering state `[2]`.
- **The halved lookahead** (`0x00100930..0x001009f0`): when `driver+0x8b4` is `1` or `2`
  the lookahead time is `0.5 * (LookAheadSecs + air)` (`0.5` at `0x8a957c`), else
  `LookAheadSecs + air`. See [below](#the-halved-lookahead-on-two-circuits).
- `Ai_SampleLookahead`, then `Ai_LanePointOnRing` on the ahead sample.
- The aim shift `0.7` ahead / `0.3` own (`0x8a9644`, `0x8a95bc`), times
  `xtrackMul * x * dist` (`record+0x0c`), the velocity lead `0.0676` (`0x8a9668`),
  the yaw prediction `dot(angvel, up) * -0.0206` (`0x8a966c`) through `sin`/`cos`
  (`0x00676fe8`, called with `r4 = 0` and `1`).
- `err = -dot(cross(fwd', dir), up)` (`fneg` at `0x00100d1c`), `derr = (err -
  state[0]) / (dt * 60)`, `steer = (err + derr * SteerDamp) * 950 * SteerMul`
  (`950` from `RM+0x13ec`, `SteerMul` from `driver+0x174`), stored to state `[1]` and
  to output `+0x0c` (HD-only, a copy).
- Saturation: above `100`, right airbrake `clamp((steer - 100) * 80 - 10, 0, 100)`;
  below `-100`, left airbrake `clamp((-100 - steer) * 80 - 10, 0, 100)`
  (`80`/`10` from `RM+0x13f0`/`RM+0x13fc`, `0x00100d48..0x00100e48`).
- Thrust on the `+0x9b0` tick when the driver is not the player's or the racer has
  finished (`0x00100d7c..0x00100d9c`): `AI_ComputeOpponentThrust`.
- **Split screen**: when `g_GameState+0xe4 > 1`, the thrust output (`100` or the law's)
  times `SplitScreenMultiplier[skill]` (`record + 0xc4 + 4 * skill`, `0x00100da4..0x00100dd0`).

Every constant equals Pulse's. Confidence 85 for the law as a whole; the yaw prediction
and the cross-track shift are long AltiVec runs read for their constants and operand
order, not traced lane by lane (75 for those two, as on Pulse).

## The halved lookahead on two circuits

`Ai_Construct` sets two bytes from the track's `PI_Track` name
(`*(g_GameState+0x1bc) + 0x78`, compared with `0x006768f8`):

- `driver+0x8b8 = (name == "05_Track") && class == 3` (`0x00782ce8`; class
  `g_GameState+0xd4`, `3` is Phantom);
- `driver+0x8b9 = (name == "20_Track")` (`0x00782cf8`).

`AI_UpdateCraft` then sets `driver+0x8b4` every tick (`0x001030b4..0x00103554`):

```text
f = racer+0x90                                   // a copy of craft+0x7010
+0x8b4 = 0
if +0x8b8 and (f > 4600 or f < 250):  +0x8b4 = 1
elif +0x8b9 and (f > 5400 or f < 400): +0x8b4 = 2
```

and `AI_ComputeControls` halves the lookahead time while `+0x8b4` is non-zero. In HD's
`PI_Track` table `05_Track` is **Ubermall forward** and `20_Track` is **Amphiseum
reversed** (`DATA00` `/data/plugins/frontend/definition.xml`); `05_Track` is on HD's
campaign grids 1 to 7 and `20_Track` on Fury's grids 10 to 15. What `craft+0x7010` is
**is the distance into the current lap**, measured (below): on Talon's Junction
(lap length `5070.3`) every racer's `+0x90` equals its race progress `+0x898` less
its completed laps times the lap length, to `0.1`, and the own sample's lap fraction
times the lap length. So both windows sit around the start line: the first 250 and the
last 470 units of an Ubermall lap for a Phantom, the first 400 and whatever lies past
5400 on Amphiseum reversed. Confidence 85 for the gate (read), 90 for the field
(measured).

## The planner

`Ai_PlanLane` (`0x00101b50..0x00102d6c`) against Pulse's `0x08854e7c`. Pulse's calls to
`Ai_AddLaneBump` are inlined, and the inlined constants are the bump's own arithmetic
for Pulse's two bumps:

| HD constant | Is | Pulse's bump |
| --- | --- | --- |
| `-0.0475` (`0x8a969c`) | `0.95 * 0.05` | `(0, 1.0, 0.05)`, the outer shoulder slope |
| `3.1667` / `-3.1667` (`0x8a96ac`/`0x8a96a0`) | `0.95 / 0.15 * 0.5` | `(preferred, 0.15, 0.5)`, outer shoulder |
| `0.3333` / `-0.3333` (`0x8a96a8`/`0x8a96a4`) | `0.05 / 0.15` | the same bump's inner slope |

The rest, each at Pulse's value: the authority ramp `0.025` toward `1`; the contact push
`+/-0.2` per kind-`1` contact (body `+0x220`, stride `0x50`, count `+0x4a0`), by the
sign of `dot(left, contact - pos)`, clamped `+/-2`, decaying `* 0.95`; the nine
candidates `i * 0.25 - 1`; the smoothing `0.9 * old + 0.1 * new`; the best of nine with
lane `0` as the default; the preferred lane on `rand() % 400 == 125`
(`0x00102198`) as `u = rand() * 2^-29 - 1`, `u >= 0 ? 1 - u^2 : u^2 - 1`; the player's
driver zeroing target and choice (`0x00102554..0x00102574`); the first-call locate
(radius `200`, `0x8a96c8`), the own lane clamped to `+/-3` into the ring and `+/-1`
into the choice; the authored band through `1/127.5` (`0.00784314` at `0x8a96b0`); the
ring fill's rates `0.0009`, `0.001`, `0.0011`, the brake test against `500`, the clamp
`+/-0.015`. **HD writes those rates as literals; Pulse builds them from one `0.001` at
run time; they are the same bits** (`0.001f * 0.9f = 0x3a6bedfb`, `1.1f * 0.001f =
0x3a902de1`, `0.5f / 0.001f = 0x43f9ffff`, `0.001f * 15 = 0x3c75c290`, each equal to
HD's literal). The own-lane computation guards a corridor narrower than `0.0001`
(`0x00102050`), which Pulse's was not recorded as doing.

HD's first-call branch also writes `driver+0x17c` (race time, `RM+0x13dc`) and
`+0x180` (that plus `13.0`); nothing in `0x000f9000..0x00105000` reads either.

The `2^-29` scale makes `u` uniform on `[-1, 1)` as Pulse's is, because the PS3 C
library's `rand()` returns 30 bits (measured below through the wander, which draws
with the same scale).

Confidence 78 for the planner as Pulse's: every constant and offset matches and the
control flow was read around each, but not every branch of the inlined bumps was
traced.

## The wander

`AI_UpdateSpreadWander` (`0x000fdd18`), against Pulse's `0x08852ef4`:

- **Finished** (`racer+0x82`): `spread = start = end = 8 - place` (`racer+0x994`),
  return (`0x000fdd40..0x000fdd98`). Change 2 of [ai-stats.md](ai-stats.md).
- Otherwise Pulse's: `f = clamp((racer+0x898 - RM+0x1948) / (laps * lap_length), 0, 1)`
  for `1 <= laps <= 19` (`cmplwi 0x13`), else `0`; the phase step `1 / segments`, the
  `0.999` stop, `next = amplitude * (rand() * 2^-29 - 1)`.
- `w = (f - (phase - 1/seg)) / (phase - (phase - 1/seg))` (`fdivs` at `0x000fde50`)
  where Pulse multiplies by `segments`: equal up to rounding.

Confidence 85.

## Measured (2026-10-12)

RPCS3 (interpreter decoder, `Assume External Debugger: true` in the lane's private
config so stop-reply PCs are exact), `hdfury-ps3-eu-dec.iso`, the Fury campaign's first
cell (Talon's Junction, mode `3`, VENOM, eight craft), the player parked on the grid
in eighth, the `0x9384e1` byte `0`. The tool is `scripts/rpcs3-hd-ai-thrust.py`: it
stops at `0x00100e3c`, right after `AI_ComputeOpponentThrust` returns, reads `f1`,
snapshots the driver's wander block, every racer's progress, place, finished flag,
craft kind and driver `spread`, the race time and the class record, and `--predict`
recomputes each sample in `float32` in HD's own operation order. The skill scale is
read once at `0x000fe03c`, right after `AI_ResolveSkillScale` returns.

| Run | Level | Skill scale | Samples | Exact (`< 1e-3`) | Race time | Branches |
| --- | --- | --- | ---: | ---: | --- | --- |
| `fury-park-1` | 0 (as the cell launched) | `-0.07` | 48 | **48** | 4 to 76 s | `WhenBehind` on, gap steps both ways and at the `+/-30 %` stops, the own-`ref` zero gap, the `100` cap after 20 s and `1000` before |
| `fury-park-hard` | 2 (written to `g_GameState+0xdc` during the load) | `3.48` | 30 | **30** | 4 to 45 s | the `SkillScalePoint2..3` branch, throttle above `100` (`104.58`) |
| trial | 0 | `-0.07` | 5 | **5** | 5.6 to 5.7 s | |

**83 samples, every one reproduced, worst residual `0.00000`**, and the logged
`driver+0xcc` matched the recomputed gap on each. That is HD's law with the place
indexing, the reference racer, the field-wide rubber band over the non-human racers
(`craft+0x628c` read `2` on the seven opponents and `0` on the player, so it is the
AI/human kind), the cap and the skill interpolation, measured on a second binary.
Confidence on `AI_ComputeOpponentThrust`: **94** (the row in the `hd-ai-classic`
section of `names.tsv` still says 85). Not exercised: `WhenLeading`, the finished
branches (changes 1 and 2), Eliminator (change 3), several humans (change 4).

- **The skill scale in a campaign cell is not the race default.** `-0.07` at Easy and
  `3.48` at Hard, both outside the `1..3` curve: the cell's branch through
  `0x001e1110` extrapolates, and the law takes the value as it comes.
- **The shuffle reads the integer level.** At level 2 the sampled driver's end slot
  differed from its start (`1` to `3`) and its segments were `4`, against `2` at
  level 0.
- **`rand()` is 30 bits.** Every wander draw seen (`next`, twelve distinct values over
  both runs) lies inside `+/-amplitude`, the most at `0.937` of it; with a 31-bit
  `rand()` three quarters of `rand() * 2^-29 - 1` would land outside. Confidence 90.
- **The rail fires.** A stop on `Ai_SnapToLane`'s `Body_SetPosition` call
  (`0x000ffda4`) hit all seven opponents in 40 stops, blend `0.48` to `0.66`, the move
  `0.006` to `0.063` units (median `0.014`; Pulse measured `0.012`). Confidence on the
  mechanism firing: 90.
- **`racer+0x90` is the lap distance**, above.

Not measured: the steering's output, the planner's choices, the pitch, and where
`AIManager_Update` falls against the world step (Pulse measured one update per step;
HD's world-step function is not identified).

To reproduce, with an RPCS3 of your own (`scripts/emu-env.sh`, then `rpcs3-drive.py
--interpreter serve` with `Assume External Debugger: true` in the private stock config,
`race`, and a `cross` at "START RACE"):

```sh
OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-ai-thrust.py --skill --samples 30 --gap-seconds 1.5 --out run.json
uv run --python 3.12 --with numpy python3 scripts/rpcs3-hd-ai-thrust.py --predict run.json
OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-ai-thrust.py --rail 40 --out rail.json
```

A run that a watchdog kills while it holds the stub costs the boot (the stub does not
come back); keep each invocation well under the tool ceiling.

## Not ported, by name

- The Eliminator spin (step 11 above): what `craft+0x628c == 2` means is not read.
  Our Eliminator handles a craft out of the race its own way.
- The Duel override and the Duel rail gap: no Duel opponent is wired in this project.
- The split-screen multiplier: no split screen.

## Omega: checked, static, the same driver

Omega's x86 `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`, image base `0x01000000`) was
read for this from data and code, not run (no PS4 emulator in this project's
toolchain). It carries HD's driver:

- The strings: `05_Track` and `20_Track` (file offsets `0x834de7`/`0x834e7d`, and a
  second pair), `"AI input %d"`, `"autopilot input"`, `SplitScreenMultiplier`,
  `TurnMul`.
- The code constants, each found as a little-endian float: `0.0676`, `-0.0206`,
  `950`, `4600`, `5400`, `-0.002715`, `95.11588`, `-1/7`. Absent: `0.0475`,
  `3.1667` and `15.708`, which the x86 compiler computes rather than loads.
- `4600.0` (`0x018011a4`) has one reader, `FUN_012c0480`: HD's `AI_UpdateCraft`
  with the planner, controls and rail inlined into one function. Its decompile
  shows the halved-lookahead gate with HD's windows (`fVar48 = racer+0x90`; `1`
  outside `[250, 4600]` under the first flag, `2` outside `[400, 5400]` under the
  second), the airborne fraction `(h - 15) * 0.025`, the `0.0676` velocity lead,
  the ring fill's `a * 15` clamp, the Eliminator branch on modes `8`/`0x14` and the
  Duel branch on mode `0xd`, which Omega extends to mode `0x15`.

**Verdict (updated 2026-10-12): checked, differs, ported.** Read whole on
[ps4-omega-eu/ai-controls.md](../ps4-omega-eu/ai-controls.md): in an HD-lineage mode
Omega's opponent runs HD's laws, plus a lane-search coin HD's `Ai_Construct`
(`0x000fee30`, no `rand() % 2`) and `Ai_PlanLane` (`0x00101b50`, no `% 200` draw) do not
have. `oag_omega::race::ORIGINAL_DRIVER` carries it and Omega's campaign flies it.
