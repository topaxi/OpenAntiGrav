---
status: measure
titles: [2048]
updated: 2026-10-27
summary: "2048 flat meshes: cause 1 wired (Vita reads its record's own declaration); cause 3 (bind_textures exits) and Omega Uv1 type-0 pair open"
size: S (30)
difficulty: easy (25)
---

# 2048: some meshes draw with no texture

Maintainer, playing 2048: "some meshes don't have any textures, but some do." A
scout pass (2026-10-27) found no single cause. It found classes that all look
untextured or flat, and ranked them. Nothing here is fixed. The counts are loader
report lines, not runtime frames.

Where the counts come from: the loader report line `oag_raceplay::loader_log`
(`crates/mesh/src/mesh/rcs/psp2.rs` `describe`, around line 331) for each
circuit. Reproduce one with a headless race and the debug log, then
`rg 'non-finite texcoord' <oag-game log>` (flags per `docs/tools/oag-game.md`:
`--race --track <vex path> --screenshot <png> --ticks 60`, logs under
`XDG_STATE_HOME`). Check the flag names before relying on them.

## How a draw ends up with no texture

- `crates/mesh/src/mesh/rcs/psp2.rs:779` `bind_textures`. A draw's `texture`
  stays `None` when its material resolves to `None`, and then it counts in
  `unresolved_draws` (about line 902). A `None` draw samples the shared white
  1x1 (`crates/mesh/src/mesh_render.rs:1016`, slot fallbacks at 1062, 1080).
  White, not a neighbour's picture and not a flat colour.
- Four exits return `None` there: no diffuse path (`psp2.rs:820-829`, with
  `diffuse_texture_without` at `crates/rcs/src/rcsmodel/psp2/material.rs:370`),
  `textures(path)?` at line 833 (archive miss), `decode_texture_form(...)?` at
  834, and `constant_colour` at 842 (lighting-only material with no colour
  value; `psp2.rs:958`).
- A draw can also be **textured but flat**: its diffuse texcoords are empty or
  zeroed, so every vertex samples texel (0,0). Nothing counts that as
  unresolved, and the loader line does not report an empty-texcoord submesh.

## Ranked causes

### 1. The Vita UV declaration lookup leaves diffuse texcoords empty or zeroed (likeliest for flat walls)

The Vita reader finds a submesh's declaration by stride, one map per file
(`crates/rcs/src/rcsmodel/psp2/vertex_decl.rs:255`, `find_by_stride`). The
record's own pointer is read only on PS4 (`crates/rcs/src/rcsmodel/psp2.rs:809`,
`ps4.then(...)`). The coordinate is then read from `off = Uv1 offset` in
`crates/rcs/src/rcsmodel/psp2.rs` around lines 812-840. Two sub-paths give a
flat submesh, and neither is counted in a `Report` field:

- **(a) No `Uv1` in the stride's declaration, or the stride is too short
  (`stride >= off + 4` fails).** `texcoords` stays empty. `mesh/rcs/psp2.rs:657`
  then reads `submesh.texcoords.get(i).unwrap_or([0,0])`, so every vertex of the
  submesh samples texel (0,0): a whole flat submesh. `SubMesh::texcoords` documents
  this as "a real, countable state (92.5% of the corpus's submeshes have one)"
  (`psp2.rs:412-418`), so about 7.5% of submeshes corpus-wide are in this state.
  Per-circuit share: not measured.
- **(b) Declaration matches the wrong layout.** A chunk whose stride is shared
  with a differently laid out declaration reads `Uv1` from the wrong bytes.
  Non-finite values are replaced with `[0,0]` (`psp2.rs:831-839`) and counted in
  `non_finite_texcoords`, and finite noise is kept and paints a wrong texel.
  The doc at `psp2.rs:438-451` says "about 21% of decoded vertices corpus-wide,
  not zero". Partial zeroing smears a texel over part of a submesh: weaker
  evidence for whole flat walls than (a).

