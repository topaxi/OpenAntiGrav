# Test-suite redundancy: what the first trim left, ranked

The first redundancy pass (2026-10-12) is measured in
[`docs/architecture/workspace-layout.md`](../../docs/architecture/workspace-layout.md),
"Trimming redundant walks", and its rule is in `CLAUDE.md`'s test-budget
section. These are the candidates it saw and did not act on, ranked by expected
summed seconds saved. Seconds are per binary from a contended full
`just test-data` on `main` that day (8,005 tests, 12,908 s summed); read them
as relative. **V** means the code was read and the redundancy confirmed, **H**
means a read-only sweep's lead, not yet verified.

## Open

1. **`omega_gnf_ground_truth`, 988 s after the merge, now the largest binary.**
   87% of its CPU is `oag_texture::bcn::bc7`. No redundancy is left in the test;
   a faster BC7 block decoder is a library change that also speeds the runtime
   CPU fallback. (V)
2. **`oag-assets` `omega_psarc_ground_truth`, 185 s** (data00 72 s): reads every
   entry of `data00`-`data04` for a magic census, the same reads
   `omega_gnf_ground_truth` makes for `.gnf` entries. Not a strict subset (a
   stronger magic check, and non-`.gnf` entries). Check the PSARC zlib backend
   first, since that would speed every HD and Omega load at runtime too. (H)
3. **`oag-game` sky-fog, engine-flare and magstrip family, about 150 s.**
   `psp2_sky_fog` loads each of ten circuits twice, once per test (98 s);
   `hd_engine_flare` repeats one load across 12 tests (55 s);
   `magstrip_wire_hd` flies the same race twice (50 s + 22 s). (H)
4. **`oag-rcs` walks repeated inside one binary, about 100 s.** `rcsmodel_decl`
   calls `every_model` in 7 tests (43 s), `rcsmodel_material` calls
   `every_material` in 4 (44 s), `psp2_rcsmodel` calls `survey` in 5 (54 s), and
   `rcsmodel_ground_truth` has 3 inline walks. Same fix as `gxp` (one walk,
   named checks), but those files keep one claim per test for legibility on
   purpose, so ask before merging. (V for `rcsmodel_decl`, H for the rest)
5. **`replay_ground_truth`, 167 s.** `record(title)` runs three times per title,
   once per property (about 55 s of it). The split is that file's documented
   tail decision; sharing the recording means one test per title. (V)
6. **`ai_roll` full grids, 587 s.** 4 of 12 tier-by-seed grids are recomputed
   (about 190 s), but sharing them puts four grids in one test, past the 300 s
   per-test ceiling under load. Worth it only after a faster raycast. (V)
7. **`omega_wem` (456 s) against `omega_music` (215 s).** Both ATRAC9-decode the
   stem `.wem` files; partial overlap only. (H)
8. **`hd_trackwall` grid (208 s) against `spawn_heading`.** Identical
   `race::load` over the same 28 HD circuits, in two binaries. (H)
9. **`gxt` (108 s + 33 s) and `gxt_coverage` (9 s)** walk the same 9,910 files;
   fold the coverage into the sweep. **`ps2_texture` (43 s)** is a
   decompress-everything `WADS2.WAD` walk whose blob counter needs every entry;
   a head sniff works only if `ps2_texture::header` does not need the full
   length. (H)
10. **`oag-vex` leftovers.** `pvs` walks `WADS2.WAD` in two tests (about 13 s
    each now); `shadow_occluder`'s lenient every-`.wad` walk (19 s) could use
    `crates/vex/tests/wad_walk/`; `vex_class` and `lane_band_census` are not
    decompression-bound (a sniff bought nothing), so profile them first. (V)
11. **`hd_original_ai`'s three `at_easy_*` tests** run one race three times
    (about 25 s). `vita_2048_original_ai`'s super-phantom test loads 10 events
    to check a table that may be per class (about 60 s); keep it unless the
    table is proved per class. (H)
12. **Not swept yet:** `oag-render` (310 s summed), and `oag-formats` beyond
    the sound banks (`audio_ground_truth` and `sblk_coverage_ground_truth`
    still carry the old decompress-everything `banks_in` copy, about 10 s). (V
    for the copies)

## Next Steps

1. A read-only sweep of `oag-render`, `oag-formats`, `oag-rcs`, `oag-vex`,
   `oag-assets` and the rest of `oag-game` against this list, then verify each
   lead by reading both tests. 1 hour.
2. Act in the order above, measuring each binary before and after under the
   gate lock. An afternoon for items 2-5 and 9-12.
