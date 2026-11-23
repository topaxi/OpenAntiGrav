---
status: ready-to-wire
titles: [hd, omega]
updated: 2026-11-23
summary: "Holding L1 (flip or EMP) zeroes the L2 airbrake; proposed fix is a flip flag in oag-input pad resolve, unbuilt"
categories: [tooling]
size: M (50)
difficulty: easy (30)
---

# L1 as flip or EMP zeroes the left airbrake the player pulls on L2

Review finding G4 (`docs/reviews/2026-11-23-gameplay-review.md`). `InputSnapshot::shoulder_flips`
(`crates/gameplay/src/input.rs`) clears `Button::L` from all four masks and zeroes `airbrake_left`
whenever `Button::Flip` is held, pressed or released. `oag_input::pad::bits_of` marks the L1 shoulder
as `L | Flip`, and `resolve` folds L1 and the L2 trigger into one `Button::L` bit and one
`airbrake_left` axis, so by the time the snapshot exists the two cannot be told apart.
HD binds the flip to L1 and the airbrakes to L2 and R2 as separate inputs
(`docs/ghidra/functions/ps3-hdfury-eu/eliminator-flip.md`).

## Open

The review's suggested fix (a snapshot carrying both shoulders) was not built: it widens
`oag-gameplay`'s `InputSnapshot`, which the replay (`oag-replay`) and the wire (`oag-net`'s
`wire.rs`) both serialise, and the edge masks in `oag-input`'s merge, for a problem that
exists in one place only.

Design to try first, one crate plus its caller:

1. `oag_input::pad::resolve` gains a `TriggerConfig` flag, `l1_is_flip`, set by the composition
   root each tick from the race (`RaceSim::flip_law.is_some()` or an armed Detonator EMP, the
   same test `player_step.rs` uses for `shoulder_flips`).
2. With the flag set, `bits_of` raises `Flip` for `LeftTrigger` and not `L`, and `resolve`'s
   `shoulder(Button::L)` term drops out of `airbrake_left`. L2 keeps raising `L` and the
   conditioned `airbrake_left` through the trigger loop unchanged.
3. `shoulder_flips` then has nothing to clear on a pad. It stays for the keyboard and touch
   paths, which raise `L | Flip` from one key; decide whether those want the same flag.
4. Test: a `Reading` with L1 held and L2 at 0.6 under `TriggerMode::Airbrakes` gives
   `Flip` held, `airbrake_left` 0.6 and `L` held from the trigger alone, with the flag set; the
   flag clear gives today's answer.

Check before building: `Controls`'s edge computation merges devices' `held` before computing
pressed/released, so `L`'s edges come from the trigger alone once L1 stops raising it;
confirm with a test that an L1 press while L2 is held does not create an `L` edge.

Recorded replays hold the raw snapshot, which still has `L` and the axis set for an L1 press, so
`shoulder_flips` must keep running in the sim for them. Pre-release, so no replay or wire version
bump either way.

## Next Steps

1. Add the `l1_is_flip` flag and the `resolve` change with the test above (`crates/input/src/pad.rs`).
2. Set it from the race in `oag-game` where the pad config is built.
3. Decide the keyboard and touch cases.
