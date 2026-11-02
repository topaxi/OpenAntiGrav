---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "Ten Clippy lints ranked by debt caught and violations measured; #1 (sim-crate disallowed methods/types) landed, 2-10 open"
size: L (35)
difficulty: medium (35)
kind: debt
---

# Clippy lints to adopt

2026-11-02, `debt-survey`. Today's config: `[workspace.lints.clippy]` in `Cargo.toml` holds three
`allow`s (`float_arithmetic`, `suboptimal_flops`, `imprecise_flops`) and `[workspace.lints.rust]` has
`unsafe_code = "deny"` and `missing_debug_implementations = "warn"`. There is no `clippy.toml`
and no `.cargo/config`. `just lint` is `cargo clippy --workspace --all-targets -- -D warnings`, so
only the default `clippy::all` set is enforced. The comment on `float_arithmetic` says it is
"enabled per-module in sim crates"; `rg float_arithmetic crates` finds no such enable, so the
sim-crate determinism rules rest on `scripts/check-transcendentals.py` alone.

Counts: one `cargo clippy --workspace --all-targets` run with each lint at `-W`, deduplicated by
span, split by whether the file path is under `tests/`. "src" therefore still includes inline
`#[cfg(test)]` modules, so the unwrap/expect/panic/print/indexing numbers overstate library code;
read them as ceilings. Per-crate leaders in brackets.

## Ranked candidates

| # | lint | violations (src / tests) | catches | rollout |
| --- | --- | --- | --- | --- |
| 1 | ~~`disallowed_methods` and `disallowed_types` via a `clippy.toml` (sim crates only)~~ | landed | see "Landed" below | done |
| 2 | `too_many_lines` with `too-many-lines-threshold = 300` | 21 over 300, 8 over 500 (201 at default 100) [game 76, raceplay 51, render 41 at default] | the 900+ line functions (long-functions thread) | warn plus ratchet: freeze the 21 in a baseline script beside `check-size`, deny new ones |
| 3 | `allow_attributes_without_reason` | 120 / 19 [raceplay 41, game 33, fx 14] | the 95 `too_many_arguments` allows and 26 `dead_code` allows carry no recorded why | deny after one sweep; mechanical (add `reason = "..."`). About 2 hours per 40 sites |
| 4 | `allow_attributes` (prefer `#[expect]`) | 167 / 11 [game 53, raceplay 39, ui-screens 26] | an `allow` never goes stale; `expect` errors when the lint stops firing, so the 95 argument allows delete themselves when fixed | warn per crate; pairs with #3, do #3 first |
| 5 | `wildcard_imports` | 164 / 0 [raceplay 120, ui 17, game 10] | `use super::*` in non-test submodules makes a file's names invisible to the reader (module-boundary smell in raceplay, 39 view-cfg files) | per-crate allow first, `raceplay` last; clippy permits `super::*` only in test mods |
| 6 | `fn_params_excessive_bools` and `struct_excessive_bools` | 2 + 41 [game 10, physics 4, raceplay 4] | flag soup (long-parameter thread) | warn; the struct form flags settings structs that are really enums or bitsets. Few hits, cheap |
| 7 | `cognitive_complexity` (nursery) at `cognitive-complexity-threshold = 40` | 26 / 19 at default 25 [game 17] | the functions that are long because they branch | warn only, nursery lint may churn between releases; use as a list, not a gate |
| 8 | `mod_module_files` | 4 / 0 (`disc`, `render`, `unity` and one more) | one module-file layout (the tree is `foo.rs` plus `foo/`, 4 stragglers) | deny now after moving 4 files |
| 9 | `unwrap_used` and `expect_used` in the byte-parsing crates only | 217 and 428 workspace (library share unmeasured) [source 147 unwrap, game 370] | panics on disc input. A sample of disc, formats, video and assets found only guarded invariants, so this is low yield | per-crate (`oag-disc`, `oag-formats`, `oag-video`, `oag-vex`, `oag-rcs`) `warn`, with `#[expect(..., reason)]` on the guarded ones; do not enable workspace-wide |
| 10 | `string_slice` | 97 / 61 [game 34, tables 33, raceplay 23] | `&s[a..b]` panics on a non-UTF-8 boundary; tables parse authored text | warn in `tables` only |

Looked at and rejected: `indexing_slicing` (4,596 src, too noisy for binary parsers where bounds are
checked up front), `as_conversions` (3,640), `cast_possible_truncation` (927; it is correct on the
`vex`/`rcs` side but the repo already has `#[allow(...)]` on a few, so it needs a typed-offset
plan first), `print_stdout` (307; `oag-trace` and the tools print on purpose), `panic` (25 in src
once inline tests are discounted), `module_name_repetitions` (174, style only), `float_cmp` (68;
tests assert exact floats by design under the determinism rule), `redundant_clone` (16, nursery,
slow), `needless_pass_by_value` (25, small win), `large_types_passed_by_value` (4).

## Determinism angle

`float_arithmetic = "allow"` workspace-wide is correct, since sim code is float math. What matters in
the sim crates is *which* float methods, and that is #1. `clippy::suboptimal_flops` stays `allow`
(it suggests `mul_add`, which the rules ban). `clippy::float_cmp` should stay off in sim tests because
bit-exact equality is the point; do not adopt it there. #1 also needs the `oag-formats`
`HashMap` hits judged: lookup-only use is fine, iteration feeding state is not.

## Landed

- **#1, 2026-11-02.** An identical `clippy.toml` beside each of `oag-core`, `oag-physics`,
  `oag-gameplay`, `oag-ai`, `oag-race`, `oag-formats`; `check-transcendentals.py` fails when the
  copies differ. Design, lists, probe and the hits fixed: `docs/architecture/determinism.md`,
  "Clippy refuses the same calls, and a few more". Test code is linted too, so 22 test files and
  modules carry `#[expect(clippy::disallowed_methods, reason)]`. The `oag-tables` `HashMap`
  (`fexml::dictionary`) is lookup-only and stays; `oag-tables` is not in the six.
- `just check-determinism` is **not** redundant with it: it scans 22 crates against six and
  matches rotation constructors by bare name. Keep both; retiring it would first need the six
  widened to its crate list.

## Open

- `crates/gameplay/tests/determinism_volley.rs` starts its craft from glam's
  `Quat::from_rotation_y`/`_z` (platform `sin_cos`) and is not in CI's `--test determinism` runs.
  Fixing it moves that scenario's committed reference: a decision for the maintainer, not a lint
  fix (determinism.md, "What is still not covered").
- Widen the Clippy lists to the script's other determinism-scanned crates (`oag-weapons` first,
  it builds hashed projectile state) if the script is ever to be retired.
- The library-only share of #9: needs a rerun with `#[cfg(test)]` excluded (`--lib` without
  `--tests`).
- The measurement used the default `too-many-lines` and `cognitive` thresholds for counting
  and read the real figure from the message; per-threshold counts above are from that.

## Next Steps

1. Sweep #3 then turn on #4 per crate.
2. Add the function-length baseline (#2) with the long-functions thread.
3. Leave 5 to 10 until 2 to 4 have landed.

Verify: `just lint` and the full `just` gate; a lint change touches `Cargo.toml`, so the lead's full
run is selected.
