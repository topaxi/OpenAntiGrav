# Omega's stick shaping: `PlayerInput_Update`, deadzone 0.1, linear, no curve

2026-10-07, stick-curve lane. **Static reading only**: Omega has no capture
path, so nothing here is above the "decompilation only, consistent call sites"
ceiling of 84. The same law was read independently in HD
([`../ps3-hdfury-eu/player-input.md`](../ps3-hdfury-eu/player-input.md)) and in
2048 ([`../vita-2048-eu-v104/player-input.md`](../vita-2048-eu-v104/player-input.md)),
which is the corroboration. Pulse PSP's own law is in
[`../psp-pulse-usa/input-bindings.md`](../psp-pulse-usa/input-bindings.md).

## Where it lives

`PlayerInput.cpp` (debug tag `C:\WOPS4\Wipeout\Code\Backend\Ships\PlayerInput.cpp`,
string `0x01814cfc`) is the class whose constructor is `FUN_012ea7c0`
(sets vtable `0x01915250`, names `"player input %d"`). Vtable slot 3 (offset
`+0x18`) is `0x012e8fb0`, the per-frame update, named
`PlayerInput_Update` here (confidence 78: the class, the debug tag, the
per-player record and Pulse's own `PlayerInput_Update` all agree).

The record it fills (`param_2`) is Pulse's: steering `+0x80`, pitch `+0x94`,
thrust `+0x84`, airbrake L/R `+0x88`/`+0x8c`, all in a **+/-100** domain
(thrust and airbrakes 0..100). The d-pad override writes `0xc2c80000` /
`0x42c80000` (-100.0 / +100.0) straight into `+0x80` and `+0x94`, which is
what pins the domain.

## The law, per axis (confidence 80)

Left stick X is `plVar19+0xcdc` (steering), left stick Y is `+0xce0` (pitch).
Both go through the same code, independently (**per axis, not radial**):

```
v in [-1, 1]
if |v| < 0.1:   out = 0
else:           out = (v - sign(v) * 0.1) * 111.111115      // = 100 / (1 - 0.1)
```

- `0.1` is the literal compared at `0x012e8fb0` (`fVar23 <= -0.1 || 0.1 <= fVar23`).
- `111.111115` is `DAT_01a11730`, written once under a C++ static guard
  (`__cxa_guard_acquire(&DAT_01a11728)`), i.e. the compiler folded
  `100 / (1 - 0.1)`. It is `0x42de38e4`.
- No S-curve, no gain above 1: a linear ramp from the deadzone edge to 100 at
  full deflection. Steering and pitch are identical, unlike Pulse where pitch
  skips the curve.
- Result lands in `param_2+0x80` (steer) and `param_2+0x94` (pitch). Later the
  code only derives `+0x90 = |steer| * 0.01` from it. The craft's own use of
  those records is the same as Pulse's and is out of this page's scope.

## A second gate before it: `Pad` byte window (confidence 70)

The pad layer at `0x01747c90` (per frame; named `Pad_UpdateState`, confidence
75) calls `scePadReadState` and turns each stick byte `b` into
`(b - 128) * 0.0078125` (`/ 128`), **unless** `b` lies inside the closed window
`[DAT_01946e50, DAT_01946e54]`, in which case the axis is exactly 0. The four
axes read there (left X, left Y, right X, right Y) are
`+0xcdc, +0xce0, +0xcd4, +0xcd8`.

- Window defaults: `115..141` (`0x73`, `0x8d`), i.e. 128 +/- 13.
- `Pad_Open` (`0x017499b0`, named `Pad_Open`, confidence 70) overwrites them
  when `scePadGetControllerInformation` reports data (`+0x1100` set):
  `DAT_01946e50 = 0x80 - (u8)info[+0xc]`, `DAT_01946e54 = info[+0xd] + 0x80`.
  Those two bytes are the controller's own reported stick dead zones; the low
  bound comes from the left value and the high bound from the right value, and
  one window serves all four axes. **That the bytes at `+0x10fc/+0x10fd` are
  `ScePadControllerInformation.stickInfo.deadZoneLeft/Right` is read off the
  SDK layout from memory, not checked against a header: confidence 60.** The
  runtime value for a DualShock 4 cannot be read statically; 13 is the
  default window above, not a measurement.
- Consequence: the first live byte is `b = 114` or `142`, i.e. `|v| = 0.109`,
  and the game-level 0.1 deadzone then gives `~1.0` out. The two stages together
  behave as one deadzone of about 0.1 to 0.11 with no visible step. A larger
  hardware window would put a visible step in (out of `1.0` at the edge).
- The `+` side tops out at byte 255 = `127/128 = 0.992`, so full right gives
  `99.1`, not 100 (the `-` side reaches exactly -100 at byte 0).

## Settings menu

String census (`search_strings "Sensitiv"`): only `"Airbrake Sensitivity"`
(`0x018235dc`) and `"Motion Sensitivity"` (`0x01826b1b`), built in
`FUN_01528930` against the per-player option record at `0x01a0fba0 + n*0x144`
(`+0x98` airbrake, `+0x9c` motion). In the update, airbrake sensitivity
divides the trigger (`fVar * 100 / (int)DAT_01a0fc38 ... min 1`), and motion
sensitivity scales the Sixaxis tilt (`* 0.025 * DAT_01a0fc3c`). A third divisor
(`DAT_01a0fc34`) scales thrust (HD has the string `"Thrust Sensitivity"`).
**There is no stick sensitivity or stick curve setting** (confidence 70:
census of the strings plus no stick multiplier in the update; the options menu
code itself was not walked).

## Motion branch (not the stick)

`uVar2 = DAT_01a0fc2c[player]` selects the control scheme; non-zero schemes
read `+0xce4/+0xcec` (Sixaxis tilt, `atanf` of the gravity vector,
`FUN_01747c90`) clamped to +/-1, scaled `* 0.025 * sensitivity`, and steering
`* 1.4285715` (`1/0.7`). That is the motion-control path; its law is not the
stick's and is not pursued here.

## Cross-title status

- HD: **ported by lineage, same constants** ([`../ps3-hdfury-eu/player-input.md`](../ps3-hdfury-eu/player-input.md)).
- 2048: **checked, applies**, same code with a configurable deadzone in motion
  schemes ([`../vita-2048-eu-v104/player-input.md`](../vita-2048-eu-v104/player-input.md)).
- Pulse PSP: **checked, differs**: 0.2 deadzone, gain 125, S-curve, pitch with
  no curve.

## The comparison table (stick deflection as 0..1, output on the +/-100 record)

| stick | 0.0 | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 1.0 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Omega / HD / 2048 (0.1, linear) | 0 | 0 | 11.1 | 22.2 | 33.3 | 44.4 | 55.6 | 66.7 | 77.8 | 88.9 | 100 |
| Pulse PSP (0.2, gain 125, S-curve) | 0 | 0 | 0 | 6.2 | 12.5 | 18.8 | 25 | 43.7 | 62.5 | 81.2 | 100 |
| Ours (`oag_input::pad`, 0.15 rescaled, linear) | 0 | 0 | 5.9 | 17.6 | 29.4 | 41.2 | 52.9 | 64.7 | 76.5 | 88.2 | 100 |

Ours already has the modern titles' *shape* (linear after a rescaled
deadzone); it differs from Omega only in the deadzone edge (0.15 against about
0.1). Pulse PSP's curve is the outlier: it is dead to 0.2, flat to 0.3-0.5,
then steep. Switching to Omega's law means changing one constant
(`STICK_DEADZONE` 0.15 to 0.1); switching to Pulse's means adding the curve.
