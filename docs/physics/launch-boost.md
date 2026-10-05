# The launch boost: a thrust multiplier graded by when thrust first lands

**Status: ported and measured against five live launches.** Pulse PSP (USA) in
PPSSPP 1.20.4, 2026-10-01, Time Trial, Venom, Assegai, `16_Track`; the held-through
launch also on `01_Track`. Code: `oag_physics::launch`, parameters from
`oag_tables::handling::StartBoost`, wired in `oag_raceplay` through
`Environment::start_boost`. Test: `crates/game/tests/launch_boost_ground_truth.rs`.

[grid-state.md](grid-state.md) measured that a craft whose thrust is held through the
countdown accelerates faster than ours did, named `craft+0x294` as the cause and left
open which of the three XML multipliers the reaction time selects. **None of them
is selected by a reaction time.** A grade is written from *when the thrust first
lands*, and the multiplier follows the grade.

## The law

Three functions, run in this order each frame from the one the craft enters state 1
on (the frame before the throttle word steps):

```text
Ship_UpdateEngine (0x0884c5c8, head)         T = T * craft+0x294 * 2.0     // LAST frame's multiplier
                                             if craft+0x2d4 == 0 and control.thrust != 0:
                                                 craft+0x2d4 = 1; craft+0x2d5 = 1   // first-thrust edge
                                             else: craft+0x2d5 = 0
Race_UpdateLaunchGrade (0x0882773c)          // racing frames only; zeroes the clock while counting down
                                             c = race_manager+0x2bc               // seconds since GO, 0 on the first racing frame
                                             if craft+0x2d5:
                                                 c <  windowStart -> grade 1 (stall)
                                                 c <  windowEnd   -> grade 2 (perfect)   [grade 3 for non-human craft, below]
                                                 c <  stallEnd    -> grade 1 (stall)
                                                 c <  overall     -> grade 0 (normal)
                                             c += dt
Ship_UpdateStartBoost (0x0883fdec)           t = player+0x888                     // seconds since state 1, reset to 0 while state != 1
                                             if t < overall + 0.5:
                                                 craft+0x294 = t < overall ? [normalMul, stallMul, boostMul][grade] : 1.0
                                             t += dt
```

The grade starts at 0, so **the frame thrust first lands is still multiplied by
`normalMul`**: the engine ran before the grader. That is the "1.4 for the frame
before GO" the first measurement recorded. The windows and multipliers are the
seven attributes of `<Handling><Global><StartBoost>` in `Data\XML\HandlingStats.xml`
(`Xml_ReadBoostSettings`, `0x088390b4`); the values are read from the player's disc and
not reproduced here.

## Measured

`scripts/psp-launch-boost.py` logs, once per frame from `RESTART RACE`, the craft's
multiplier (`+0x294`), edge (`+0x2d5`), latch (`+0x2d4`), the engine's output
(`+0x328`), the manager's clock, the player's timer (`+0x888`) and grade (`+0x36c`)
and the body. Rows are counted from the first state-1 row (`k = 0`); the thrust edge
is the row `+0x2d5` reads `1`.

| Thrust first lands | Edge row | Grade | Multiplier the engine reads | Thrust (`+0x328`) at the edge, next |
| --- | --- | --- | --- | --- |
| held through the countdown | `k = 1` | 1 stall | normal on the edge frame, then stall to `k = 60` | `47.6`, `41.7` |
| 11 frames after release | `k = 12` | 2 perfect | normal on the edge frame, then boost to `k = 60` | `47.6`, `59.1` |
| 25 frames after release | `k = 26` | 1 stall (the window after the perfect one) | normal, then stall | `47.6`, `41.7` |
| 31 frames after release | `k = 32` | 0 normal | normal throughout | `47.6`, `48.7` |
| 81 frames after release | `k = 82` | 0 (no write: the window is over) | `1.0` | `34.0`, `34.6` |

The engine's output on the first thrust frame, `47.6` against `34.0`, is `1.4 x`
the plain figure in every row that has a window left. In every row the multiplier
reads `1.0` from the engine's read at `k = 61`: **sixty frames of boost**, which is the
window `overallDuration` at the capture's `1/59.94` step.

