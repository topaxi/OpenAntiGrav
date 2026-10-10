# 2048's Pilot Assist: Normal and Extreme (2026-10-10, pilot-assist)

Wipeout 2048 offers three Pilot Assist settings (`OptionsPilot`'s `Pilot Assist` touch list
in `data/plugins/frontend/NEWGUI/Options_Definition.xml`: `FE_OFF`, `FE_NORMAL`, `FE_SUPER`,
**default `FE_NORMAL`**). They are two copies of HD's corridor law with two different tables,
not one law with a strength knob. Read statically in Ghidra (`/2048/eboot-vita-2048-eu-v104.elf`);
not run live. HD's law, its sign conventions and its live capture are on
[ps3-hdfury-eu/pilot-assist.md](../ps3-hdfury-eu/pilot-assist.md).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x811ace10` | `Handling_ReadGlobalSettings` | 80 | parses `<Zone>`, `<Special>`, `<GlobalClass>` (five names, index at `0x818a63d4`), `<PilotAssist>`/`<PilotAssistPenalty>` into `0x818a62b0 + class * 0x2c`, `<SteerAssist>` into `0x8151fcc0`/`0x8151fcd4`, cameras, `<StartBoost>`, `<Collision>`; every attribute named by string in the decompile |
| `0x811d4cf0` | `PilotAssist_UpdateExtreme` | 72 | HD's `PilotAssist_Update` term for term on the global table: `-0.5` upright test, `+2 dt`/`-20 dt` blend (`0x40000000`, `0x41a00000`), `maxAngVel` gate, `25.0` threshold (`0x41c80000`) setting the acting sign and the timer to `penaltyDuration` |
| `0x811d5148` | `PilotAssist_ThrustScale` | 80 | HD's three returns on `0x818a62cc`/`0x818a62d0` times `0.01` |
| `0x811a5df6` | `PilotAssist_UpdateNormal` | 65 | the same law on a per-ship block (`*state + 0x84 ..`), blend target the ramped strength `state[2]` instead of `1`, skipped while that strength is `<= 0.001` (`0x3a83126f`) |
| `0x811a5b38` | `PilotAssist_SetStrength` | 70 | `state[2] = f`, and `state[5] = f` when the flag argument is set |
| `0x81152488` | `Hud_UpdateAssistIndicator` | 72 | four widgets at `hud+0x594..0x5a0` driven by the option byte and the craft's acting sign (below) |

## Which setting turns on what

The settings screen's handler (`0x811360e6`, `0x8113631e`-`0x8113635e`) compares the chosen
entry's string id and writes **two** bytes per local player in the save block
(`*0x81545468`):

    +0x3260e + player = (entry == "FE_SUPER")    # Extreme
    +0x32664 + player = (entry == "FE_NORMAL")   # Normal

So Normal and Extreme are mutually exclusive, and Off clears both. `0x811212c0` toggles
`+0x3260e` between `0` and `1` from a second path.

## Extreme (`0x811cfe64`-`0x811cff52`)

    enabled = craft+0x5dd == 0 and save[+0x3260e + player] != 0 and ship+0x58dc == 0
          and (call 0x810018d4) == 0 and craft+0x278 == 0.0 and craft+0x564 == 1
    online: enabled &= byte(0x8153fc40 + 0xef) & 8
    PilotAssist_SetEnabled(state, enabled); PilotAssist_UpdateExtreme(dt, state, body, grounded = craft+0x570 != 0, cursor)

The global table's values (2048's own `data/xml/handlingstats.xml`, the `VENOM`..`PHANTOM`
rungs; `VECTOR` and `SUPERPHANTOM` also authored): `laDistConst 10`, `laDistVelMul 0.4`,
`laDistMax 75`, `springMul -15`, `torqueMul 50`, `maxTorque 600`, `maxAngVel 2`, penalty
`99/92`, `99/92`, `98/90`, `97/88` and `3` s. HD's Venom-Phantom rungs are stiffer
(`torqueMul 60`-`80`, `maxTorque 1000`, `maxAngVel 3`).

## Normal (`0x811d0010`-`0x811d0130`)

    if craft+0x5dd == 0 and save[+0x32664 + player] != 0 and r6 == 0:
        k = clamp((craft+0x5a8 - SteerAssist.min_speed) / SteerAssist.ramp_up_range, 0, 1)
        PilotAssist_SetStrength(craft+0x5fc, k * (*(craft+0x8c))+0x10, false)
    PilotAssist_UpdateNormal(dt, craft+0x5fc, body, grounded, cursor)

`<SteerAssist min_speed="50" ramp_up_range="25"/>` is authored on every rung. The per-ship
block is the team file's `<Assist>` element (`data/HandlingStats/<team>2048/<n>/handlingstats.xml`),
e.g. `laDistConst 10, laDistVelMul 0.4, laDistMax 75, springMul -12, torqueMul 10,
maxTorque 50, maxAngVel 2, thrustPercentOnUse 95, penaltyDuration 0.25, notInUseStrength 0.1`.
Which `<Assist>` attribute sits at the block's `+0x84`..`+0xa0`, and what `(*(craft+0x8c))+0x10`
is (`notInUseStrength` is the candidate), was not read: the per-ship reader is not located.
Confidence on Normal stays 65 for that reason.

## The HUD indicator (`Hud_UpdateAssistIndicator`, `0x81152488`)

Four widgets, the same four HD's `/data/xml/hud_assist_indicator.xml` authors
(`AssistIndicatorBG`, `AssistIndicatorMain`, `AssistIndicatorLeft`/`LeftBig`,
`AssistIndicatorRight`/`RightBig`, the arrows tinted `0xFFFF4040`):

    widget0 (+0x594) visible while save[+0x3260e + player] (and the online allowance)
    on acting == -1: t_main = 3.0, t_right = 1.5        # 0x40400000, 0x3fc00000
    on acting == +1: t_main = 3.0, t_left  = 1.5
    each timer -= dt, floored at 0; phase += dt while t_main > 0, else 0
    widget1 (+0x598) visible while t_main > 0
    widget2 (+0x59c) visible while t_left  > 0 and fmod(phase * 4, 1) < 0.5   # blinks at 4 Hz
    widget3 (+0x5a0) visible while t_right > 0 and fmod(phase * 4, 1) < 0.5

Acting `-1` is the torque turning the craft left, away from the right-hand wall, so the arrow
that blinks is on the side of the wall. 2048's own in-race HUD (`Hud_UpdateWidgets`) also shows
its `PilotAssist` image (`data/xml/2048_hud/HUD_assist_indicator.xml`,
`Icon_PilotAssist_Extreme`) while the Extreme byte is set, scaled by `1 + 0.1 * sin(phase)` with
`phase += 10 dt` wrapped at `pi` (`0x81195fa2`-`0x8119600e`).

## Omega

**Checked, applies, not wired.** Omega's executable carries every string above (`laDistConst`,
`PilotAssist`, `PilotAssistPenalty`, `generalThrustPercentWhenEnabled`, `SteerAssist`,
`notInUseStrength`, `PilotAssistDisable_Importer.cpp`), so it inherits 2048's two-table shape.
Its global `Data/xml/handlingstats.xml` sits in `data00.psarc`; reading its values is open.

## Pulse and Pure

**Neither has Pilot Assist.** No `PilotAssist`, `laDist` or `springMul` string in Pulse's
`BOOT.BIN` (Ghidra) or Pure's (`strings`); both carry only the Autopilot pickup's names.

## Open

- The `pilot_assist_disable` scene objects (`PilotAssistDisable_Importer.cpp`, `0x814cced4`):
  who reads them, and whether they gate Normal, Extreme or both. The tip text says Extreme
  "kicks in automatically in some of the trickiest parts of the track", which no code read
  here explains.
- `0x810018d4`'s result (`r6`), and `craft+0x5dd`/`craft+0x278`, in the gates.
