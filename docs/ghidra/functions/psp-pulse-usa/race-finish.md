# The finish hand-over, and the spectator camera that follows it

How a craft that has crossed the line for the last time ends up flown by the AI while the
race keeps running behind the end-race panels, and what the camera does there. The behaviour
and its measurements are [`after-the-finish.md`](../../../gameplay/after-the-finish.md); this
page is the code. Image base `0x08804000`; `psp-pulse-usa`, `BOOT.BIN`.

Read from the decompiler (Ghidra headless, 2026-10-01) and **checked against live PPSSPP
runs** (breakpoint on `Ship_SetState`, write watchpoints on `craft+0x1d4` and on the control
record, a per-frame log of every craft; `scripts/psp-postrace.py`).

## The finish, craft by craft

| Address | Name | Confidence | What it is |
| --- | --- | --- | --- |
| `0x08842a18` | `Craft_UpdateLapProgress` | (already named) | at crossing `laps + 1` sets `entity+0x912` (finished) |
| `0x088418e0` | `FUN_088418e0` | not renamed | the per-entity update. For an entity with `+0x368 == 0` (the local player) or `2` (an AI craft): `if (entity+0x912 != 0 && Ship_State(entity) == 1) Ship_SetState(entity, 2)`; and `if (!(entity+0x860 & 0x800000)) Craft_SetAutopilotBlend(0.0, craft)` every frame |
| `0x08848664` | `Craft_SetAutopilotBlend` | 90 | `*(craft + 0x1d4) = value`. The autopilot weight |
| `0x0883b3b8` | `PlayerStatus_Update` | (already named) | the local player's per-frame status. Sets `craft+0x1d4` **`1.0`** when `entity+0x912 != 0 \|\| Ship_State(entity) == 2`; while the Autopilot pickup runs (`weapon record +0x1b8 & 0x800`) it sets the weight to `min(1, record+0x148)` and `craft+0x1c0 \|= 4`; otherwise `0.0` |
| `0x08849618` | `Ship_UpdateCraft` | (already named) | `craft+0x78 = craft+0x3c`; if `craft+0x1d4 != 0`, copies the pad's `0x28` bytes to the blend buffer `craft+0x44`, **then the record at `craft+0x40` over it**, and points `craft+0x78` at the buffer. A weight of any non-zero value therefore hands the craft to the AI's record whole, not a mix |
| `0x0883c870` | `PlayerInput_Update` | (already named) | writes the pad into the `player_input` record (`+0x44` steer, `+0x48` thrust, `+0x4c`/`+0x50` airbrakes, `+0x54` pitch) |

