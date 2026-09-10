# `--trace-out` still ignores `--give`/`--autopilot*`; scripts lose their analog axes in `oag-game`

Two residual findings from the sweep that closed the `--trace-out
--input-script` bug (`crates/main/headless.rs::write_trace` now honours
`--input-script` through `HeldButtons::advance`, and `--trace-out --hold
cross --press left --ticks 300` proves the fix: `throttle`/`steer` are
nonzero past `oag_race::COUNTDOWN_TICKS` and a pinning test asserts it). Both
are the same shape of bug the fixed one was - a flag accepted on the command
line, silently doing nothing, with no error - but neither is fixed here: each
needs its own review pass and its own assertion, and bundling either into the
input-script fix would have recreated the exact hazard that fix exists to
close.

## Open

### 1. `write_trace` never reads `--give`, `--autopilot`, `--autopilot-pilot` or `--autopilot-skill`

Every other headless leg wires these four:

- `run_windowless`'s `--screenshot` leg (`capture::Options`, `headless.rs`)
- `run_race`'s `--screenshot` leg (`race::CaptureOptions`, `headless.rs`)
- the windowed `--race` route (`App`, `headless.rs`)

`write_trace` reads none of them. `--trace-out out.csv --autopilot --ticks
900` writes a CSV of a craft driven by `--hold`/`--press` (or coasting, with
neither), not by the AI driver `--autopilot` names - and `--trace-out
out.csv --give rocket` never puts a weapon in the pickup slot, so a CSV
taken to verify a projectile's physics has no projectile in it. Same
failure shape as the fixed bug: no error, a plausible row count, a plausible
column set, and a run that silently measured something other than what it
was asked to measured.

Fixing it is not simply plumbing the same four fields through - `--give`
writes the pickup slot from *outside* `Race::tick` (`capture.rs`'s own doc:
"so nothing here can reach a determinism hash"), and `--autopilot` calls
`Race::set_autopilot` before the tick loop starts. `write_trace`'s loop has
neither hook today, so wiring this in is a second pass through the same
function, not a one-line addition.

### 2. `oag-game`'s script-driven captures drop a script's analog axes

`oag_trace::script::State` carries `stick_x`, `stick_y`, `airbrake_left` and
`airbrake_right` alongside `buttons` - a script can author `stick_x=0.5` or
`airbrake_left=1` directly. `HeldButtons::advance` (and, before this fix,
`capture::advance_one_tick` alone) reads only `.buttons` and re-derives the
axes through the keyboard path, the same binary -1/0/1 a real key press
gives. A script naming `stick_x=0.5` drives identically to one naming `left`
(full deflection) in every `oag-game` capture - `--screenshot` and
`--trace-out` alike.

`crates/trace/src/replay.rs` (`oag-trace run`/`drive`) does not have this
gap: it builds its `InputSnapshot` straight from `state.airbrake_left`/
`state.stick_x` etc., so the same script drives differently depending on
which binary replays it. A script written to test a partial airbrake or a
part-deflected stick is silently flattened to a full press by every
`oag-game` capture and not by `oag-trace run`.

Fixing this needs `HeldButtons` (or its caller) to bypass the keyboard
derivation for a scripted tick and set the axes directly - `HeldButtons`
exists specifically because the two capture legs must derive axes exactly
the way a keyboard would, so this is a real design question (bypass the
mapping when a script gives an exact value, keep it for `--hold`/`--press`)
rather than a one-line fix.

## Next Steps

1. Decide whether `--trace-out --autopilot`/`--give` should be refused (the
   way `--dump-audio` without `--screenshot` already is) until wired, or
   wired properly - review-pass territory, not a quick patch.
2. Wire `HeldButtons::advance`'s script branch to read `stick_x`/`stick_y`/
   `airbrake_left`/`airbrake_right` off the script's `State` when present,
   instead of only `.buttons`, and add a test that a partial-deflection
   script produces a partial `InputSnapshot` rather than a full one.
