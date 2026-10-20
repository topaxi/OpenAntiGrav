# Wipeout Pure's craft laws, measured against Pure PSP (2026-10-20)

Lane `pure-physics`. The maintainer: "Pure's physics feel quite different from the other
titles and should be measured specifically on Pure PSP." Until this page every Pure law was
Pulse's, inherited. This page diffs Pure's craft update against Pulse's instruction by
instruction, then measures each difference live in PPSSPP.

**Verdict.** Pure's craft update is Pulse's in shape: the same twelve force functions in the
same order, the same rigid body, the same integrator and the same hover probes. Six constants
differ, and every one was read off `BOOT.BIN` and then measured live to float precision:

| Law | Pulse | Pure | Where (Pure USA) | Confidence | Status here |
| --- | --- | --- | --- | --- | --- |
| Engine output doubling | `T * 2.0` | **none** | `Ship_UpdateEngine` `0x0892f1f0` | 92 | ported, `CraftLaws::engine_output_double` |
| Grounded quadratic drag | `-0.005` | **`-0.0025`** | `Ship_ApplyQuadraticDrag_q` `0x0892cba0` | 92 | see below |
| Bank-to-yaw gain, two-point hover | `30.0` | **`50.0`** | two-point hover `0x0892dea8` | 92 | see below |
| Pitch torque per unit input | XML `pitch_air`/`pitch_ground` (`0.3`/`1.0` on every Pulse team) | **`0.1` air, `0.4` grounded**, code literals | `Ship_UpdatePitch` counterpart `0x0892caf8` | 92 | see below |
| Pitch damping | XML `pitch_damping` (`3`) | **`1.5`** racing, `0.6` on the grid (`3 / c`) | `Ship_ApplyAngularDamping_q` `0x0892cc2c` | 92 | see below |
| Sideshift force | XML `Airbrake sideshift` | **`200.0`**, a code literal | airbrake counterpart `0x0892f64c` | 92 | see below |
| Launch boost | `<StartBoost>` XML, every craft | **code law**, player only, `x4.2`/`x1.2`/`x0.8` for 1 s | `0x0880f870`, `0x089252e8` | 80 (static) | open |

Confidence 92 on the six measured rows: a runtime trace that reproduces the instruction-level
reading exactly, on one binary. Not 95: the EU pressing was not run.

## How it was measured

PPSSPP 1.20.4 (software renderer, muted), `pure-psp-usa.chd`, Time Trial, Alpha league, Vineta
K, Feisar, Venom (the class index `0x08b173e0` reads `1`). Pure's craft update runs at **30 Hz**
(`dt = 0.0333`, `7.4M` CPU cycles between updates).

`scripts/psp-pure-force-capture.py` stops at Pure's craft frame (`0x0892d214`, `a0` the craft,
`a1` the body), reads the craft's inputs, then stops at each force function's entry and at its
return site inside the frame, and records the change in every VFPU register across the call.
Pure keeps its force and torque accumulators **in VFPU registers** through the whole frame
(`S222` thrust, `C230` the local torque, `C300` the world force), so the change is exactly what
the function added. This PPSSPP fires only the most recently added breakpoint, so the script
walks one breakpoint at a time through the frame's call order.

The handling block was read out of memory (`*(craft+0x74) + 0x88 + class * 0x6c`): Feisar
Venom `Engine amount 0.398` (scaled), `accelcap 30`, `Turning gain 650 falloff 900 amount 1.8`.

Runs: a standing start to `94` u/s (420 frames), a run with pitch held (150), a run with
steering and the left airbrake held (90), a run lifted `60` units into the air at a breakpoint
so the craft falls for a second (60), and a sideshift armed by writing its timer (16).

## The static diff

Pure's frame is `0x0892d214` (Pulse `Ship_UpdateCraft`, `0x08849618`). Its callees, in order:

