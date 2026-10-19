# The opponent driver: controls, lane, thrust and the rail

**Binary:** `pulse-psp` `BOOT.BIN` (USA), image base `0x08804000`. Read
2026-10-11 by the `orig-ai-pulse` lane, which recovers the original opponent AI
for the campaign's third AI mode. This page is the instruction-level evidence;
[gameplay/ai.md](../../../gameplay/ai.md) carries the status table and the
implementable specification.

**What this page answers, in one paragraph.** An opponent is driven by the same
craft physics as the player: its driver writes a control record with the
player's own layout (steer, thrust, two airbrakes, pitch), the craft registers
that record as its input source, and `Ship_UpdateCraft` runs the identical engine,
airbrake, steering and pitch code on it. **But the physics is not the whole
story.** After the controls are written, `Ai_SnapToLane` (`0x088542e4`) moves
the rigid body directly: it removes part of the velocity across the track and
pulls the position toward the driver's lane point, up to 70 % of the lateral
error per frame, whenever the craft is fast, near the surface and touching
nothing. That is a kinematic rail on top of the physics (measured firing every tick; how much of the line it holds
is open). It is plain vector
arithmetic on the body, so it is reproducible deterministically, but our
physics crate has to expose a write to the body's position and velocity for it.

Confidence is per claim below. Static reading unless a row says measured.

## The cast