Lineage: **checked, differs.** Omega had the same stride lookup and fixed it with
the record's own pointer. A PS4 submesh holds a 64-bit pointer to its
declaration at `+0x28` that resolves on every submesh of the five base archives
(`docs/formats/omega-status.md:862-878`). The Vita `u32` at the same offset
resolves 14.2% of submeshes, so the Vita fix is not a port of the PS4 code. Section
B of a 2048 chunk does carry per-chunk declarations (`docs/formats/2048-rcsmodel.md`,
"Section B carries a vertex declaration"). The Vita pointer's relocation is a
hypothesis from HD's container (`docs/formats/rcsmodel.md`, the `+0x04` relocation
table), unmeasured on Vita.

Evidence (loader lines, base circuits and DLC; runs of 2026-10-19 and 2026-10-22).
Zeroed texcoords per circuit, against the authored-normal count. Authored normals
are at most the vertex count, so the share shown is an **upper bound**:

| Circuit | Zeroed texcoords | Authored normals | Share at most |
| --- | ---: | ---: | ---: |
| `environments\tower` (Empire Climb) | 90,201 | 688,628 | 13% |
| `DLC1\environments\Moa_Therma` | 15,871 | 181,887 | 8.7% |
| `DLC1\environments\Vineta_K` | 2,900 | 230,781 | 1.3% |
| `DLC1\environments\chenghou_project` | 2,440 | 246,127 | 1.0% |
| `DLC1\environments\Ubermall` | 3,032 | 389,259 | 0.8% |
| `DLC1\environments\Metropia` | 510 | 321,407 | 0.2% |
| `environments\mall` | 1,095 | 688,731 | 0.2% |
| `environments\altima` | 742 | 531,735 | 0.1% |

Empire Climb is `tower` (`docs/formats/2048-campaign.md:202`). The compare-page
walk noted its walls and track drawing flat-coloured and brighter than the
original (`handover/rendering/hd-fidelity-lanes-queued-after-the-cold-originals.md`,
item 8). A flat wall is what (a) predicts, and (b) predicts only partly. Confidence
50 on the link until the census below counts (a) per circuit.

Measure: the census below. The number that should move when the lookup is
fixed: submeshes with empty or all-zero texcoords, and their triangles. Ship
models log fully textured, but their empty or zeroed counts were not read; include
them.

Risk. Touches only the Vita/PS4 mesh reader (`oag-rcs`, `oag-mesh`). Not
simulation: no crate in the determinism list is involved and no hash moves. A
wrong declaration paints a picture from a colour channel read as UV, a plausible
wrong picture, so each fix is judged against the original frame, not a green test.

### 2. Zone race road and walls are flat colour, by design (only in Zone mode)

- `docs/formats/2048-material-params.md:60-75`: a Zone race draws
  `trackZone.rcsmodel`. Its `fc01_dummy` (1,716 submeshes) outputs `fogColour`
  flat with no sampler, and `zonefc06_colour_emissive_scalar_*` (1,587 submeshes)
  are `Zone_ColourN` colours, unlit. Both have no texture on purpose.
- The loader line for it (`environments\altima\trackZone`, 6,349 submeshes)
  reports 3,252 unresolved. That is roughly 1,716 + 1,587 = 3,303, so the two
  groups explain almost all of it; the exact overlap is not measured. The code
  says `Zone_ColourN` submeshes count in `unresolved_draws`; for `fc01_dummy`
  check `psp2::Report::placeholder_submeshes` before quoting it.
- Confirm first: which mode the maintainer played. If it was a Zone race, this is
  the expected look and nothing is wired wrong. The open question is whether the
  original's Vita road is the flat `fogColour` the program outputs; the Omega side
  is not confirmed (`2048-material-params.md:116-130`).
- Related: `handover/rendering/omega-and-2048-draw-normal-maps-as-colour.md`
  (constant-colour lighting-only materials, the same `psp2.rs:839-842` exit).

### 3. Diffuse, archive or decode exits on a few percent of draws (cause unknown)

- Base circuits lose 19-29 draws in 2,817-7,357 (`altima`, `tower`, `mall`);
  DLC circuits more: `Moa_Therma` 126 of 3,155, `Metropia` 85 of 3,957,
  `Ubermall` 32, `Vineta_K` 31. `talons_junction` loses 1.
