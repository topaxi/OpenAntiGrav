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
two; this family adds one to the other. So a scroll has nothing to scroll
until the additive emissive layer exists, and that is exactly the per-material
shader path
[hd-needs-a-per-material-shader-path-and.md](hd-needs-a-per-material-shader-path-and.md)
records as missing, where two general classification rules have already been
tried and refuted. Wiring the clock first would be motion nobody can see, and
would make the missing layer harder to notice rather than easier.

**A second, cheaper thing came out of the same sweep and should land before
any of it.** `uvScale` (`0xea1dcc4c`) and `uvOffset` (`0x1eb13436`) are the
**two commonest parameters on the disc**, 1,364 uses each, always paired and
always in equal number - `uv * uvScale + uvOffset` is the static coordinate
transform every material gets, and **nothing reads either**. A surface
carrying one tiles wrongly today, before anything animates. That is a
static fix with a picture attached, and it is the honest first step.

`crates/render/examples/hd_param_names.rs` is the sweep: it named 64 of the
300 parameter hashes off `EBOOT.elf`'s own strings. The `.rcsmaterial` files
are no corpus at all here - their tables store hashes and never strings, so 0
of 300 preimage against the whole shader library.

## Open

- `uvScale`/`uvOffset` are read and unwired, and they are static: a surface
  carrying one tiles wrongly today.
- The additive emissive second-texture layer does not exist in `mesh.wgsl`,
  and the `time` scroll cannot show until it does.
- 236 of the 300 parameter hashes have no preimage. The named ones are
  artist-facing (`Colour`, `Speed`, `Brightness`); the unnamed ones cluster on
  one material each.
- Whether the engine's `time` is the race clock, a free-running clock, or
  something that pauses is not established - `scene.time` in `mesh.wgsl` is
  bound from `mesh_render::Scene::time` and only the flame path reads it.

## Next Steps

1. Wire `uvScale`/`uvOffset` on the CPU at build time, the way `slots::FLIP_V`
   already is - it is a property of the material, not of the pixel - and
   measure how many of a circuit's chunks move with `model_probe`.
2. Only then take the additive emissive layer, and only then the clock.
