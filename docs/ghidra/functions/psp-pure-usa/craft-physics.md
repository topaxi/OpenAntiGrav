# Pure's craft update, engine, forces and launch

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Wipeout Pure PSP, USA), image base `0x08804000`,
named in [names.tsv](names.tsv). The laws, the captures and what is ported are on
[pure-physics.md](../../../physics/pure-physics.md); this page is the address list and the
evidence for each name. Pulse's counterparts are in
[psp-pulse-usa/engine.md](../psp-pulse-usa/engine.md).

**Code immediates in this binary are unrelocated** (`jal 0x0012f1f0` is `0x0892f1f0`, a
`lui 0x2b`/`-0x2f8` pair is `_DAT_002afd08`; see
[psp-pure-eu/string-anchors.md](../psp-pure-eu/string-anchors.md)). Every address here is the
loaded one.

## Live evidence

PPSSPP 1.20.4, `pure-psp-usa.chd`, Time Trial, Vineta K, Feisar, Venom, 2026-10-20. A
breakpoint on `0x0892d214` stops once per craft update with `a0` the craft and `a1` the body
(the body pointer is `craft+0x1cc`, as on Pulse), `f12` the frame time `0.0333`: Pure updates
its craft at 30 Hz. From there `scripts/psp-pure-force-capture.py` stops at each callee's entry
and at its return site, and reads the VFPU register file around the call. Every name below
marked "live" was hit in that order on every captured frame, and its register change is the
law its body reads (the numbers are on the physics page).

## The frame and its callees

| Address | Name | Conf. | Evidence |
| --- | --- | --- | --- |
| `0x0892d214` | `Ship_UpdateCraft` | 92 | live: `a0` craft, `a1` body, `f12` dt; calls every function below in Pulse's order; Pulse's `0x08849618` statement for statement apart from the absent mag-lock, the absent `+0x74` hover offset and the absent per-frame `Body_SetMass` from the pre-scaled class block (Pure calls `0x08933730` with `class block +0x5c`) |
| `0x0892f1f0` | `Ship_UpdateEngine` | 92 | live, 374 frames: thrust law with no doubling, then `* craft+0x24c` for the player's craft only |
| `0x0892c7dc` | `Ship_UpdateBrakes` | 80 | static: Pulse's `0x088489d8` with `+0xa0`/`+0xa4`/`+0xa8` (Brakes gain, amount, falloff); gated as on Pulse; not exercised live |
| `0x0892f64c` | `Ship_UpdateAirbrakes` | 90 | live: the yaw term `speed * turn * (R - L) * 0.001` reproduces on every frame with the left brake held; the sideshift `right * 200` lands in `body+0x140` |
| `0x0892edfc` | `Ship_ApplySteeringTorque` | 90 | live: the ramp steps `-gain * dt` toward the stick and `+falloff * dt` back, unclamped, and `torque.y = state * Turning.amount`; Pulse's `Ship_UpdateSteering` (`0x08848788`). The name predates this page and is kept |
| `0x0892caf8` | `Ship_UpdatePitch` | 92 | live: `torque.x += pitch * (grounded ? 0.4 : 0.1)`, every frame, grounded and airborne |
| `0x0892cba0` | `Ship_ApplyQuadraticDrag` | 92 | live: coefficient `-0.0025` grounded, `-0.002` airborne, `-0.9` in state 0; force `velocity * fs * k` exact |
| `0x0892c6b0` | `Ship_UpdateHover` | 85 | static: Pulse's dispatcher (`0x0884870c`) without the `Ship_UpdateMagLock` call; live: hit between the drag and the grip every frame |
| `0x0892dea8` | `Ship_HoverTwoPoint` | 90 | live: the bank-to-yaw term `torque.y = 50 * right.y` on 129 frames; the rest of the body has Pulse's literals (`0.3`, `0.5`, `-0.1`, `2.0`, `0.2`, `5.0`, `0.75`, `-400`) |
| `0x0892e5b8` | `Ship_HoverFourCorner` | 75 | static: picked by the same Zone test as Pulse's; its bank-to-yaw literal is `50.0` (`0x0892ebe8`); not run live |
| `0x0892c994` | `Ship_ApplyLateralGrip` | 80 | static: Pulse's law with `+0x98`/`+0x9c` (grip ground, air) and `slidegrip +0xe0`, into `body+0x150` |
| `0x0892cb58` | `Ship_ApplyWeathervaneTorque` | 80 | static: `-0.3` / `-0.1` by contact, as Pulse's |
| `0x0892cc2c` | `Ship_ApplyAngularDamping` | 92 | live: `(3 / c, -5, c) * omega`, `c = -2` racing, `-5` in state 0 (`vdiv.s S610,S610,S612` at `0x0892cc80`), to `2e-6` |
| `0x0892cc94` | `Ship_ApplyRollingResistance` | 85 | live: `2 * unit(velocity)` while the speed is positive |
| `0x0892ccd0` | `Ship_ApplySpeedupPad` | 80 | static: Pulse's pad law; per-class time and amount tables |

