# The grid state: what a craft on the start line does differently

**Status: one term ported (the bank-to-yaw coupling), three read, tried and left
out, the launch effect that was measured here ported on its own page
([launch-boost.md](launch-boost.md)), and the grid walk ported on 2026-10-02 from the
original's own code - the slots along the track and every heading, below.** Measured on Pulse PSP (USA) in
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
digits quoted.

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

`Race_PlaceGrid` puts every craft in state `0` and `Race_StartRacing` moves every
craft to state `1`, both through `Ship_SetState`, which calls `Craft_SetState`
(`0x08848590`) on the craft. That writes the craft's own state word, `craft+0x2a4`,
after running `Craft_EnterGridState` (`0x088486d4`, state 0: `craft+0x1c0 |= 2`) or
`Craft_ReleaseFromGrid` (`0x088486e4`, state 1: `craft+0x1c0 &= ~2` and the brake,
steering and both airbrake words zeroed). Live: `flags_1c0 = 0x2`/`0x3` and
`state_2a4 = 0` through the countdown, `0x1`/`1` from the frame before the throttle
word steps. Confidence **90** (instructions read, effects watched live on four
runs; the PS2 build not compared). Evidence page:
[grid.md](../ghidra/functions/psp-pulse-usa/grid.md#the-crafts-own-state-craft_setstate-grid-state-0-and-racing-state-1-2026-10-01).

Four things in the force law read that state, all in the original and all read
from the decompile on 2026-10-01:

| Where | State 0 | State 1 | Ported |
| --- | --- | --- | --- |
| `Ship_HoverTwoPoint`'s epilogue, `0x0884ad2c  lw a2,0x2a4(s0)` / `0x0884ad30  beq a2,zero,0x0884ad78`: `craft+0x2a4 != 0` guards `localAngular.y += 30 * right.y * (1 - magLockBlend)` | **skipped** | applied | **yes**, `ShipState::on_grid`, read in `hover::evaluate` |
| `Ship_ApplyAngularDamping` (`0x08848ed0`): the roll-axis coefficient | `-5` | `-2` | no: tried, indistinguishable from nothing |
| `Ship_HoverTwoPoint`'s head: the `rebound` base in the spring's damping factor | `1.0` | `handling+4` | no: tried, indistinguishable from nothing |
| `Ship_UpdateCraft`'s control record, while `craft+0x1c0 & 2`: `+8` and `+0xc` forced to `100.0`, `+0`, `+4` and `+0x10` zeroed | both airbrakes full (the brake rises with them), no steering, no thrust | the live record | no: tried on top of the boost and it moves neither the pose nor the launch, see [launch-boost.md](launch-boost.md#the-three-other-state-0-terms-tried-and-left-out) |

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

`ShipState::on_grid` is the original's state `0` (measured on Pulse PSP only; the
race applies it to every title by extension, which is **chosen, not measured**,
and not to Zone, whose four-corner hover epilogue carries the same guard but which stays off, see
[Zone](#zone-the-grid-state-gates-the-auto-speed-and-on_grid-is-on-2026-10-02)), written from the race's
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
rows; the last two are from `ours-tt-16-gate.txt`.)

## What is still different at GO, and what is not this page's

**Heading, ported second.** Ours started exactly along `+X` because every slot
took the node's orientation; the original starts at `-0.146` degrees and its
eight slots read `-0.0025`, `-0.0032`, `-0.0039`, `-0.0044`, `-0.0049`, `-0.0053`,
`-0.0057` and `-0.0061` of forward `z`, slot 1 to slot 8, a smooth ramp. That is
`grid.md`'s "the original re-derives orientation from each slot's own sample
(`FUN_0882663c`)", and the track's own sample tangent at those positions reads
within 0.05 degrees where the node's is out by up to 0.35. Pulse PSP slots now
take that frame (`oag_gameplay::orientation_on_sample`, gated by
`Setup::grid_frame_from_sample`); the worst of the eight is 0.045 degrees out,
and the Time Trial's yaw at GO is `-0.191` against `-0.146` (0.045), from 0.145.

**Superseded 2026-10-02.** `FUN_0882663c` read to the end: the heading is the unit sum of the
left-edge and right-edge chords over 20 units, not the tangent. Through
`oag_gameplay::grid_walk` the eight slots are within **0.001 degrees** of the original's (it
was 0.045), and the Time Trial's yaw at GO follows. See
[grid.md](../ghidra/functions/psp-pulse-usa/grid.md#the-grid-walk-read-to-the-end-2026-10-02).

**Position, 0.10 units in z.** The original places the craft 2 units above the
floor and the spring raises it along the craft's own up axis, which on this
banked start moves it 0.04 in z, and it slides 0.03 more through the countdown
under brake and airbrake that ours does not hold. Ours is placed at its rest
height. Not ported: sub-0.1 and not on the path of anything measured.

**Slots along the track: the walk, ported 2026-10-02.** The 2026-10-01 attempt (an exact
walk of the resampled samples) made Metropia worse and was reverted, and left two residues
unexplained: a `0.07` units per slot bias (ours further along the track) and a `0.4` to `0.5`
offset on every `16_Track` slot. Both are in the original's code and neither is a fit.
`Race_ComputeGridLayout` steps `19.8` along the located record's **own tangent** and locates
again - `p(k+1) = locate(p(k) + tangent * 19.8)` - and the locate is a projection onto the
B-spline of the lifted control points (three Gauss-Newton steps), not a nearest sample. The
record it writes is **scaled by `0.999756`** (`FUN_0887c7e8` multiplies its weights by the
half-float immediate `0x3155`, `0.16662598` for `1/6`): the position moves toward the world
origin by `0.000244` of its coordinate and the tangent shortens by as much, so a point at
`x = -721` is 0.17 units from where its coordinates say and the step along `z = 283` loses 0.07
a slot - which is the `19.82`, `19.74` and `19.67` the three circuits fitted separately.

| worst slot, xz | before | now |
| --- | ---: | ---: |
| `01_Track` | 1.73 | **0.001** |
| `16_Track` | 0.62 | 0.043 |
| Metropia reversed | 1.13 | 0.070 |

The eight located records read live on Metropia match the chain to 0.0002 units. The
chain, the scale and the heading are `oag_gameplay::grid_walk`; the account with every
address is [grid.md](../ghidra/functions/psp-pulse-usa/grid.md#the-grid-walk-read-to-the-end-2026-10-02),
and `crates/game/tests/grid_walk_ground_truth.rs` pins it (dropping the scale fails the
located-chain test, `01_Track`'s and Metropia's, and two of the unit tests). Side note from the old attempt that stands: the Time Trial's slot 1 on `16_Track`
was 0.014 units from the original along the track "by luck"; it is 0.038 now, by the walk.

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

**Ported since, and the open question answered**: the grade is selected by *when the
thrust first lands*, not by a reaction time, and the window and the three multipliers
are the disc's `<StartBoost>`. See [launch-boost.md](launch-boost.md): five launches
watched, held through the countdown is within 0.4 units of the original at 120 frames
(it was 18 behind). The `craft+0x2d8`/`+0x2dc` pair that "runs 100 to 0" over the same
window is not a boost percentage: it is the airbrake flaps' visual position
(`Ship_UpdateAirbrakes`) releasing from the grid's forced-full.

## Zone: the grid state gates the auto-speed, and `on_grid` is on (2026-10-02)

Zone runs `Ship_HoverFourCorner` (`0x0884ae90`) where every other mode runs `Ship_HoverTwoPoint`. Its epilogue
(`0x0884b76c`) carries the same `craft+0x2a4` guard with gain `50.0`. **That coupling is ported since**
(`ShipState::four_corner`, `hover::BANK_TO_YAW_GAIN_FOUR_CORNER`; evidence on
[zone-rest.md](../ghidra/functions/psp-pulse-usa/zone-rest.md)); the rest of the four-corner function is not.

`Ship_UpdateEngine`'s Zone branch is `((flags & 1) && !(flags & 2)) ? autospeed : 0.0`, and bit 1 is the grid state, so
a Zone craft **stands through the countdown**: read live on a Zone engine, `flags` `0x3` and `0.02` units/s for the whole
state 0, `0x1` and thrust `95.2` from the first state-1 frame. Our Zone craft had been drifting through it (358 units
before GO in the new `zone_ground_truth` test). `on_grid` now covers Zone, and `engine::engine` zeroes the auto-speed
while it is set. Evidence, the table and the caveat (mode patched on a Single Race grid) are on
[zone-start.md](../ghidra/functions/psp-pulse-usa/zone-start.md); confidence **85**.

The launch multiplier is in the same tail and applies to Zone's auto-speed too: see
[launch-boost.md](launch-boost.md#zone).

## The second circuit: `01_Track` (Basilico Black), Time Trial and Single Race

Reached through the dev-unlock byte ([ppsspp-debugger.md](../reverse-engineering/ppsspp-debugger.md#every-circuit-not-three-the-dev-unlock-byte-2026-09-29)),
17 downs from Talon's Junction White; identified by its start, `(-721.16, 4.01,
282.58)`, which is slot 1 of this project's `01_Track` grid. Two Time Trial runs
(`orig-tt-01-{a,b}.json`) and a Single Race grid (`orig-grid-01-single-a.json`).

| | Original | Ours |
| --- | --- | --- |
| Yaw, frames 0 to GO | `90.396` degrees, constant to the third decimal | `90.176`, constant |
| Yaw at GO, and the first Single Race slot | `-0.0069` of forward `x` | `-0.0069` (it was `-0.0031`, 0.22 degrees out) |
| The other seven slots' forward `x` | `-0.0014 ... +0.0002` | within 0.1 degrees |
| Position at GO | `(-721.163, 4.012, 282.583)` | 0.001 out (it was 0.95 along the track, the sawtooth) |
| GO + 120 | `z 400.96`, `106.4` units/s | `z 381.71`, `100.3` |

The gate holds on the second circuit (a banked start with a different heading
and a node on the corridor's *left*, the branch `grid.md` had only from the
decompile): yaw constant through the countdown in both. Slot 1's heading is an
outlier in the original (`-0.0069` against `-0.0014` for slot 2): **it is the edge
chord** (2026-10-02). The heading is not the tangent but the direction the track's
two edges run, and the track widens at that slot; `FUN_0882663c`'s rule reproduces
all eight `01_Track` headings to 0.0005 degrees.

The eight Single Race craft hold their heading through the countdown too (read
at placement, then 2.5 and 3.5 s later on `16_Track`: forward `z` identical to
four places on all eight), so `on_grid` is right for the opponents as well.

## Reproducing

```sh
# a PPSSPP with its debugger on, in a Time Trial on Talon's Junction (psp-drive.py menu)
python3 scripts/psp-start-pose.py --port 45681 --hold --out data/scratch/<lane>/orig-tt.json
python3 scripts/psp-start-pose.py --port 45681 --hold --release-at 120 --max-rows 500 --out ...   # no thrust after 120
python3 scripts/psp-start-pose.py --port 45681 --hold --full --after-go 150 --out ...             # the raw craft, every frame
python3 scripts/psp-grid-pose.py --port 45681 --out ...                                          # all eight Single Race slots at placement
OAG_REQUIRE_GAME_DATA=1 just test-data     # start_pose_ground_truth
```
