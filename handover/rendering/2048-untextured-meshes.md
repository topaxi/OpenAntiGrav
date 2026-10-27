---
status: measure
titles: [2048]
updated: 2026-10-27
summary: "2048 untextured or flat meshes: stride-keyed UV declarations zero texcoords (likeliest, Empire Climb); Zone road flat by design; DLC exits unknown"
size: M (35)
difficulty: medium (35)
---

# 2048: some meshes draw with no texture

Maintainer, playing 2048: "some meshes don't have any textures, but some do." A
scout pass (2026-10-27) found no single cause. It found three classes that all
look untextured or flat, and ranked them. Nothing here is fixed, and no claim
below is runtime-verified beyond the loader counts quoted.

Where the counts come from: the loader report line `oag_raceplay::loader_log`
(`crates/mesh/src/mesh/rcs/psp2.rs` `describe`, around line 331) for each
circuit. Reproduce a count with a headless race and the debug log, then
`rg 'non-finite texcoord' <oag-game log>` (flags per `docs/tools/oag-game.md`:
`--race --track <vex path> --screenshot <png> --ticks 60`, logs under
`XDG_STATE_HOME`). Verify the flag names before you rely on them.

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
- A draw can also be **textured but flat**: its texcoords were zeroed, so it
  samples texel (0,0). Nothing counts that as unresolved.

## Ranked causes

### 1. Stride-keyed vertex declarations zero or scramble the diffuse UVs (likeliest for flat walls)