Forward speed against ours, after the port (`launch_boost_ground_truth`, one
percent tolerance, worst `0.65 %`), frames after the edge:

| | 20 | 30 | 60 | 90 / 120 |
| --- | --- | --- | --- | --- |
| held through, original | `16.723` | `26.978` | `65.782` | `90.713` (90) |
| held through, ours | `16.717` | `27.017` | `66.010` | `91.183` (90) |
| perfect window, original | `25.586` | `43.318` | `88.398` | `115.056` (120) |
| perfect window, ours | `25.581` | `43.350` | `88.698` | `115.798` (120) |

Held through on both circuits, speed and distance from the first thrust frame at 120
frames: `16_Track` `106.637` units/s and `118.449` against ours `107.287` and
`118.812`; `01_Track` `106.450` and `118.376` against `107.114` and `118.785`. The 18
units behind that [grid-state.md](grid-state.md) recorded at 120 frames is now 0.4.
Without the boost the first comparison is 21 % off at 20 frames, which is the test's
falsifier.

## What is ported, and what is chosen

- The edge, the grader (grades 0 to 2) and the window writer, for every craft: the
  player and the field alike. **AI crafts obey the player's law** (chosen, not
  measured): an AI that thrusts at GO is graded stall, as a human who held the
  button is.
- The original's **grade 3 is not ported.** `Race_UpdateLaunchGrade` writes grade 3
  to any craft whose player record is not the local human (`player+0x368 != 0`) on
  every frame of the perfect window, with no thrust edge needed, and the multiplier is
  `boostMul` times a per-grid-slot figure at
  `race_manager + class * 0x10c + player+0x914 * 4 + 0x320`: the AI stat block's
  `StartBoost[8]` ([ai-stats.md](../ghidra/functions/psp-pulse-usa/ai-stats.md)).
  Read from the decompile, **not watched**, confidence 80. That is the original's AI
  getting a launch boost the player cannot earn without a thrust edge in the window,
  and the maintainer's rule is that the AI obeys the player's physics.
- **Applied on Pulse PSP only** (`is_pulse_psp`: the USA and EU discs), where it was
  measured (USA). Pulse PS2's and Wipeout HD's `HandlingStats.xml` author the same
  `<StartBoost>` and the loader reads it, but neither was watched, so the loader drops
  it for them and says so in its report; `only_pulse_psp_gets_the_launch_boost` pins
  this. Pure authors none.
- The craft enters state 1 a tick before thrust is released, so
  `ShipState::on_grid` falls one tick earlier than it did
  (`thrust_gated(tick + 1)`); the yaw coupling gains the same tick and its first-30-tick
  drift is `0.1027` degrees against the original's `0.102`.
- **Zone is left at `1.0`.** Its four-corner branch reaches the same multiply in the
  original (the shared tail, `0x0884c918`), so a Zone launch is graded too: a coasting
  craft `normalMul`, one holding accelerate `stallMul`. That was not watched (Zone is
  greyed on a fresh profile), and applying it makes the speed 28 ticks after GO depend on
  whether accelerate was held (127.6 against 114.9 km/h on `16_Track`), which
  `zone_ground_truth::zone_speed_is_automatic_and_exhaust_intensity_now_follows_it_too`
  asserts it does not. The assertion is itself a decompile-based expectation, not a
  measurement, so this is a finding for whoever measures Zone: the clock already starts
  at the right tick in Zone (`ShipState::released`), and the multiply is one line in
  `engine::engine`.

## A respawn does not replay the window

The decompile reads as if a craft re-entering state 1 after a respawn (state 3) would
run the window again with the persisted grade: the timer is zeroed whenever
`craft+0x2a4 != 1`. **Watched not to happen** (`--drop-at`: the craft is moved 400 units
down at `k = 100`, respawns through state 3 for two frames and is back in state 1):
`player+0x888` holds `1.702` through it and the multiplier stays `1.0`.
`Ship_UpdateStartBoost` is evidently not called in state 3, so the reset branch never
runs. The port runs the window once, at the start: `Ship::place_at`, which every respawn goes
through, rebuilds the physics state from its default and **carries `launch` across**
(`a_respawn_does_not_replay_the_launch_window`; without the carry a released craft with
an idle state opens a fresh window at `normalMul` after every respawn).

