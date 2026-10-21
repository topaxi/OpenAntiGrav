# HD's craft flag word `craft+0x264`, its engine and its drag (2026-10-21, `hd-throttle-and-flags`)

**Binary:** Wipeout HD/Fury EU `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), every
function on TOC `r2 = 0x008ad4d8`. **Live:** RPCS3 `0.0.42`, private config with audio
`Null`, display `:95`, player input only (the original's Autopilot never armed: slot bit
`0x800` clear on every row). Pulse's map of the same word is
[psp-pulse-usa/craft-flags.md](../psp-pulse-usa/craft-flags.md).

`craft` is `*(ship+0x5fac)`, the pickup slot `*(ship+0x5edc)`. This page closes the five
craft laws HD had inherited from Pulse unread (`engine_output_double`, `drag_ground`,
`sideshift_grid_gate`, `turbo_running_laws`, `eliminator_wrong_way_kill`).

## The writers, by bit

How they were found: every `stw`/`sth`/`stb` with displacement `0x264`..`0x267` in the
image (Ghidra instruction search, operand `0x264(`; no byte or halfword store reaches
`0x265`..`0x267`), each traced to its base; then every `addi rX,rY,0x264..0x267` and every
`li rX,0x260..0x264` near an `stvx`/`stwx` in `0x000c0000..0x00110000` (a computed-address
or AltiVec store would not show in the first pass). The `li` hits are all stack slots
(`r1`), the `addi` hits are in four unrelated functions; so the displacement census is the
whole writer set. `rlwinm rX,rX,n,1,31` then `rlwinm rX,rX,32-n,0,31` clears bit `32-n-1`
counted from the low end, which is how most clears below are written.

| Bit | Set by | Cleared by | Meaning | Pulse's | Confidence |
| --- | --- | --- | --- | --- | --- |
| `0x1` | `Craft_HoverFourPoint` `0x000ee2dc` | `Craft_UpdateHover` `0x000f18e4`; `Craft_Update` `0x000f1fb4` | a hover probe touched | `0x1` | 90 (live: `0x1` on the ground, `0x0` airborne) |
| `0x2` | `Craft_SetState(0)` `0x000f0678`; `Craft_EnterGridState` `0x000ed5b0` | `Craft_SetState(1)` `0x000f06b4`; `Craft_ReleaseFromGrid` `0x000ed5d0` | the grid state | `0x2` | 95 (live: `0x3` through the countdown, `0x1` from the frame `craft+0x2f8` reads `1`) |
| `0x4` | `PlayerStatus_Update` `0x000ccaec` | the same, `0x000cc090` | the Autopilot's speed-up | `0x4` | 90 ([autopilot.md](autopilot.md)) |
| `0x8` | `Craft_SetUncappedThrust` `0x000ed588` | the same, `0x000ed59c` | uncapped thrust | `0x8` | 92 ([autopilot.md](autopilot.md)) |
| `0x10` | **nothing** | - | the slowdown arm's escape (`Craft_UpdateThrottle` `0x000efbf8`) | `0x10` | 85: no store |
| `0x20` | `Ship_UpdateTurboFlag` `0x000cf7f8` | the same, `0x000cf848` | **the Turbo pickup is running**: slot `+0x200 & 0x200`, copied every frame | `0x200` | 95: live |
| `0x40` | `Ship_UpdateSideshiftInput` `0x000da1a4` | the same, `0x000d9f9c`, `0x000da85c` | the barrel roll's landing payout (`ship+0x6ab8` running) | `0x400` | 80: static, the same shape as Pulse's |
| `0x80` | `Ship_UpdateSideshiftInput` `0x000da090` | the same, `0x000da248` | the left sideshift timer `ship+0x6ac8` is running | `0x800` | 95: live |
| `0x100` | `Ship_UpdateSideshiftInput` `0x000da0d4` | the same, `0x000da220` | the right sideshift timer `ship+0x6acc` is running | `0x1000` | 95: live |
| `0x200` | **nothing** | - | read once, by the engine's kill (`0x000efd6c`) | `0x2000` (the Eliminator wrong-way kill) | 90: static census and live (below) |
| `0x400` | **nothing** | `Ship_SetState_q` `0x000e1eb8` | read by `Craft_IntegrateHull` (`0x000ef4c8`), `0x00108040`, `0x001086d0` | - | 80: no store sets it |

`Ship_Construct`-style initialisers zero the word (`0x000c4e5c`, `0x000c50f4` clear a whole
block). From `0x20` up, HD's bit is **Pulse's shifted right by four**; Pulse's five
dead bits `0x20`..`0x100` have no counterpart.

## `Craft_UpdateThrottle` (`0x000efb50`), read whole

Constants: `0.0` (`-0x438c(r2)`), `1.0` (`-0x4394`), `0.5` (`-0x434c`), `100.0` (`-0x4330`),
`0.01` (`-0x4338`), `0.2` (`-0x4360`), `1e10` (`-0x42ec`).

    craft+0x320 = (flags & 0x20) ? 0.0 : craft+0x320 + dt        # 0x000efb74..0x000efb88, 0x000efe28
    if controls.thrust != 0: craft+0x328 = craft+0x329 = 1         # 0x000efb98..0x000efba8, 0x000efe60
    stunned = craft+0x2d8 > 0                                      # 0x000efbb4
           or (!(flags & 0x10) and craft+0x334 > 0)                # 0x000efbf8..0x000efc0c
    if stunned and !(flags & 0x20): craft+0x30c = 0; return        # 0x000efbc0..0x000efbf0
    craft+0x30c ramps by class+0x24 / class+0x2c * dt ...          # 0x000efc10..0x000efc3c, 0x000efe38
    craft+0x30c = controls.thrust                                  # 0x000efc50..0x000efc54: the ramp is overwritten
    craft+0x30c *= PilotAssist_ThrustScale(craft+0x380)            # 0x000efc58..0x000efc7c
    if Zone (mode < 0x16, bit in 0x206040): T = (flags & 1 and !(flags & 2))
                                              ? zone target : 0     # 0x000efeb8..0x000eff30
    else:
        cap = 0.5 * craft+0x340 + class+0x30                      # 0x000efca0..0x000efcac
        if throttle > 100: cap *= throttle * 0.01                  # 0x000efcb0..0x000efcbc
        if flags & 0x8: cap = 1e10                                 # 0x000efcc4..0x000efcd0
        T = throttle * class+0x28
        T = T * g + T * 0.2 * (1 - g)                              # g = craft+0x304; 0x000efcd4..0x000efcec
        T = min(T, cap)                                            # 0x000efcf0..0x000efcf8
    if flags & 0x4: T *= craft+0x2f0                               # 0x000efcfc..0x000efd0c
    lift = 0
    if (flags & 0x20 or flags & 0x40) and craft+0x2f8 == 1:        # 0x000efd10..0x000efe78
        T += class+0x34                                            # the turbo add
        if controls+0x28 & 1: lift = T * *(-0x42d8(r2))            # d-pad Up
    T *= craft+0x2dc                                               # the launch multiplier
    if craft+0x2e0 > 0 and flags & 1: T += 100.0                   # 0x000efd2c..0x000efd54, HD's own term
    T = T + T                                                      # 0x000efd5c: fadds f0,f12,f12
    if craft+0x378 < 1.0: T *= craft+0x378; craft+0x378 = 1.0       # the one-shot scale
    if flags & 0x200: throttle = 0; T = 0; lift = 0                # 0x000efd6c..0x000efd88
    body+0x160 += (0, lift, T)                                     # 0x000efd8c..0x000efe10

Pulse's `Ship_UpdateEngine` ([engine.md](../psp-pulse-usa/engine.md#engine)) branch for
branch, with HD's bit numbers, two differences:

- **`T += 100` while `craft+0x2e0 > 0` and grounded**, ahead of the doubling. Writers of
  `craft+0x2e0`: `0x000d2058` (`+= dt`, in `FUN_000d1e98`), `0x000e5c08` (`+=` a TOC
  constant), `Ship_ApplyDamage_q` `0x000e88e4` (drained), zeroed at `0x000ef03c` and
  `0x000f021c`. Not identified; **open**. No capture logs `craft+0x2e0`, so whether it ran
  on the walked rows is not known: the walk leaves 23 of 193, 8 of 154 and 11 of 215
  grounded rows more than `1.0` u/s per frame slower than the original (Ubermall PHANTOM
  `centred-1`, `centred-2`, Venom `centred-1`), unattributed.
- The **Pilot Assist scale** multiplies the throttle the engine reads
  ([pilot-assist.md](pilot-assist.md)).

**The doubling, confidence 95.** One instruction, `fadds f0,f12,f12` at `0x000efd5c`, on
the only path to the force store. Live: the HD air pose walk
(`crates/raceplay/tests/hd_air_pose_walk.rs`) steps ours once from every row of the
player-input captures in `data/reference/hd-air-player-input/` and compares the next row.
With the doubling, the median forward residual per frame on Ubermall PHANTOM
(`ubermall-phantom-feisar-centred-1.csv`, 121 airborne and 193 grounded rows) is `+0.03`
airborne and `+0.03` grounded; with it off (`OAG_HD_DOUBLE=0`), `-0.37` and `-1.76`.

**The throttle, confidence 92.** The ramp at `0x000efc10..0x000efc3c` is overwritten by
the raw input at `0x000efc54`, as on Pulse. Live (`ubermall-phantom-tt-flags.csv`, below):
releasing cross drops `craft+0x30c` from `88` to `0` on the next frame (`t` 18.52 to
18.54), and pressing it again lifts it from `0` to `88` on the next frame (19.52 to
19.54). `88` is `100 x 0.88`, Pilot Assist's scale.

## `Craft_ApplyQuadraticDrag` (`0x000ed700`)

    if r4 != 0: body+0x190 *= (body+0x210 == 2 ? 0.95 : 0.99)       # 0x000ed714..0x000ed75c, 0x008c1724
    else:
        k = craft+0x340 < -0.2 ? -0.1                              # 0x000ed768, 0x008a9118, 0x008a911c
          : flags & 1          ? -0.005                            # 0x000ed774..0x000ed78c, 0x008a9120
          :                      -0.002                            # 0x000ed7e0..0x000ed7e8, 0x008a9124
        Body_AddForce(craft+0x230 (velocity) * craft+0x340 * k)    # 0x000ed7a0..0x000ed7c4, 0x000f5860

Pulse's `Ship_ApplyQuadraticDrag` with Pulse's four constants, in the same order in the TOC.
`craft+0x340` is `|dot(velocity, forward)|`: `Craft_Update` stores it at `0x000f1c34`
when the dot is non-negative and its `fneg` at `0x000f2714` otherwise, so the `-0.1`
reversing arm is unreachable, as on Pulse. `Craft_Update` calls it once, at `0x000f1edc`,
with `r4 = r30 & 1`, and `r30` is loaded `0` at `0x000f1ea8` on the straight path to the
call; the velocity-scaling arm (`r4 != 0`) is reached from no path read here (Pulse's
`craft+0x2a4 == 0` arm adds a `-0.9` force instead). **Open:** whether any path reaches
it with `r4 = 1`.

**The grounded coefficient, confidence 92.** Static as above; live through the same walk:
grounded median `+0.03` with `-0.005`, `+1.54` with Pure's `-0.0025` and `-1.46` with
`-0.0075` (`OAG_HD_DRAG`). The airborne rows do not move with it (`+0.03` each way), as
they should not.

## `Ship_UpdateSideshiftInput` (`0x000d9e18`), the grid gate

The barrel roll and the sideshift, one function as on Pulse. The sideshift block:

    if !(craft+0x264 & 2) and ship+0x6ad0 (lockout) <= 0:          # 0x000da04c: the grid gate
        for each of the two bindings (FUN_000bd3e8(player, 5) and (player, 6)):
            a press opens a 0.25 s tap window (ship+0x6ac0 / +0x6ac4);
            a second press inside it arms the shift timer (ship+0x6ac8 / +0x6acc)
    craft+0x264 bit 0x80  = ship+0x6ac8 > 0, and ship+0x6ad0 = 1.0 while it runs
    craft+0x264 bit 0x100 = ship+0x6acc > 0, the same

**Confidence 95** (`ubermall-phantom-tt-flags.csv`): during the countdown (flags `0x3`,
state `0`) double taps of L1, R1, L2 and R2 open no tap window (`+0x6ac0`, `+0x6ac4` stay
`0`) and arm nothing. After GO, L2's double tap opens the window (`0.25`) on its first
press and sets `0x80` on its second, the timer from `0.2`, the lockout `1.0`; R2's sets
`0x100` the same way. L1 and R1 do nothing here (the sideshift binds L2 and R2 on HD's
default layout; L1 flips the craft in an Eliminator race, below).

## `Ship_UpdateTurboFlag` (`0x000cf7a8`), and the pad fade

    if slot+0x200 & 0x200: craft+0x264 |= 0x20 else craft+0x264 &= ~0x20

Pulse's mirror in `Ship_UpdateStartBoost`, at HD's bit. Its three consumers are Pulse's
three Turbo laws:

1. **the stun return's escape** in `Craft_UpdateThrottle` (`0x000efbc0`), above;
2. **the pad fade clock** `craft+0x320` held at `0` while the bit is set
   (`0x000efb84`), read by the speed pad's force in `Craft_ApplySpeedPad_q`
   (`0x000eeb70`): `f = (craft+0x320 - 1.0 >= 0) ? f : f * craft+0x320`
   (`0x000eec48..0x000eec68`), then `Body_AddForce` along the pad's row;
3. **the turbo add and its d-pad Up lift**, above.

**Confidence 92** (live, `ubermall-phantom-tt-flags.csv`): a Turbo given by writing the
held pair `slot+0x204`/`+0x208 = 4` ([rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md))
and fired with triangle sets slot `0x200` at `t` 22.530, craft `0x20` one frame later
(22.550), the pad clock reads `0.0` from the next frame (22.563) to the frame after the
slot bit clears (23.331), then climbs by the frame time (`0.0166`, `0.0333`, `0.0500`).
The pickup's run lasted `0.78 s`. The stun escape and the lift were not exercised live.

## The Eliminator wrong-way kill does not exist on HD

Bit `0x200` has its engine reader and **no writer** (the census above). Pulse's second
reader, the forced airbrakes in `Ship_UpdateCraft`, has no counterpart: `Craft_Update`
tests only `0x1` and `0x2` (`0x000f1af0`, `0x000f1ca4`, `0x000f1e90`, `0x000f1fec`,
`0x000f22f0`, `0x000f23ac`), `Craft_UpdateAirbrakes` only `0x1`, `0x80` and `0x100`.

**Live, confidence 90** (`eliminator-venom-flip.csv`): Racebox Eliminator
(`GetMode()==8` in `TTY.log`), Venom, novice AI, cross held from 7 s. The intro screen
says "Flip your ship - L1 button (default)", and L1 turns the craft round in about
`0.15 s`. Flipped at 14.0 s and driven back down the track until 24.0 s, then flipped
forward, and flipped back again at 30.0 s to the end at 40 s: the forward row against the
pre-flip heading read `-0.95` to `-1.00` for those stretches, the craft reached `102`
km/h backwards, `craft+0x30c` stayed at `92`..`99` and the airbrake inputs at `0`, and
the flag word was `0x1` on every racing row (OR over all 3,806 rows: `0x3`). On Pulse the
same turn sets `0x2000` under `dot < -0.4` and kills the engine.

So HD lets a craft run the wrong way in an Eliminator: the flip is a control, not a
penalty. **Ours has no flip**; that is open, not part of this page's laws.

## Captures

`data/reference/hd-throttle-and-flags/` (gitignored, local):

| File | What |
| --- | --- |
| `ubermall-phantom-tt-flags.csv` | Racebox Time Trial, Ubermall, PHANTOM, Feisar, from Restart Race: countdown taps of L1, R1, L2, R2; cross held from 10 s; post-GO double taps; cross released at 18.5 s and pressed at 19.5 s; Turbo given at 21.5 s and fired at 22.5 s |
| `eliminator-venom-flip.csv` | Racebox Eliminator, default circuit, Venom: cross from 7 s, L1 at 14, 24 and 30 s |

Both were taken with `scripts/rpcs3-hd-air-hold.py --script` (one row per new frame, the
flag word, the pad clock `craft+0x320`, the sideshift cells `ship+0x6ac0..+0x6ad0`, the
held pair and the control record's button words `+0x24`/`+0x28`). The Time Trial capture
followed `rpcs3-drive.py restart` run on its own: inside the script the pause-menu walk
missed twice on this state.

## Cross-title

- **Omega and 2048 inherit HD's craft laws.** Their doubling and drag were labelled
  inherited from Pulse; they are now inherited from Wipeout HD, which measured the same
  values.
- **2048 (v1.04, `eboot.elf`), checked statically, differs in part; open.** Stores to
  `craft+0x264` (`str.w ..., [rX,#0x264]` with an `orrs`/`bfi`): `0x1` (`0x811cdf8a`, cleared
  `0x811d015a`), `0x2` (`0x811d128c`, `0x811d12bc`), `0x4` (`0x811b238a`), `0x8`
  (`0x811d1344`), `0x20` (`0x811bf0b4`), `0x40`, `0x80`, `0x100` (`0x811bf7aa`, the
  sideshift's shape), plus `0x400` (five setters, `0x811c7366` among them) and `0x800`
  (`0x812bdc0e`, in `FUN_812bda96`, a virtual method with no direct caller; cleared at
  `0x812bdb00`) that HD does not have. **No store sets `0x200`**, but 2048 keeps Pulse's
  second reader: at `0x811cfb0c` a set `0x200` writes `100.0` into both airbrake inputs of
  the control record, and the next test (`0x811cfb2c`) does the same kind of override under
  **`0x800`**, which is set. So 2048 has no `0x200` kill, as HD, but it has an `0x800` state
  that overrides the controls, and what sets it (a wrong-way rule or something else) is
  unread. 2048 inherits HD's "no wrong-way kill" with that gap named.
- **Omega: not checkable here.** Its craft structure is laid out for 8-byte pointers and
  its `craft+0x264` counterpart has not been located; no PS4 capture path exists. It
  inherits HD's laws.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000cf7a8` | `Ship_UpdateTurboFlag` | 92 | the mirror read whole; live one frame behind the slot bit |
| `0x000d9e18` | `Ship_UpdateSideshiftInput` | 90 | decompiled; the grid gate, windows and bits live |
| `0x000ed700` | `Craft_ApplyQuadraticDrag` | 90 | Pulse's constants in order; the coefficient discriminated live |
| `0x000ed5a8` | `Craft_EnterGridState` | 75 | three instructions, `|= 0x2`; caller not read |
| `0x000ed5b8` | `Craft_ReleaseFromGrid` | 75 | `&= ~0x2` and zeroes `+0x310..+0x31c`, as `Craft_SetState(1)` does; caller not read |
| `0x000e1e20` | `Ship_SetState` | 65 | calls `Craft_SetState`, clears `0x400`, a ten-way jump table on the state; the table not read |
| `0x000eeb70` | `Craft_ApplySpeedPad` | 65 | only the tail read: the pad force scaled by `craft+0x320`, the pad's row at `+0x250` |
| `0x000f5860` | `Body_AddForce` | 80 | [pilot-assist.md](pilot-assist.md): four instructions adding into `body+0x150` |
