# The barrel roll is playable; its visual is read but not drawn

A headline Pulse mechanic this project did not have. Every number it needs is
**authored data on the disc**, so nothing here required inventing a constant.
The reverse engineering is finished at confidence **90**: see
[input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-tap-history-path-is-the-barrel-roll)
and the subsection immediately after it, which supersedes that section's three
open questions. This thread is the implementation half, deliberately split off
because the RE half was its own session.

**As of 2026-09-06 the mechanic is reachable end to end.** The state
machine, the shield-gated arm, the phase ramp and all three landing-payout
consumers landed on 2026-09-05; the gesture that reaches them landed the day
after - see "Implemented" below. A real `InputSnapshot` now arms a roll,
from the d-pad or from the stick alone, and
`crates/gameplay/tests/barrel_roll.rs` proves it through
`oag_gameplay::controls::ship_controls` and `oag_physics::step`, the two
calls the race loop itself makes.

**The render search is done, later the same day, and it found a consumer.**
`entity+0x87c` is eased into `entity+0x880` and that field rotates the
ship's own display matrix about its nose, and the internal camera's up
vector with it - confidence **88**, evidence in
[input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose).
Drawing it is a job of its own and has its own thread,
[the-barrel-roll-has-a-drawing-job.md](the-barrel-roll-has-a-drawing-job.md);
**this** thread keeps only what is left on the mechanic itself.

## What the original does

**The gesture.** A three-entry tap history at `entity+0x88c`/`+0x890`/`+0x894`
records a *direction*, not a button: `1` for `LEFT` d-pad or the steering axis
crossing below `-90`, `2` for `RIGHT` or the axis crossing above `+90`. An entry
shifts the previous two down only if `entity+0x884` is below `0.6`, and `+0x884`
accumulates `dt` and is zeroed on every entry - a **0.6 s inter-tap timeout**.
The two patterns are the three-tap alternations `2,1,2` and `1,2,1`.

**The charge.** On a match the code calls `Ship_BarrelRollCost` (`0x08840770`)
and `Ship_Shield` (`0x0883e68c`), and **arms nothing unless `cost < shield`**.
The cost is `<Global><Special roll_cost>` percent of the ship's shield capacity
- the same expression `Ship_SetShield` (`0x0883e6f4`) clamps to, which is
`oag_physics::params::Dimensions::shield` here. `roll_cost="8"`, so **8%**.

**The phase.** `2,1,2` sets `entity+0x860 & 0x100`, `1,2,1` sets `& 0x80`; those
ramp `entity+0x87c` toward `+1.0` and `-1.0` at `<Special roll_speed>` = **1.5**
per second, so a full roll takes **0.667 s**. When the flags clear the phase does
**not** reverse - it continues to the nearer end, with `0.5` the split: past half
way it runs on to `1.0`, short of it, it falls back to `0.0`.

**The payout.** On the airborne-to-grounded transition, and only when
`|entity+0x87c| > 0.5`, it sets a timer `entity+0x898`, plays a cue and
increments a counter at `settings+0x238`. While `+0x898` is positive,
`craft+0x1c0 |= 0x400`. `<Special roll_turbotime>` = **0.5**, so the timer runs
half a second.

**What the flag does** - three consumers, a complete `.text` scan, all handling:

| Function | Effect while `craft+0x1c0 & 0x400` |
| --- | --- |
| `Ship_ApplyLateralGrip` (`0x08848b78`) | both grip terms, `stats+0x10`/`+0x14`, x **1.5** |
| `Ship_HoverTwoPoint` (`0x0884a658`) | a hover scalar forced from `stats+0x4` to **1.0** |
| `Ship_UpdateEngine` (`0x0884c5c8`) | enables the uncapped turbo add |

`crates/physics/src/ship.rs` already models the third as a seconds timer and
already cites `roll_turbotime` in its docs, so that arm has somewhere to land.

## Where it goes

`oag_physics::ship::ShipState`, beside `sideshift_timers` - the tap history, the
inter-tap timer, the roll phase and the payout timer are all per-craft state in
the original, and none of them belongs in the input layer. `InputSnapshot` needs
nothing new: the d-pad and the steering axis are already on it, exactly as the
sideshift's own port found. `oag-physics` is determinism-bound (`f32`, no
`mul_add`, no reassociating, `TickClock` only).

The four tunables come through `oag_tables::handling::global::Special`, which
parsed **only** `speedpad_jump` when this thread opened. Adding `roll_cost`,
`roll_speed` and `roll_turbotime` there was the first step, and its
ground-truth test asserts against the disc rather than a literal - done, see
"Implemented" below.

## Implemented, as of 2026-09-05

