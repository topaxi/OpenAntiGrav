# HD's Pilot Assist (2026-10-10, pilot-assist)

How Wipeout HD/Fury's Pilot Assist keeps a new player off the walls: a corridor spring on the
AI track spline that yaws the craft back toward the middle and pushes it sideways, paid for
with a thrust percentage. Read in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, every function
below on TOC `r2 = 0x008ad4d8`, `scripts/ps3-toc.py toc` reports each `exact`), past Ghidra's
AltiVec truncation by hand from the bridge's disassembly. The behaviour page is
[pilot-assist.md](../../../physics/pilot-assist.md); the option flag and the throttle reading
92 come from [rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md#a-per-frame-craft-trace-2026-10-08-hd-handling).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000c6af0` | `Handling_ReadGlobalClasses` | 80 | the only function that names `"PilotAssist"`, `"laDistConst"` and the other ten attributes (`ps3-toc.py attrib`); writes them at `0x0098d6b0 + class * 0x2c` |
| `0x000efb50` | `Craft_UpdateThrottle` | 72 | ramps and writes `craft+0x30c`, the throttle [physics.md](physics.md) reads, then multiplies it by `PilotAssist_ThrustScale` |
| `0x000f8c30` | `PilotAssist_Construct` | 72 | zeroes `+0x0`, `+0x4` (byte), `+0xc`, `+0x10` of `craft+0x380`; called from both craft constructors (`0x000eef08`, `0x000f00e8`) on `craft+0x380` |
| `0x000f8c50` | `PilotAssist_SetEnabled` | 75 | `stb r4, 0x4(r3)`; its one caller (`Craft_Update`, `0x000f1dbc`) passes the option flag with the gates below |
| `0x000f8c58` | `PilotAssist_ThrustScale` | 80 | three returns read straight off the table: `thrustPercentOnUse`, `1.0`, `generalThrustPercentWhenEnabled`, each times `0.01` |
| `0x000f8cb8` | `PilotAssist_ProbeCorridor` | 68 | locates on `"AI track data"` and measures the lateral offset against `+0x30`/`+0x44`/`+0x48` of the 0x70-byte spline record ([track.md](../../../formats/track.md#control-points)) |
| `0x000f8fe8` | `PilotAssist_Update` | 75 | the per-tick law below; one caller, `Craft_Update` at `0x000f1e24` |
| `0x0098d6b0` (data) | `g_PilotAssistClassTable` | 80 | `0x008a9398`'s target, the parser's `0x2c` stride from `0x0098d600 + 0xb0`, and `PilotAssist_ThrustScale` indexes it with `g_GameState+0xd4` (the speed class) |

`0x0098d600` is the global settings block `<Global>` fills (`0x008a8674` in the parser's TOC);
not renamed here, since only its `+0x10` (the class being parsed) and `+0xb0..` are read in
this lane.

## The table

`Handling_ReadGlobalClasses` walks each `<GlobalClass name=...>`, matches the name against four
class names (`FUN_00011ed8(x, 0..3)`) and stores the index at `0x0098d600+0x10`. HD's
`/data/xml/handlingstats.xml` authors five rungs with `VECTOR` first; `VECTOR` matches none of
the four, so its values land in whatever index was left there and are overwritten by the
`VENOM` rung that follows. Either `<PilotAssist>` or `<PilotAssistPenalty>` opens the same
eleven-attribute loop (`0x000c7400`-`0x000c75c8`), so an attribute may sit in either element:

| Offset | Attribute | Venom/Flash | Rapier | Phantom |
| --- | --- | --- | --- | --- |
| `+0x00` | `laDistConst` | 10 | 10 | 10 |
| `+0x04` | `laDistVelMul` | 0.35 | 0.4 | 0.4 |
| `+0x08` | `laDistMax` | 60 | 75 | 75 |
| `+0x0c` | `springMul` | -100 | -15 | -15 |
| `+0x10` | `torqueMul` | 60 | 80 | 80 |
| `+0x14` | `maxTorque` | 1000 | 1000 | 1000 |
| `+0x18` | `maxAngVel` | 3 | 3 | 3 |
| `+0x1c` | `generalThrustPercentWhenEnabled` | 99 | 98 | 97 |
| `+0x20` | `thrustPercentOnUse` | 92 | 90 | 88 |
| `+0x24` | `timePenaltyOnUse` | not authored | | |
| `+0x28` | `penaltyDuration` | 3 | 3 | 3 |

The values are the disc's; this page quotes them so the law below can be read, and the
engine reads them from the disc (`oag_tables::handling::Global`). `timePenaltyOnUse` is parsed
and never read by any function in this lane: no reader of `+0x24` was found.

## The state, `craft+0x380`

| Offset | Field |
| --- | --- |
| `+0x0` | acting: `+1` or `-1` (the sign of the torque) on a tick the torque exceeds `25`, else `0` |
| `+0x4` | enabled (byte) |
| `+0x8` | the `"AI track data"` handle, looked up on first use (`0x0031d298`, name at `0x00782920`) |
| `+0xc` | penalty timer, seconds |
| `+0x10` | blend, `0..1` |

## `PilotAssist_ThrustScale` and the throttle

    if timer > 0:   return thrustPercentOnUse * 0.01           # 0x008a93a4 = 0.0, 0x008a939c = 0.01
    if !enabled:    return 1.0                                 # 0x008a93a0
    return generalThrustPercentWhenEnabled * 0.01

`Craft_UpdateThrottle` ramps the throttle and then stores `craft+0x30c = target * scale`
(`0x000efc48`-`0x000efc58`). So with the option off the scale is the literal `1.0`: Pilot
Assist cannot be the hd-handling lane's 1% late-speed gap. The timer is only set while
enabled, so an off craft always reads `1.0`.

## When it is enabled (`Craft_Update`, `0x000f1d5c`-`0x000f1dbc`, `0x000f26b4`)

    enabled = options[ship+0x7a60 + 0x473] != 0         # the save flag, one byte per local player
          and ship+0x628c == 0                          # not a network-remote craft (absorb-feedback.md)
          and (byte 0x009384e1 != 0 or mode not in {6, 13, 14})
          and craft+0x2f8 == 1                          # racing, not grid (Craft_SetState, hover-target.md)
    if mode > 15: enabled &= (byte(g_GameState+0xed) >> 3) & 1   # an online lobby's own allowance

Mode `14` is Detonator ([mode-manager.md](mode-manager.md)); `6` and `13` are the other two
no-`ModeManager` modes, Zone among them by elimination, which 2048's tip text agrees with
("Pilot Assist is disabled automatically in Zone events"). **Nothing reads the steering
input**: the assist acts whatever the player holds.

## `PilotAssist_Update` (`0x000f8fe8`)

`f1` is the frame's delta, `r4` the rigid body (`craft+0x270`), `r6` bit 0 is
`craft+0x304 != 0` (any hull point grounded), and the four words at `ship+0x7820` are the
ship's track cursor, passed through to the locate.

    acting = 0
    timer  = max(0, timer - dt)
    if !enabled: return
    target = grounded ? 1 : 0
    speed  = |body+0x190|                       # velocity
    spin   = |body+0x1a0|                       # angular velocity
    la     = min(laDistConst + speed * laDistVelMul, laDistMax)
    ahead  = body+0x200 + body+0x1f0 * la       # position + forward row * la
    (aL, aR) = ProbeCorridor(ahead, forward, radius 2.0)      # 0x008a93c8
    (pL, pR) = ProbeCorridor(position, forward, radius 5.0)   # 0x008a93cc; leaves its record in the buffer
    if dot(record.down, body+0x1e0 (up row)) > -0.5: target = 0   # 0x00782910; craft not upright on the track
    blend  = blend <= target ? min(blend + 2 * dt, target)        # 0x008a93c8
                             : max(blend - 20 * dt, target)       # 0x008a93d0
    force  = body+0x1d0 (right row) * (pL + pR) * springMul * blend    -> 0x000f5860, body+0x150 (world force)
    torque = clamp((aL + aR) * torqueMul * blend, -maxTorque, maxTorque)
    if spin > maxAngVel: torque = 0
    if |torque| > 25:  acting = sign(torque); timer = penaltyDuration  # 0x008a93b0 = 25.0
    local torque += (0, torque, 0, 0)                                  -> 0x000f5dd8, body+0x170 (local torque)

`0x000f5860` and `0x000f5dd8` are four-instruction adders into `body+0x150` and `body+0x170`
(read in full); `body+0x170` is the local torque accumulator `Craft_UpdateSteering` writes
([hover-four-point.md](hover-four-point.md)).

## `PilotAssist_ProbeCorridor` (`0x000f8cb8`)

`(point v2, dir v3, radius f1, record buffer r5, track r6, cursor r3/r4, out_left, out_right)`.

    rec = locate(track, point, cursor, search 1000.0)   # 0x000a97f0
    d   = dot(point - rec.pos, rec.lateral)             # +0x00, +0x30
    L   = max(0, radius - rec.half_width_left - d)      # +0x44: within radius of the left edge, >= 0
    R   = min(0, rec.half_width_right - radius - d)     # +0x48: within radius of the right edge, <= 0
    if dot(dir, rec.tangent) < -0.6: L = R = 0          # 0x00782900; facing back down the track
    if sibling(track, cursor) (0x000a96d8): the same for the neighbour record; when the two
        records' |dot(down, point - pos)| differ by more than 25 take the nearer, else the one
        with the larger push
    out_left  = max(out_left, L); out_right = min(out_right, R)

So `L + R` is zero in the middle of the corridor, positive within `radius` of the left edge
and negative within `radius` of the right. The edges are the track's **half-widths**
(`+0x44`/`+0x48`), not the AI corridor (`+0x4c`/`+0x50`).

## Confidence

75 on the update and 80 on the scale from the static read; the signs (which side `+0x44`
sits on, whether `body+0x1d0` is right, the sense of local yaw) are the part a static read
cannot close. See the live section on [pilot-assist.md](../../../physics/pilot-assist.md).