| Address | Name | Confidence | What it is |
| --- | --- | --- | --- |
| `0x08834b14` | `AiManager_Update` | 80 (already named) | per tick: `AiManager_SnapshotRacers`; once, `AiManager_ShufflePack`; sets the round-robin `+0x360` byte on one racer; `Ai_Update` on every driver; then every `WeaponAi` |
| `0x088352c0` | `AiManager_SnapshotRacers` | 80 | copies each craft's body pose and its `craft+0x900..+0xbf0` block into the racer table at `0x08b34420`, clears every racer's `+0x360`, refreshes `DAT_08b34410`/`DAT_08b34418` (player's racer record and craft) |
| `0x08834c5c` | `AiManager_AddDriver` | 85 | allocates `0x890` bytes, `Ai_Construct`, appends to the racer table (`+0x00` craft, `+0x04` driver, driver `+0x00` = racer record) and to the manager's driver list (`+0x40` count, `+0x44` array), sets `mgr+0xb0` |
| `0x088354c8` | `AiManager_ShufflePack` | 90 | the `AIPackSwapping` consumer, below; measured |
| `0x088357a0` | `AiStats_SetDefaults` | 88 | the AI-stats header constants and the per-class defaults, below |
| `0x08853a80` | `Ai_Update` | 85 (already named) | per driver per tick, below |
| `0x08855dc4` | `Ai_ComputeControls` | **85** (was 75) | the steering controller and the thrust slot, read at instruction level |
| `0x08855904` | `AI_ComputeOpponentThrust` | **94** (was 82) | the thrust law, read whole and reproduced on 1,140 live samples |
| `0x08852ef4` | `Ai_UpdateSpreadWander` | 92 | the pack-position wander (HD's name for the same function); measured |
| `0x08854e7c` | `Ai_PlanLane` | 80 | picks a lane in the AI corridor and fills the lane ring |
| `0x08853550` | `Ai_AddLaneBump` | 85 | adds a trapezoid score bump over the nine candidate lanes |
| `0x088533bc` | `AiLane_At` | 85 | the planned lane at a track progress |
| `0x08853438` | `AiLane_Advance` | 80 | consumes ring slots as the craft's progress passes them |
| `0x088534c8` | `AiLane_IsFull` | 90 | `count > 79` |
| `0x088534d8` | `AiLane_Push` | 90 | appends one slot |
| `0x08853508` | `AiLane_Last` | 90 | the newest slot |
| `0x08853534` | `AiLane_Reset` | 85 | one slot holding the given lane |
| `0x08852dac` | `Ai_LanePoint` | 85 | world point of a lane on a track sample, racing line included |
| `0x08852364` | `Ai_LanePointFromCentre` | 85 | the same **without** the racing-line term, see below |
| `0x088530a0` | `Ai_SampleLookahead` | 80 | the steering's lookahead track sample |
| `0x08852afc` | `AiTrack_SampleAtProgress` | 80 | walks the path to a progress and evaluates the spline there |
| `0x088542e4` | `Ai_SnapToLane` | 90 | the kinematic rail; measured, see below |
| `0x08853148` | `Ai_ComputePitch` | 75 | pitch input while airborne |

## Who drives whom: the control record

**The opponent's controls go through the player's input path.** Confidence 85,
four legs:

1. `Ai_Construct` registers `driver + 0x08` in the named registry as
   `"AI input %d"` (`0x08a7bf08`, with the racer index) for an opponent, and as
   the literal `"autopilot input"` for the local player
   (`craft+0x368 == 0`). The player's own pad record is `"player_input"`
   ([cannon-quake-leachbeam.md](cannon-quake-leachbeam.md), the
   `PlayerInput_Construct` row).
2. `Ai_Update` writes exactly the player record's layout at that base:
   `driver+0x08` steer, `+0x0c` thrust, `+0x10` left airbrake, `+0x14` right
   airbrake, `+0x18` pitch. That is `player_input`'s `+0x00/+0x04/+0x08/+0x0c/+0x10`.
3. `Ship_UpdateCraft` (`0x08849618`) reads every control through
   `craft+0x78 = craft+0x3c` and switches it to `craft+0x40`'s record only when
   the autopilot blend `craft+0x1d4` is non-zero
   ([engine.md](engine.md#the-frame-ship_updatecraft)). It then runs
   `Ship_UpdateEngine`, `Ship_UpdateBrakes`, `Ship_UpdateAirbrakes`,
   `Ship_UpdateSteering` and `Ship_UpdatePitch` on that record, for every craft.
4. `Ship_UpdateCraft` writes `record+0x19 = 1` on whichever record it consumed.
   For a driver, `record+0x19` is `driver+0x21`, the byte `Ai_PlanLane`,
   `Ai_ChooseBranch` and `Ai_SnapToLane` all test. So `driver+0x21` means "the
   craft is consuming this driver's controls": always for an opponent, only
   under the autopilot blend for the player. This is also where the autopilot
   pickup swaps the player's control source, which
   [autopilot.md](autopilot.md) left open.

**Measured (below):** every opponent's `craft+0x3c` holds `&driver+0x08`, so this is 92.

**What `AIThrust` multiplies, then.** The thrust output is the record's
`+0x04`, the same field the player's accelerate button writes as `0`/`100`.
`Ship_UpdateEngine` reads it as `throttle`: `T = throttle * Engine.amount`, and
a throttle above `100` also raises the acceleration cap,
`cap = throttle * 0.01 * (0.5 * speed + Engine.accelcap)`
([engine.md](engine.md#engine)). So `AIThrust`, `ThrustOffset` and the rubber
band are **percent of the player's full throttle on the same craft's own engine
stats**, and values above 100 are real: they push harder and lift the cap.
Confidence 85.

## `Ai_Update` (`0x08853a80`), in order

Skipped (all outputs zeroed) when the racer record's `+0x254` (the track-cursor
point) is null, the craft's `+0x94 -> +0x1c0` has bit `2`, or the craft's
`+0x860` has bit `0x1000`.

1. `AiLane_Advance(racer.progress, driver)`.
2. `DAT_08ab0edc = dt * 60` (frames this tick; exactly `1` at 60 Hz).
3. First call only: `driver+0x38` = the race manager (`"Race manager"`),
   `driver+0x138` = this class's AI record (`RM + 0x2d8 + class * 0x10c`).
4. `driver+0x8c..` copies; `driver+0x18c = 1.0`; `driver+0xa8 = 1` when this is
   the player's craft.
5. `craft+0x8f0 = driver+0x60` (the excluded fork path, see
   [ai-branch-choice.md](ai-branch-choice.md)); the player gets `-1` unless
   `driver+0x21`.
6. `driver+0x140 = LookAheadSecs` (`record+0x08`), `driver+0x144 = SteerMul`
   (`record+0x00`).
7. `driver+0x170 = 0.95 * driver+0x170 + 0.05 * |velocity|` (smoothed speed).
8. `Ai_PlanLane`, `Ai_ChooseBranch`, `Ai_ComputeControls(LookAheadSecs, driver, out, driver+0x174)`.
9. Steer and both airbrakes are written **every tick**, each times the
   authority `driver+0x64`: `driver+0x08 = out.steer * a`,
   `+0x10 = out.left * a`, `+0x14 = out.right * a`.
10. Thrust is written **only on the racer's `+0x360` tick** (below):
    `driver+0x0c = out.thrust`, and on that same tick, for a craft that is not
    the player (`RM+0x2c0`) and whose current track point has
    `SplinePt.unk_0x66 = x < 35`: if `|velocity| < -0.002715 x^2 - 0.869027 x + 95.11588`
    the thrust is forced to `100`. See [the speed floor](#a-per-point-speed-floor).
11. A global debug byte `DAT_08ab0ed8` zeroes thrust and airbrakes.
12. `Ai_SnapToLane(dt, driver)`, then `Ai_ComputePitch(driver)`.
13. A 20-tick moving average of the steer into `driver+0x130`.
14. A block gated on `driver+0x880` that overwrites steer and airbrakes from a
    second controller (`FUN_08856d1c`..`FUN_08856e68`). **`driver+0x880` has no
    writer but `Ai_Construct`'s zero** (a full-program `sb` sweep for `0x880(`
    and for `0x4c0(` off the `driver+0x3c0` sub-object), so the block is read as
    dead. Confidence 75; a live read of the byte closes it.

### The round-robin `+0x360` tick

`AiManager_SnapshotRacers` clears `racer+0x360` on every racer every tick, and
`AiManager_Update` then sets it on exactly one, `racer[mgr+0xa8]`, with
`mgr+0xa8` counting `0..7` and wrapping (`0x08b34780 = 0x08b34420 + 0x360`).
So **each driver's thrust law, lane planner and fork choice run once every eight
ticks**, phased by its index in the racer table, and its thrust is held between
updates. Steering runs every tick. The counter wraps at eight whatever the field
size, so a smaller field gets idle ticks and the period stays eight. Measured, 92.

## The AI-stats header

`AiStats_LoadAll` is called with `RM + 0x2c4` (`0x08829400`), and
`AiStats_SetDefaults` (`0x088357a0`) fills the first five words before the five
files are parsed. These are executable constants, not disc data:

| Offset | Address | Value | Read by |
| --- | --- | --- | --- |
| `+0x00` | `RM+0x2c4` | `950.0` | `Ai_ComputeControls`: steer gain, times `SteerMul` |
| `+0x04` | `RM+0x2c8` | `80.0` | `Ai_ComputeControls`: airbrake gain on steer saturation |
| `+0x08` | `RM+0x2cc` | `0.002` | no reader found |
| `+0x0c` | `RM+0x2d0` | `23000.0` | no reader found |
| `+0x10` | `RM+0x2d4` | `10.0` | `Ai_ComputeControls`: airbrake dead offset |

and per class, before parsing: `SteerMul = 1`, `SteerDamp = 0`,
`LookAheadSecs = 0.5`, `xtrackMul = 0`, `xtrackMax = 1`, `xtrackDamp = 0`.

**`xtrackMax` and `xtrackDamp` have no reader.** Every load off the class record
in `0x08850000..0x08858000` touches `+0x00`, `+0x04`, `+0x08`, `+0x0c` and the
balancing block; the `0x10(`/`0x14(` loads in that range are off the driver or
the stack. Confidence 70: a reader computing the address into a register first
would hide from that sweep. HD records the same for `xtrackMax`
([ps3-hdfury-eu/ai-stats.md](../ps3-hdfury-eu/ai-stats.md#three-extra-tags-and-who-reads-them-static-2026-10-08)).

## Track samples

`AiTrack_SampleAtProgress` and the racer's own projection both produce a
`SplinePt`-shaped record ([track.md](../../../formats/track.md#control-points)),
evaluated on the uniform cubic B-spline by `Spline_SampleSegment`
(`0x0887c7e8`): `+0x00` position, `+0x10` tangent, `+0x20` down, `+0x30`
lateral, `+0x40` progress, `+0x4c` `ai_bound_left`, `+0x50` `ai_bound_right`,
`+0x54` `racing_line`. The racer's own sample sits at `racer+0x260`
(progress at `racer+0x2a0`, its point pointer at `racer+0x254`).

**Lane coordinates.** A lane `l` in `[-1, 1]` is a position inside the authored
AI corridor: `0` is the racing line, `-1` the left bound, `+1` the right bound.

```text
Ai_LanePoint(sample, l)            = pos + lateral * (racing_line + span(l) * l)
Ai_LanePointFromCentre(sample, l)  = pos + lateral * (span(l) * l)
span(l) = l < 0 ? racing_line - ai_bound_left : ai_bound_right - racing_line
```

`Ai_LanePointFromCentre` omits the `racing_line` term (`0x08852364`: `lwc1
0x54`, `lwc1 0x4c`, `sub.s`, `mul.s`, then `pos + lateral * that`, no add of
`+0x54`). It is what `Ai_ComputeControls` uses for the craft's own cross-track
error, so that error is measured from the track centre shifted by the lane
span, not from the racing line. Read as written; confidence 85 that the
instruction sequence is as stated, and no claim about intent.

## `Ai_PlanLane` (`0x08854e7c`): which lane

Runs every tick, but its lane choice only on the `+0x360` tick.

- **Authority ramp.** `driver+0x64` moves `0.025` per tick toward `1`
  (toward `0` for the finished player while the autopilot is not consuming).
  Steer and airbrakes are multiplied by it.
- **First call:** project the craft (`AiTrack_LocatePosition`, radius `200`), set
  the lane ring to the craft's current lane coordinate clamped to `[-1, 1]`.
- **Contact push.** For each of the body's contacts with kind `1` (`body+0x170`
  stride `0x40`, count `body+0x370`): `driver+0x3b8 += 0.2` if the contact is on
  the craft's left (`dot(left, contact - pos) >= 0`), else `-= 0.2`. Clamped
  `[-2, 2]`, decays `* 0.95` per tick.
- **The choice (on the `+0x360` tick).** Nine candidate lanes `-1, -0.75, ..., 1`.
  Each tick's scores start at zero and get two bumps
  (`Ai_AddLaneBump(centre, half_width, weight)`, a trapezoid: full `weight`
  across `centre +/- half_width`, a `0.05/0.95` shoulder, falling to zero at
  `+/- 2 * half_width`):
  `(0, 1.0, 0.05)`, a weak pull to the racing line, and
  `(driver+0x1c8, 0.15, 0.5)`, a strong pull to the driver's **preferred lane**.
  Scores are smoothed `0.9 * old + 0.1 * new` into `driver+0x1f0..`, and the best
  of the nine wins, then refined around the craft's own current lane by a local
  search over lanes scoring above `-1` (nothing ever scores at or below `-1`, so
  this search never blocks anything).
- **The preferred lane** changes when `rand() % 400 == 125`, about once in 400
  ticks: `u = 2 * Ai_RandUnit() - 1`, lane `= u >= 0 ? 1 - u^2 : u^2 - 1`, a draw
  biased toward the corridor edges.
- **The player's driver** zeroes the choice: the autopilot follows the racing line.
- **Lane target** `driver+0x44 = clamp(chosen + push, -1, 1)`.
- **Authored lane band: kept out of, not held inside.** With
  `lo = SplinePt.unk_0x5c / 127.5 - 1` and `hi = unk_0x5d / 127.5 - 1` of the
  craft's current point (`craft+0xae4`): when `lo < hi`, a side is picked once
  (`driver+0x148`): `1` if the target is below the band's middle, else `2`.
  Side `1` caps the target at `lo` (`0x08855708: lwc1 f13,0x44(s0)`,
  `c.le.s f13,f12`, `bc1t` past `swc1 f12,0x44(s0)`), side `2` lifts it to `hi`
  (`0x08855730..0x08855748`). So the target never enters `[lo, hi]`: the band
  is an obstacle the lane goes round, on the side it approached from. Read at
  instruction level 2026-10-12 (the `orig-driver-pulse` build); an earlier
  version of this bullet said "held inside", which the instructions do not
  do. The side resets to `0` on any point without a band. The shipped default
  `0xff`/`0x00` gives `lo = 1 > hi = -1`, which disables it.
- **The ring.** `driver+0x220` holds 80 lane values, one per `1/4000` of a lap of
  progress (`driver+0x214` the head's progress slot, `+0x218` the head index,
  `+0x21c` the count). The planner appends until 80 are queued, rate-limited:
  a lateral velocity `driver+0x3b4` per slot changes by `0.0009` (accelerate),
  `0.001` (brake, when `v^2 * 500` reaches the remaining distance) or `0.0011`
  (reverse), and is clamped to `+/-0.015` lane units per slot. At instruction
  level (`0x08855758..0x088558bc`, 2026-10-12) every rate is built at runtime
  from one `a = 0.001` (`0x3a83126f`): `a * 0.9` to speed up, `1.1 * a` to
  reverse, `a` to brake, once `v^2 * (0.5 / a)` (the `500`) is at least the
  distance left, and the clamp `a * 15` through `vmin.s`/`vmax.s`.
- **The ring reads nearest-slot.** `AiLane_At` returns the queued slot
  `(int)(p * 4000) - head` slots on (wrapped into `-2000..2000` by
  `+ 102000) % 4000 - 2000`), clamped to the queue; no interpolation.
  `AiLane_Advance` reads the **own sample's normalised progress**
  (`racer+0x2a0`, `0x08853b50`), not the race progress at `+0x240`.

Confidence 80: read whole. Census (2026-10-12, [track.md](../../../formats/track.md)):
only `07_Track` authors a band, on 311 points over both directions; the data has
the shape of a band (confidence 75 for that reading of the bytes, no live leg).

## `Ai_ComputeControls` (`0x08855dc4`): steer and airbrakes

Inputs: `la = LookAheadSecs`, the driver, the output record, and the driver's
steering state at `driver+0x174` (`[0]` last heading error, `[1]` last steer,
`[2]` last cross-track). Craft frame from the racer record: `+0x10` position,
`+0x20` up, `+0x30` forward, `+0x40` left, `+0x50` velocity, `+0x60` angular
velocity (`AiManager_SnapshotRacers` copies them from the body).

```text
out = { thrust: 100, steer: 0, left: 0, right: 0 }
h        = dot(pos - own.pos, up)                      // height over the track sample
air      = clamp((|h| - 15) * 0.025, 0, 1)
own_lane = AiLane_At(own.progress)
x        = dot(lateral_own, pos - Ai_LanePointFromCentre(own, own_lane)) * (1 - air)
ahead    = Ai_SampleLookahead(la + air, driver)        // see below; on failure: return out
T        = Ai_LanePoint(ahead, AiLane_At(ahead.progress))
dist     = |T - pos|
c        = xtrackMul * x
T'       = T - lateral_ahead * (c * dist * 0.7) + tangent_ahead * |c * dist| * 0.7
             - lateral_own   * (c * dist * 0.3) + tangent_own   * |c * dist| * 0.3
dir      = normalize(T' - (pos + velocity * 0.0676))
yaw      = dot(angular_velocity, up) * -0.0206
fwd'     = (fwd.x * cos(yaw) + fwd.z * sin(yaw), fwd.y, fwd.z * cos(yaw) - fwd.x * sin(yaw)), normalized
err      = -dot(cross(fwd', dir), up)
derr     = (err - state[0]) / (dt * 60);  state[0] = err
steer    = (err + derr * SteerDamp) * 950 * SteerMul
if steer >  100: right = clamp((steer - 100) * 80 - 10, 0, 100); steer =  100
if steer < -100: left  = clamp((-steer - 100) * 80 - 10, 0, 100); steer = -100
if racer.+0x360 and (not the player's driver or racer.finished):
    thrust = AI_ComputeOpponentThrust(driver)
```

`Ai_SampleLookahead(t, driver)`: `d = max(driver.smoothed_speed * t, 15)`,
`p = own.progress + d / lap_length * 1.5` wrapped past `1`, sampled with the
craft's excluded fork path. `lap_length` is the first float of `"AI track data"`.

The tail (`0x08856750..0x088568d8`) was read instruction by instruction: the
`neg.s` on the heading error, the `div.s` by `DAT_08ab0edc`, `SteerDamp` from
`record+0x04` (`lw a0,0x138(s1)`, `lwc1 f15,0x4(a0)`), the clamp to `+/-100`
with `80.0` and `10.0` from `RM+0x2c8`/`RM+0x2d4`, and the `lbu 0x12(racer+0x70)`
finished test. Confidence 85. Two pieces rest on the decompile only: the
`0.7/0.3` split of the cross-track shift and the yaw prediction, which are long
VFPU sequences; 75 for those.

## `AI_ComputeOpponentThrust` (`0x08855904`): the thrust law, whole

Read with every load against the class record. `R` is the class record, `A[]`
`AIThrust` (`R+0x68`), `S[]` `SpreadDist` (`R+0x88`).

```text
Ai_UpdateSpreadWander(driver)                            // refreshes own.spread = driver+0x74
finished = racer.+0x82 ;  pstate = Ship_State(player craft)
s        = AI_ResolveSkillScale()
(P, Q, base) = s < 2 ? (Point1, Point2, 1) : (Point2, Point3, 2) ;  t = s - base
off  = lerp(P.ThrustOffset, Q.ThrustOffset, t)           // a(1-t) + bt, extrapolates outside [0,1]
mul  = lerp(P.ThrustMultiplier, Q.ThrustMultiplier, t)
smul = lerp(P.SpreadMultiplier, Q.SpreadMultiplier, t)
r    = player.place                                       // DAT_08b34410 + 0x344, 1-based
base_thrust = A[r - 1]
spread_dist = finished ? 50 : S[r - 1] * smul             // R + 0x84 + r*4: SpreadDist[r-1]
ref_place   = r - 1 < 1 ? 2 : r - 1
lead = trail = 100000
for every racer k:
    if k.place == ref_place: ref = k
    if finished or k.craft != player.craft:
        d = player.progress - k.progress
        lead  = min(lead, d)           // > 0: the player leads every opponent by at least this
        trail = min(trail, -d)         // > 0: every opponent leads the player by at least this
lead  = max(lead  - WhenLeading.MinSpeedChangeDist, 0)
trail = max(trail - WhenBehind.MinSpeedChangeDist,  0)
ref   = ref or own racer
gap   = ref.progress + (own.spread - ref.driver.spread) * spread_dist - own.progress
driver+0x9c = gap
T = RM+0x2b8                                              // race time, seconds
if |gap| > max(1000 - 50 T, 100): driver+0xa0 -= gap      // accumulator, no reader found
x = base_thrust + clamp(gap * max(2 - 0.08 T, 0.5), -0.3 base_thrust, 0.3 base_thrust)
if pstate not in {4, 5, 6}:
    x += min(lead  * WhenLeading.SpeedMul, WhenLeading.MaxSpeedChange)
    x -= min(trail * WhenBehind.SpeedMul,  WhenBehind.MaxSpeedChange)
thrust = off + (x - A[0]) * mul + A[0]
cap    = (T > 20 and |own.progress - player.progress| < 350) ? 100 : 1000
return max(min(thrust, cap), 1)
```

What it says, in words:

- **Every opponent indexes the tables by the player's place**, not its own.
  `AIThrust[place-1]` is the base throttle and `SpreadDist[place-1]` the spacing.
- **Position keeping.** Each opponent steers its throttle toward a slot
  `(own.spread - ref.spread) * spread_dist` units ahead of `ref`, the racer one
  place ahead of the player (place 2 when the player leads). With
  `spread = 8 - grid slot` before the shuffle, the field holds a formation
  around the player, front grid slots ahead. The gain starts at `2` per unit and
  falls `0.08` per second to a floor of `0.5`, and the correction is held to
  `+/-30 %` of the base.
- **Rubber banding is field-wide.** `lead` and `trail` are minima over all
  opponents, so the term is the same for every opponent: when the player leads
  **every** opponent by more than the dead band, every opponent gets
  `+min(lead * SpeedMul, MaxSpeedChange)`; when **every** opponent leads the
  player, every opponent gets the `WhenBehind` term subtracted. The names read
  from the player's side: `WhenLeading` is the player leading. This agrees
  with HD's direction ([ps3-hdfury-eu/ai-stats.md](../ps3-hdfury-eu/ai-stats.md#the-thrust-law-pulses-with-four-changes)),
  and it is **measured** here: with the player leading every opponent past
  the dead band, the logged thrust carries the added `WhenLeading` term on every
  sample (see [measured](#measured-2026-10-11)). Confidence 94.
- The rubber band is off while the player's craft is in ship state `4..6`.
- **Units.** Gap, dead band and `350` are in `racer+0x240` units, which is
  `craft+0xad0`'s lap progress. The [live check](#measured-2026-10-11) is what
  fixes it against the lap length.

`ai-stats.md`'s summary wrote `SpreadDist[grid_position]`; the index is the
player's place, corrected there.

## `Ai_UpdateSpreadWander` (`0x08852ef4`) and `AiManager_ShufflePack` (`0x088354c8`)

`own.spread` (`driver+0x74`) is the slot the thrust law steers toward.

**Setup, once per race.** `AiManager_Update` calls `AiManager_ShufflePack` the
first tick after a driver was added (`mgr+0xb0` set, `+0xb1` clear). With the
opponents (not the player) in table order and `start[i] = end[i] = driver+0x74`
(`8 - grid slot`, from `Ai_Construct`):

```text
level = g_skill_level                                     // 0..2, the integer difficulty, NOT the skill scale
row   = R + 0xc4 + level * 0x18                           // AIPackSwapping<level+1>
repeat (int) row.numAIPositionSwaps times:
    i = rand() % (n - 1);  swap(end[i], end[i + 1])       // adjacent transpositions
for each opponent:
    +0x78 = start, +0x7c = end, +0x80 = +0x84 = start
    +0x88 = 0                                              // wander phase
    +0x8c = max((int) row.numIntermediatePositions, 1)     // segments per race
    +0x90 = row.intermediateOffsetSize                     // wander amplitude
mgr+0x3c = -10000.0
```

**Every tick (inside the thrust law).**

```text
f = clamp((own.progress - RM+0x7ac) / (laps * lap_length), 0, 1)   if 0 < laps < 20, else 0
if phase <= f:
    phase = phase < 0 ? 0 : phase + 1 / segments
    prev  = next
    next  = phase < 0.999 ? amplitude * (2 * rand() * 2^-31 - 1) : 0
w = (f - (phase - 1/segments)) * segments
own.spread = prev * (1 - w) + next * w + start * (1 - f) + end * f
```

So over the race each opponent's slot slides linearly from its grid order to a
shuffled order, `numAIPositionSwaps` adjacent swaps away, plus a random offset
of up to `intermediateOffsetSize` re-drawn `numIntermediatePositions` times a
race. **This is the "pack swapping" choreography `ai.md` described, now with its
mechanism.** `RM+0x7ac` is written at race setup as the start point's progress
times the lap length (`0x088293ec`). Confidence 85 for both functions.

`Ai_Construct` leaves `phase = 1e6` and `amplitude = 0`, so a driver
`AiManager_ShufflePack` never reached keeps `spread = 8 - slot` flat.

## `Ai_SnapToLane` (`0x088542e4`): the rail

```text
P = Ai_LanePoint(own, AiLane_At(own.progress))           // own sample, racing line included
d = pos - P
a = clamp((|velocity| - 50) * 0.01, 0, 1)                 // 0 below 50, full from 150
b = clamp(1 - (|dot(d, down_own)| - 7) / 7, 0, 1)         // full within 7 of the surface point, 0 past 14
w = a * b
if driver+0x21 == 0: w = 0                                 // controls not consumed
if not the player and body.contacts (+0x370) != 0: w = 0
driver+0x4c = w != 0 ? driver+0x4c + dt : 0                // time on the rail
blend = driver+0x48
blend = w < blend ? w : min(blend + dt * (Ship_State == 2 ? 1.5 : 0.6), w)
f = blend^2 * 0.7
if f >= 1e-6:
    v' = v - lateral_own * dot(v, lateral_own) * f * 0.25
    body.velocity = normalize(v') * |v|                    // speed kept, direction turned
    body.position = pos - lateral_own * dot(d, lateral_own) * f
```

A rotation by the angle between the old and new velocity is built
(`asinf`, `Math_BuildAxisAngleMatrix` about the craft's up) into a stack buffer
at `sp+0xb0` and **never used**: the function returns right after
(`0x088548f0`..`0x0885491c`). So only velocity and position are written, never
orientation. Confidence 80 for that.

**Why this matters.** At full blend the craft is moved 70 % of the way to its
lane point across the track every frame, and 17.5 % of its sideways velocity is
removed, so any lateral error an opponent at speed, on the ground and clear of
contact picks up is closed within a few ticks. Measured below: it fires every
tick; how much of the line-holding is its doing is open.

## `Ai_ComputePitch` (`0x08853148`)

While the craft's ground fraction (`craft+0x94 -> +0x2b0`) is `<= 0.9`:
`rate = dot(cross(fwd, last_fwd), left)` (`last_fwd` at `driver+0x190`),
`err = dot(cross(fwd, tangent_own), left)`,
`pitch = clamp(-2500 * (err + 40 * rate), -100, 100)`. Otherwise `0`.
Confidence 75.

## A per-point speed floor

`SplinePt.unk_0x66`, read through the racer's `+0x254` point pointer, is a
per-point byte. On an opponent's `+0x360` tick, when it is below `35`, the
opponent's thrust is forced to `100` if it is slower than
`-0.002715 x^2 - 0.869027 x + 95.11588` (about `95` at `x = 0`, `62` at `x = 34`).
Read only. Census (2026-10-12, [track.md](../../../formats/track.md)): only
`06_Track` authors a value below `35` (all zeros, 80 points over both
directions), and that circuit's other values (at or above `128`) are a use of the
byte this reading does not cover. Confidence 70 on the reading of `+0x254`.

## Measured (2026-10-11)

PPSSPP, `pulse-psp-usa`, Single Race, Talon's Junction, VENOM, eight craft, weapons
as the fresh profile holds them. The tool is `scripts/psp-ai-thrust.py`: it stops
right after `AI_ComputeOpponentThrust` returns to the opponent branch
(`0x088568b0`), reads the result in `f0`, snapshots every input the law reads
(race time, every racer's progress and place, every driver's spread, the class
record), and `--predict` recomputes each sample from the log in `float32`. The
skill scale is read once at `0x088559b8`, after `AI_ResolveSkillScale` returns.
Two skills, set by writing `g_skill_level` (`0x08b31044`) and restarting the race
from the pause menu, which is the same global the front end's difficulty row
writes:

| Run | Skill level / scale | Player | Samples | Exact (`< 1e-3`) | Branches exercised |
| --- | --- | --- | ---: | ---: | --- |
| `easy-park` | 0 / `0.9` | parked on the grid, 8th | 160 | **160** | `WhenBehind` at its `MaxSpeedChange` clamp, gap step in both directions and at the `+/-30 %` stops, the own-`ref` zero gap |
| `easy-auto` | 0 / `0.9` | autopilot, 8th | 320 | **320** | the same, two later stretches of the race |
| `easy-lead`, `easy-far`, `easy-lead2` .. `easy-lead5` | 0 / `0.9` | moved ahead of the field in six steps, 8th to 1st | 500 | **500** | rubber band off (inside both dead bands), the `100` cap within `350` of the player after 20 s, `ref` = the place-2 racer when the player leads, and the **`WhenLeading` term added** once the player led every opponent by more than its dead band |
| `hard-park` | 2 / `3.0` | parked, 8th | 160 | **160** | the `SkillScalePoint2..3` branch at `t = 1` |

**1,140 samples, every one reproduced to `1e-3`, worst residual `0.00000`**, and
the logged `driver+0x9c` matched the recomputed gap on every sample. On the same
1,140 samples the sampled driver's `spread` (`driver+0x74`, rewritten by
`Ai_UpdateSpreadWander` at the law's entry) recomputes from its logged wander
state and progress to within `5e-7`: the wander is measured too. That is the
thrust law above, the place indexing, the field-wide rubber band and its
direction, the `ref` choice, the cap and the skill interpolation, measured.
Confidence on `AI_ComputeOpponentThrust`: **94** (runtime trace; HD's static
read is the same function, a second binary short of a runtime leg there).

**What these samples cannot tell apart**, so the confidence above is for the
formula as exercised, and these pieces of it rest on the read alone:

- `ThrustMultiplier` and `SpreadMultiplier` ship at unity, so the `A[0]` pivot
  of the final line and the `SpreadMultiplier` scaling fit any formula of the
  form `(x - c) * 1 + c`.
- `WhenBehind` was only seen at its `MaxSpeedChange` clamp, so its `SpeedMul`
  and dead band are untested; `WhenLeading` never reached its clamp.
- The ship-state `4..6` gate on the rubber band never fired.
- Writing non-unity multipliers into the live class record before a
  `--predict` run is the one-restart falsifier for the first item.

**Two skills, not two campaign cells (a deviation from the brief).** The runs
are Single Races with `g_skill_level` written to `0` and `2`. A campaign cell
reaches the same law through the same global (`CellSelection_CommitSelection`
writes the browsed rung to it, [race-campaign.md](race-campaign.md)), so the
shuffle's input is the cell's rung; but `AI_ResolveSkillScale`'s campaign
branch (the cell's `SkillScaleValue` position) was not exercised live here.

The "moved ahead" runs write the player's body position along the leader's
heading (`--teleport`), because the software renderer runs the emulator at a
few percent of real time and a player who catches the pack on merit is minutes of
waiting per step. Only the law's inputs change that way; the law itself is what
is measured.

**The rest, from the same runs:**

- **The control record.** Every opponent's `craft+0x3c` (craft = entity
  `+0x94`) equals its `driver + 0x08`; the player's `craft+0x40` equals its
  driver's `+0x08`. The opponent drives through the player's input path:
  **confidence 92** for the "same physics" answer, the rail below aside.
- **The round-robin tick.** In a run of consecutive stops the driver cycles with
  period eight (`racer+0x360` set on indices `2,3,4,5,6,0,1`, and the player's
  index 7 is the tick with no stop, its branch never calls the law). Confidence 92.
- **The pack shuffle.** After the restart at level 2 every opponent's wander has
  `segments = AIPackSwapping3.numIntermediatePositions` and its end slots are a
  different permutation; at level 0 the same fields hold `AIPackSwapping1`'s.
  The parity of each end permutation matches the authored swap count (an odd
  count of adjacent transpositions gives an odd permutation, both runs). So the
  shuffle reads the integer `g_skill_level`, not the skill scale. Confidence 90.
- **The rail.** `Ai_SnapToLane`'s `Body_SetPosition` call is hit for every
  opponent on every tick it is fast and grounded: 400 hits over 66 ticks in
  `easy-park` (five opponents every tick, two on about half), every opponent
  every tick in `hard-park`, the player too under the autopilot. Blend `0.17`
  to `1.0`. **But the move it makes is small in steady state**: over 120 calls
  the position changed by `0.0001` to `0.042` units, median `0.012` (the
  script's rail phase logs it; the stop is on the `jal`, whose delay slot loads
  the target's z, so the target is read from the stack buffer `a1` points at).
  So in steady state an opponent sits within a few hundredths of a unit of its
  lane point. **Whether that is the rail's doing or the steering already
  holding the lane is not separated by this**: small moves fit both. The test
  that separates them is to NOP the `jal` at `0x088548dc` and the velocity
  stores before it, then log the lateral error. Only the position write is
  measured; the velocity write and the speed and height gates are static.
  Confidence on the mechanism firing: **90**; on its effect on the racing
  line: open.
- `driver+0x880` read `0` on every driver in every run (the second controller
  is dead in a single race), and `driver+0x21` read `1` on every opponent.
- Lap length (`*("AI track data")`) `5068.73` on this circuit; `racer+0x240`
  is in those units, consistent with the wander's `(progress - start) / (laps *
  lap_length)`.

To reproduce, with a PPSSPP of your own in a Single Race (`psp-drive.py menu
--single-race`):

```sh
uv run --with websocket-client --with numpy python3 scripts/psp-ai-thrust.py \
    --port <port> --out data/scratch/<lane>/easy-park.json --park --samples 160 --gap-seconds 0.25
uv run --with numpy python3 scripts/psp-ai-thrust.py --predict data/scratch/<lane>/easy-park.json
```

**Not measured:** the steering controller's outputs against its formula (the
lookahead sample would have to be logged), the lane planner's choices, the
pitch controller, and the `SplinePt.unk_0x66` speed floor. Those stay at their
static scores above.

## Where `AiManager_Update` sits in the frame (measured 2026-10-12)

`AiManager_Update` is slot 4 of the vtable at `0x08ac9f20` (constructor
`FUN_08834934`, which also sets `DAT_08b34358` to the manager); the world step
is slot 4 of the vtable at `0x08aca218` (constructor `FUN_0884e4a4`), whose
update `FUN_0884e5bc` calls `Body_StepWorld` once with `dt` clamped to
`0.06666`. Both vtables share their other entries (`0x089444d0`,
`0x0894479c`, ...): one base class, updated in list order by whatever walks
it. Neither has a static caller.

PPSSPP v1.20.4 fires only the most recently added execution breakpoint, so
the two cannot be interleaved in one run (`scripts/ppsspp_debugger.py`'s
`each_hit_any` records the same limit). Instead, a breakpoint on
`Body_StepWorld` read the manager's round-robin counter (`mgr+0xa8`) on 40
consecutive hits in a Single Race (VENOM, `psp-drive.py menu --single-race`):
`7, 0, 1, 2, ... 6`, advancing by exactly one at every step. **One AI update
per world step**, so `Ai_SnapToLane`'s body write, made inside the AI update,
lands between the controls it accompanies and the next `Body_StepWorld`.
Confidence 90. `oag-raceplay` applies the rail there.

## The player's own driver: the autopilot

Read 2026-10-19 by the `autopilot-original-ai` lane. **No new names**: every
function below is already in the cast. The question was whether the autopilot
(the Autopilot pickup, and the craft past the finish) flies the opponents'
driver or steering of its own, and whether it carries a pace rule that would
explain it overtaking opponents.

**It is the opponents' `Ai_Update` on the player's own driver**, the record
`Ai_Construct` registers as `"autopilot input"` and `Ship_UpdateCraft` reads
through `craft+0x40` (above, "Who drives whom"). That driver is updated every
tick like any other; four branches on "this is the player's craft" set it apart:

1. **Lane `0`.** `Ai_PlanLane` (`0x08854e7c`), after the contact push is added,
   clamped and decayed: `lw a0,0x4(s0)` (the driver's craft), `lw a1,0x8(s1)`
   with `s1 = 0x08b34410` (so `0x08b34418`, the player's craft),
   `bnel a0,a1` at `0x08855664`, then `swc1 f28,0x1b0(s0)`,
   `swc1 f28,0x1c0(s0)`, `swc1 f28,0x44(s0)` at `0x0885566c..0x08855674`, with
   `f28` set by `mtc1 zero,f28` at `0x08854ecc`. The chosen lane and the target
   are `0`: the racing line. The authored band test after it still runs, and the
   ring fills toward `0` at the usual rate. A second, earlier zeroing on
   `driver+0xa8` (`lbu a0,0xa8(s0)` at `0x088555f8`) clears the same fields
   before the push is added. Confidence **88**: instruction level, and HD's
   planner does the same at `0x00102554..0x00102574`
   ([ps3-hdfury-eu/ai-controls.md](../ps3-hdfury-eu/ai-controls.md)).
2. **Throttle `100`, no thrust law.** `Ai_ComputeControls` (`0x08855dc4`) loads
   `f26 = 100.0` at entry (`lui a0,0x42c8; mtc1 a0,f26`, `0x08855dd0..0x08855dd8`)
   and stores `f26` as the output thrust (`swc1 f26,0x4(s4)`, `0x088568d8`). It
   calls `AI_ComputeOpponentThrust` only on the round-robin tick and only when
   `driver+0xa8` is clear (`0x0885689c..0x088568b4`) or the racer has finished
   (`lbu a0,0x12(racer+0x70)`, `0x088568b8..0x088568d4`). So a racing player's
   driver flies a flat `100`: no `AIThrust[place - 1]`, no formation, no rubber
   band, no `100` cap within `350` units after `20` s. `Ai_Update`'s speed floor
   is skipped for the player too (`RM+0x2c0`). Confidence **90**: instruction
   level, and two other binaries agree (HD `0x00100d7c..0x00100d9c`, Pure's
   cruise path "keeps the output default `100`",
   [psp-pure-usa/ai-controls.md](../psp-pure-usa/ai-controls.md)).
3. **The rail through contact.** `Ai_SnapToLane` zeroes the rail on a contact
   only for a craft that is not the player's (above, "the rail"); the player's
   rail holds through a wall or a craft. Confidence 80, as the rail's read.
4. **Authority.** `Ai_PlanLane`'s head ramps `driver+0x64` by `0.025` a tick
   toward `1`, and toward `0` only for a finished player whose controls are not
   consumed (`driver+0x21 == 0`). So a racing player's driver sits at authority
   `1` all race and is warm whenever the autopilot takes over.

**No pace rule exists to port.** Nothing in these three functions scales the
player's driver's throttle, steer gain or lookahead; the only difference in pace
is the one above: the autopilot holds `100` where an opponent flies
`AIThrust[player.place - 1]` plus a formation term, capped at `100` near the
player after 20 s ([the thrust law](#ai_computeopponentthrust-0x08855904-the-thrust-law-whole)).
The opponents' throttle the captures above logged ran from `34.6` (Easy, the
lower stop) to `101.7` (Hard, the upper stop), with the reference racer at `75.0`
on Hard, and the autopilot also holds the racing line through contact, so an
autopiloted craft gains on a field that sits in formation around it. That is the
explanation for "the autopilot overtakes", and it is the law, not a bonus.

**An opponent holding the Autopilot never fires it in a race.**
`WeaponAi_Update`'s switch has no case for id 6, so its default arm sets the
absorb index `+0x34 = 3` and leaves the fire index `+0x38` at `0`, and
`g_weapon_ai_rate[0]` is `0.0`: fire chance `0` per decision
([weapon-ai.md](weapon-ai.md)). In an Eliminator (`g_game_mode == 8`) the fire
index becomes `1` and `rate_eliminator[1] * use * 5` can fire it (Pulse's
`WeaponAIstats.xml` authors the row `useAgainstPlayer 1.1`, `useAgainstAI 1.2`,
`absorb 1`, read 2026-10-19 through `oag_tables::weapons::ai`, so `0.012` a
decision with no target in range); what
`Autopilot_Fire`'s running bit then does to a craft already flown by its driver
is not read. An opponent is flown by its driver anyway, so outside that case
there is nothing for the pickup to change.

**Read since (2026-10-19):** what sets `craft+0x1d4` is `PlayerStatus_Update`. The
same branch gives the craft thrust x1.2 and no acceleration cap while the pickup runs
([autopilot.md](autopilot.md#what-the-running-bit-does-to-the-craft-2026-10-19-autopilot-rail)).
`Ai_SnapToLane` has no player gain: measured live, the player's lateral offset from
the lane point is p50 0.2 to 0.5. **Not read:** whether the
`"autopilot input"` driver is flown in a split-screen race (HD scales its `100`
by `SplitScreenMultiplier` when more than one person races).

**Wired** in `oag_ai::original::Autopilot`, flown by
`oag_raceplay`'s `Race::autopilot_controls` on every title whose original tables
read, whatever the opponents fly; see [gameplay/ai.md](../../../gameplay/ai.md#the-autopilot-flies-the-originals-player-driver-2026-10-19).