| Pure | Pulse | Law |
| --- | --- | --- |
| `0x0892f1f0` `Ship_UpdateEngine` | `0x0884c5c8` | differs: no doubling; a player-only launch multiplier |
| `0x0892c7dc` | `Ship_UpdateBrakes` `0x088489d8` | same |
| `0x0892f64c` | `Ship_UpdateAirbrakes` `0x0884c9a4` | same force block; sideshift `200.0` literal |
| `0x0892edfc` `Ship_ApplySteeringTorque` | `Ship_UpdateSteering` `0x08848788` | same (the Disruptor's drunk wobble differs) |
| `0x0892caf8` | `Ship_UpdatePitch` `0x08848d08` | differs: literals, no gate, no bias |
| `0x0892cba0` | `Ship_ApplyQuadraticDrag` `0x08848e28` | differs: grounded `-0.0025` |
| `0x0892c6b0` | `Ship_UpdateHover` `0x0884870c` | same dispatch, no mag-lock |
| `0x0892c994` | `Ship_ApplyLateralGrip` `0x08848b78` | same |
| `0x0892cb58` | `Ship_ApplyWeathervaneTorque` `0x08848dc4` | same (`-0.3` / `-0.1`) |
| `0x0892cc2c` | `Ship_ApplyAngularDamping` `0x08848ed0` | differs on x only |
| `0x0892cc94` | `Ship_ApplyRollingResistance` `0x08848f4c` | same (`2.0`) |
| `0x0892ccd0` | `Ship_ApplySpeedupPad` `0x08848f9c` | same law |

Also the same: the inertia box (`Craft_Construct` passes `0.9`, `(12, 8, 12)`, damping `0.01`,
`0.01`, `0.4`), `Body_Integrate` (`0x089345e8`, instruction for instruction Pulse's apart from
the register bank and Pure's body being `0x40` longer before the velocity), the hover probes
(`0x0892cf5c` writes `(0, -1.5, +/-6) * 0.75`), the craft scale `0.75`, the hover target
(`ride_height - min(slowdown, 4)` times `0.75`, with no mag-lock term because Pure has no mag
strips), the gravity shape, the brake law and the lateral grip.

### The handling block

`stats + 0x88 + class * 0x6c` (Pulse `+0x94 + class * 0x80`). Block-relative, every field from
`ride_height` to `Airbrake slidegrip` sits where Pulse's does. Pure has **no `Airbrake
sideshift`** and **no `<pitch>` block**, so `Physical` moves up: `mass +0x5c`,
`normal_gravity +0x60`, `flight_gravity +0x64`, `track_gravity +0x68`. Parsers: Antigrav
`0x0891fadc`, Physical `0x0891f93c`, Engine `0x0891fd10` (amount `* 0.001`), Brakes `0x0891ff38`
(amount `* -0.01`), Turning `0x089200c8`, Airbrake `0x0892021c` (amount and slidegrip
`* 0.0001`), AirbrakeGraphics `0x089204b4`, Misc `0x089205c0` (`width +0x78`, `length +0x7c`,
`height +0x80`, `shield +0x84`; no `weight_distribution`), dispatched from `0x08920a4c`.

## The engine does not double its output

```text
0892f620  lwc1  f12,0x24c(a0)     ; the player-only launch multiplier
0892f628  mul.s f12,f13,f12
0892f63c  vadd.s S222,S222,S600   ; thrust, with no * craft+0x294 * 2.0
```

`T = min(throttle * amount * (g + 0.2 (1 - g)), 0.5 * fs + accelcap)`, then `* craft+0x24c`
for the player's craft only. Live: **374 racing frames reproduce `S222` to `3e-6`**, 336 of
them on the cap and 38 on the throttle, plus 50 airborne and half-grounded frames on the `0.2`
air factor. Pulse's law misses by up to `47.8`.

## Grounded drag is half Pulse's

`k = -0.0025` grounded (`0xbb23d70a`), `-0.002` airborne, `-0.1` reversing, `-0.9` in state `0`,
then `force = velocity * fs * k`. Live: the coefficient register reads `-0.0025` on every
grounded frame and `-0.002` on every airborne one; the force reproduces exactly from the
cached velocity (`craft+0x190`) and speed (`craft+0x2a4`). With the doubling gone too, a
Venom Feisar's terminal speed barely moves (`sqrt((amount * 100 - 2) / 0.0025) = 123.0`
against Pulse's law's `124.6`), and its acceleration halves.

## Bank-to-yaw is 50, not 30

The two-point hover's grounded yaw coupling is `right.y * 50.0` on Pure (`0x0892dea8`), the
gain Pulse uses only in its four-corner (Zone) hover. Live: `S231 / right.y = 50.0000` on 129
banked frames.

## Pitch is three code literals

```text
torque.x += pitchInput * (grounded ? 0.4 : 0.1)          ; 0x0892caf8
torque.x += (3 / c) * omega.x,  c = state 0 ? -5 : -2     ; 0x0892cc80  vdiv.s S610,S610,S612
```

No alignment gate (`Ship_TestPitchAlignment` has no counterpart call) and no
`weight_distribution`. Live: input `-100` adds `-40.0` grounded and `-10.0` in the air on every
frame; the damping reproduces `-1.5 * omega.x` racing and `-0.6 * omega.x` on the grid to
`2e-6`.

## Sideshift is 200

`if (grounded) { if (flags & 0x800) F += right * 200; if (flags & 0x1000) F -= right * 200 }`,
`right` being row 0 (`craft+0x170`), through `Body_AddForceWorld` (`0x089334a4`, `body+0x140`).
The flags come from `entity+0x9dc`/`+0x9e0`, set to `0.2` when a shift fires (`0x0892b094`).
Live: writing `0.2` to the left timer gives `body+0x140 += right * 200.000` for the next five
updates, then nothing.

## The launch is Pure's own

`FUN_0880f870` grades the moment each racer's throttle is first pressed (`craft+0x28d`, the
edge flag `Ship_UpdateEngine` raises) against a race-manager timer (`+0x130`): before `0.1` s
grade `1`, `0.1`-`0.4` s grade `2` (and the exhaust flare), `0.4`-`0.5` s grade `1`, `0.5`-`1.3`
s grade `0`. `FUN_089252e8` turns the grade into `craft+0x24c` for the first second of state
`1`: `1.2`, `0.8` or `4.2`, then `1.0`. Only the player's engine reads it. Live so far: holding
throttle through the countdown gives `0.8`. Where the timer starts relative to GO is not yet
read, so this law is not ported.

## Cross-title

Pure is not in the 2048/Omega lineage; the HD, Omega and 2048 tables carry each new field as
`inherited` from Pulse.
