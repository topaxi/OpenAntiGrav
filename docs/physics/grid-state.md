# The grid state: what a craft on the start line does differently

**Status: one term ported (the bank-to-yaw coupling), three read and left, one
launch effect measured and not implemented.** Measured on Pulse PSP (USA) in
PPSSPP 1.20.4, 2026-10-01, Time Trial, Venom, Assegai, `16_Track`; the Single
Race grid on the same circuit.

The question this page answers came from the weapons lane: the original's craft
at the same place as ours was "2.4 units and 1.7 degrees off", enough to move a
Rocket's detonation by 30 ticks. **It was not the start pose.** The start pose
is right to 0.12 units. What differed was a force term that runs on the grid in
ours and does not in the original, so the craft yawed through the whole
countdown and arrived at GO pointing 1.3 degrees away.

## The measurement

`scripts/psp-start-pose.py` restarts the race and breaks on `Weapons_DispatchFire`
once a frame, which is once per craft per frame and so once per frame for a Time
Trial, and reads the player's rigid body each time (position, the three axes,
velocity, angular momentum), the craft's throttle word, and with `--full` the
first `0x400` bytes of the craft. Cross held from the restart through the
countdown makes the first non-zero throttle word GO. Four runs agree to the
digits quoted (`data/scratch/pulse-start-pose/orig-tt-16-{a,b,c,d}.json`).

| Frames from placement | Original | Notes |
| --- | --- | --- |
| 0 | `(6.145, -52.221, -195.876)`, vy `-6.33` | placed 2 units above the floor with the hover spring running (`Race_PlaceGrid`: raycast, `+2`), `dt` 0.0667 on this first frame |
| 25 | `(6.096, -50.435, -195.931)`, vy `+3.2` | the settle overshoots |
| 50 | `(6.083, -50.019, -195.937)` | back to rest |
| 100 - 332 | `(6.082 ... 6.081, -50.05 ... -50.064, -195.937 ... -195.945)` | rests; z creeps 0.008 units |
| 333 (GO) | `(6.081, -50.064, -195.945)` | first non-zero throttle word |

**Yaw reads `-0.146` degrees on every frame from 0 to GO**, to the third decimal.
Pitch and roll move for the first 60 frames (the settle) and then hold. Angular
momentum on the yaw axis is `1e-5` through the countdown.

**From GO, with cross released at frame 120 so nothing is being pushed, yaw
moves at `0.29` degrees a second** (`-0.146` to `-0.685` degrees by frame 450 in
`orig-tt-16-b.json`). The angular momentum on that axis steps from `1e-5` to
`-0.0096` on the GO frame and ramps by `0.0096` a frame to the `-0.1147` that
`data/traces/talons-junction-standing-start.csv` (`avel_y`) already records:
a constant torque of `0.573` against the yaw damping `-5 * L`.

## What the original does: `craft+0x2a4`, state 0 and state 1

`Race_PlaceGrid` puts every craft in state `0` (`Ship_SetState(craft, 0)`,
`FUN_08848590`), `Race_StartRacing` moves every craft to state `1`. The craft's
own state word is `craft+0x2a4`; `FUN_08848590` writes it after calling
`FUN_088486d4` (state 0: `craft+0x1c0 |= 2`) or `FUN_088486e4` (state 1:
`craft+0x1c0 &= ~2` and the brake, steering and both airbrake words zeroed).
Live: `flags_1c0 = 0x2`/`0x3` and `state_2a4 = 0` through the countdown,
`0x1`/`1` from the frame before the throttle word steps. Confidence **94**
(decompile of all three functions, live reads of both words on four runs).

Four things in the force law read that state, all in the original and all read
from the decompile on 2026-10-01:

| Where | State 0 | State 1 | Ported |
| --- | --- | --- | --- |
| `Ship_HoverTwoPoint`'s epilogue, `0x0884ad2c`: `if (craft+0x2a4 != 0)` guards `localAngular.y += 30 * right.y * (1 - magLockBlend)` | **skipped** | applied | **yes**, `ShipState::on_grid`, read in `hover::evaluate` |
| `Ship_ApplyAngularDamping` (`0x08848ed0`): the roll-axis coefficient | `-5` | `-2` | no |
| `Ship_HoverTwoPoint`'s head: the `rebound` base in the spring's damping factor | `1.0` | `handling+4` | no |
| `Ship_UpdateCraft`'s control record, while `craft+0x1c0 & 2`: `+8` and `+0xc` forced to `100.0`, `+0`, `+4` and `+0x10` zeroed | both airbrakes full (the brake rises with them), no steering, no thrust | the live record | no, only thrust is gated (`RaceState::thrust_gated`) |

The bank-to-yaw coupling is the one that moves the pose: on a start banked
`1.1` degrees (`right.y = -0.0191` on this slot) it is a torque of `0.573` every
frame. The other three are small at rest and are listed so the next reader does
not rediscover them. Confidence **92** for the bank-to-yaw gate (decompile, and
the onset of the drift on the GO frame in every run); **84** for the other three
(decompile only).

