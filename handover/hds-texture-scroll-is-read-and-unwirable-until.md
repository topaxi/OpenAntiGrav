# HD's texture scroll is read, and unwirable until the additive emissive layer exists

2026-08-31. The mesh half of "animate HD" landed the same day and is gone from
this list; this is what is left. **HD authors no texture-transform keyframe
block**, and that is where the block lives rather than a gap: Pulse keeps one
`0x40`-byte block per material inside the mesh payload, and a PS3 `Mesh`
payload is a bounding-box pair and a chunk hash. The animation moved into the
fragment program, and it is read at instruction level -
[rcsmaterial.md](../docs/formats/rcsmaterial.md), "A surface scrolls off an
engine `time`", with the disassembly and the census.

**The mechanism.** Unit 1 is sampled at `(u, (v + a) * b + time)`, `a` and `b`
the material's own authored floats and `time` an engine parameter
(`0x906b67ba`, a preimage over `EBOOT.elf`); the result is **added** to the
diffuse, gated by the diffuse alpha. On `mt_uvanim_diffuse_emissive2` the
floats are `0` and `-1`, so the layer scrolls one texture unit per second in
`-v`. **310 of the disc's 1,186 `.rcsmaterial` declare `time`**, and a
circuit's own table runs from 4 of Sol 2's 442 slots to 101 of Modesto
Heights' 859 (`crates/render/examples/hd_uv_time_census.rs`).

**Why it is not wired, which is the whole point of this row.** `mesh.wgsl`
samples the second texture at the diffuse coordinate and *selects* between the
two; this family **adds** one to the other, `MAD H0.xyz, H0.wwww, H1, H0` -
albedo plus diffuse-alpha times the tinted emissive sample. So a scroll has
nothing to scroll until that layer exists, and wiring the clock first would be
motion nobody can see.

**The blocker is smaller than it first looked, and the distinction matters.**
What this needs is an `ADD_SECOND` role bit - the same shape as the
`ALBEDO_FROM_SECOND` and `ALPHA_FROM_SECOND` that `mesh::slots` already carries
and `mesh.wgsl` already reads - **not** the per-material *lighting* branch
[hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
records, where two general classification rules were tried and refuted. Those
rules failed at telling a lit surface from an emissive one; this is a different
question, asked of one instruction shape.

**The census that decides it has now been run** (`scripts/hd_time_shapes.py`,
which tracks the patch chain to the instruction reading `time` and then taints
forward through the stream), and **the answer is uniform**:

| | Count |
| --- | ---: |
| Materials with a fragment block reading `time` | 291 |
| **Of those, taking it into a texture coordinate** | **290** |
| Blocks reading `time` | 2,305 |
| **Of those, combining the sample by accumulate** | **2,230 (96.7 %)** |
| Combining by multiply | 71 |
| Neither | 4 |

**The 35 materials that are not *purely* add-shape are not a counter-example.**
Every one of them mixes add-shape blocks with multiply-only ones, which is the
*pass* dimension rather than a second combining rule - a material ships a
shadow and a depth variant beside its lit one, and `skin::variants` already
resolves which is which. Exactly one material on the disc, `hd_waketrail`, is
multiply-only throughout.

So this is one coherent change with a pixel to verify it, not the classifier
problem that killed `output_lit_by`.

**A withdrawn recommendation.** This row used to say `uvScale`
(`0xea1dcc4c`) / `uvOffset` (`0x1eb13436`) - the disc's two commonest
parameters - should land first, because "a surface carrying one tiles wrongly
today". **The values say no**: 2,510 of 2,582 records author the identity,
`uvScale` is `(1, 1)` on every record on the disc, and about fifty billboard
surfaces carry a real sub-tile offset
(`crates/render/examples/hd_uv_transform_census.rs`). The operation itself is
confirmed - `uv * uvScale + uvOffset`, read off
`simpletextureandtexturealphauvoffsetscale`'s vertex block - so this is a
correction about *magnitude*, not about the reading.

`crates/render/examples/hd_param_names.rs` is the sweep: it named 64 of the
300 parameter hashes off `EBOOT.elf`'s own strings. The `.rcsmaterial` files
are no corpus at all here - their tables store hashes and never strings, so 0
of 300 preimage against the whole shader library.

## Open

- The additive emissive second-texture layer does not exist in `mesh.wgsl`,
  and the `time` scroll cannot show until it does. The census above says it is
  one change rather than a classifier problem.
- The scroll is computed **per pixel** on `f[TC3]`, not per vertex, so it
  cannot ride `slots::FLIP_V`'s CPU-at-build-time path: it needs `scene.time`
  in the shader and the material's two floats as per-material data.
  `Model::material_slots` is a `u32` bit field with no room for them.
- `uvScale`/`uvOffset` are read and unwired, and are **not** worth doing first:
  2,510 of 2,582 records author the identity.
- Whether the renderer's texture coordinate is the same attribute
  (`0x7a3f521c`) the `uvOffset` vertex program transforms is not established.
  Applying a transform to the wrong set breaks surfaces that work today, which
  is the failure mode `ALBEDO_FROM_SECOND` was made additive-only to avoid.
- 236 of the 300 parameter hashes have no preimage. The named ones are
  artist-facing (`Colour`, `Speed`, `Brightness`); the unnamed ones cluster on
  one material each.
- Whether the engine's `time` is the race clock, a free-running clock, or
  something that pauses is not established - `scene.time` in `mesh.wgsl` is
  bound from `mesh_render::Scene::time` and only the flame path reads it.

## Next Steps

1. ~~Run the instruction-shape census over the 310.~~ Done - uniform, see above.
2. Now: `slots::ADD_SECOND` plus the emissive tint
   (`0xe8bcd7f5`), then the scroll on top, verified with `model_probe` and a
   two-clock capture the way
   `crates/render/tests/hd_scenery_animation_ground_truth.rs` does it.
3. `uvScale`/`uvOffset` are not on this list any more; see above.