The state machine, the shield-gated arm, and all three landing-payout
consumers are in `oag-physics` and `oag-formats`, in three commits: the
`<Special>` decode and its ground-truth test; `crates/physics/src/
barrel_roll.rs` (the tap history, the phase ramp, `arm`); and the payout
wiring - `ShipState::roll_payout_timer`, armed by `crate::forces::evaluate`
on the airborne-to-grounded transition, driving `crate::airbrake::
lateral_grip`'s `ROLL_GRIP_MULTIPLIER`, `crate::hover`'s
`barrel_roll::rebound_override`, and `crate::engine::engine`'s turbo add.
Both crates' committed determinism hashes moved twice, isolated and
recorded in each file's own History note per the reasoning
`crates/physics/tests/determinism.rs` asks for.

## The gesture, as of 2026-09-06

Both of the original's tap sources are wired, and the mapping decision the
previous session left open was resolved by following the sideshift's own
split rather than inventing a second shape:

- **The d-pad leg is two edge fields on `ShipControls`** -
  `roll_tap_left`/`roll_tap_right` - filled in
  `oag_gameplay::controls::ship_controls` off the *pressed* mask, the same
  way the veteran airbrake taps are, so a held direction is one tap and not
  one a tick. Filled for **both** control schemes: the roll is not a
  scheme-dependent gesture, and neither scheme spends the d-pad on anything
  else.
- **The axis leg stays in `oag-physics`.** `barrel_roll::advance_gesture`
  reads `ShipControls::steer_x` against `AXIS_TAP_THRESHOLD` (`0.9`, the
  original's `+-90` on the `0..=100` internal scale), and the previous
  tick's side of that threshold is latched in the new
  `ShipState::roll_axis_zone`. A crossing is an edge and needs last tick's
  axis, which is per-craft state - the same division `shift_armed` already
  makes for the novice flick. It is the **raw** axis, not the ramped
  `ShipState::steer`, because the original reads it out of the same input
  block it reads the d-pad bits from.

**The two legs are ORed into one tap and never recorded twice.** That `or`
is the original's own and it is load-bearing here rather than incidental:
this project's input layer drives the analog axis from the d-pad as well
(`oag_input::pad::larger`), so a real d-pad press arrives as a button edge
*and* an axis crossing on the same tick. Recording both would shift the
history twice and leave a doubled direction in it that can never match an
alternation - which would have made the d-pad leg silently unreachable while
every test still passed.

`advance_gesture` advances the inter-tap timer itself and **replaced**
`forces::evaluate`'s bare `advance_tap_timer` call rather than joining it;
calling both would halve the 0.6 s timeout with nothing to catch it.

Both crates' determinism hashes moved a third time, isolated the same way
and recorded in each file's History note: the movement is entirely the one
`u8` per craft per tick that `roll_axis_zone` adds to the stream. Neither
gate's scenario arms a roll - `probe::controls` holds `steer_x` at `+-0.8`,
inside the threshold, and the race-level scenario flies no craft at all.

**This crate's own resolution, not a traced fact**: `barrel_roll::release`
is called on the airborne-to-grounded transition because that is the only
other traced event in this mechanic (the payout's own `|entity+0x87c| > 0.5`
check), not because the original's arming flags were traced clearing there.
See the module's own doc comment if a future trace contradicts it.

## Open

- ~~**Nothing gates the arm on being airborne, because nothing in the
  recovered chain does.**~~ **Closed 2026-09-06, and the answer was yes.**
  `Ship_UpdateSideshiftInput_q` branches past every tap when the contact bit
  `craft+0x1c0 & 1` is set (`0x08846bd0`) and **zeroes the three-slot tap
  history outright** (`0x08847018`-`0x08847030`), so a grounded craft cannot
  hold a partial gesture, let alone complete one, and an alternation cannot
  span a takeoff. Confidence 88; evidence in
  [input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md)'s
  "The tap history is the player's pad, and it is cleared on the ground", and
  ported as `advance_gesture`'s `contact` argument. The same read also turned
  the release event from this port's own guess into a reading (it is the
  airborne-to-grounded transition, as guessed) and found a third behaviour the
  port was missing: a completed alternation writes `0.0` to the phase whether
  or not the shield could pay for it.