**Live** (Talon's Junction; the pad released and the pickup off 40 frames before the line):

- `craft+0x1d4` reads `0.0` on every one of the 40 frames the pad flew the craft and `1.0` on frame `F`, the frame
  `entity+0x912` and `craft+0x2a4` (the craft state) both read their finished values.
- `craft+0x78` (the control record the craft reads) is the pad's record (`craft+0x3c`) up to `F` and the blend buffer
  (`craft+0x44`) from `F+1`. `craft+0x3c` and `craft+0x40` never change.
- `Ship_SetState(entity, 2)` was caught **eight times in one Single Race, all from `0x08841ab8` (inside
  `FUN_088418e0`)**, the player's first and the seven opponents' at their own crossings. No other caller of
  state 2 fired.
- The control record is the AI's from `F+1`: `[steer, thrust, 0, 0, 0]` reads `[23.5, 100.0, 0, 0, 0]`, then `[-11.1, ...]`,
  `[18.2, ...]` (a bang-bang follower's steer), and the thrust slot goes `100 -> 56.7` and stays.

### `Race_FinishAllCrafts` is not this path

[`grid.md`](grid.md) and [`shield.md`](shield.md) say `Race_FinishAllCrafts` (`0x08824e10`) "is the flag":
it switches every craft to `autopilot_input` / `AI_input_%d` and sets state 2. That is **true of the
function and wrong for Single Race and Time Trial**. Its only caller found is `FUN_0882461c`, whose only
caller is `FUN_088dfb90`, inside the `GriefReport_*` screen code (a multiplayer path). **Live, it never ran**: the
mode flag it sets (`manager+0x1a78`) read `0` through every capture; the player's `entity+0x368` stayed
`0` (the function sets `2`); `flags860` stayed `0x600` (it sets `0x800000`); and no driver pointer changed. The
real finish is the per-craft rule above. The function and its names stand (confidence unchanged); what is
corrected is that nothing in these modes reaches it.

## The finished player's thrust: `AI_ComputeOpponentThrust` with the player's own rank (2026-10-04)

**Found and measured 2026-10-04 (lane `pulse-postfinish`).** The `56.7` the previous captures read is not a
constant of its own: it is the AI's position-balancing thrust law run on the player's own driver, with the
player's finishing place as the index. Confidence **90** for the law as stated below (decompile plus three live
runs at two finishing places, every constant they show reproduced to 0.1), **80** for `+0x82` being the
racer record's copy of `finished`.

| Address | Name | Confidence | What it is |
| --- | --- | --- | --- |
| `0x08853a80` | `Ai_Update` | (already named) | per-driver update. Sets the driver's `+0xa8` byte to `1` when its craft is the player (`DAT_08b34418 == driver[1]`), then calls `0x08855dc4` |
| `0x08855dc4` | `Ai_ComputeControls` | 75 | the driver's control record for this frame: steer (`out+8`), airbrakes (`out+0x10`, `+0x14`) and thrust (`out+4`). Thrust defaults to **`100`**; if the racer record's `+0x360` is set it calls `AI_ComputeOpponentThrust` **unless** the driver is the player's (`+0xa8`) and the racer record's `+0x82` is clear. Only the thrust branch is read |
| `0x08855904` | `AI_ComputeOpponentThrust` | 82 (unchanged) | the law below. **It also drives the finished player**, which is new |

The racer records are `0x08b34420 + i * 0x370`: `+0x00` the entity, `+0x80/0x81/0x82` three bytes that read
`started`, a one-frame `crossed` pulse and `finished` one frame after the entity's own `+0x910..0x912` (watched:
the player's `+0x82` goes `0 -> 1` on `F+1`, every opponent's on its own crossing), `+0x240` track progress,
`+0x344` the place. `DAT_08b34410` points at the player's record, `DAT_08b34418` holds the player's entity.

So the player's driver writes `100` until `+0x82` is set on `F+1`, and the law from `F+2`: the `100` for one to six
frames and the step on `F+2..F+7` the four 2026-10-01 captures saw.

### The law, as it reads for a craft whose `+0x82` is set

```text
A[]      = AIThrust of the race's speed class (AIRaceStats_<class>.xml, PlayerInPos1..8), A0 = A[0]
(off, mul) = lerp over the class's SkillScalePoint1..3 at AI_ResolveSkillScale()   // ThrustOffset, ThrustMultiplier
r        = the PLAYER's place (DAT_08b34410 + 0x344, 1-based)
ref      = the racer whose place is r - 1 (the craft one place ahead); the place-2 racer when r == 1
gap      = ref.progress + (own.spread - ref.spread) * 50 - own.progress    // spread: driver +0x74
step     = clamp(gap * max(2 - 0.08 * race_time, 0.5), -0.3 * A[r-1], +0.3 * A[r-1])
thrust   = off + (A[r-1] + step - A0) * mul + A0
thrust   = min(thrust, 100)   if race_time > 20 and |own.progress - player.progress| < 350
thrust   = max(thrust, 1)
```

