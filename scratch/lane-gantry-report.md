# lane/hd-gantry-glyph-walk - final report

Wound down mid-session on the team lead's instruction. Both tasks landed and
are committed. Everything below is in the git history already; this file is
the summary the report format asks for.

## Commits on this branch (`lane/hd-gantry-glyph-walk`)

1. `effbb719` - the main finding: the runtime write located, plus the full
   12-circuit gantry mount sweep (Task 2).
2. `8bacbb06` - `just gen-status` regeneration (names.tsv changed).
3. `e5906174` - the two full `just` gate run logs, for the record.

**Note on timing**: the team lead's "zero commits" messages crossed in
flight with `effbb719`/`8bacbb06` - both had already landed by the time
those messages were sent. Nothing was lost; the state they were worried
about does not exist on disk.

## Task 1 (primary): locate the runtime write - YES, located

Used `just build-rpcs3-watchpoints`'s patched RPCS3 (already built at
`/home/topaxi/build/rpcs3-oag/build/bin/rpcs3` from an earlier pass, verified
still matching the tracked patches before reuse - no rebuild needed).

**Method** (`scratch/gantry_watch.py`, committed, re-runnable - see its own
docstring): read `Billboard_LoadModelAndBind`'s decompile for the *runtime*
addresses of slot 8's 19 per-submesh instance blocks (a heap allocation, not
knowable statically), armed a `Z2` write watchpoint on all 19 at once (the
patch's registry is an unbounded `std::vector`, no need to guess which
submesh matters ahead of time), then let an actual countdown play under GDB
control with `Assume External Debugger: true` (already in the shared stock
config) for a trustworthy stop-reply PC.

**Result, reproduced on two independent boots with two different heap
layouts**: a write fires at submesh index 2's own `uvOffset.x`, same PC
`0x005f9c9c` both times, on a non-main PPU thread. Traced the full call chain
by decompile and named three functions (confidence in `billboards.md`,
`names.tsv`):

- `Billboard_UpdateAndRender` (`0x003a5f68`, 82) - per-frame counterpart to
  `Billboard_LoadModelAndBind`, walks `g_BillboardSlots` with identical
  gating, does RSX render-to-texture setup per slot.
- `Billboard_UpdateInstanceUvs` (`0x003e4f18`, 88) - per-slot, per-frame. Two
  passes: a **static** per-node `uvOffset.xy`/`uvScale.zw` override from
  `node+0xe4` (a newly-found, distinct mechanism - a `-1` sentinel means "no
  override"), then an **animated** pass calling the curve evaluator for each
  entry in the resource's own animated-target list.
- `AnimCurve_EvaluateChannels` (`0x005f9bd0`, 78) - the actual writer.
  **Generic, 5 callers total, not billboard-exclusive** - confirmed as *a*
  mechanism billboards route through, not confirmed as existing for
  billboards alone. Said explicitly at this confidence rather than
  overclaimed.

**On the two non-identity values the lead asked about**
(`uvOffset = [1.0, 0.25, 0, 0]` on submesh 9, `[0, 0.022, 0, 0]` on submesh
18): these are **not** the watched write. Both were armed the same as every
other instance, and neither ever produced a watchpoint hit across a 45-second
window (one boot) or the shorter first window (the other) - they were
already at those values at load time, before the countdown started. That is
direct evidence they come from the **static** `node+0xe4` override pass, not
the animated one, and `billboards.md` says so explicitly rather than letting
the `0.25` coincidence (which does resemble Pulse's own -0.25 staircase step)
carry the conclusion unchecked. The actual watched write's own value was
`0.0` both times - consistent with the live-capture progression
("nothing, nothing, a faint sliver, a clear `3`...") starting from a
blank state, not with the Pulse staircase pattern directly.

**Not implemented, on purpose**: the curve's own authored content
(`target+0x20+0xc`) was not decoded. The resource it hangs off is the loaded
`.rcsmodel`'s own in-memory object, so it is very likely authored in that
file, in a section `oag-rcs` does not parse yet - a format-recovery task,
then a replay hook in `crates/render/src/mesh/rcs.rs` (owned by
`lane-hd-material-curve` while that lane runs, so correctly out of this
lane's reach regardless). Writing a synthetic offset instead would be the
invented mechanism CLAUDE.md's rule forbids - the write's address and caller
are now known; only its data is not.

Docs updated: `docs/ghidra/functions/ps3-hdfury-eu/billboards.md` (new
2026-09-17 section + names table), `docs/rendering/start-gantry.md` (new
section, "still open" bullet rewritten), `HANDOVER.md`'s gantry summary line,
`handover/gameplay/race-start-countdown-and-launch-boost.md`'s Open list.
Also corrected a stale lead in the same pass: `billboards.md`'s own
2026-09-15 section had already ruled out all 14 `lwz 0x834(` sites before
this pass started; the "eleven unexamined" phrasing elsewhere on the page
and in the handover thread was stale and is fixed in both places.

## Task 2 (guaranteed deliverable): DONE, all 12 circuits, committed

`crates/game/tests/gantry_mount_hd_circuits_ground_truth.rs` - one `#[test]`
per HD race environment (macro-generated, so `cargo nextest` parallelises
across circuits rather than serialising a `for` loop in one test - no
`BASELINE` row needed, the split doesn't change what's asserted). Covers all
12 of `oag_hd::names::ENVIRONMENTS` minus the four Zone ones: `amphiseum`,
`modesto_heights`, `talons_junction`, `tech_de_ra`, `01_vineta_k`,
`02_track`, `03_track`, `04_chenghou_project`, `05_ubermall`,
`10_sebenco_climb`, `12_sol_2`, `15_anulpha_pass`. Each asserts
`oag_render::gantry::mount(&model).is_some()` and prints centre/width/
height/thickness. **Not run against real data this session** - RPCS3's
single-instance constraint consumed the available emulator time on Task 1;
`cargo test -- --list` confirms all 12 register correctly. Whoever merges
should run `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all -E 'binary(gantry_mount_hd_circuits_ground_truth)'`
once to confirm all 12 actually pass on real data (5 of the 12 were already
hand-measured in the 2026-09-13 pass and are expected to; the other 7 are
new).

## Gate

Ran the full `just` gate twice (serialized through the shared lock):
`scratch/gate-run1.log` (red on `check-status` only - names.tsv had grown
three rows and `docs/overview/status.md` hadn't been regenerated yet),
`scratch/gate-run2.log` (fully green after `just gen-status`). **Did not run
`just test-data`** - queued behind the shared lock and never started before
wind-down; per the lead's own instruction, a green `just` plus a lone `.rs`
test file is sufficient, and they'll run `test-data` after merging.

## Explicitly not touched

`crates/render/src/mesh/**`, `mesh.wgsl`, `renderer.md` - all `lane-lighting`
territory, read but never edited. No synthetic UV offset written anywhere.
No RPCS3 boot happened after the lead's final wind-down message.
