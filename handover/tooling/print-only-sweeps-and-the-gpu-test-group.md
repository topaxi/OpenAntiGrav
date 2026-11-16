---
status: ready-to-wire
titles: [all]
updated: 2026-11-16
summary: "Six print-only probes run outside OAG_SWEEP (about 3.3 CPU-s); the gpu-vram cap costs no measured wall clock; two mesh modules are stale in the group"
size: S (25)
difficulty: easy (25)
---

# Print-only sweeps and the `gpu-vram` test group (2026-11-16)

A read-only scout of two questions from the test-optimisation sweep: which tests
only print and skip the `OAG_SWEEP` gate, and whether the `gpu-vram` group
throttles tests that never open a GPU. Both are small. The cap's wall-clock cost
is already measured in `docs/architecture/workspace-layout.md`, "Test groups",
and this scout's own model does not overturn it.

## Measured

- Print-only tests, outside `OAG_SWEEP`: six probes were read, and three of them
  (`ps2_effects`, `limiter_cost`, `grid_layout_survey`) are ignored and run in
  `just test-data`. Each is listed under Findings with its runtime.
- `#[test]` bodies in `crates/*`: 12,292 by a brace-matching census; 96 gated by
  `OAG_SWEEP` in the body; 2,675 `#[ignore]`d. The census counts `.unwrap()` as an
  assert, so each print-only candidate was read by hand.
- `gpu-vram` group: 63 modules, and `scripts/check-test-groups.py` passes.
  Against the lead's full `test-data --full` run of 2026-11-13 (12,372 tests),
  its 259 group tests take 982 CPU-s. 170 of them reach a spawn or a capture
  marker (905 CPU-s); 74 do not (60 CPU-s); 15 cross a file boundary (17 CPU-s).
- Wall clock of the cap, as already measured in `workspace-layout.md`, "Test
  groups": the same tree with no group ran 981 s, and with `gpu-vram` at 4 it ran
  959 s (load swinging 13 to 77, so "no cost seen"). A replay of per-test walls
  through a 16-slot scheduler gives 948 s without the cap and 947 s at 4; at 24
  slots, 640 s without and 639 s at 4; at 32 slots, 486 s and 485 s.
- This scout's own replay of the Nov 13 log gives +44 s at 16 slots (527 s capped
  against 483 s). It is order-sensitive: longest-first ordering gives 461 s both
  ways. The log's PASS lines are in completion order, not start order, and the
  model ignores nextest's priority-50 rule for ground-truth tests. The documented
  replay is the better number. Treat the +44 s as an unmeasured upper bound.

## Findings

Ranked by player-visible win per unit of work. None touches the simulation.

1. **Two stale group modules.** `crates/mesh/tests/hd_emissive_lit_path.rs` (1
   test) and `crates/mesh/tests/hd_pad_ne_lit_path.rs` (4 tests) are in the
   `gpu-vram` override of `.config/nextest.toml`. Their device comes from
   `oag_gpu::test_adapter::request` (`hd_emissive_lit_path.rs:207-213`,
   `hd_pad_ne_lit_path.rs:121-127`). Under the group's own policy, a test that
   picks its adapter through `test_adapter` is not in the group. The only
   marker in either file is the word `capture_pixels_from` in a `//!` doc
   comment (`hd_emissive_lit_path.rs:9`, `hd_pad_ne_lit_path.rs:5`). The
   checker matches raw text, comments included, so it passes. Change: drop both
   modules from the group, and make `check-test-groups.py` strip `//` comments
   before it matches. Saving: not measured, and at most the cap's wait for five
   tests. Risk: none to the simulation; a real spawn or capture in these files
   would still need the group, and the checker would demand it.

2. **Mixed modules hold every test in the group.** A group filter names a module,
   so all its tests take a GPU slot. The modules below have tests that never
   reach a spawn or capture marker, and none of their files contains
   `request_device`, `test_adapter` or `Instance::`. The check is per file, so a
   helper in another file is not seen. Non-marker tests by census: `demo` 4 of 6,
   `split_head_to_head` 6 of 7, `vita_2048_boot` 24 of 27, `boot` 9 of 10,
   `hd_ship_preview` 6 of 10, `omega_campaign_screens` 6 of 7, `wreck` 3 of 4,
   `shine` 2 of 4, `split_screen` 2 of 10, `depth_bias_portal` 1 of 3,
   `photo_mode` 1 of 12, `pose_hold` 1 of 2.
   Change: a per-test filter (`test(/^boot_ground_truth::(a|b)$/)`), or move
   the non-GPU tests to a file outside the group. `check-test-groups.py` accepts
   only the module form, so the checker needs the test form first.
   Saving: at most about 4 s under the simulation, since 60 CPU-s leaves the group.
   Risk: a test moved out that spawns a game is the failure the group exists
   for. The checker must still match `CARGO_BIN_EXE_oag-game` at test granularity.

