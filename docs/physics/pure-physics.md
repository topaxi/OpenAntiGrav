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
| Engine output doubling | `T * 2.0` | **none** | `Ship_UpdateEngine` `0x0892f1f0` | 92 | **on**: `CraftLaws::engine_output_double` (see [the gates](#what-is-on-and-what-is-held)) |
| Grounded quadratic drag | `-0.005` | **`-0.0025`** | `Ship_ApplyQuadraticDrag` `0x0892cba0` | 92 | **on**: `CraftLaws::drag_ground` |
| Bank-to-yaw gain, two-point hover | `30.0` | **`50.0`** | `Ship_HoverTwoPoint` `0x0892dea8` | 92 | **on** since 2026-10-21: `CraftLaws::bank_to_yaw` |
| Pitch torque per unit input | XML `pitch_air`/`pitch_ground` (`0.3`/`1.0` on every Pulse team) | **`0.1` air, `0.4` grounded**, code literals | `Ship_UpdatePitch` `0x0892caf8` | 92 | **on**: `oag_pure::race::CODE_PITCH`, `CraftLaws::code_pitch` |
| Pitch damping | XML `pitch_damping` (`3`) | **`1.5`** racing, `0.6` on the grid (`3 / c`) | `Ship_ApplyAngularDamping` `0x0892cc2c` | 92 | **on**, with the pitch torque; the grid's `0.6` is `oag_physics::laws::Laws::pitch_damping_grid` |
| Sideshift force | XML `Airbrake sideshift` | **`200.0`**, a code literal | `Ship_UpdateAirbrakes` `0x0892f64c` | 92 | **on**: `oag_pure::race::SIDESHIFT`, `CraftLaws::code_sideshift` |
| Launch boost | `<StartBoost>` XML | **code law**: windows `0.1`/`0.4`/`0.5` s, `x0.8`/`x1.2`/`x4.2` for 1 s, player only | `Race_UpdateLaunchGrade` `0x0880f870`, `Ship_UpdateStartBoost` `0x089252e8` | 88 | **on**: `oag_pure::race::START_BOOST`, `CraftLaws::code_start_boost` |

Confidence 92 on the six measured rows: a runtime trace that reproduces the instruction-level
reading exactly, on one binary. Not 95: the EU pressing was not run. The launch is 88: its
grade-2 branch and its multipliers were watched, its window edges are read.

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
updates, then nothing. Five, not six, because the timer was written after that frame's input
pass had already run; a shift the player fires pushes for `0.2` s, as ours does.

## The launch is Pure's own

`FUN_0880f870` grades the moment each racer's throttle is first pressed (`craft+0x28d`, the
edge flag `Ship_UpdateEngine` raises) against a race-manager timer (`+0x130`): before `0.1` s
grade `1`, `0.1`-`0.4` s grade `2` (and the exhaust flare), `0.4`-`0.5` s grade `1`, `0.5`-`1.3`
s grade `0`. `FUN_089252e8` turns the grade into `craft+0x24c` for the first second of state
`1`: `1.2`, `0.8` or `4.2`, then `1.0` until `1.5` s. Only the player's engine reads it
(`entity == *(0x538e8)+0x13c`, `0x0892f60c`). Live: the race-manager clock reads `0.0` on the
first racing update (it starts at GO, as Pulse's does); a first press at `0.200` s wrote grade
`2` and `craft+0x24c` read `4.2` until the craft's state-1 clock reached `1.0`, then `1.0`; no
press read `1.2`; throttle held through the countdown read `0.8`. That is Pulse's
`<StartBoost>` law (`launch-boost.md`) with the seven figures as code literals, so it ports as
data: `oag_pure::race::START_BOOST` = windows `0.1`/`0.4`/`0.5`, held `1.0` s, `0.8`/`1.2`/`4.2`.
Pure's last grading window runs to `1.3` s, past the multiplier's `1.0`, so a press there
changes nothing on either engine. The original applies the multiplier to the player's engine
alone, and so would ours (`CodeStartBoost::player_only`: the field launches unboosted). Applied
to every craft instead (Pulse's rule), our AI, which holds throttle from GO, took Pure's `x0.8`
stall for a second, and the sweep's Ace branch-A walls rose `13 -> 118`. **On**, player-only,
with the speed laws it rides on (below).
The perfect start's own effect is Pulse's too: `ExhaustFlare_OnPerfectStart` (`0x0886b600`)
arms the flare's boost at `0.8` and plays `TURBO`.

## What is on and what is held

**On (2026-10-20, lane `pure-laws-on`): the speed pair, the launch, the sideshift and the pitch
with its grid damping. The bank-to-yaw gain was held at Pulse's `30` until 2026-10-21**, when the
maintainer decided the original's law is always the default and our AI adapts to it
([below](#bank-to-yaw-on-2026-10-21-deviations-to-original)).

### The noise bar

Our AI's gates are chaotic: a meaningless change to the start or to one craft's torque reshuffles
the pack and moves the wall counts a long way (a player-only launch, which no AI craft's physics
ever reads, once moved the sweep's novice walls `23 -> 73`). So each law was judged against a
noise distribution, not one run.

- **The perturbation.** A constant pitch torque `eps` added to `acc.local_angular.x` in
  `oag_physics::forces` after the pitch term, airborne only (`!control_contact`), read from an
  `OAG_X_EPS` environment variable. The same knob `driver-vs-new-physics` used on Pulse's gates.
  It is a scratch patch, never committed. Twelve values: `0`, `+-1e-5`, `+-2e-5`, `5e-5`,
  `+-1e-4`, `+-3e-4`, `+-1e-3`. The launch was rerun on twelve more (`+-3e-5`, `-5e-5`, `+-7e-5`,
  `+-2e-4`, `4e-4`, `+-5e-4`, `+-2e-3`).
- **Each sample** is one law state and one `eps`, eight seeds a level:
  `OAG_SWEEP=1 OAG_SWEEP_IMAGE=pure OAG_SWEEP_SEEDS=8 OAG_SWEEP_DIFFICULTY=<novice|ace>` over
  `opponent_autopilot_sweep::pure` (64 stints a level, wall ticks in branch A, our driver, and
  B, the original's) and `opponent_autopilot_sweep::races_others` (eight whole races a level,
  Autopilot off and on). The metrics are summed or averaged over the eight seeds: sweep walls
  A and B, race walls, wrecks, lap time, mean shield per lap and mean end-of-run shield.
- **The rule**, per metric, the law's twelve samples against the base's twelve:
  - the median by Mann-Whitney;
  - the tail by `k`, the number of law samples beyond the base's worst in the regression
    direction. With no effect, `P(K >= k) = C(12, k) / C(24, k)`: `k = 2` is `0.24`, `4` is
    `0.047`, `7` is `0.002`.
  - A law stays held when a metric regresses at `P < 0.01` by either test.
  - Max against max is not a test: with no effect, the law's max beats the base's half the time.
- Each law was judged on top of the laws already on: the speed pair on Pulse's laws; the launch
  on the speed pair; the sideshift on speed and launch; bank-to-yaw and pitch each on speed,
  launch and sideshift.

### The verdicts

| Law (base) | Race walls Ace / novice, median base -> law | Sweep walls Ace A / B | Worst tail | Verdict |
| --- | --- | --- | --- | --- |
| Speed pair (Pulse's laws) | `2680 -> 1658` (p `0.001`) / `593 -> 786` (p `<0.001`) | `433 -> 106` / `124 -> 87` | novice race walls `k = 12`; novice end shield `88.2 -> 85.7`, `k = 6` (`P 0.007`); novice sweep B `16 -> 51`, `k = 6` | **on**, on the lead's decision: outside noise both ways |
| Launch (speed, 24 samples a side) | `1823 -> 1826` / `769 -> 733` | `135 -> 128` / `93 -> 104` (p `0.060`) | novice sweep B `k = 3` (`P 0.12`) | **on**: inside |
| Sideshift (speed, launch) | `1745 -> 1911` (p `0.21`) / identical | `109 -> 106` / `121 -> 112` | Ace race walls `k = 2` (`P 0.24`) | **on**: inside |
| Bank-to-yaw `50` (speed, launch, sideshift) | `1911 -> 2594` (p `0.13`) / `747 -> 722` | `106 -> 151` / `112 -> 96` | **Ace race walls `k = 7` (`P 0.002`)**: `3052`-`4338` against a worst of `2339`; Ace laps `k = 5` | held until 2026-10-21, **on** since |
| Pitch with grid damping (speed, launch, sideshift) | `1911 -> 1683` / `747 -> 809` (p `0.18`) | `106 -> 140` / `112 -> 92` | sweep Ace A `k = 2` (`P 0.24`): `585` and `232` against `216`; the `585` is seed 7 at `eps = +1e-5`, `514` ticks of it | **on**: inside; laps `0.3`-`0.5` s faster (p `< 0.01`) |

Under the 12/12 split the launch's sweep Ace B read p `0.011`; 24 samples a side put it at
`0.060`. One p below `0.05` in 23 metrics is what chance gives. With the sideshift on, the
novice columns are bit-identical: our novice driver never sideshifts. Bank-to-yaw splits the
perturbations: half go badly (Ace races at `45`-`46` s laps, with respawns) and half come out a
little better. That is why its median test reads `0.13` while its tail reads `0.002`.

**The speed pair is a real change, not noise.** Ace flies far cleaner: race walls `-38 %`,
sweep A `-75 %`. Novice hits the walls more: race walls `+32 %`, every one of the twelve
samples above the base's worst, and its end shield is `2.5` lower. Laps lengthen from `49.8` to
`54.1` s novice and from `41.2` to `44.2` s Ace. That is the measured half acceleration. Headless,
with Cross held from the countdown, the race's own driver on Vineta K reaches `90` u/s
`87` ticks after GO on Pulse's laws and `193` ticks after on Pure's. Pure's figure includes its
`x0.8` stall for thrust held through the countdown. The novice regression is our driver's to
fix, not the law's.

### At the tip

- Unperturbed, `eps = 0`, every Pure law on but bank-to-yaw. Eight seeds, Autopilot off.
- **Per-lap shield** is the mean over the three laps of each race, averaged over the eight
  seeds: novice `92.1`, Ace `72.8`.
- **End-of-run shield**, averaged the same way: novice `88.2`, Ace `64.5`.

| Level | Laps | Race walls (8 races) | Wrecks |
| --- | --- | --- | --- |
| Novice | `52.9`-`54.3` s | `701` | 1 (seed 4) |
| Ace | `43.1`-`43.8` s | `1665` | 2 (seeds 3, 8) |

On Pulse's laws, measured the same way:

| Level | Laps | Race walls | Wrecks | Per-lap shield | End-of-run shield |
| --- | --- | --- | --- | --- | --- |
| Novice | `49.5`-`50.4` s | `467` | 0 | `90.3` | `87.6` |
| Ace | `40.5`-`41.7` s | `2568` | 3 | `70.4` | `62.8` |

Gates at every landing:

- The Pulse lone-craft gate,
  `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`, held at
  12/12 clean, `no clean lap: []`, every circuit `lost at []`.
- `pure_race_ground_truth`, `pure_original_ai_ground_truth` and the Pure replay test passed.
- `test-data-affected` passed (FULL selection), and no golden or determinism reference moved.

### Bank-to-yaw on (2026-10-21, `deviations-to-original`)

The maintainer (2026-10-20): "Original shall always be the default." So the gain is `50`, the
measured value, and the Ace walls the noise bar found are this engine's driver's to fix. Unperturbed,
eight seeds, `opponent_autopilot_sweep::races_others` (Autopilot off is the field alone; on, the
player's slot is flown by the race's Autopilot too):

| State, level | Race walls | Wrecks | Laps | Per-lap shield | End-of-run shield |
| --- | ---: | ---: | --- | ---: | ---: |
| `30`, novice | `690` | 1 | `52.9`-`54.2` s | `92.0` | `87.8` |
| `50`, novice | `780` | 0 | `53.6`-`54.7` s | `87.4` | `84.2` |
| `30`, Ace | `1899` | 1 | `43.4`-`43.9` s | `68.6` | `60.4` |
| `50`, Ace | `2058` | 3 | `43.4`-`44.3` s | `68.5` | `57.4` |

The Pulse lone-craft gate (`race_ground_truth`) stays twelve of twelve clean, `no clean lap: []`,
and HD's twelve `hd_ai_craft_laws_ground_truth::lone_opponent_on_*` pass; neither reads Pure's
table. `pure_race_ground_truth`, `pure_original_ai_ground_truth` and the Pure replay test pass.

### The first pass (4 seeds, one run a state)

Lane `pure-physics` judged each law against one run of four seeds. That held them all. The
noise bar above superseded this table:

| State | Sweep walls novice A/B | Sweep walls Ace A/B | Race walls off, novice / Ace | Ace wrecks (off) | Lap novice / Ace |
| --- | --- | --- | --- | --- | --- |
| Base (Pulse's laws on Pure) | 31 / 0 | 38 / 18 | 269 / 1104 | 0 | 49.8 / 41 s |
| + no doubling, drag `-0.0025` | 23 / 25 | 14 / 25 | 399 / 731 | 0 | 54.5 / 44 s |
| + bank-to-yaw `50` | 56 / 28 | 42 / 265 | 422 / 758 | 4 | 54 / 44 s |
| + pitch `0.1`/`0.4`/`1.5` | 58 / 78 | 53 / 72 | 459 / 829 | 2 | 54 / 44 s |
| + sideshift `200` | 23 / 25 | 13 / 32 | 399 / 855 | 0 | 54.5 / 44 s |
| + launch, every craft (not kept) | 8 / 50 | 118 / 44 | 311 / 958 | 1 | 54 / 44.5 s |
| + launch, player only | 73 / 17 | 24 / 24 | 397 / 778 | 4 | 54 / 44.5 s |
| Base + sideshift `200` alone | 31 / 0 | 51 / 14 | 269 / 1650 | 1 | 49.8 / 41 s |

### The Pure pad replay test

`replay_ground_truth::pure_replays_reproduce_catch_a_tampered_input_and_ghosts_change_nothing`
held the speed pair in the first pass. Its pad was a bare `oag_ai::Driver` with default tuning,
on the racing line alone: no speed plan, no field, no hull yaw ceiling. That pad hits the wall at
the end of Vineta K's start straight and stays pinned there, on Pulse's laws and on Pure's alike.
On Pulse's laws it crawls at about `10` u/s and still laps inside 7,200 ticks; on Pure's it
crawls at about `5` and does not.

Pure's pad is now the race's own driver for the player's craft (`Race::autopilot_wants`, as
`--net-bot` flies). It still records real stick inputs, and its best lap is `2206` ticks on the
held laws and `2234`-`2250` with the laws on. The tampered stick is still caught at tick `420`.
Pulse, HD and 2048 keep the bare pad. The bare driver's pin is open for the driver lane.

## The inertia frame is Pulse's world axes (2026-10-21, `deviations-to-original`)

Pure's `Body_Integrate` (`0x089345e8`) against Pulse's (`0x0884e230`), read off both
disassemblies: the same 87-instruction loop. It loads the body's inertia block (`body+0x40`) into
a matrix (Pulse `E700`, Pure `E600`), forms `M200 * (I * basis)`, stores it to `body+0x80`, and
takes the angular rate from that product times the momentum (`vtfm3.t C310,E600,C510`, Pulse
`0x0884e39c`, Pure `0x08934774`). The differences are the register bank, Pure's body offsets
`0x40` higher from the velocity on (`+0x180`/`+0x190`/`+0x1a0` against `+0x140`/`+0x150`/`+0x160`),
Pulse's step counter (`0x08ab0e98`, which Pure lacks), and two cosmetic stores (Pulse zeroes the
`w` of `body+0x90` and the translation row of the inverse basis at `body+0xc0`; Pure leaves them).
None touches the rate. So Pure's tensor frame is whatever Pulse's is, and Pulse's is world-axis,
measured ([airborne-ground-truth.md](airborne-ground-truth.md#the-tensor-is-applied-in-world-axes-measured-dynamically),
confidence 90). **Confidence 85** for Pure: a full instruction match on Pure's own binary to a law
measured on Pulse's; no Pure capture separates the frames (a flat-ground run cannot: the `(12, 8,
12)` diagonal commutes with a yaw).

**On** (`CraftLaws::inertia_rotates` `measured(false)`); it was `chosen(true)`, HD's rotated frame,
because the world-axis frame once raised Pure's opponent stint walls `69 -> 541`. On today's tree,
with every other Pure law on, one unperturbed run (eight seeds) shows no such rise. This table is
one draw; the noise bar on the combined state is [below](#the-noise-bar-on-the-combined-state-2026-10-21-ai-rejudge-after-switches):

| Metric | Rotated (before) | World-axis (after) |
| --- | ---: | ---: |
| Race walls, novice / Ace (`races_others`, field alone) | `780` / `2058` | `767` / `1860` |
| Wrecks, novice / Ace | `0` / `3` | `0` / `0` (Ace one respawn) |
| Per-lap shield, novice / Ace | `87.4` / `68.5` | `92.2` / `73.4` |
| End-of-run shield, novice / Ace | `84.2` / `57.4` | `90.4` / `66.2` |
| Stint walls A (our driver), novice / Ace (`opponent_autopilot_sweep::pure`) | `70` / `74` | `40` / `113` |
| Stint walls B (the original's driver), novice / Ace | `5` / `6` | `1` / `0` |

The Pulse lone-craft gate stays twelve of twelve clean and HD's twelve lone opponents pass.

## The noise bar on the combined state (2026-10-21, `ai-rejudge-after-switches`)

Bank-to-yaw `50` and the world-axis frame together, judged the way [the noise bar](#the-noise-bar)
judged each law: twelve samples a side, the same twelve airborne pitch torques (`0`, `+-1e-5`,
`+-2e-5`, `5e-5`, `+-1e-4`, `+-3e-4`, `+-1e-3`, the scratch `OAG_X_EPS` knob, never committed),
eight seeds a level, `opponent_autopilot_sweep::races_others`. **Base** is today's tree with both
switches undone by a scratch patch (gain `30`, HD's rotated frame); **law** is today's tree. Both
carry every other Pure law and the verified fallback plan
([ai.md](../gameplay/ai.md#the-fallback-plan-is-verified-2026-10-21)). Field alone (Autopilot off):

| Metric | Novice, median base -> law | Ace, median base -> law | Worst tail |
| --- | --- | --- | --- |
| Race walls | `780 -> 772` (p `0.56`) | `1719 -> 1774` (p `0.38`) | Ace `k = 1` (`P 0.5`): one sample at `5567`, two seeds grinding (`2175`, `1617`); the base's worst is `3065`, one seed at `1099` |
| Wrecks | `0 -> 0` | `2 -> 1.5` | none |
| Laps | `53.9 -> 54.2` s (p `0.034`) | `43.8 -> 44.0` s (p `0.050`) | novice `k = 5` (`P 0.019`), Ace `k = 4` (`P 0.047`) |
| Per-lap shield (mean pool at each lap line) | `90.2 -> 89.5` (p `0.33`) | `71.4 -> 72.7` (better) | none |
| End-of-run shield | `87.2 -> 87.2` (p `0.44`) | `61.7 -> 64.4` (better) | novice `k = 1` |

With the opponents flying their Autopilot pickups (`Race::set_opponent_autopilot`, the sweep's "on" half) the picture is the same: race walls novice
`757 -> 766`, Ace `1748 -> 1803` (p `0.10`, `k = 1`); laps `+0.2`-`0.3` s (p `0.025`-`0.028`,
`k = 5`); end-of-run shield novice `87.0 -> 86.8`, Ace `59.5 -> 63.5`.

Pure's AI gates over the same twelve torques: `pure_race_ground_truth`, `pure_original_ai_ground_truth`
and the Pure replay test, 17 tests, failed none in any base sample and one in two law samples
(`pure_original_ai_ground_truth::the_field_finishes_chenghou_project`, one opponent destroyed, at
`2e-5` and `-3e-4`): median `0` both, max `0 -> 1`. That test flies the recovered original
driver (`oag_ai::original`), not ours.

**Verdict: inside the bar.** No metric regresses at `P < 0.01` by either test. The Ace walls that
held bank-to-yaw (`k = 7`) are gone once the world-axis frame is on with it. The laps are `0.2`-`0.3` s
longer (p `0.02`-`0.05`), above the `0.01` hold line; whether that is the physics or our driver on
it was not separated. So no driver change for Pure. Left open: the Ace grinding tail (two seeds at
`eps = -1e-5`, one in the base too) and the lap split.

## Cross-title

Pure is not in the 2048/Omega lineage; the HD, Omega and 2048 tables carry each new field as
`inherited` from Pulse.
