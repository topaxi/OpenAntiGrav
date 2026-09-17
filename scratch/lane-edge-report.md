# lane/hd-edgeanim - report

## Summary

- **Edge's key encoding: decoded, for the path every curve on
  `321go_startfinish.rcsmodel` actually uses.** Every clip on the file has
  `offsetPackingSpecs` absent (the bit-packed path the previous lane left
  open) and no rotation/translation/scale channel - so the "not decoded" gap
  that lane closed with was for a codec path these particular curves never
  exercise. The scalar path they do use has **no bit-packing at all**: a
  self-relative-offset header, a binary-searchable frame-set table, a
  sparse-keyframe presence bitmap, and plain big-endian `f32` values, linearly
  interpolated. Implemented in `oag_rcs::edgeanim` (the clip) and
  `oag_rcs::rcsmodel::material::curve` (a material's own pointer to one).
- **Matched against what, since "use public documentation" didn't hold.**
  No reachable Edge Animation Tools SDK header or manual was found. The
  layout therefore rests entirely on this binary: `EdgeAnim_EvaluateClip`'s
  own decompiled arithmetic, its two embedded assert strings
  (`edgeanim_evaluate_ppu.cpp:469`/`:283`) which name the struct's fields
  directly, and empirical cross-validation across all four clips on
  `321go_startfinish.rcsmodel`. `offsetPackingSpecs` is pinned at `+0x4c` by
  its own assert firing there and at no neighbouring offset. See
  `docs/formats/edge-animation.md` for the full trail.

- **The curve is a wipe, not a glyph selector - measured, not assumed.** All
  four curved materials' `uvOffset.y` sampled across the full 13.333 s loop:
  each is a single continuous ramp at its own staggered window (materials
  ramp at 0-6.4s, 6.1-7.5s, 9.6-12.6s, 12.5-13.3s respectively), never the
  four `{0, 0.25, 0.5, 0.75}`-plateau pattern a glyph selector needs (contrast
  Pulse's own gantry, `crates/vex/tests/start_gantry_ground_truth.rs`).
- **Playback: not wired.** No time base connects this curve's own 13.333 s
  loop to the ~6 s countdown a player actually sees -
  `Billboard_UpdateInstanceUvs`'s own clock argument was not traced further
  (a new RE thread, out of this lane's scope). Wiring a guessed time base
  into `crates/render/src/mesh/rcs.rs` to make the digits line up would be
  exactly the invented-mechanism failure `CLAUDE.md`'s "never invent" rule
  forbids - the curve landing on a plausible frame by construction of a
  guess is not the digit board reading correctly for the reason this curve
  drives it. **Did not see `3`/`2`/`1`/`GO` read correctly** - this curve
  does not produce that, by its own measured shape, and nothing was fitted
  to make it look like it does.
- **A named, not chased, lead for whoever picks this up next**: the per-node
  `+0xe4` static UV override table `Billboard_UpdateInstanceUvs` also reads
  (`billboards.md`'s 2026-09-17 first section, from the previous lane) is at
  least as likely the real glyph-selection mechanism as this curve.
- **Verified against real disc data**: `hd_gantry_curve_ground_truth.rs`, 5
  tests, all passing against `hdfury-ps3-eu-dec.iso` (`OAG_REQUIRE_GAME_DATA=1
  cargo nextest run -p oag-rcs --run-ignored all -E
  'binary(hd_gantry_curve_ground_truth)'`) - curve presence follows the
  material name (4 of 5), every channel is `uvOffset.x`/`.y` by hash (not
  position), the clip header matches the disc, all four curves vary
  continuously rather than in glyph-sized steps, and material 1's `y` channel
  reproduces the four frameset-boundary values (0.1755/0.4585/0.7415/1.0) with
  no discontinuity at the seam.

## Gate

- Full `just` run under `flock "$HOME/.cache/oag/gate.lock"`, watched to
  completion: **exit 0**, 3827 tests passed, 895 skipped, all `just` checks
  (docs, deps, transcendentals, file-size, ghidra-names, ghidra-captures,
  handover-size, link-worktree-data self-check, strings, just-args,
  gen-re-coverage) clean.
- `just test-data` under the same lock, `OAG_REQUIRE_GAME_DATA=1`, watched to
  completion (`/tmp/test-data.log`, read after exit, not left for a
  notification): **4722/4722 tests passed, 0 skipped** (5 more than the
  baseline 4717 - this lane's own new tests, all real, none skipped). Overall
  recipe exit 1, but only from `check-test-budget`: `ai_roll_ground_truth`'s
  two grid tests at 396s/401s under full-suite contention, the exact known
  non-test red this lane's brief names (measured 71.4s in isolation on
  `main`) - not this lane's own regression, no `BASELINE` row added, nothing
  split.
- `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
  present verbatim before touching anything and unmodified after (grep-checked,
  not re-derived).
- No `names.tsv`/`docs/ghidra/**` rename landed this pass (only prose added to
  an existing evidence page), so `just gen-status` was run as a verification
  step only, not because a name changed - clean, no drift.
- `just check-docs` clean after the `billboards.md`/`start-gantry.md` pointers
  and the new `docs/formats/edge-animation.md` page.

## Scope discipline

- No new RE thread opened. The clock/time-base question and the `+0xe4`
  static-override table are both named in `docs/formats/edge-animation.md`
  and `billboards.md`, not chased.
- `data/derived/` extraction artifacts are gitignored and were not staged;
  `err.log` and all scratch artifacts under `/tmp` were deleted before
  committing.
- `Material`'s new `curve: Option<Curve>` field forced `curve: None,` into
  four struct literals outside this lane's file list
  (`crates/render/src/mesh/flame/tests.rs`,
  `crates/render/src/mesh/rcs/skin.rs`) - mechanical, no logic touched, noted
  for the lead.

## What's still open (named, not chased)

- The real glyph-selection mechanism for `3`/`2`/`1`/`GO` - most likely the
  per-node `+0xe4` static UV override table, not this curve.
- The time base `Billboard_UpdateInstanceUvs` feeds its curve evaluation with.
- Whether Edge Animation Tools is used anywhere else in HD's own animation
  system (nothing here checked that).
- The joint (rotation/translation/scale) decode path - no data on the disc to
  develop or check it against yet, `Clip::parse` refuses rather than guesses.
