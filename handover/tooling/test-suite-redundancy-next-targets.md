---
status: blocked-maintainer
titles: [all]
updated: 2026-11-16
summary: "Test-suite redundancy: rcs walk files (psp2 pair merged), render repeats, GNF profiling"
---

# Test-suite redundancy: what two trims left, ranked

Two redundancy passes (2026-10-12) are measured in
[`docs/architecture/workspace-layout.md`](../../docs/architecture/workspace-layout.md),
"Trimming redundant walks" and its second pass, and the rule is in
`CLAUDE.md`'s test-budget section. Seconds are per binary from contended runs
that day; read them as relative. **V** means the code was read and the
redundancy confirmed, **H** means a read-only sweep's lead, not yet verified.

## Done (second pass)

- ~~BC7 decoder speed~~: a quarter faster per block, bit-exact.
- ~~PSARC inflate~~: one reused decompressor per entry, bit-exact on all 40
  archives; not zlib-bound on Omega, which is barely deflated.
- ~~`omega_psarc` reading whole `.gnf` entries for their magic~~: 136 s to 16 s.
- ~~`psp2_sky_fog`, `hd_engine_flare`'s load-only claims, `hd_trackwall`'s
  grid against `spawn_heading`, `hd_original_ai`'s Easy readings,
  `gxt_coverage`~~: one load each.
- ~~`ps2_texture`'s silent drop~~: the two known outliers are named, any other
  fails.

## Open

1. **The maintainer's call, left alone on purpose:**
   - ~~`replay_ground_truth`~~: one recording per title since 2026-10-13
     (62 s to 51 s isolated).
   - ~~`psp2_rcsmodel` and `psp2_rcsmodel_material`~~: one walk since
     2026-11-16 (10 tests to 3, 94 to 13 s user CPU), the `Claim` shape in
     `psp2_rcsmodel_ground_truth.rs`.
   - `rcsmodel_decl` (7 walks), `rcsmodel_material` (4),
     `rcsmodel_ground_truth` (3), about 70 s summed: those files keep one claim
     per test for legibility. Recommendation: the `gxp` pattern (one walk, each
     claim a named function with its doc comment) keeps that legibility in the
     source while losing it only in the test list.
   - ~~`ai_roll` full grids~~: one test per seed races each tier once since
     2026-10-13, and the world broadphase halved what is left.
2. **`omega_gnf_ground_truth`, 781 s, still the largest binary.** Besides the
   per-block decode, its time is the size of what it writes: 2.1 G texels for
   `data03` alone, each a fresh `Vec<[u8; 4]>`, and the chain claim decodes a
   base level a second time to compare it with `decode()`. Profile with `perf`
   (not installed on the measuring host) before changing either. (V)
3. ~~**`magstrip_wire_hd` (72 s):**~~ merged 2026-11-16: the audio and no-audio
   logs were compared field by field (equal), `every_craft` folded into the hum
   flight, and the rumblers share one flight (24.7 to 16.8 s summed).

4. **`oag-render` small repeats, each 6-15 s:** `hd_pad_ne` builds the same pad
   pair three times per circuit, `hd_emissive_glow`'s two tests build the same
   three circuits, `hd_light_bar` builds Vineta K twice, `hd_weapon_pad_split`
   builds Talon's Junction in both tests, `hd_lod` walks all seven archives in
   two tests. Each is a one-load merge; together about 30 s. (H)
5. **`omega_wem` (456 s) against `omega_music` (215 s):** both ATRAC9-decode the
   stem `.wem` files, but each also decodes files the other does not, so
   neither is a subset. A shared decode would need a cross-binary fixture. (H)
6. **`psys_inventory`'s two PS2 tests** scan `WADS2.WAD` twice, cheap since
   `Archive::peek` decodes only heads (18 s for the binary); not worth a merge.
   `shadow_occluder` (19 s) could take `crates/vex/tests/wad_walk/` if its
   lenient every-`.wad` walk is kept. (V)
7. **`vita_2048_original_ai`'s super-phantom test** loads ten events to check a
   table that may be per class (about 60 s); keep it unless the table is proved
   per class. (H)
8. **`audio_ground_truth` and `sblk_coverage_ground_truth`** still carry the old
   decompress-everything `banks_in` copy; about 1 s, `sblk_banks` would do. (V)

## Next Steps

1. Item 4, the render repeats, verified one by one. An hour.
2. Item 2 with a profiler. An afternoon.
3. Item 1's remaining rcs files (`rcsmodel_decl`, `rcsmodel_material`,
   `rcsmodel_ground_truth`): the maintainer approved the `Claim` shape for the PSP2
   pair; measure each file's walk alone first. An hour.