## The three other state-0 terms, tried and left out

[grid-state.md](grid-state.md) listed three more things the grid state does: roll
damping `-5` instead of `-2`, the `rebound` base `1.0`, and the control record's
airbrakes forced to full with its brake, steering and thrust zeroed. All three were
ported experimentally behind `on_grid` on top of the boost and measured against the
original's 270 countdown frames and its launch:

- **Forced airbrakes (and the brake held at zero, which is what release zeroing is for
  a craft at rest):** the position at GO and every launch speed in the tables above are
  unchanged to three decimals. There is no lateral velocity for the airbrake terms to
  act on (`imbalance` is zero with both held, `slide` needs steering) and the brake is
  zero. The 24 % loss in the earlier trial was the brake ramping up on the grid and not
  being zeroed at release.
- **Roll damping and `rebound` base:** the position at GO moves by `0.002` to `0.004`
  units either way (x `6.067` to `6.071` against the original's `6.081`, z `-195.842` to
  `-195.844` against `-195.942`), indistinguishable from nothing; the settle's series
  cannot be compared because the original places the craft two units above the floor
  and ours at rest height.

None pays, so none is ported. The 0.1 units in z that remain at GO are the placement
geometry [grid-state.md](grid-state.md) already named, not these terms.

## Open

- ~~`FUN_08904fd4`, the perfect-start effect's trigger~~ **Read and wired
  2026-10-04**: `ExhaustFlare_OnPerfectStart` arms the engine flare's boost timer
  to `0.8` (the pad's) and plays `TURBO` from `weapons.bnk`, on the human's grade-2
  edge only; `race::perfect_start`, `Cue::Turbo`. No `.POB`. See
  [perfect-start.md](../ghidra/functions/psp-pulse-usa/perfect-start.md).
- **The AI's grade 3 is confirmed from both ends** (2026-10-04, 85, static): the
  grader writes it with no thrust edge and `Ship_UpdateStartBoost` multiplies
  `boostMul` by the per-slot `StartBoost[8]`, which on Venom falls from the front
  of the grid to the back. Not ported, by the maintainer's rule. **Tried and
  reverted the same day**: the field timing its first thrust into the perfect
  window (earning grade 2 the player's way, chosen) put eight
  `ai_dekonstruct_black_ground_truth` seeds one or two destroyed craft over their
  bounds, made `stall_rescue_ground_truth`'s healthy-craft check see one stalled
  tick (the held opponent) and moved `leach_energy_ground_truth`'s scenario. **The
  cause was not isolated**: the boost itself, the hold of about six ticks after the
  release, and the field's changed bunching off the line are all candidates. The
  bounds were not loosened.
- The AI's grade 3 and its per-slot table have not been watched. Reading `craft+0x294`
  of the seven opponents in a Single Race through the first second would confirm the
  decompile and give the original's AI start for the AI lane to compare against.
- The disc's four times all sit exactly on tick boundaries at 60 Hz, so
  `launch.ticks * dt` is used rather than a running sum of
  `dt` (the sum of sixty `1/60` in `f32` boosts a sixty-first tick, which a test pins).

## Zone

Watched 2026-10-02 on a Zone engine (`g_game_mode` patched to 6, see
[zone-start.md](../ghidra/functions/psp-pulse-usa/zone-start.md)): the auto-speed goes through `craft+0x294` like the
throttle. Coasting (no thrust edge, grade 0) reads `1.4` for 60 frames and thrust `34.0 * 1.4 * 2 = 95.2`; accelerate
held reads grade 1 and `1.2`. Speed 28 frames after state 1 is `42.11` against `36.34` units/s (ratio 1.159; ours
1.154). Ported, Zone included, so a Zone craft's first second depends on whether accelerate was held.
