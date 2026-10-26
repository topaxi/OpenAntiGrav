# HD's Autopilot engine law (2026-10-19, `hd-autopilot-law`)

What the Autopilot pickup does to the player's engine on Wipeout HD/Fury, read in
Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, every function on TOC `r2 = 0x008ad4d8`,
`scripts/ps3-toc.py toc` reports each `exact`) and measured live on RPCS3. Pulse's
reading is [psp-pulse-usa/autopilot.md](../psp-pulse-usa/autopilot.md#what-the-running-bit-does-to-the-craft-2026-10-19-autopilot-rail);
HD's is the same law, instruction for instruction, at HD's own offsets.

**The law, confidence 95** (runtime trace on HD, and a second binary agreeing):
while the pickup's running bit `0x800` is set, the player's craft gets engine bit
`0x8` (the acceleration cap becomes `1e10`) and bit `0x4` with `craft+0x2f0 = 1.2`
(thrust times `1.2` after the cap). No per-class or per-speed-class term exists:
`1.2` is one TOC constant, and a class enters only through its own authored
`Engine amount` and `accelcap`, which is why lifting the cap matters most at
PHANTOM.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000cb928` | `PlayerStatus_Update` | 72 | the slice `0x000cbff8..0x000ccafc` read below matches Pulse's `PlayerStatus_Update` (`0x0883b3b8`) branch for branch and constant for constant, and its writes were seen live; the rest of the function (over 1,000 instructions) was not read |
| `0x000ed578` | `Craft_SetUncappedThrust` | 92 | seven instructions, read in full: `r4 != 0` ORs `0x8` into `craft+0x264`, else clears it; the bit was read set live under the pickup |
| `0x000ed5e0` | `Craft_SetAutopilotBlend` | 90 | `stfs f1, 0x278(r3); blr`; the blend read `1.0` live under the pickup |
| `0x00129b40` | `Autopilot_Fire` | 85 | read in full: sets slot `+0x200` bit `0x800`, `+0x204 = -1`, and `+0x158` to the weapon table's `+0xb4` (the `<Weapon type="Autopilot"><Stats time>`) |
| `0x00129988` | `Autopilot_Update` | 85 | read in full: `+0x158 -= dt`; on crossing `1.0` it plays `"disengaging"`; at `<= 0.0` it clears bit `0x800` |

`Craft_UpdateThrottle` (`0x000efb50`, already named on [pilot-assist.md](pilot-assist.md))
is also HD's engine: its tail below is Pulse's `Ship_UpdateEngine`.

## The engine (`0x000efb50`, past the AltiVec truncation)

Constants through the TOC: `0.5` (`0x008a918c`), `100.0` (`0x008a91a8`), `0.01`
(`0x008a91a0`), `1e10` (`0x008a91ec`), `0.2` (`0x008a9178`), `1.0` (`0x008a9144`).

    T   = throttle * class+0x28 (engine amount)                 # 0x000efcd4
    T   = T * grounded + T * 0.2 * (1 - grounded)               # 0x000efcd8..0x000efcec
    cap = 0.5 * craft+0x340 (speed) + class+0x30 (accelcap)    # 0x000efca0..0x000efcac
    if throttle > 100: cap *= throttle * 0.01                   # 0x000efcb0..0x000efcbc
    if craft+0x264 & 0x8: cap = 1e10                            # 0x000efcc4..0x000efcd0
    T   = min(T, cap)                                           # 0x000efcf0..0x000efcf8
    if craft+0x264 & 0x4: T *= craft+0x2f0                      # 0x000efcfc..0x000efd0c

then the turbo add, the launch multiplier `craft+0x2dc`, the doubling and the one-shot
scale `craft+0x378`, as on Pulse ([engine.md](../psp-pulse-usa/engine.md#engine)).
Pulse's `craft+0x1c0` and `craft+0x2a0` are HD's `craft+0x264` and `craft+0x2f0`.

## `PlayerStatus_Update`'s Autopilot branch (`0x000cbff8..0x000ccafc`)

`ship+0x5edc` is the pickup slot, `ship+0x5fac` the craft.

    if ship+0x7002 (finished) or 0x000d19b0() == 2:            # 0x000cbff8..0x000cc010
        Craft_SetAutopilotBlend(craft, 1.0)                     # 0x000cc0a8
        Craft_SetUncappedThrust(craft, 1)                       # 0x000cc0bc; 0x4 left as it was
    elif slot+0x200 & 0x800:                                    # 0x000cc024
        sprintf("%1.2f", slot+0x158)                            # 0x000cca84, the TOC slot 0x008a89bc
        Craft_SetAutopilotBlend(craft, min(slot+0x158, 1.0))    # 0x000cca9c, 0x000ccc94 / 0x000ccaac
        Craft_SetUncappedThrust(craft, 1)                       # 0x000ccac0
        craft+0x2f0 = 1.2; craft+0x264 |= 0x4                   # 0x000ccad4..0x000ccaec, 0x3f99999a at 0x008a89c0
        if this+0xa4 == 0: this+0xa4 = 6                        # 0x000ccaf4, a HUD message id
    elif not (ship+0x6a64 & 0x8) and not (ship+0x6a64 & 0x10):  # 0x000cc030..0x000cc048
        Craft_SetAutopilotBlend(craft, 0.0)
        Craft_SetUncappedThrust(craft, 0)
        craft+0x2f0 = 0.0; craft+0x264 &= ~0x4                  # 0x000cc080..0x000cc090

Pulse's branch is the same (`0x0883bdc0..0x0883bf5c`): the same three arms, the same
`min(timer, 1.0)`, the same `1.2`, the same `6`, and the same two guard bits
(`entity+0x860 & 0x8`, `& 0x10` there).

**The `6` is the pickup icon id, not a message** (2026-10-26, `cannon-ammo-hud`): `this+0xa4`
is set from the held weapon's id by a switch just above this branch, `-1` giving `0`, and
`6` is the Autopilot's own icon, so it stays up while the Autopilot runs with nothing held.
**The `sprintf` string goes to the `SubWeapon` text**, written once per frame at `0x000cc124`
(`bl 0x00202d28`, the widget `hud+0x2a4`): the Autopilot's seconds, then, for
`slot+0x204 == 3` (the Cannon, `0x000cca40..0x000cca50`), `sprintf("%d", slot+0x164)` over it,
the rounds left (the formats are the two strings at `0x00781b40` and `0x00781b48`). The same
line as Pulse's: [sub-weapon.md](../psp-pulse-usa/sub-weapon.md). Ported to ours as
`oag_hud::SubWeapon`; HD's `hud_pickups.xml` authors the widget under the icon (`HUDSmall`,
scale `0.9`, centred, `y=22`).

**The blend is only zero-tested.** The two craft-side readers of `craft+0x278` compare it
against `0.0`: `Craft_Update` at `0x000f19fc` (non-zero copies the driver's record into
`craft+0x4c`, the control-record swap) and `0x000ecab4`. So, as on Pulse, the controls
hand over in one tick both ways and the fade over the last second does not reach the
handling. The other `0x278` hits in the image are other structures. Confidence 85.

## The pickup (`Autopilot_Fire`, `Autopilot_Update`)

    Autopilot_Fire(slot):     slot+0x200 |= 0x800; slot+0x204 = -1
                              slot+0x158 = table[*(0x009389a4)+0xc]+0xb4   # the mode's weapon file
    Autopilot_Update(dt, slot):
        slot+0x158 -= dt
        if slot+0x158 < 1.0 and slot+0x154 > 1.0: cue "disengaging"
        if slot+0x158 <= 0.0: slot+0x200 &= ~0x800; stop the held loop

The time is authored, one value for every class: `/data/xml/weaponstats_race.xml`
(`DATA00.PSARC`) has `<Weapon type="Autopilot"><Stats absorb="20" time="5"/>`. The
port reads it off HD's own file already (`oag_tables::weapons`, Simple `time`).
**Release** is the not-running arm above, on the first frame after the bit clears:
cap and multiplier go, unless `ship+0x6a64` has `0x8` or `0x10` set (not read).

## Measured live (RPCS3)

RPCS3 `0.0.42`, HD EU (`BCES00664`), private config copy with **Write Color Buffers
on** and audio `Null`, display `:96`. Racebox Single Race, **PHANTOM**, weapons off,
AI novice, Ubermall forward, Feisar (the default), Pilot Assist written off
(`*(*0x008b098c)+0x473 = 00 00`) before the race was built. After GO the slot's bit
`0x800` was set and its timer `+0x158` held at 10000, so the original's own player
driver flew the whole race. `scripts/rpcs3-hd-autopilot.py` logged one row per new
body position (26,530 rows, two whole races on two boots).

| Read | Value under the pickup |
| --- | --- |
| `craft+0x264` | `0xd` (`0x8`, `0x4`, ground bit `0x1`); `0xc` airborne |
| `craft+0x2f0` | `1.2` |
| `craft+0x278` (blend) | `1.0` |
| `craft+0x30c` (throttle) | `100` while racing; about 52 to 76 after the finish (the finished-racer thrust law) |
| `craft+0x84` | `craft+0x4c` (the control record is the driver's) |
| Shield `ship+0x5fa0` | `140.0` to `134.5` over the race, in about a dozen single-frame scrapes |
| Result | 1st, 2:21.36 |

The bits appeared on the first frame after the poke and stayed for the whole race.

**A trap.** Writing the slot's held-weapon state `6` (the Autopilot's id on Pulse) during
the pre-race flyby crashed the game. Setting the bit alone, after GO, did not.
`craft+0x308` stood still for the whole two-second jump below, so a sampler keyed
on it misses every airborne frame; key on the body's position.

## Ubermall PHANTOM: the law is not the regression

The port already ran HD on this law, inherited from Pulse. Reading HD's own law
changes nothing: the autopilot-rail sweep reruns byte-identical
([ai.md](../../../gameplay/ai.md)). The regression is the craft's flight over
Ubermall's long jump (x `-216` to `-600`, about 2.5 s airborne), matched by position
against the original (cells: y, z):

| x | Original (5 passes) | Ours, engine bits on | Ours, engine bits off |
| --- | --- | --- | --- |
| -300 | 110, 348-351 | 111-112, 348-350 | 108-111, 348-351 |
| -400 | 107-108, 351-352 | 109-116, 351-354 | 97-108, 352 |
| -500 | 72-75, 355-358 | 80-96, 340-348 | 52-77, 346-355 |
| -550 | 43-46, 351-355 | 55-75, 327-333 | 20-47, 333-347 |
| -600 | 7-8 (lands), 334-338 | 22-50, 317-319 | 7-12, 319-329 |

- **Ours weaves in the air where the original ramps.** The original's steer
(`craft+0x4c`) holds near zero to x `-400` (`-3..+8`, `-21..-28` on two passes), then
ramps smoothly right (`+17..+23` at `-415`, `+43..+57` at `-450`, `+81..+85` at `-500`,
`+100` at landing). It touches no airbrake in the air, and it lands on the line at z
`334-338`. Ours commands `+65..+100` right with the right airbrake at x `-370..-415`,
where the original holds near zero. It then swings left (`-32..-53`, left airbrake up
to `100`) at `-450`, then right again, and drifts to z `317`, 17 units off. On the
ground the two steer channels agree in sign (correlation `0.76` over 2,375 matched
ground positions), so this is the airborne law, and it reads as phase or damping (a
weave), not a sign flip. Both stay open. This is the same signature `autopilot-rail`
found on Pulse's Fort Gale Black crest.
- **Ours flies higher, on the original's own team too.** It enters at the original's speed (`246-249`), but it peaks
at y `117` against `112` and still has y `95` at x `-500`. Every contact episode ours has on
Ubermall is this flight ending against the wall at y `45-51`, about 15 units lateral,
and then sliding down it.
- **Team.** The sweep flies HD's default `assegai`; the original raced Feisar (the
Racebox default). Ours on Feisar, same cell: 3 rescues, 211 wall ticks in 25 episodes,
shield `90.8, 81.7, 70.7, 44.2, 27.3` at the lap ends and `27.3` at the end. It peaks
at y `117` at x `-370` like Assegai, and weaves the same way (`+65..+100` at
`-370..-400`, `-32..-53` at `-450`). So neither the height nor the weave is a team difference.
- **The engine bits only move the drift into the wall.** Engine bits off, ours already
drifts 17 units on this jump and takes a ResetZone rescue on lap 1. Split
(rescues, wall ticks (episodes), end shield): none `1, 5 (3), 78.3`; cap lift alone
`0, 7 (3), 91.6`; `1.2` alone `1, 78 (10), 43.9`; both `2, 143 (27), 6.3`.

Next address: HD's `AI_ComputeControls` (`0x00100658`): its air term (`x * (1 - air)`, lookahead
`LookAheadSecs + air`, [ai-controls.md](ai-controls.md)) and `Ai_ComputePitch`
(`0x000fe510`), against `oag_ai::original` on the airborne frames of this jump.

## The other titles (read off each executable, no Ghidra program opened)

Read with capstone over the raw files, because the Omega and 2048 Ghidra programs
belong to another lane.

| Title | Where | What it carries | Result |
| --- | --- | --- | --- |
| Pulse (USA) | `0x0883be54..0x0883bec0` | the law above (live, [autopilot.md](../psp-pulse-usa/autopilot.md)) | measured |
| Pure (USA) | `BOOT.BIN` VA `0x089223c8..0x089223e4` (file `0x11e448`) | `ori 4` into `craft+0x1c0`, `1.2` to `craft+0x258`, blend `min(timer, 1)` (timer `+0x15c`), uncapped setter `0x0892c620` (`ori 8`, the twin of Pulse's `0x0884866c`), message id `7` | checked, applies; wired (the axis carries it, `Origin::Measured`) |
| Omega (EU) | `eboot.bin` VA `0x012e66a7..0x012e66b8` (file `0x2eb797`; VA = file + `0xffaf10`, found from `ModeManager`'s `lea rcx, [0x1252a60]` at `0x01251b5b` and checked on `AiManager_AddDriver` `0x012c80c0`, which lands on `push rbp; mov rbp, rsp`) | `or [craft+0x2c4], 0xc` (both bits inlined), `[craft+0x604] = 1.2`, blend `vminss(1.0, slot+0x158)` into `craft+0x2e8`, message `6`; slot bit `0x800` is `test byte [slot+0x215], 8` | checked, applies; wired |
| 2048 (v1.04) | `eboot.elf` `0x811b2376..0x811b238a` | `craft+0x55c = 1.2`, `orrs #4` into `craft+0x264`, uncapped setter `0x811d132c` (bit `0x8` of `+0x264`), blend setter `0x811d1326` (`+0x278`, HD's offsets), message `6` | checked, applies; wired |

So every title runs the same law, and the axis `oag_title::autopilot_engine` carries
`Origin::Measured` on all five: live on Pulse and HD, off the code on Pure, Omega and
2048. Confidence on the three static reads: 85 (decompile-level, one site each, the
engine reader of the multiplier not traced on those three).

## Who hears the "disengaging" call (2026-10-21, static)

`Autopilot_Update` (`0x00129988`, read in full at instruction level, the decompiler
does not finish on it) raises the one-second warning with no test of which craft or
seat it concerns:

```text
00129a80  lfs f0,0x154(r4)          ; previous remainder
00129a84  fcmpu f0, f12 ; ble ...   ; previous > 1.0
00129a8c  lwz r9,-0x30f4(r2)        ; global (sound manager)
00129a94  lwz r3,0xf0(r4)           ; the craft's owner object
00129a98  lwz r5,-0x30f0(r2)        ; the cue name "disengaging"
00129aa0  bl  0x000cfeb8            ; r3 owner, r4 bank, r5 name, r6 = 0
```

The helper `0x000cfeb8` (unnamed, 90 bytes read in full) returns at once when
`owner+0x628c != 0`, then reads a mode word (`[0x00054628()]+0x1970 == 2`) and a
table word (`+0xe0 == 2`, taken only while a byte global is clear) before it hands
`(bank, name, volume)` to `0x00310bd8`, the **dry** play with no emitter. `owner+0x628c`
is also what `Autopilot_Update`'s held-voice release tests (`0x001299f8`). None of the
three flags is read to a meaning, so nothing is renamed and no confidence is claimed:
what the code does show is that **there is no viewport or local-seat comparison on the
path** - the call is not routed to the window of the seat it concerns.

Our rule is therefore **chosen, not measured**: `Race::heard_here` (a network client
hears its own seat's dry voice lines alone; one machine flying several seats has one
output and plays each local person's), applied to `Cue::Disengaging` at its raise site
and to the held `~AUTOPILOT` level (`Race::autopilot_is_active`). Pulse and Pure have
no split screen. Omega and 2048: not checked, the announcer path is
unread there, so the same chosen rule applies to both.