## The body and the craft's construction

| Address | Name | Conf. | Evidence |
| --- | --- | --- | --- |
| `0x08933730` | `Body_SetMass` | 80 | static: called each frame with the class block's `mass` (`+0xe4` absolute) and the body; Pure's body keeps its mass at `+0x3b4` (Pulse `+0x374`) |
| `0x089334a4` | `Body_AddForceWorld` | 85 | live: the sideshift's `right * 200` appears in `body+0x140` across the airbrake call that calls it |
| `0x08934530` | `Body_SetBoxInertia` | 88 | static: Pulse's `0x0884e1ac`, `12 / (mass * (a^2 + b^2))` on the three axes into `body+0x40` |
| `0x089345e8` | `Body_Integrate` | 88 | static: Pulse's `0x0884e230` instruction for instruction apart from the register bank and the `+0x40` shift of everything after the inverse inertia; it rotates its inverse tensor with the basis exactly as Pulse's does |
| `0x0892cf5c` | `Ship_InitCraft` | 80 | static: writes the hover probes `(0, -1.5, +6)` and `(0, -1.5, -6)` at `craft+0xc0`/`+0xd0`, scales both by `0.75`, sets state `1` |

`Craft_Construct` (`0x089261ac`, named in [camera.md](camera.md)) passes `0.9` to the inertia
mass, `(12, 8, 12)` to `Body_SetBoxInertia`'s caller and stores `0.01`, `0.01`, `0.4` at
`body+0x3c0`/`+0x3c4`/`+0x3c8`: Pulse's figures.

## The sideshift and the launch

| Address | Name | Conf. | Evidence |
| --- | --- | --- | --- |
| `0x0892b094` | `Ship_UpdateSideshiftInput` | 85 | live: writing `0.2` to `entity+0x9dc` makes `craft+0x1c0` carry `0x800` for the next five updates (`0x0892b97c`); `+0x9e0` sets `0x1000` (`0x0892b9c4`). The tap windows above it (`0.25` at `+0x9d4`/`+0x9d8`, the `+/-10` flick) are read statically |
| `0x0880f870` | `Race_UpdateLaunchGrade` | 88 | live: the race-manager clock at `+0x130` starts at `0.0` on the first racing update; a first throttle edge (`craft+0x28d`) at `0.200` wrote grade `2` to `entity+0x48c`. Windows `0.1`, `0.4`, `0.5`, `1.3` are its literals |
| `0x089252e8` | `Ship_UpdateStartBoost` | 88 | live: `craft+0x24c` read `4.2` (grade 2) until the craft's state-1 clock (`entity+0x9c0`) reached `1.0`, then `1.0`; `1.2` before the edge; `0.8` after a throttle held through the countdown |
| `0x0886b600` | `ExhaustFlare_OnPerfectStart` | 85 | static: called from `Race_UpdateLaunchGrade`'s grade-2 branch for a non-AI craft; stores `0.8` to the flare's boost timer (`+0x19c`) and plays `TURBO` (`0x08a490c4`), where `ExhaustFlare_OnSpeedupPad` (`0x0886b548`, beside it) plays the pad's cue: Pulse's `0x08904fd4` |

## The handling loader

