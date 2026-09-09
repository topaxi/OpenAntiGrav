# HD's specular-exponent population needs re-measuring after the `dp3_feeding` fix

2026-09-04. Split out of [hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
when `Program::dp3_feeding` (`crates/rcs/src/rcsmaterial/fragment.rs`) turned out to be
lane-unsound in both directions - crediting a `DP3` that wrote the wrong lane (1,679 of 6,141
then-resolved blocks, a false positive) and missing a real one behind an unrelated write to a
different lane (5,556 blocks, a larger false negative). Fixed the same day: lane-aware,
clobber-checked, exactly the logic `crates/render/examples/hd_specular_unresolved_reasons.rs`'s
own `feeding` already used and `hd_dp3_feeding_lane_check.rs` now independently cross-verifies
(10,087 of 10,087 currently-resolved blocks lane-sound, 0 mismatches). Full evidence and the
arithmetic that separated the two directions is in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "Ships have no Lambert diffuse
either" - not repeated here.

**What the fix changed, measured so far, and what it did not touch.** The disc-wide resolved
count moved from 6,141 to 10,087 fragment blocks (`specular_exponent()`). `Report`'s own count
for Talon's Junction moved from 426 to 422 of 442 materials unresolved - a small, in-proportion
move for one circuit against a ~64 % disc-wide jump, not investigated further here. The pinned
`crates/rcs/tests/specular_power_ground_truth.rs` ground-truth test still passes unchanged.
Everything below this line is *not* re-measured yet and is the open work this thread tracks -
every specific number renderer.md's "Ships have no Lambert diffuse either" published for the
specular-exponent population was measured under the buggy gate and needs redoing, not assumed
to still hold.

## Open

- **The `200`/`250`/`260`/`35` re-run (2026-09-05) reproduces this thread's own first-pass
  numbers exactly, once compared to the right column.** `hd_specular_unresolved_trace.rs` never
  applies the Zone exclusion - filtering its output by `declares_zone=false` reproduces the
  thread's Zone-excluded **168** exactly (`200`: 52, `250`: 8, `260`: 84, `35`: 24, unchanged);
  its raw, unfiltered total is **172**, the four extra being the already-known Zone-declaring
  `martin_inflatable2` `250`s. (An earlier draft of this note compared the tool's unfiltered 172
  against the thread's filtered 168 and wrongly called the thread's number stale - corrected.)
  The operand-evidence tally over the unfiltered 344 operands: `Sum` 142, `Normalize` 138,
  `Neither` 62, `NoSingleWriter` **2** - a category that was 0 at 168 operands. Traced to source:
  both are the ship's own `nitro_perspex_new.rcsmaterial` (`260`, non-Zone, two variants), **not**
  the Zone-declaring `250`s - a small, real gap in the operand-shape method within the `260`
  bucket, unrelated to the Zone finding below. Full numbers and method in
  `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "Ships have no Lambert diffuse either",
  sixth dated entry (2026-09-05).
- **The 64 % Zone-declaring jump does not mean what the "consistent with rim^N chains" hypothesis
  suggested, checked by hand with a positive control.** `hd_specular_population_recheck.rs`'s own
  filtered/unfiltered split shows `5`/`10` (the confirmed rim exponents) are only 3.2 % of the
  6,464 Zone-declaring total; `200`/`260`/`35` have zero Zone-declaring blocks at all; the other
  96.8 % of the growth is in the `0` and `32` buckets. `hd_specular_zone_sample.rs` (new) samples
  13 distinct `.rcsmaterial` files from those two buckets across four archives, three circuits, a
  DLC copy, and a front-end rank-medal UI material with no relationship to Zone-mode gameplay:
  **every one declares the identical four-parameter cluster**
  (`zoneColourTint`/`zoneEffectInner`/`zoneBaseInner`/`zoneBaseAltInner`). Sampling `5`/`10`
  directly as a control shows their DP3 chains have the *identical* `Sum`/`Normalize`/`Neither`
  shape mix as the `0`/`32` majority - this page's own history already warned an uncontrolled
  operand-shape read proves nothing ("`32` was never a valid control"), so that comparison is
  dropped rather than kept as supporting evidence. What does answer the question: the confirmed
  `5`/`10` rim materials declare the *same* four-parameter cluster the `0`/`32` majority does, so
  the cluster's presence cannot itself distinguish a rim chain from an ordinary one. A second
  check - dumping `declares_zone` across every variant of one sampled material, not just the one
  variant the main loop selects - shows the cluster is not blanket file-level boilerplate: it
  toggles across contiguous variant ranges of the same file
  (`01_normal_diffuse_specularonalpha.rcsmaterial`, 0-14 false/15-34 true/35-49 false/50-69 true).
  **This file alone does not settle orthogonality**: only 4 of its 70 variants resolve an
  exponent at all (all `32.0`), and all 4 sit on the Zone-declaring side - equally consistent
  with "the flag is unrelated to the specular chain" and "only Zone variants happen to carry a
  resolvable chain here". The orthogonality claim rests on the rim control, not this file: a
  confirmed rim material declaring the identical cluster rules out the cluster as a rim detector
  regardless of whether any one file's Zone/non-Zone variants both resolve. **Only `5`/`10`
  remain confirmed Zone-rim exponents.** Confidence 85 (the declared-parameter overlap against
  the rim control is direct, controlled evidence; the per-variant split independently shows the
  cluster isn't file-level boilerplate but not, by itself, orthogonality; 13 hand-read materials
  against several thousand is not an exhaustive sweep).
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
  circuits, to check whether the "Zone-mode-variant flag, not a rim chain" finding holds
  disc-wide or is itself circuit-specific. Confirming the per-variant toggle against the game's
  own render-path/variant-selection code (rather than inferring it from one file's variant
  pattern alone) would raise this past its current 85.
- Settle the orthogonality question directly: find a material with at least one
  `declares_zone=false` variant that still resolves a specular exponent.
  `01_normal_diffuse_specularonalpha.rcsmaterial` could not answer this on its own - all four of
  its resolving variants happen to sit on the Zone-declaring side, so it is equally consistent
  with "unrelated" and with "only Zone variants resolve here". A material with a resolving
  non-Zone variant would close this cleanly; one without would instead be worth investigating for
  why non-Zone variants of this shader family never resolve at all.
- Once both of the above are closed, fold the settled numbers into `fragment.rs`'s
  `specular_exponent`/`dp3_feeding` doc comments in place of the "not pinned to a population
  count, see renderer.md" placeholder text.
