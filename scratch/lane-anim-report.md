# lane/hd-gantry-anim - report

## Summary

- **2048 reuse: no.** HD does not ship `.rcsanimclip`/`.rcsskeleton` at all -
  zero matches for either extension across all seven `DATA0*.PSARC` archives
  on `hdfury-ps3-eu-dec.iso` (`psarc_list` example, filtered) - and
  `321go_startfinish.rcsmodel`'s own bytes contain no `0xca5caded` container
  magic in either byte order. 2048's skeleton/clip mechanism does not apply
  here; this is a third, independent mechanism from both Pulse's material
  `TEXOFFSET` track and 2048's skeleton/clip pair.
- **Curve's on-disk source: located, not decoded.** The curve pointer is
  `material_record + 0x20`, then `+ 0xc` - a field of the material record
  `oag_rcs::rcsmodel::material` already parses but stops short of (it reads
  through `+0x1c`). Confirmed on all 5 materials of
  `321go_startfinish.rcsmodel`: the 4 `uvoffsetscale`-named materials all
  carry a live curve pointer, the 1 plain `simpletexture` one does not, and
  all 4 share one period value, `13.333333` seconds (new number, not tied to
  the 6.000 s teleport or the 4.533 s thrust gate this page already tracks).
  Channel descriptors for all 4 map to target index 2, components 0 and 1 -
  verified as `uvOffset.x`/`.y` **by an exact hash match** against
  `oag_rcs::rcsmaterial::name_hash("uvOffset")` on the material's own
  parameter-table entry, not assumed from position - matching the field the
  2026-09-17 GDB watch (prior session) caught being written.
- **The bytes past that pointer are Sony's own Edge Animation Tools clip
  format**, identified off two of the evaluator's own embedded assert
  strings (`edgeanim_evaluate_ppu.cpp:469`/`:283`, `anim->offsetPackingSpecs`)
  - not decompiled speculation, the binary names its own source file.
  Renamed `0x0066b840` -> `EdgeAnim_EvaluateClip` (85 - the middleware ID is
  solid, the verb is inference from the decompile shape, same gap
  `AnimCurve_EvaluateChannels` sits at 78 for) and `0x0066c3c8` ->
  `AnimCurve_SampleChannel` (80, was used in prose before this pass but
  never actually applied in Ghidra or `names.tsv` - fixed).
- **Edge's own bit-packed key encoding is NOT decoded.** That's a
  format-recovery task scoped to Edge's container, not Wipeout's - frame
  index table, per-component bit widths, fixed-point scale/bias, quaternion
  slerp reconstruction, read off the evaluator's own decompile but not
  reduced to a byte layout. Per this lane's brief, no value is synthesised
  in its place.
- **Playback: not wired.** There is nothing honest to play back yet - wiring
  a `crates/render/src/mesh/rcs.rs` replay hook now would mean inventing the
  curve's values, which `CLAUDE.md`'s "never invent" rule and this lane's own
  brief both forbid. `mesh/rcs.rs`, `gantry.rs` and `race/gantry.rs` are
  untouched.
- **12-circuit real-data ground truth: 12/12 pass.**
  `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E
  'binary(gantry_mount_hd_circuits_ground_truth)'` against
  `hdfury-ps3-eu-dec.iso` - all 12 previously-unverified tests pass.
- **Gate**: `just` run to completion under the shared lock, watched to exit
  (not left for a notification). See below for the exact result and whether
  `test-data` was also run.

## Evidence trail (reproducible)

1. `crates/assets/examples/psarc_list` against all seven `DATA0*.PSARC` on
   `hdfury-ps3-eu-dec.iso`, filtered on `rcsanimclip`/`rcsskeleton` - zero
   hits both. `321go_startfinish.rcsmodel` extracted via `psarc_cat` to
   `data/derived/` (gitignored, not committed) and checked byte-for-byte for
   `0xca5caded` - absent.
2. `oag_rcs::rcsmodel::coverage` over the extracted file: 69.0% claimed,
   biggest gaps at the relocation table (documented, just not claimed by
   `coverage.rs`) and inside each material record past `+0x1c` (104-200
   bytes per record, 4 of 5 unclaimed tails non-trivial).
3. Read `AnimCurve_EvaluateChannels` (already named, `0x005f9bd0`) fresh:
   confirms `resource+0x2c`/`+0x30` (the "animated-target list") are
   word-for-word the same fields `rcsmodel.rs`'s own header doc already
   names as material count / material offset table - the "targets" are
   materials, not a separate section.
4. `crates/rcs/examples/hd_gantry_curve_probe.rs` (committed) reads
   `material_record+0x20` -> `+0xc` for all 5 materials of the extracted
   file directly, off the same offsets the decompile names, and prints the
   correlation table in the summary above.
5. Decompiled `AnimCurve_SampleChannel` (`0x0066c3c8`) and its callee
   (`0x0066b840`, now `EdgeAnim_EvaluateClip`) - the latter's own body
   contains the two Edge assert strings.
6. `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`'s new 2026-09-17
   section (second one, same day) has the full trace, tables and addresses.
   `docs/rendering/start-gantry.md`'s "What is still open" updated to match.

## Gate

- `just check-names`, `just gen-status` (names.tsv + billboards.md changed -
  `docs/overview/status.md` regenerated, ps3-hdfury-eu row 343->345), `just
  check-docs` - all clean.
- `just fmt`/`cargo fmt -p oag-rcs` run after an initial `fmt-check` failure
  on the new probe's formatting.
- Full `just` under `flock "$HOME/.cache/oag/gate.lock"`, watched to
  completion (see final message for exit code and log).

## What's still open (for the next pass)

- Edge Animation Tools' own bit-packed keyframe decode - the real remaining
  work, scoped to Edge's container rather than Wipeout's, and potentially
  reusable beyond this billboard if Edge is used elsewhere in HD (not
  checked).
- Everything else this lane's brief listed as pre-existing and out of scope
  (FINAL LAP/chequered triggers, board background alpha, Zone negative
  control) is unchanged.