- **AI craft can now arm rolls off their own cornering, and how often was
  measured rather than guessed.** `oag_ai::Driver` emits
  `ShipControls::steer_x` at full deflection and the axis leg has no
  human-only gate, unlike the novice flick's `shift_modifier`. Eight seeded
  drivers, 3,600 ticks each, against `oag-ai`'s own closed-loop fixtures on
  2026-09-06: **zero** arms on the ordinary oval, **three across the eight**
  on `oval_of(60.0, 300.0)`, the pathological corner
  `a_differential_holds_a_corner_the_steering_alone_cannot` keeps precisely
  because the craft cannot make it on the stick alone and sits on the
  steering stop for 300+ ticks. So this is not a live shield drain on every
  opponent; it is a driver missing a corner badly throwing a roll about once
  a minute, at 8% of shield. Left ungated deliberately - the original's
  gesture reads whatever is in the craft's input block, and a "human only"
  flag would be inventing a mechanism nothing was traced to - but **whether
  the original's AI reaches its gesture by this path at all is untraced**,
  and that is the open question, not the rate. Not pinned by a test: the
  number moves with any controller tuning, the same reason `closed_loop.rs`
  keeps its own bounds loose.

  **Two corrections, both 2026-09-06.** The rate above was measured on
  `oag-ai`'s synthetic ovals and understates the disc by an order of
  magnitude: on the real circuits an Ace opponent armed four to eight rolls a
  race on ten of the twelve and spent 30-54 of its 95 shield, which destroyed
  it on `07_Track`. And the untraced half is now traced, against this port:
  the original's tap leg early-outs on a null `ship+0x78` (`0x08846be0`,
  the *player's* pad block, its buttons indexed by `Options_ButtonForAction`)
  and its axis edge detector keeps its previous sample in a **single global**
  at `0x08ae4cf0`, referenced only from that one function - a per-process
  scalar cannot serve eight craft, so the original's opponents almost
  certainly never arm a roll at all. Confidence 85.

  **This project keeps them able to, deliberately.** Maintainer's ruling,
  2026-09-06: "AI may barrel roll, if they have enough shield energy." What
  makes that affordable is the grounded gate in the bullet above, which is a
  port; the AI's access to the gesture is a documented deviation.
- **`oag_physics::barrel_roll::AI_ROLL_SHIELD_FLOOR` is invented, and it is the
  first invented number this mechanic carries.** An AI craft below 20% of its
  shield pool does not arm a roll; a pad on the same shield does. The value was
  chosen by the maintainer on 2026-09-06, who said plainly that what the
  original does here is unknown, and it deliberately has **no confidence
  score** - a score would let a later reader cite a choice as evidence. It sits
  on top of the recovered `cost < shield` gate rather than replacing it, and
  `ShipControls::computer_driven` is the flag that keeps it off the human path.
  If the original's own AI gate is ever recovered, this constant is *replaced*
  by it, not reconciled with it. Changing the value is a one-line edit.

  **Measured on the disc, it fires zero times.** With the floor at `0.20` and
  at `0.00`, all twelve forward circuits report identical arms, shield spent and
  finishing shield. The grounded gate accounts for the whole of the `07_Track`
  regression on its own, so the floor is a dormant safety net rather than a live
  rule - worth knowing before anyone tunes it.
- **A completed roll used to draw two rotations, and now draws one.** Reported
  from play 2026-09-06. `release` leaves `roll_target` at `+-1.0` and the phase
  parks there, which is level on screen; the next roll the *other* way then
  travelled `+1.0 -> -1.0`, twice a turn. Fixed by the original's own phase
  levelling on any completed alternation, and pinned by
  `barrel_roll::tests::a_second_roll_the_other_way_travels_one_turn_and_not_two`,
  which asserts the **traversal** rather than the endpoint - the endpoint was
  always right, which is why no existing test could see it.
- ~~Nothing draws the roll and no render-side consumer of `entity+0x87c` has been
  searched for.~~ **Closed 2026-09-06**: there is one, the visual is a rotation
  derived from the phase and not a canned animation, and it moved to its own
  thread - [the-barrel-roll-has-a-drawing-job.md](the-barrel-roll-has-a-drawing-job.md).
  The `0x400` flag was never how it gets there, as this thread already recorded.
- `<Special turbo_jump>` = `0.1` sits in the same element with a recovered name
  (`g_turbo_jump`, `0x08b36be8`) and **no traced reader**, so what it does is
  open. It is probably not the barrel roll's, given the naming pair
  `turbo_jump`/`speedpad_jump`.
- `craft+0x2a4 == 1` gates `Ship_UpdateEngine`'s turbo add and nothing decodes
  that enum, so a rolled turbo here will fire in states where the original might
  not - a pre-existing gap, noted in `ship.rs`, that this work inherits.
- `settings+0x238`, the completed-roll counter, has no identified consumer; it
  is presumably a stat or an accolade trigger.
- Unchecked on PS2 (`SCES_547.48`). The PSP gesture uses the d-pad *or* the
  steering axis, and the PS2 pad's stick makes the axis leg behave differently.

## Next Steps

