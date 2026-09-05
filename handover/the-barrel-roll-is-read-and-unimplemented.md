# The barrel roll's mechanics are implemented; nothing feeds it a tap yet

A headline Pulse mechanic this project did not have. Every number it needs is
**authored data on the disc**, so nothing here required inventing a constant.
The reverse engineering is finished at confidence **90**: see
[input-bindings.md](../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-tap-history-path-is-the-barrel-roll)
and the subsection immediately after it, which supersedes that section's three
open questions. This thread is the implementation half, deliberately split off
because the RE half was its own session.

**As of 2026-09-05, the state machine, the shield-gated arm, the phase ramp
and all three landing-payout consumers are implemented and unit-tested in
`oag-physics`** - see "Implemented" below. **What is not done is wiring a
real tap into it**: `ShipControls` has no `LEFT`/`RIGHT` event, so nothing in
a live race can reach `barrel_roll::record_tap` or `arm` yet. That is the
thread's remaining next step.

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

The four tunables come through `oag_formats::handling::global::Special`, which
today parses **only** `speedpad_jump` and says so in its own doc comment. Adding
`roll_cost`, `roll_speed` and `roll_turbotime` there is the first step, and its
ground-truth test asserts against the disc rather than a literal.

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

**Not reachable in a live race.** Nothing produces a tap event yet:
`ShipControls` carries no `LEFT`/`RIGHT` d-pad press or steering-axis-crossing
field, so `crate::forces::evaluate` never calls `barrel_roll::record_tap` or
`arm`. Every unit test drives `ShipState` directly; the one integration test
(`a_completed_roll_arms_the_payout_and_the_payout_drives_both_consumers`)
still starts from `arm` rather than from an input snapshot. Wiring the real
gesture is what remains - see Next Steps below.

**This crate's own resolution, not a traced fact**: `barrel_roll::release`
is called on the airborne-to-grounded transition because that is the only
other traced event in this mechanic (the payout's own `|entity+0x87c| > 0.5`
check), not because the original's arming flags were traced clearing there.
See the module's own doc comment if a future trace contradicts it.

## Open

- **The input mapping is the next real gap.** `ShipControls` needs a way to
  carry the `LEFT`/`RIGHT` tap event - the d-pad bit or the steering axis
  crossing `+/-90` - so `crate::forces::evaluate` (or `oag_gameplay::
  controls`) can call `barrel_roll::record_tap` and `arm`. The thread's own
  RE names both sources but not how `oag_gameplay`'s existing button/axis
  layer should expose them; that mapping decision is left to whoever takes
  this, rather than guessed here.
- Nothing draws the roll. **No render-side consumer of `entity+0x87c` has been
  searched for**, so whether the visual is an angle derived from the phase or a
  canned animation is unknown. The `0x400` flag is *not* how it gets there -
  that was checked, and all three consumers are handling code.
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

1. Give `ShipControls` a `LEFT`/`RIGHT` tap event (or expose the steering-axis
   crossing another way) and wire it through `crate::forces::evaluate` into
   `barrel_roll::advance_tap_timer`/`record_tap`/`arm`, so a real input
   snapshot can actually arm a roll - closing the reachability gap this
   thread's implementation half left open.
2. Once reachable, search for a render-side consumer of `entity+0x87c` (or
   decide there is none and the roll is handling-only) before drawing
   anything - per this project's "parse it and play it, or draw nothing"
   rule.
3. Read `<Special turbo_jump>`'s consumer, if one exists, and `settings+0x238`'s
   reader, and check the gesture on PS2.
4. Expect the committed state hash in `crates/physics/tests/determinism.rs`
   and `crates/gameplay/tests/determinism.rs` to move again once a race can
   actually reach `barrel_roll::arm`. **Explain the movement; never edit the
   constants to make the test pass.**