- Mechanism. On Vita the declaration for a submesh is found by stride, one map
  per file (`crates/rcs/src/rcsmodel/psp2/vertex_decl.rs:255`
  `find_by_stride`). The record's own pointer is used only on PS4
  (`crates/rcs/src/rcsmodel/psp2.rs:809`, `ps4.then(...)`). A chunk whose stride
  is shared with a differently laid out declaration reads its `Uv1` from the
  wrong bytes. Non-finite values are substituted with `[0,0]` (psp2.rs:831-839)
  and counted in `non_finite_texcoords`; finite noise is kept and paints a
  wrong texel. Documented at `crates/rcs/src/rcsmodel/psp2.rs:438-451` ("about
  21% of decoded vertices corpus-wide, not zero").
- Omega had the same bug and fixed it. A PS4 submesh holds a 64-bit pointer to
  its declaration at `+0x28`; the Vita `u32` at that offset resolves only 14.2%
  of submeshes (`docs/formats/omega-status.md:862-878`, "Both are gone"). On
  2048 the fix is not wired (**checked, applies, not wired**). Section B does
  carry per-chunk declarations (`docs/formats/2048-rcsmodel.md`, "Section B
  carries a vertex declaration"), so the missing piece is resolving the Vita
  pointer or the section-B record per submesh.
- Evidence (loader lines, base circuits and DLC, 2026-10-22 and 2026-10-19
  runs). Non-finite texcoords zeroed per circuit, against the circuit's
  vertex count:

  | Circuit | Zeroed texcoords | Vertices | Share |
  | --- | ---: | ---: | ---: |
  | `environments\tower` (Empire Climb) | 90,201 | 470,840 | 19% |
  | `DLC1\environments\Moa_Therma` | 15,871 | 153,045 | 10% |
  | `DLC1\environments\Ubermall` | 3,032 | 262,789 | 1.2% |
  | `DLC1\environments\Vineta_K` | 2,900 | 157,887 | 1.8% |
  | `DLC1\environments\chenghou_project` | 2,440 | 157,180 | 1.6% |
  | `DLC1\environments\Metropia` | 510 | 215,552 | 0.2% |
  | `environments\mall` | 1,095 | 491,212 | 0.2% |
  | `environments\altima` | 742 | 419,023 | 0.2% |

  Empire Climb is `tower` (`docs/formats/2048-campaign.md:202`). The compare-page
  walk noted its walls and track drawing flat-coloured and brighter than the
  original (`handover/rendering/hd-fidelity-lanes-queued-after-the-cold-originals.md`,
  item 8). A flat wall is the symptom this cause predicts. Confidence 60 on the
  link. The table counts vertices, not submeshes, so it does not yet show a flat
  submesh.
- Measure. A census on each 2048 circuit, both in `psp2::build` (counts only,
  no behaviour change):
  - per submesh: vertices zeroed, and whether **all** vertices are zeroed (the
    flat case); sum of triangles over submeshes with any zeroed vertex (area
    proxy; see Risks);
  - oracle: on the subset where the Vita record pointer does resolve (14.2%),
    does the stride map's declaration agree with it? Disagreement counts the
    mis-keyed submeshes directly.
  Expected: the all-zero count on `tower` and the Moa DLC is the number that
  should move when the declaration is fixed. Ship models log fully textured,
  but their zeroed-texcoord counts were not read; include them in the census.
- Risk. Touches only the Vita/PS4 mesh reader (`oag-rcs`, `oag-mesh`).
  Not simulation: no crate in the determinism list is involved, no hash moves.
  A wrong declaration would paint a picture from a colour channel read as UV,
  so check each fix against the original frame, not a green test.

### 2. Zone race road and walls are flat colour, by design (only in Zone mode)

- `docs/formats/2048-material-params.md:60-75`: a Zone race draws
  `trackZone.rcsmodel`. Its `fc01_dummy` (1,716 submeshes) outputs `fogColour`
  flat with no sampler, and `zonefc06_colour_emissive_scalar_*` (1,587 submeshes)
  are `Zone_ColourN` colours, unlit. Both have no texture on purpose.
- The loader line for it (`environments\altima\trackZone`, 6,349 submeshes)
  reports 3,252 unresolved. That is close to 1,716 + 1,587 = 3,303, so the two
  groups explain almost all of it; the exact overlap is not measured.
  Whether `fc01_dummy` submeshes count in `unresolved_draws` is not stated in
  the code; check `psp2::Report::placeholder_submeshes` before quoting it.
- Confirm first: which mode the maintainer played. If it was a Zone race, this
  is the expected look, and nothing is wired wrong. The open question is whether
  the original's Vita road is the flat `fogColour` the program outputs; the
  Omega side is not confirmed (`2048-material-params.md:116-130`).
- Related: `handover/rendering/omega-and-2048-draw-normal-maps-as-colour.md`
  (constant-colour lighting-only materials, the same `psp2.rs:839-842` exit).

### 3. Diffuse, archive or decode exits on a few percent of draws (cause unknown)

- Base circuits lose 19-29 draws in 2,817-7,357 (`altima`, `tower`, `mall`);
  DLC circuits more: `Moa_Therma` 126 of 3,155, `Metropia` 85 of 3,957,
  `Ubermall` 32, `Vineta_K` 31. `talons_junction` loses 1.
- The reason is not logged. Likeliest exits by prior:
  - archive miss, `psp2.rs:833`: a DLC material naming a `.gxt` that is not in
    the archive set the loader reads (an HD-inherited or base-tree path).
    Ruled out as a name problem: `crates/formats/src/wad.rs:437-441` lowers case
    and turns `\` into `/` before hashing, so spelling cannot cause a miss.
  - no diffuse path, `psp2.rs:820-829`: the only `.gxt` is the lightmap
    (excluded at `material.rs:378`) or a sampler outside
    `DIFFUSE_SAMPLER_NAMES`.
  - decode refusal, `psp2.rs:834`: low prior. `docs/formats/gxt.md:65-70`
    lists six formats, all decoded; that census was not run on DLC archives.
- Measure: the reason counters (below) on `Moa_Therma` and `Altima`.

## Omega comparison

- Omega's stride bug is fixed (see cause 1), so the same class of draw is
  **ported** there, not on 2048.
- Unresolved counts are not like-for-like. Omega circuits are re-cooked with
  3 to 14 times the triangles (`docs/formats/2048-status.md`, "Re-cooked on
  Omega"). `Environments\01_vineta_k` is Omega's own layout, not 2048's
  `DLC1\Vineta_K`. Do not compare totals.
- Next step: a per-material-path census on both titles for the same circuit
  (2048 `altima`, Omega `Environments\...altima` if it ships), counting which
  material paths lose their texture. That settles whether cause 3 is shared.

## Open

1. **Confirm the mode first.** Ask the maintainer which mode and circuit showed
   the untextured meshes. Zone race means cause 2 (by design), Empire Climb
   (`tower`) means cause 1.
2. **Census (counts only, no behaviour change).** Add to `psp2::Report`:
   - per submesh, zeroed texcoords and "all zeroed", with the triangle count;
   - the four `bind_textures` exits as separate counters, one debug line per
     unresolved submesh with its path and reason and its triangle count;
   - the pointer-versus-stride disagreement count from cause 1.
   Then run each 2048 circuit headless (see the top of this thread) and read
   the new counters.
3. **Rank by area, not count.** A count of 19 can be a whole road or a few props.
   The census above logs triangles, so quote area before choosing the fix.
4. **Fix cause 1 by resolving the Vita declaration per submesh.** Start from
   the Omega reader (`crates/rcs/src/rcsmodel/psp2.rs`, the PS4 `of_record`
   path) and the section-B records. Read the Vita pointer's relocation first:
   a `u32` that resolves 14.2% of the time is likely a relocated pointer, not a
   plain offset (the same `+0x04` relocation table `docs/formats/rcsmodel.md`
   describes). This is RE, not a wire, until the relocation is measured.
5. **Omega per-material census** (see Omega comparison).

## Next Steps

1. Ask the maintainer for the mode and circuit of the untextured meshes (one
   question, no code).
2. If Empire Climb or another base circuit: a lane runs the cause 1 census
   above and reports the all-zero submesh count per circuit.
3. If Zone race: close this thread as by design, and write the Vita flat-colour
   law into `2048-material-params.md` if it is not already there.
