# `--trace-out` silently ignores `--input-script`

`cargo run -p oag-game -- --race --trace-out out.csv --input-script
verification/scenarios/<any>.inputs --ticks N` writes a CSV that never saw
the script: `crates/game/src/main/headless.rs::write_trace` drives its own
tick loop straight off `cli.hold`/`cli.press` via `held.pulse(...)` and never
reads `cli.input_script` at all. Confirmed live on
`verification/scenarios/steer-left.inputs`: every row of the resulting CSV
has `throttle == 0` and `steer == 0`, for the full 200 ticks, even though the
script is "200 cross left" from tick 0.

This is the same *class* of bug as `--log-every` being unwired for the front
end (already known and documented in the CLI's own flag help): a
verification flag that looks like it drives the scripted run because a
sibling flag (`--screenshot --input-script`) does, but silently doesn't, and
produces a CSV of a craft sitting still rather than an error. Nothing in the
current output says the script was ignored - the trace writes cleanly, with
a plausible-looking row count and column set.

The two code paths that *do* honour `--input-script` are
`crates/game/src/race/capture.rs::advance_one_tick` (behind `--screenshot`)
and `oag-trace drive`/`run` (`crates/trace`, a different binary entirely,
`just scripted-sim`). Routed verification work around `write_trace` via
`oag-trace drive`/`run` instead of fixing this - see
`docs/formats/pob.md`'s "screenshot-verified" paragraph in "The authored
emitter-node frame lives outside the `.pob`" for what that produced. Found
while chasing a banked wall-hit frame for that thread's own verification;
not fixed here - out of that thread's lane (`crates/game/src/main/headless.rs`
isn't render code) and a CLI-plumbing fix wants its own review pass rather
than riding in on a docs-only change.

## Open

- `write_trace` (`crates/game/src/main/headless.rs`, the `--trace-out`
  handler) needs the same `input_script` wiring
  `race::CaptureOptions`/`advance_one_tick` already have: read
  `cli.input_script`, parse it with `oag_trace::script::Script::parse` the
  same way `run_race`'s `--screenshot` branch does a few lines later in the
  same file, and drive `held.set_held(script.at(tick).buttons)` per tick
  instead of `held.pulse(...)` when a script is given.
- Once fixed, `--trace-out --input-script` becomes a second, non-rendering
  way to get a scripted run's full per-tick basis (including `up_x/y/z`),
  which is more convenient than `oag-trace drive`/`run` for anyone working
  inside `oag-game` rather than against a captured reference trace - no
  `--source`/`--track`/`--team`/`--class` to keep in sync with `oag-game`'s
  own defaults, since it's the same load path.
- Whether an error (refuse `--trace-out --input-script` until wired, the way
  `--dump-audio` without `--screenshot` is refused today) is better than a
  silent no-op in the meantime is worth deciding at fix time - a refusal
  would have caught this immediately instead of costing a debugging pass.

## Next Steps

1. Add `input_script: Option<oag_trace::script::Script>` handling to
   `write_trace`'s tick loop, mirroring `advance_one_tick`'s
   `held.set_held(script.at(tick as usize).buttons)` branch.
2. Add a regression test: a `--trace-out` run with `--input-script
   verification/scenarios/steer-left.inputs` should show nonzero
   `throttle`/`steer` in its early rows, the way the script says it should -
   this bug would have failed that test on day one.
3. Decide the refuse-vs-silently-fix question above before landing.