With `+0x82` set the spread multiplier is the constant `50` and the field loop that finds the nearest craft ahead
and behind includes the craft itself, so both rubber-band terms come out zero (`WhenLeading`/`WhenBehind` need a
gap over `150`). Every craft indexes `A[]` and picks `ref` by **the player's** place, opponents included; that is
the player coupling [`ai-stats.md`](ai-stats.md#the-skillscale-consumer-ai_computeopponentthrust) already records.

**For the finished player `step` sits on its lower stop**: `ref.spread` comes out at about `5` (implied by the gap logged at `F+2`
was `-240.5` with the craft ahead only 10 units away), so the player aims about 250 units behind the craft ahead
and lands on `-0.3 * A[r-1]`. The finished player's thrust is then

```text
thrust = off + (0.7 * A[r-1] - A0) * mul + A0
```

until it falls about 195 units behind its target, when the clamp lets go and the thrust rises a little.

### The measurements

Venom, Talon's Junction, Single Race, `g_skill_level = 0` (Easy). Read live off the running race: `A = 92, 92, 91, 91,
90, 90, 89, 89`, `SkillScalePoint1..3` `ThrustOffset` `-7, 0, 6`, `ThrustMultiplier` `1, 1, 1`; `AI_ResolveSkillScale`'s
inputs `SkillScaleValue[Venom][Easy] = 0.9` (16_Track's `stats.xml`), mode 3 (Race), weapons on, eight crafts, so
`+ FullGridWithWeapons = 0.0` and no campaign cell: **skill `0.9`**, below Point1, so the lerp extrapolates:
`off = -7 * 1.1 + 0 * -0.1 = -7.7`, `mul = 1`.

| Run | Place | Predicted | Read |
| --- | --- | --- | --- |
| `sr-test`, `sr-noinput` (2026-10-01) | player 1st | `-7.7 + 0.7 * 92 = 56.7` | `56.7` |
| `sr-back` (2026-10-04, `--hold-back`) | player **4th** | `-7.7 + 0.7 * 91 = 56.0` | **`56.0`** on 1,241 of 1,497 frames from `F+3`; the rest `56.1..60.4` as the clamp lets go (`F+1140`: gap `-51.8`, `-7.7 + 91 - 25.9 = 57.4`, read `57.38`) |
| `sr-test` | an opponent at place 2 when the player won: its own `ref`, `gap = 0` | `-7.7 + 92 = 84.3` | `84.3` |
| `sr-back` | the opponent at place 3, the 4th-placed player's `ref`, `gap = 0` | `-7.7 + 91 = 83.3` | `83.3` |
| `sr-test`, `sr-noinput` | opponents behind the place-2 craft, far from the player | `-7.7 + 1.3 * 92 = 111.9` | `111.9` |
| `sr-test` | the same, within 350 of the player after 20 s | `min(.., 100)` | `100.0` |

The prediction written before the `sr-back` run was `100` flat for a not-first finish (the positive stop, `ref` ahead);
it **failed**, and the reason is `ref.spread`, which puts the target behind `ref` rather than on it. The place is the
finishing place and stays put: the player's `+0x344` read `4` for all 25 s although three crafts passed it on progress.
Capture: `scripts/psp-postrace.py --hold-back --arm-after 300 --laps-hack 1`, which also logs the racer records and the
player's driver now; `log.json`.

**Not read**: the `spread` value's own law (`FUN_08852ef4`, a random wander between per-driver bounds, not renamed),
so the unsaturated regime cannot be reproduced without the opponents' spreads; the steering half of
`Ai_ComputeControls`.

## The mode state and the HUD

`ArcadeRace_UpdateRacing` (`0x0882c5c4`, [shield.md](shield.md)) tests `player.flags & 0x1000 \|\| Ship_State(player) == 2`, then
`Hud_Hide`, `Race_BuildEndRaceResult`, `RaceMode_SetState(3)`. **Live**: `manager+0x7c8` goes `2 -> 3` and
`g_hud+0x168` goes `0 -> 1` on frame `F+1`, a frame after the craft's state reads 2.

## The spectator camera

The global camera object (`0x08b32c64`, constructed by `FUN_0887f9bc`) has a **spectator director** the
race-end flow drives. Static reading, with the timing checked live.

The functions below are named on [`camera.md`](camera.md#the-destroy-camera-mode-5-read-and-measured-2026-10-01)
(read independently by the destroy-camera lane the same day, from the same code): `Camera_UpdateSpectator`
`0x0887fd3c`, `Camera_UpdateSpectatorView` `0x08880c04`, `Camera_SetSubject` `0x0888058c`, `Camera_PickStation`
`0x0887fedc`, `Camera_FramingFov` `0x08880984`, `Camera_RepickNearSubject` `0x08880168`. This page adds two:

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x08880a58` | `Camera_PickSubject` | 85 | starts from the player (`manager+0x2c0`); if the drawn craft (`+0x1e4`) carries `flags(+0x860) & 0x1000` it is reset to the player; then while the subject equals the drawn craft and more than one craft is live takes a random live craft (`rand() % count`). With `cam+0x274` set the subject is the global `DAT_08ab0df8` instead |
| `0x08880b38` | `Camera_PickRandomMode` | 85 | `Psys_RandIntRange(0, 100)`: `< 26` mode `3`, `< 51` mode `2`, `< 76` mode `6`, else mode `7`; called when `Camera_UpdateSpectator` takes a new node. **Gated on `cam+0x26c`** (a byte, `1` from the constructor): with it zero the function changes nothing, which is what `Ship_SetState` case 4 does at a player's wreck |

`Camera_SetMode` (`0x08880724`) stores the view width `+0x268`: mode `5` -> `35.0`, `6` -> `17.0`, `7` -> `50.0`; the
constructor leaves `60.0`. `Camera_PickStation` and `Camera_RepickNearSubject` pick by the node's **aim point**.

`Camera_UpdateSpectator(dt)`, every frame: `+0x3c += dt`; past `10.0` s it resets the timer, clears the subject (unless
`+0x274` is set) and `+0x1e8 = -1`. It re-picks a cleared subject, then: with no node, takes the nearest; with a node,
runs the 60-unit test, and **if a new node was taken** it rolls a random mode and records the subject as the previous
one. Then it renders.

**Live**: the subject became the player on frame **`F+61`** in all four captures and was re-picked at **`F+661`** and
**`F+1261`** (600 frames, `10.0` s) in both Single Race captures, each time a different craft from the last (a Time Trial has one
craft, so its subject stays the player). The mode changed at irregular frames among `3`, `2`, `6`, `7`, which is the node test firing.

### The node cameras (modes 5, 6, 7)

One body. The camera sits **at the node** (`node+0x90`), looks at a **smoothed** subject point and zooms:

```text
smoothed += (subject_position - smoothed) * rate          // +0x230 toward +0x240, rate = +0x250 (= +0x264)
fov      += (target_fov - fov) * 0.06                     // +0x220 toward +0x224 (FUN_08880984), rate +0x228 (= 0.06)
look      = node_position - smoothed
look.y   *= max(1 - fov * 0.008, 0.4)                     // squash, so a wide zoom keeps the horizon
view      = look_at(look normalised, up = (0, 1, 0))      // FUN_0897019c
```

`rate` is `0.4 / 0.5 / 0.6 / 0.6` for speed class `0..3` (the constructor sets `0.3` by default), the
fov rate `0.06` (`0x3d75c28f`). Mode `5` is the death camera (`Ship_SetState` state 4 calls `Camera_SetMode(cam, 5)`), so
**the destroy camera and this one are the same code with a different view width**: the port shares it
(`oag_render::camera::destroy`: the station pick, the framing field, the pose).

**The node list was read live** (`16_Track`, forward track, PPSSPP): ten nodes at `cam+0x40`, each with a unit vector at `node+0x80` (the camera's back axis), the **eye at `node+0x90`** and an **aim point at `node+0xa0`**; the eye is the track `.vex` node's world translation and the aim is the three floats at `+0x10` of its `Camera` payload, all ten matching to 0.1 unit and in file order. The director picks nodes by the aim point and sits at the eye. Ported as `oag_raceplay::finish_camera`.

Modes `2` and `3` are read and measured: a rigid rear view and a rigid front view, see
[camera.md](camera.md#the-spectator-views-craft-relative-modes-and-the-directors-hand-off-rules-2026-10-02).
Cases `1` and `4`, `+0x274` and the reachability of `0` and `8` are [below](#race-end-photos-d-pad-modes-1-and-4-and-the-unreachable-cases-2026-10-04). **Not read**: the trigger that makes `Race End Photo` call `FUN_08880788` on frame `F+61`.

## `Race End Photo`'s d-pad, modes 1 and 4, and the unreachable cases (2026-10-04)

**Read and measured 2026-10-04 (lane `pulse-postfinish`).** `RaceManager_Update` (`0x08829778`, [gantry-clock.md](gantry-clock.md))
sets `manager+0x19ed` when the front end's state name compares equal to `Race End Photo` and then, every frame while it is
set (or under `g_game_mode 0xc`, `AI Race`), reads four buttons through `Input_IsPressed(g_input, n)`:

| `n` | Button (live) | What it calls |
| --- | --- | --- |
| `0` | up | `Camera_CycleModeForward` (`0x088807c8`): `1 -> 4`, `4 -> 3`, `3 -> 2`, `2 -> 7`, `6`/`7 -> 1` |
| `1` | down | `Camera_CycleModeBack` (`0x08880838`): `1 -> 7`, `7`/`6 -> 2`, `2 -> 3`, `3 -> 4`, `4 -> 1` |
| `2` | left | the craft one slot before the subject (`entity+0xbd4` is its 1-based slot, wrapping over `DAT_08b30f90`): `Camera_SetSubject(cam, craft, 1)`, `cam+0x1e4 = cam+0x1e0`, then `Camera_SetMode(cam, the mode it had)` |
| `3` | right | the craft one slot after |

The two cycle functions store `cam+0x1dc` directly, **not** through `Camera_SetMode`, so the view width a node mode set
(`+0x268`) is kept; mode `5` (and `0`, `8`) has no case and is left alone. `Camera_SetSubject` with no craft-attached node
(`entity+0xc3c == 0`, every craft seen) runs `Camera_PickStation` on the new craft: the nearest node, mode written `7`
(or `2` with no node), the zoom restarted. The chosen mode lasts until the director's next cut on its subject rolls
another (`Camera_UpdateSpectator`, unchanged).

**Live** (PPSSPP, `Race End Photo` after the `sr-back` finish): up took `+0x1dc` `4 -> 3 -> 2`, down `2 -> 3`; right moved
the subject one slot on twice, left one back (`manager+0x1a0c` `7 -> 4 -> 5 -> 4`). One aside, not ported: after a
left/right press the subject and the drawn craft can stay different for minutes (the 10 s re-pick takes the player back,
the drawn craft stays the chosen one, the node follows the drawn craft through the second, roll-free repick), and the
mode held `1` then `4` for over 100 s with the node changing every 4 s.

**Cases 1 and 4 of `Camera_UpdateSpectatorView`**, read and measured. Case 1 copies the drawn craft's matrix, turns it 180
degrees about its up (`vcst_s(5) * pi`: `2/pi * pi` is two quarter turns of the VFPU's `vsin`/`vcos`), then moves the
position back 5 along the turned forward: **an eye 5 units ahead of the nose at the craft's own height, looking the way it
flies**. Case 4 is case 3 with the same 180-degree turn first, so case 3's `(0, 3, 12)` offsets (`0x08ab10c8/cc/d0`) land
**12 behind and 3 above, looking the way it flies**. Both set `65` degrees. Measured with the mode word forced to 1 and 4
(`scripts/psp-spectator-capture.py --attach --force 1:20:180,4:220:380`, breakpoint on the view's exit): 200 and 179
frames, rotation exact, eye to `5.4e-5` (a scratch directory, not kept, `v0100.png` mode 1, `v0300.png` mode 4).
Confidence **92**.

**Not reachable after a single-player finish** (clean negatives):

- **`cam+0x274` and mode `8`.** The only writer of `+0x274` besides the constructor's zero is `Camera_SetSubjectLock`
  (`0x08880718`, a one-byte store), called from `FUN_088df590` (`GriefReport_Skin`, clears it) and `FUN_088dfb90`, which
  sets it to `1` together with `Camera_SetMode(cam, 8)` and `cam+0x26c = 0` after `FUN_0882461c`, the multiplayer path
  whose `Race_FinishAllCrafts` never ran in Single Race or Time Trial. With `+0x274` set the 10 s timer no longer clears the
  subject and `Camera_PickSubject` takes the global `DAT_08ab0df8`. Mode 8 is that path's view; not read further.
- **Mode `0`.** No store of `0` reaches the camera's `+0x1dc`: the constructor writes `7`, `Camera_SetMode`'s callers pass
  `1`-`8` (photo mode `FUN_08814014`: `4, 3, 2, 1, 7`), `Camera_PickStation` `7` or `2`, `Camera_SetSubject` `7`, the
  cycle functions `1, 2, 3, 4, 7`; the other `sw ..., 0x1dc(..)` in the binary are other objects (`FUN_08844ec4` counts
  into the profile at `0x08b31774`, `FUN_08891908` is a constructor). Read only: case 0 sits at the node eye
  (`node+0x90`), aims at the drawn craft and takes the node's own field of view (`node+0x50`), with no smoothing or zoom.
  Confidence **80** for the negative (a computed store would escape the operand search).

## Names

`Craft_SetAutopilotBlend`, `Camera_PickSubject`, `Camera_PickRandomMode`, `Ai_ComputeControls`,
`Camera_CycleModeForward` (90), `Camera_CycleModeBack` (90) and `Camera_SetSubjectLock` (75) are in
[`names.tsv`](names.tsv). The two cycle functions are complete five-case tables whose effect was watched live on the
buttons that call them; the lock setter is a one-byte store whose readers (`Camera_UpdateSpectator`,
`Camera_PickSubject`) are read, with no live leg. Confidence: the setter is a one-store function whose effect was watched live (a write
watchpoint on `craft+0x1d4` caught it from the two call sites above); the two camera names rest on the decompile and
on the timing the captures reproduce (the `10.0` s re-pick to the frame, the `F+61` start, the mode set), not on a
breakpoint inside them.
