---
status: measure
titles: [2048, omega]
updated: 2026-11-04
summary: "Flat meshes closed; open: 235/432 Omega submeshes with a sampler but no coordinate, and the colour a no-texture material draws"
size: S (25)
difficulty: medium (45)
---

# 2048 and Omega: flat meshes, what is left

The maintainer's report ("some meshes don't have any textures, but some do") is closed: cause 1
(the Vita reader takes its record's own declaration), cause 2 (Zone road and walls are flat colour by
design), cause 3 (every unbound draw on every 2048 and Omega circuit is a material that names no
texture, after two fixes of ours: a truncated Vita texture path, Omega's micro-tiled BC1 terrain
grass) and Omega's type-0 `Uv1` are all in `docs/formats/2048-rcsmodel.md` ("The flat-submesh
census, and the record pointer", "What `bind_textures` leaves without a picture, by exit") and
`docs/formats/gnf.md` ("BC1 under `TileMode(13)`"). Guards:
`crates/render/tests/omega_transparency_ground_truth.rs` (`only_by_design`),
`crates/rcs/tests/psp2_flat_census_ground_truth.rs`.

## Open

1. **235 (`data00`) and 432 (`data04`) Omega submeshes have a diffuse-family sampler but a
   declaration with no coordinate**: unexplained, so they draw flat with a texture bound.
2. **No-texture materials draw the white texel**, though they author a colour parameter
   (`fc05_simplecolouremissive` carries one four-float parameter). Flat is by design; whether the
   colour is the parameter or the vertex colour is unmeasured.

## Next Steps

1. Name the 235 and 432 submeshes' materials (the census already finds them) and read what their
   shader programs sample.
2. Read `fc05_simplecolouremissive`'s parameter against a frame of the original (Empire Climb,
   `tower`) before painting anything with it.
3. Re-render the 2048 and Omega compare-page pairs after merge (the lead does): every Vita
   circuit's loader line changed, and Omega `park`, `arena`, `square`, `tower` draw their grass.