| Address | Name | Conf. | Evidence |
| --- | --- | --- | --- |
| `0x08920a4c` | `Handling_ParseStats` | 88 | static: walks `Stats`, the five camera blocks, `Misc`, `AirbrakeGraphics` and `Class name=` with the six class blocks below; the strings are read out of memory |
| `0x0891fadc` | `HandlingXml_ParseAntigrav` | 88 | static: `ride_height +0x88`, `rebound +0x8c`, `landing_rebound +0x90`, `rebound_jump_time +0x94`, `grip_ground +0x98`, `grip_air +0x9c`; live values (Feisar Venom `6.0`, `0.6`, `0.5`, `1.0`, `2.5`, `1.0`) |
| `0x0891ff38` | `HandlingXml_ParseBrakes` | 88 | static: `gain +0xa0`, `amount +0xa4` (`* -0.01`), `falloff +0xa8` |
| `0x0891fd10` | `HandlingXml_ParseEngine` | 92 | static and live: `gain +0xac`, `amount +0xb0` (`* 0.001`), `falloff +0xb4`, `accelcap +0xb8`, `turbo +0xbc`; the engine law reproduces with these values |
| `0x089200c8` | `HandlingXml_ParseTurning` | 92 | static and live: `gain +0xc0`, `amount +0xc4`, `falloff +0xc8`; the steering ramp reproduces with them |
| `0x0892021c` | `HandlingXml_ParseAirbrake` | 88 | static: `gain +0xcc`, `falloff +0xd0`, `amount +0xd4` (`* 0.0001`), `turn +0xd8`, `drag +0xdc`, `slidegrip +0xe0` (`* 0.0001`); **no `sideshift`** |
| `0x0891f93c` | `HandlingXml_ParsePhysical` | 88 | static: `mass +0xe4`, `normal_gravity +0xe8`, `flight_gravity +0xec`, `track_gravity +0xf0` |
| `0x089204b4` | `HandlingXml_ParseAirbrakeGraphics` | 80 | static: `amount` in degrees `* 0.017453292` at `+0x6c`, `up_speed +0x70`, `down_speed +0x74`; live `0.5236` (30 degrees), `500`, `100` |
| `0x089205c0` | `HandlingXml_ParseMisc` | 85 | static: `width +0x78`, `length +0x7c`, `height +0x80`, `shield +0x84`; **no `weight_distribution`**; live `5.5`, `13`, `3.5` |

All offsets are `stats + class * 0x6c` for the class blocks and `stats` for the rest; the
class stride is `0x6c` against Pulse's `0x80` because Pure has no `sideshift` and no `<pitch>`
block.

## The hover escape's translate (2026-10-21, `pulse-wall-escape-census`)

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x089338c8` | `Body_Translate` | 85 | adds `f12`, `f13`, `f14` to the vector at `a0+0x30` (`lv.q C400,0x30(a0)` ... `sv.q C400,0x30(a0)`, `jr ra`) and writes nothing else. That is Pulse's `Body_Translate` (`0x0884d9f8`) instruction for instruction, in VFPU form. Its call site is `Ship_HoverTwoPoint`'s escape (`0x0892e0b8`, the unrelocated `jal 0x0012f8c8`), with `a0 = craft+0x1cc`, the body, and the move `up * (1 - h)`. Static only. |

Because it moves only the position, the two passes of `Ship_HoverTwoPoint`'s probe loop
(`0x0892e00c`-`0x0892e358`) each escape off a height taken from the probe point and hit cached
before the loop, and the escapes add, as on Pulse
([pure-physics.md](../../../physics/pure-physics.md#the-hover-escapes-sum-as-on-pulse-2026-10-21-pulse-wall-escape-census)).

## Not established

- The EU pressing was not compared; these are the USA addresses.
- `Ship_UpdateBrakes`, `Ship_ApplyLateralGrip`, `Ship_ApplyWeathervaneTorque` and
  `Ship_ApplySpeedupPad` were not separated live (the capture's VFPU diff does not see the grip,
  which writes the body directly, and no brake or pad was taken). They are Pulse's laws by
  reading.

## The drawn steering lean (2026-10-21, `ship-lean`)

| Address | Name | Conf. | Evidence |
| --- | --- | --- | --- |
| `0x08925054` | `Ship_UpdateSteerLeans` | 85 | static: Pulse's `0x0883fab4` instruction for instruction. Camera lean `entity+0x978`/`+0x974` (`0.01`, clamps `0.6`/`0.3`, `5.4`, `4.0`); drawn lean `entity+0x988`/`+0x984` (`stick * 0.01 - left * 0.005 + right * 0.005` at `0x089251a8`-`0x089251e0`, clamps `0.5`/`0.3` on the raw stick's side, follower `5.0` at `0x089252a4`, smooth `3.0` at `0x089252b0`). Pulse's two flag bits are bytes here: the negate is `entity+0x991`, the destroyed gate `entity+0xb95`. Not run live |

The display angle is composed at `0x0892718c`-`0x089271a4` in `FUN_08926ae4`:
`entity[0x984] * f20 + entity[0x9b8] * 6.28` (`lui 0x40c8; ori 0xf5c3`), `f20` being `0.5`
(`lui 0x3f00` at `0x08926b00`, `mtc1` at `0x08926b0c`), so Pure draws Pulse's lean exactly.
**Checked, same law**: `oag_pure::race::CRAFT_LAWS.tilt_steer_lean` is `None`, Pulse's gains
(`oag_physics::SteerLean::PULSE`), and Pure's files author no `<Tilt>`. See
[`../psp-pulse-usa/camera.md`, "The drawn lean"](../psp-pulse-usa/camera.md#the-drawn-lean-entity0x854-read-and-measured-2026-10-21).