- The reason is not logged. Likeliest exits by prior:
  - archive miss, `psp2.rs:833`: a DLC material naming a `.gxt` not in the archive
    set the loader reads (an HD-inherited or base-tree path). The loader does read
    the DLC track, so this is a path outside `DLC1`, not a missing archive.
    Ruled out as spelling: `crates/formats/src/wad.rs:437-441` lowers case and
    turns `\` into `/` before hashing.
  - no diffuse path, `psp2.rs:820-829`: the only `.gxt` is the lightmap (excluded
    at `material.rs:378`) or a sampler outside `DIFFUSE_SAMPLER_NAMES`.
  - decode refusal, `psp2.rs:834`: low prior. `docs/formats/gxt.md:65-70` lists
    six formats, all decoded; that census was not run on DLC archives.

## Omega comparison

- Omega's stride lookup is fixed by the record's own pointer (see cause 1). The
  2048 side is **checked, differs**: same symptom, the pointer resolves on every
  Omega submesh and on 14.2% of Vita ones.
- Unresolved counts are not like-for-like. Omega circuits are re-cooked with 3 to
  14 times the triangles (`docs/formats/2048-status.md`, "Re-cooked on Omega").
  `Environments\01_vineta_k` is Omega's own layout, not 2048's `DLC1\Vineta_K`. Do
  not compare totals.
- Next step: a per-material-path census on both titles for the same circuit,
  counting which material paths lose their texture or their texcoords. That
  settles whether cause 3 is shared.

## Open

Census done 2026-10-27 (`2048-flat-census`; numbers and evidence in
`docs/formats/2048-rcsmodel.md`, "The flat-submesh census, and the record pointer";
test `crates/rcs/tests/psp2_flat_census_ground_truth.rs`).

1. **Mode confirmed 2026-10-27: Empire Climb (`tower`), normal race** (maintainer).
   So cause 1, not cause 2. Cause 2 stays by design; cause 3 is still open.
2. ~~Census.~~ **Done as a test, not as `Report` counters** (no draw behaviour or
   `psp2::Report` change). Empire Climb `track.rcsmodel` (4,957 submeshes, 470,840
   triangles): flat (empty + all-zero) 763 submeshes / 61,080 triangles (13%); partly
   zeroed 1,760 / 209,561; **on 3,897 submeshes / 353,130 triangles (75%) the reader's
   offset lands on the record's own lightmap, vertex colour, tangent or nothing**
   (wrong whichever attribute is the diffuse one; 4,477 / 390,244 differ in all); 658 / 38,563 of the flat ones read a real coordinate through
   the own declaration. Per circuit without the Zone model, flat share of triangles: bridge 85%
   (authored: Omega's bridge matches), tower 16%, sol 14%, park 13%, subway 10%.
3. ~~Rank by area.~~ Done above. The bigger defect than the flat 13% is the wrong
   attribute on 75% of the triangles (lightmap UV, vertex colour or tangent as diffuse UV).
4. ~~Measure the Vita pointer and wire it.~~ **Done 2026-10-27 (`2048-vita-decl`).** The
   reader takes the diffuse set from the record's own declaration; the stride map is
   deleted; `2x9` float pairs are decoded; census and Empire Climb numbers are in
   `docs/formats/2048-rcsmodel.md`, "After the wiring". On a normal Empire Climb race the
   walls and road draw their textures. The confidence-70 fallback answers for 4,761 of
   `track.rcsmodel`'s 4,957 submeshes (named `Uv1` is rare on the Vita); it is correct on the
   rendered frame but has no second oracle.
5. **Omega per-material census**: Omega's flat set is authored (its pointer is used
   already; `environments2048/bridge` 5,828 of 11,795 submeshes have no coordinate vs
   2048's 5,809 flat). 235 (`data00`) and 432 (`data04`) Omega submeshes have a diffuse-
   family sampler but a coordinate-less declaration: unexplained.

## Next Steps

1. Cause 3 (the four `bind_textures` exits, still not logged).
2. Omega: 23 `data02` submeshes name a `Uv1` of PS4 type `0` (an `f32` pair) that the reader
   still decodes as halves; decode it and re-pin Omega's renders (one line in
   `Psp2` `submeshes`, `crates/rcs/src/rcsmodel/psp2.rs`).
3. Re-render the 2048 compare-page pairs after merge (the lead does); the fix touches every
   Vita circuit, not only Empire Climb.