3. **Print-only ignored probes, not gated by `OAG_SWEEP`.** Each runs in
   `just test-data` and asserts nothing.
   - `crates/pob/tests/pob_initial_particles_ground_truth.rs:131`
     `the_ps2_effects_are_surveyed_for_templates`: 2.15 s, prints each template
     of `pulse-ps2-eu.chd`. Change: an early return unless `OAG_SWEEP` is set, or
     pin the template count with an assert.
   - `crates/audio/src/limiter/tests.rs:102` `limiter_cost`: 0.77 s, a timing
     probe whose doc says to run it in release. Change: `OAG_SWEEP` gate, or move
     it to `benches/`.
   - `crates/raceplay/tests/grid_stagger_ground_truth.rs:96`
     `grid_layout_survey`: 6 ms, gated by `OAG_GRID`, not `OAG_SWEEP`. Change:
     rename the gate, or name `OAG_GRID` as an exception in CLAUDE.md.

4. **Print-only non-ignored probes in the gate.** Each runs in every
   `gate-affected` and `test-data` run and fails on no value.
   - `crates/raceplay/src/tests/extension_examples.rs:140`
     `per_tick_wall_time_of_the_examples_on_this_engine`: 0.33 s; its doc says
     "nothing is asserted". Change: `#[ignore]` plus an `OAG_SWEEP` early return.
   - `crates/modhost/src/tests/engines.rs:99`
     `per_tick_wall_time_of_each_engine`: behind the `pulley` feature, so not in
     the default run; same change as above.
   - `crates/gpu/src/timing/tests.rs:67` `every_adapter_reports_what_it_can_be_timed_with`
     (0.08 s): prints per adapter; its body has no assert. Change: `OAG_SWEEP`
     gate. Its doc's `cargo nextest run ... --no-capture` command then needs
     `OAG_SWEEP=1` added, in the same change.

   Items 3 and 4 together save about 3.3 CPU-s per `test-data` run, under 1 s of
   wall clock. Read the output before gating a probe: a person may depend on
   `--no-capture`.

## Checked and not findings

- `crates/gpu/src/timing/tests.rs:104` `a_timestamp_pair_resolves_to_a_forward_going_duration`
  and `crates/gpu/src/timing/timer/tests.rs:141` `a_bracketed_pass_reads_back_naming_its_own_frame`
  print, but their helpers (`time_a_cleared_pass`, `time_a_pass`) assert the
  period, the resolved pair and the readback. Not print-only.
- `crates/post/src/hd_bloom/tests.rs:40` `the_chain_reads_back_naming_its_own_frame`
  calls `time_the_chain`, which asserts. Not print-only.
- `crates/gpu/src/texture/tests.rs:128` `the_sampler_builds_on_a_device` has no
  assert; a panic in the build fails it. A smoke test, 0.07 s. Left as it is.
- `crates/render/tests/pvs_placement_ground_truth.rs:84` prints a header per disc
  (13 s) but its `sweep()` helper asserts. Not print-only.
- `crates/ballisticng/tests/track_ground_truth.rs:108` `the_workshop_tracks` is the
  longest test in the log (32 s). It asserts through helpers. It belongs to the
  disc-tests thread, not this one.
- No device user is outside the group's policy. Across `crates/*/tests`, every
  file that calls `request_device` or `request_adapter` also uses
  `test_adapter` or is a known opener in the checker's `OPENERS` list. That
  list is enforced by `check-test-groups.py`.

## Open

- The group's module filter cannot express a mixed module; item 2 needs the
  checker to learn the test form first.
- The wall-clock cost of the cap is settled only by the documented replay. A
  start-order trace of a real run would settle the 44 s model.
- The sibling threads `tests-that-could-move-down-the-dependency-tree`,
  `disc-tests-that-repeat-one-setup` and `tests-that-compute-what-they-never-assert`
  are not on `main`. `disc-tests-that-repeat-one-setup` exists on the
  `scout-test-dupes` branch. Items 3 and 4 may overlap
  `tests-that-compute-what-they-never-assert`.

## Next Steps

1. Drop the two `oag-mesh` lit-path modules from `.config/nextest.toml`'s
   `gpu-vram` override, and strip `//` comments in
   `scripts/check-test-groups.py` before matching. Run
   `python3 scripts/check-test-groups.py`, which needs no GPU.
2. Put the three probes in item 4 and the three in item 3 behind `OAG_SWEEP`,
   with the doc-comment command updated. Run `just test-data-affected` on the
   affected packages.
3. Skip item 2 unless a start-order trace of a real run shows the cap binding.