The state-0 control record also answers a question [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-engine-has-an-early-return-that-produces-no-thrust-at-all)
left open: something other than `Ship_UpdateEngine`'s early return pins
`throttleState` to zero through the 272 countdown ticks. It is `Ship_UpdateCraft`
re-writing the control record every frame while flag `0x2` is set, which zeroes
the thrust input and holds the airbrakes at full. Confidence **88**.

## What ours did, and the fix

Ours has no craft state, so the coupling ran from tick 0. A craft starting
level on this slot (the spawn pose is the track node's, which is horizontal)
settles onto the `1.1` degree bank in the first 40 ticks and then yaws at
`0.29` degrees a second: `1.29` degrees by the 272 ticks of countdown.

`ShipState::on_grid` is the original's state `0`, written from the race's
countdown clock (`RaceState::thrust_gated`) on every ship at the top of
`Race::tick` - for every craft, not only the player, which is how the original
does it - and read by the one term. Derived from the clock rather than stored,
so a respawn after the start can never re-enter it.

| | Original | Ours before | Ours after |
| --- | --- | --- | --- |
| Yaw drift over the countdown, ticks 30 - 272 | under `0.001` degrees | `-1.29` | under `0.001` |
| Yaw at GO | `-0.146` | `-1.29` | `-0.001` |
| Position at GO | `(6.081, -50.064, -195.945)` | `(6.078, -50.061, -195.827)` | the same, 0.118 off in z |
| Yaw moved in the first 30 ticks from GO | `0.102` degrees | | `0.098` |
| Yaw at GO + 120 | `-0.42` | `-1.74` | `-0.375` |
| z at GO + 120 | `-196.86` | `-198.70` | `-196.39` |

(`crates/game/tests/start_pose_ground_truth.rs` pins the first, second and fourth
rows; the last two are from `data/scratch/pulse-start-pose/ours-tt-16-gate.txt`.)

## What is still different at GO, and what is not this page's

**Heading, 0.145 degrees.** Ours starts exactly along `+X` because every slot
takes the node's orientation; the original starts at `-0.146` degrees and its
eight slots read `-0.0025`, `-0.0032`, `-0.0039`, `-0.0044`, `-0.0049`, `-0.0053`,
`-0.0057` and `-0.0061` of forward `z`, slot 1 to slot 8, a smooth ramp. That
is `grid.md`'s "the original re-derives orientation from each slot's own sample
(`FUN_0882663c`)", and the track's own sample tangent at those positions reads
`-0.0033` to `-0.0057`: within 0.05 degrees, where the node's is out by up to
0.35.

**The launch boost.** The original's craft accelerates faster than ours once
GO comes with thrust already held:

| | Original | Ours |
| --- | --- | --- |
| Thrust held through the countdown, GO to GO + 4 | `47.5, 39.7, 40.5, 41.3, 42.1` units/s per second | `34.0, 32.5, 33.0, 33.6, 34.1` |
| Thrust first pressed 30 frames after GO | `47.5, 46.6, 47.7, 48.8, 49.9` | |
| Thrust first pressed 15 s after GO (the existing `data/traces/` method) | `33.7, 32.3, 32.9, 33.4, 33.9` | the same |

The difference is `craft+0x294`, the start-line boost multiplier
(`Ship_UpdateStartBoost`, `0x0883fdec`, [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#craft0x294-is-the-start-line-boost-multiplier)).
Read live, a held-thrust launch has it at `1.0` until the frame before GO, **`1.4`
for that one frame, `1.2` for the next 58 frames, then `1.0`**; a craft that
presses late sees `1.0` throughout. `craft+0x2d8` and `+0x2dc` count down `100`
to `0` linearly over the same 59 frames, which is the boost window as a
percentage. The thrust the engine writes (`craft+0x328`) is `47.6` against `34.0`
on the first thrust frame. That is the `x1.2` and not a rebuild of the force law:
the old captures under `data/traces/` were all taken by pressing late, which is
why the engine's launch acceleration matched them to 2 %.

Consequence for any matched-state comparison started from a held-thrust GO: ours
is `3.1` units/s slower at GO + 20 and `18` units behind at GO + 120 (`106.1`
against `124.5` on x; speeds `100.5` and `106.6`), the largest term left.
`grade 0, 1, 2, 3` (`player+0x36c`) and the four XML values
(`windowStart`, `overallDuration`, `stallMul`, `normalMul`, `boostMul`) are what
decide `1.2` against `1.4`, and which of them the reaction time selects was not
read here. Not implemented: it is its own thread, with the measurement above as
its starting point.

## Reproducing

```sh
# a PPSSPP with its debugger on, in a Time Trial on Talon's Junction (psp-drive.py menu)
python3 scripts/psp-start-pose.py --port 45681 --hold --out data/scratch/<lane>/orig-tt.json
python3 scripts/psp-start-pose.py --port 45681 --hold --release-at 120 --max-rows 500 --out ...   # no thrust after 120
python3 scripts/psp-start-pose.py --port 45681 --hold --full --after-go 150 --out ...             # the raw craft, every frame
python3 scripts/psp-grid-pose.py --port 45681 --out ...                                          # all eight Single Race slots at placement
OAG_REQUIRE_GAME_DATA=1 just test-data     # start_pose_ground_truth
```
