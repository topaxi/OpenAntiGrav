# HD's specular-exponent population needs re-measuring after the `dp3_feeding` fix

2026-09-04. Split out of [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
when `Program::dp3_feeding` (`crates/formats/src/rcsmaterial/fragment.rs`) turned out to be
lane-unsound in both directions - crediting a `DP3` that wrote the wrong lane (1,679 of 6,141
then-resolved blocks, a false positive) and missing a real one behind an unrelated write to a
different lane (5,556 blocks, a larger false negative). Fixed the same day: lane-aware,
clobber-checked, exactly the logic `crates/render/examples/hd_specular_unresolved_reasons.rs`'s
own `feeding` already used and `hd_dp3_feeding_lane_check.rs` now independently cross-verifies
(10,087 of 10,087 currently-resolved blocks lane-sound, 0 mismatches). Full evidence and the
arithmetic that separated the two directions is in
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Ships have no Lambert diffuse
either" - not repeated here.

**What the fix changed, measured so far, and what it did not touch.** The disc-wide resolved
count moved from 6,141 to 10,087 fragment blocks (`specular_exponent()`). `Report`'s own count
for Talon's Junction moved from 426 to 422 of 442 materials unresolved - a small, in-proportion
move for one circuit against a ~64 % disc-wide jump, not investigated further here. The pinned
`crates/formats/tests/specular_power_ground_truth.rs` ground-truth test still passes unchanged.
Everything below this line is *not* re-measured yet and is the open work this thread tracks -
every specific number renderer.md's "Ships have no Lambert diffuse either" published for the
specular-exponent population was measured under the buggy gate and needs redoing, not assumed
to still hold.

## Open

- **The Zone-excluded population histogram moved substantially, and only a first-pass recheck
  exists so far** (`crates/render/examples/hd_specular_population_recheck.rs`, kept as this
  thread's starting evidence). Old (pre-fix, Zone-excluded): `0`: 573, `32`: 409, `260`: 24,
  `200`: 8, `40`: 2, `35`: 4, `300`: 10, `250`: 4 (~1,053 total). New (post-fix): `0`: 1965,
  `32`: 1444, `260`: 84, `200`: 52, `40`: 32, `35`: 24, `300`: 14, `250`: 8 (3,623 total) - every
  bucket grew, several by 3-10x. The `5`/`10` Zone-rim exclusion was re-checked and still holds
  exactly (134/76 unfiltered, 0/0 Zone-excluded) - that finding survives the fix untouched.
- **Zone-declaring blocks went from a small minority of the resolved population to the majority**
  (6,464 of 10,087, 64 %, versus a small fraction before). Checked once, not fully explained: this
  is consistent with the false-negative fix disproportionately unlocking `rim^5`/`rim^10`-style
  chains, whose registers get written piecewise in exactly the shape that tripped the naive gate -
  but "consistent with" is not "confirmed as". Worth a per-material read of a handful of newly-Zone
  chains before trusting the 64 % figure in anything downstream.
- **The `SpecularPower`-patch census (`hd_specular_patch_census.rs`) moved too and was not
  re-examined beyond the raw counts.** Old: resolved-`0.0` bucket ~295, of which 62 patched
  (authored 30-100), leaving 233 (79 %) unexplained. New: resolved-`0.0` 469, of which 86 patched
  (all 86 authored non-zero), leaving 383 (82 %) unexplained. The *shape* of the finding
  (SpecularPower patching explains only a minority of the zero bucket) survived, but the exact
  percentages, and whether the same 62-material set is a subset of the new 86, were not checked.
- **`200`/`250`/`260`/`35` roughly doubled** (84 total occurrences, Zone-excluded, before; 168
  after - `200`: 52, `250`: 8, `260`: 84, `35`: 24). The five-draft operand-shape trace in
  renderer.md (`Sum`/`Normalize`/`Neither`, 82/58/28 of 168 operands) was run against the *old*
  population and is now sized against roughly half of the current one - needs a full rerun, not
  just an update of the total.
- `fragment.rs`'s `specular_exponent`/`dp3_feeding` doc comments were updated to stop citing the
  stale numbers rather than restate them, but do not yet cite the new ones - once this thread's
  numbers are trustworthy, they belong back in those doc comments, not just here.

## Next Steps

- Re-run the `200`/`250`/`260`/`35` operand-shape trace (`hd_specular_unresolved_trace.rs`,
  already lane-correct - no changes needed there, just a rerun) against the doubled population,
  and update renderer.md's five-draft history with a sixth, dated entry rather than editing the
  old numbers in place - this project's own evidence-keeping rule.
- Read a handful of the newly-Zone-declaring blocks by hand to confirm the 64 % jump is real
  `rim^N` chains and not a `declares_zone` false-positive on some other parameter shape - the
  unfiltered-vs-excluded check in `hd_specular_population_recheck.rs` already rules out the
  crude version of this (5/10 still behave exactly as before), but a handful of the *new* Zone
  chains specifically, not just the two known rim values, would close it properly.
- Re-run `hd_specular_patch_census.rs`'s full analysis (not just the raw counts already pasted
  above) and check whether the 62-material patched set from the old run is a subset of the new
  86, or whether the fix surfaced different materials entirely.
- Once the above settle, fold the confirmed numbers back into `fragment.rs`'s doc comments
  (`specular_exponent`, `dp3_feeding`) in place of the "not pinned to a population count, see
  renderer.md" placeholder text the 2026-09-04 fix left there.