1. Confirm the gesture off the original with a capture, the way the
   sideshift's veteran double-tap was confirmed in
   `docs/reverse-engineering/ppsspp-debugger.md` - the whole chain here is
   still a static read. Both questions the capture was owed are now answered
   statically, so what it would add is the runtime leg that takes the grounded
   gate past 88 and `ship+0x78 == 0` on an opponent past 85; read `ship+0x78`
   for an AI craft and watch `craft+0x88c` on a grounded player. `scripts/psp-drive.py` can already script a d-pad alternation. **The
   same capture also settles the one thing the render read could not** - which
   way the ship rolls - so running it once serves both threads; see
   [the-barrel-roll-has-a-drawing-job.md](the-barrel-roll-has-a-drawing-job.md).
2. Read `<Special turbo_jump>`'s consumer, if one exists, and `settings+0x238`'s
   reader, and check the gesture on PS2.

## From the HANDOVER.md index (moved 2026-09-25)

`oag_physics::barrel_roll` has the tap history, the shield-gated arm (8% of shield, refused at or below cost), the self-completing phase ramp, and the landing payout wired into all three of `craft+0x1c0 & 0x400`'s handling consumers (1.5x lateral grip, the hover rebound forced to 1.0, the turbo add), all off values read live from both PSP pressings. **Reachable from a real race as of 2026-09-06**: `ShipControls` carries the d-pad tap edges, `barrel_roll::advance_gesture` reads those and the steering axis crossing `+-90` against the new `ShipState::roll_axis_zone` latch, and `crates/gameplay/tests/barrel_roll.rs` arms a roll from an `InputSnapshot` through `ship_controls` and `oag_physics::step`. **The render search landed 2026-09-06 and found a consumer** - `+0x87c` eases into `+0x880`, which rolls the ship about its nose and the internal camera's up vector with it (confidence 88); drawing it moved to its own thread. **The airborne gate is recovered and ported 2026-09-06** (confidence 88): `Ship_UpdateSideshiftInput_q` branches past every tap when the contact bit `craft+0x1c0 & 1` is set (`0x08846bd0`) and zeroes the three-slot tap history outright (`0x08847018`-`0x08847030`), so a grounded craft cannot hold a partial gesture and an alternation cannot span a takeoff - which closed the `07_Track` regression, taking an Ace opponent from 4-8 arms and 30-54 shield a race to **zero arms on all twelve circuits**, and took `14`/`09`/`10` from 3.9/9.5/10.8 shield left to 41.4/59.2/57.0. The same read made the release event a reading rather than a guess (it is the landing, as guessed) and added the phase levelling on any completed alternation. **The original's AI almost certainly never rolls at all** (confidence 85): the tap leg early-outs on a null `ship+0x78` - the player's pad, its buttons indexed by `Options_ButtonForAction` - the axis edge detector's previous sample is a **single global** at `0x08ae4cf0` referenced from that one function, and a complete scan for arm-bit stores finds only the two in that block plus a network/replay applier. This port lets every craft reach the gesture anyway, on a maintainer's ruling and as a documented deviation. **`AI_ROLL_SHIELD_FLOOR` retired as a bare constant, `cfb196f5`, 2026-09-06 later the same day**: opponents now barrel-roll on purpose, behind a per-personality `roll_floor` (`oag_ai::Pilot`, `Span` 0.30-0.75) reached via `ShipControls::roll_shield_floor`, not a single flat AI-only floor - a materially more deliberate mechanism than the flat rate/floor this thread's own prose still describes. Whether the "dormant, fires zero times" measurement still holds at the new, higher floor values has not been re-verified anywhere. Also fixed 2026-09-06, reported from play: a second roll the other way drew **two** rotations, because the phase parks at `+-1.0` and the next roll travelled 2.0 of it - the original's phase levelling on any completed alternation is the fix, pinned on the traversal rather than the endpoint. Still open: no capture, so the whole chain is a static read; `<Special turbo_jump>` has a name and no reader; PS2 unchecked

**2026-10-03:** the landing payout is once per arm - `ShipState::roll_armed` gates `release`, as the original's arm bits do. A roll used to pay on every later landing (maintainer report). Still open: the live leg (watch `+0x87c` and `craft+0x1c0 & 0x400` across a bump after a roll).

**2026-10-07:** one roll per flight, as the original. An armed craft (`ShipState::roll_armed`) skips the pattern match, the charge and the phase levelling until the next ground/air transition, so a roll cannot be retriggered mid-roll or after it completes in the same flight (maintainer report). Measured live on PPSSPP (confidence 95), see input-bindings.md "One roll per flight". Still open from the capture: what `settings+0x23c` counts (it did not move on a second arm). Not checked on HD/2048/Omega, and not planned: the maintainer is confident the rule is the same there.
