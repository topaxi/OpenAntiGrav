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

- **The `200`/`250`/`260`/`35` re-run (2026-09-05) landed past this thread's own first-pass
  estimate.** `hd_specular_unresolved_trace.rs` against the current disc: **172** occurrences,
  not 168 - `200`: 52, `250`: **12** (this thread's own first-pass number of 8 was itself stale),
  `260`: 84, `35`: 24. The operand-evidence tally over 344 operands: `Sum` 142, `Normalize` 138,
  `Neither` 62, `NoSingleWriter` **2** - a category that was 0 at 168 operands. **Unexplained**:
  what the two new `NoSingleWriter` operands actually trace to was not investigated - a genuinely
  new open item this re-measurement raised rather than closed. Full numbers and method in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Ships have no Lambert diffuse either",
  sixth dated entry (2026-09-05).
- **The 64 % Zone-declaring jump does not mean what the "consistent with rim^N chains" hypothesis
  suggested, checked by hand.** `hd_specular_population_recheck.rs`'s own filtered/unfiltered
  split shows `5`/`10` (the confirmed rim exponents) are only 3.2 % of the 6,464 Zone-declaring
  total; `200`/`260`/`35` have zero Zone-declaring blocks at all; the other 96.8 % of the growth
  is in the `0` and `32` buckets. Reading 13 distinct `.rcsmaterial` files from those two buckets
  by hand (`hd_specular_zone_sample.rs`, new), across four archives, three circuits, a DLC copy,
  and - decisively - a front-end rank-medal UI material with no relationship to Zone-mode
  gameplay: **every one declares the identical four-parameter cluster**
  (`zoneColourTint`/`zoneEffectInner`/`zoneBaseInner`/`zoneBaseAltInner`) as boilerplate on a
  shared material-template family, not because its resolved chain computes a rim exponent - the
  DP3 chains read as ordinary half-vector/normalize-tail specular, the same shapes seen
  elsewhere. **Only `5`/`10` remain confirmed Zone-rim exponents.** Confidence 85 (13 hand-read
  materials against several thousand - strong structural evidence, not an exhaustive sweep).
- **The `SpecularPower`-patch census subset question is answered, not just the raw counts.**
  Re-running `hd_specular_patch_census.rs` at the commit before the `dp3_feeding` fix
  (`ec762612^`, temporary worktree, removed after) reproduced the old numbers exactly (295/62/62/
  233) and gave the actual old material list to diff against, rather than trusting the aggregate
  alone: 12 of the old run's 13 distinct `(material, value)` pairs survive unchanged into the new
  86 (near-strict superset, not a replacement population); the one exception
  (`05_ubermall/materials/reflectplane_dc_seawater.rcsmaterial`, authored `52`) most likely moved
  to `other_nonzero` under the now-lane-sound gate rather than being lost. New aggregate: 469
  resolved-`0.0`, 86 patched (all authoring non-zero, 30-100), 383 (82 %) still unexplained;
  4,586 blocks checked, 0 slot collisions (identical to the pre-fix run - this census's walk
  does not depend on `dp3_feeding`'s correctness).
- **`fragment.rs`'s doc comments were deliberately left un-updated.** The fourth next step
  (folding these numbers into `specular_exponent`/`dp3_feeding`'s doc comments) needs the numbers
  to have settled first, and this pass raised a new unexplained discrepancy (`NoSingleWriter`
  above) and revised what the 64 % figure means rather than just re-measuring it - both need to
  survive review before `fragment.rs` states them as fact. The "not pinned to a disc-wide
  population count" placeholder stays accurate and is left in place.

## Next Steps

- Trace what the two new `NoSingleWriter` operands (out of 344, in the `200`/`250`/`260`/`35`
  re-run) actually resolve to - a genuinely new gap in `hd_specular_unresolved_trace.rs`'s own
  method, not present in the pre-fix 168-operand tally.
- Extend the by-hand Zone-declaring read beyond the current 13 materials (all sampled from
  amphiseum, 05_ubermall, 01_vineta_k plus its DLC copy, and one front-end medal) to more
  circuits, to check whether the "boilerplate template cluster, not a rim chain" finding holds
  disc-wide or is itself circuit-specific.
- Once both of the above are closed, fold the settled numbers into `fragment.rs`'s
  `specular_exponent`/`dp3_feeding` doc comments in place of the "not pinned to a population
  count, see renderer.md" placeholder text.
