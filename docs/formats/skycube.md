# `Skycube` and `fogCube`: a track's sky and fog

The two environment classes a Pulse circuit always authors. `Skycube` `0x3c6` is
the sky; `fogCube` `0x3d3` is the fog volume and its parameters.

Before this page, [`vex.md`](vex.md) recorded only that those IDs *have* those
names, from the class-ID table. No byte of either payload was documented, no
handler address was recovered, and a race in this engine drew a black sky.

Everything here comes from shipped data rather than from the executable. The
handlers are still unrecovered - see [Open](#open) - which is why the confidence
scores below are about *what the data is*, not about how the original draws it.

## Summary

| Claim | Confidence |
| --- | --- |
| A `Skycube` payload has the same layout as a `Mesh` `0x125` payload | **90** |
| Every track file authors exactly one, parented to the world node | **95** |
| The sky's textures are the track file's own, by the same ordinals | **90** |
| A `fogCube` payload is a 64-byte 4x4 then two `{rgb, 0, near, far}` sets | **92** |
| `Data\Defaults\Skycube.vex` is an unreferenced version-4 legacy asset | **90** |
| How the original draws either one | **not recovered** |

Validated by `crates/formats/tests/skycube_ground_truth.rs` against all 40
`Skycube` and 36 `fogCube` nodes on the PSP disc (`just test-data`).

## `Skycube` is a `Mesh`

The payload is byte-for-byte the same shape as a `Mesh` node's:

| Offset | Field |
| --- | --- |
| `+0x00` | `u16`, unread; `u16` **material count** at `+0x02` |
| `+0x04` | `u32` offset to batch list A |
| `+0x08` | `u32` offset to batch list B, `0` on every sky |
| `+0x0c` | `u16` unread, `u16` `0x00ff` |
| `+0x10` | bounding box min, three `f32` and a zero |
| `+0x20` | bounding box max, same shape |
| `+0x30` | material array, stride `0x14` |

So [`vex::mesh_materials`] and [`vex::mesh_batches`] decode it **unchanged**, and
`oag_render::mesh::build_sky` is `build_with_textures` pointed at a different
class id rather than a second decoder.

The evidence is the arithmetic, not the resemblance. On `01_Track` the material
count at `+0x02` reads 6, which puts the array's end at
`0x30 + 6 * 0x14 = 0xa8`; `+0x04` reads `0xb0`, the next 16-byte boundary. A
wrong count field or a wrong stride does not land on the alignment boundary of
the offset stored four bytes away. The ground-truth test turns that into a check
over all 40 nodes, together with three constraints a near-miss cannot satisfy at
once: every material's texture index is inside the file's own `Texture` count,
every bounding box is a real box, and every decoded vertex falls inside the
bounds its own batch declares.

### What the shipped skies contain

40 `Skycube` nodes across the PSP disc's `.vex` files, payloads 3872 to 8432
bytes. Material counts:

| Materials | Files |
| --- | --- |
| 6 | 6 |
| 5 | 22 |
| 1 | 12 |

**These are not face counts.** A material is a texture run, and geometry is 474
to 553 triangles per sky whichever group it falls in - not the 12 a bare cube
needs, because the faces are subdivided, which is what the PSP's affine texture
mapping requires to keep a large textured quad from swimming. The reading that
suggests itself - six materials is a full cube, five the same cube without the
floor - is a **hypothesis with nothing in the payload behind it**; see
[Open](#open). The twelve 1-material files are the Zone variants.

**Every sky is parented to node 0, the world node.** Nothing has to compose a
transform hierarchy to place it, which is consistent with it being drawn relative
to the camera rather than to the scene.

### `Data\Defaults\Skycube.vex` is not the sky

The disc carries this file - entry 58 of `Data.wad`, hash `7b245803`, 405,488
bytes - and its name is a real string in `BOOT.BIN`. It is **not** what a track
draws:

- It is **format version 4** in a version-6 archive, so its class IDs are the
  older numbering (`0x0ee` world, `0x373` texture, `0x378` shape). A version-6
  `CLASS_SKYCUBE` cannot appear in it.
- Its Maya source path is `Z:/Data/Defaults/skycube.mb`, its one shape is
  `skycube1_nolightShape`, and its six textures are
  `Z:\Art_Resources\Skies\Default\Textures\skybx1..6_nomip.tga`.
- Every shipped track carries its own sky inline instead.

It is worth recording for two things it corroborates independently of any
decode: the class name means a **literal six-faced cube**, and the sky is
**unlit** (`_nolight`) and **unmipped** (`_nomip`). The `Skies\Default\` path
also implies non-default sky texture sets, which is what the per-track skies are.

### One shipped sky animates: Vertica's

**Confidence 85, measured 2026-08-11.** A `Skycube` payload being a `Mesh`
payload goes further than the decode: it also means a sky can carry the
per-material [texture-transform keyframe block](vex.md#the-texture-transform-keyframe-block-at-0x30--material_count--0x14),
and exactly one does. `06_Track`'s (Vertica, both layouts - the forward and
reversed files share the circuit's sky) drifts its texture one whole tile
**diagonally**, `(0, 0)` to `(255, 255)` over key times 0..1997, on a
**33.3-second** loop, across 71 of the sky's 494 vertices. A slow cloud layer,
and by a wide margin the slowest authored track on the disc - the next slowest
is 10 s.

Swept over all 40 shipped sky nodes; the other 39 author nothing. Worth
recording because "the sky is static" is the natural assumption, is what this
project's own test asserted first, and is wrong for one circuit.

It also matters as a decode check in the other direction. Arbitrary bytes read
as a *plausible* keyframe block often enough to be dangerous, so a sky picking
one up is exactly what a mis-parse looks like. What separates the two is the
material's `& 0x10` flag, which is the engine's own gate
(`Mesh_UpdateTextureTransforms`, `0x0890e160`) and which
`oag_formats::vex::mesh_tex_transform` now applies:
`crates/render/tests/authored_uv_ground_truth.rs` asserts that no *other* sky
animates and that this one's period stays in cloud-layer territory rather than
sliding the horizon.

## `fogCube`

128 bytes on every track that authors one, and the layout is legible on sight:

| Offset | Field |
| --- | --- |
| `+0x00` | 64-byte row-major 4x4, same convention as `Transform` |
| `+0x40` | `f32[3]` colour, `f32` zero, `f32` near, `f32` far |
| `+0x58` | a second set of the same six floats |
| `+0x70` | `f32` overlap tiebreak - smallest wins when volumes nest |
| `+0x74` | `f32` `500.0` on every track: the **cube's edge length** |
| `+0x78` | eight zero bytes |

The matrix's last row is a position with `w == 1.0`, which is what identifies the
first 64 bytes as a matrix rather than as more parameters. Its rows 0 to 2 are
**not** orthonormal - each carries its own scale, and on some tracks a different
scale per axis - so the node is a *box volume*, which is what the class name says
it is.

Per-track values, as read:

| Track | Colour | Near | Far |
| --- | --- | --- | --- |
| `01` | 0.173, 0.193, 0.264 | 250 | 1000 |
| `03` | 0.946, 0.893, 1.000 | 30 | 2000 |
| `05` | 0.796, 0.545, 0.349 | 165 | 1000 |

**These corroborate the sky decode independently.** `05_Track`'s sky is a red
sunset skyline and its fog is warm orange; `03_Track`'s is near-white. Two
decoders that were derived separately agree about what each circuit looks like.

Two findings a loader has to handle:

- **`06_Track` authors no `fogCube` at all** - 36 of 40 files have one. Asserted
  in the ground-truth test rather than assumed, so it surfaces as a count that
  moved if it was ever a name-resolution miss.
- **The two colour/near/far sets are byte-identical on 28 of the 36** and differ
  on the rest, `01_Track` among them. **They are the two ends of a linear
  interpolation across the volume's local Z**, so colour, near and far all slide
  as the camera moves through the box - identical sets mean uniform fog, differing
  sets mean graded fog. Recovered from `FogCube_Sample` (`0x089069b0`); see
  [`fog.md`](../ghidra/functions/psp-pulse-usa/fog.md).

## What this engine does with it

`oag_render::mesh::build_sky` builds the `Skycube` into its own `Model` from the
same track blob, indexing the same textures. `race::Scene` draws it **first** in
the existing pass, with `mesh_render::Depth::Sky` - `depth_compare: Always`,
`depth_write_enabled: false` - and a model matrix of
`Mat4::from_translation(camera_position())`.

Three consequences worth stating, because each replaces something that looks
obvious and is wrong:

- **The sky is not scaled to the far plane.** Its authored cube is tens of units
  across while a track is thousands. Riding on the camera means the faces never
  approach, so the size never matters.
- **It is not a clear-colour change.** The race pass's black clear covers the
  whole attachment while the scene draws into a sub-rectangle, so that colour is
  what the letterbox bars are made of.
- **It is exempt from both culling tiers**, which is what
  [ADR-0011](../architecture/adr/0011-authored-pvs-before-frustum-culling.md)
  already assumed for a skybox.

Fog **is** implemented. `oag_formats::fog` decodes the volumes and reimplements
`FogCube_Sample`; `race::Scene` samples them at the camera each frame and writes
`oag_render::mesh_render::Fog` into bind group 2, which `mesh.wgsl` applies as
the same linear ramp `Gu_Fog` sends. The sky is deliberately left unfogged - it
rides on the camera at a radius of 18 to 62 units while fog starts at 30 to 250,
so fogging it would drown it, and the original's sky geometry is authored
`_nolight` and stands in for infinity.

**One known divergence:** the shader measures *radial* distance from the eye
where the hardware uses view-space depth. The two differ by up to `1/cos(fov/2)`
- about 15 % at the screen corners. The view matrix is not in the uniform block
that would be needed to fix it; recorded rather than silently accepted.

## Open

- **Neither handler is recovered.** No address, no method table. The route is the
  46 `Vex_RegisterClass` (`0x08908eb8`) call sites, alphabetical by class name
  with method tables `0x88` apart - see [`vex.md`](vex.md) and
  [`exhaust.md`](../ghidra/functions/psp-pulse-usa/exhaust.md). Note that
  `engine_fire`, `exitglow` and `gate` have no registration site yet are
  authored, so `Skycube` may have none either.
- **The GE state the original draws the sky with** - depth func, depth range,
  blend, cull, whether the sky is itself fogged. The depth handling here is
  chosen to work, not measured. `Trail_BuildStateList` is the worked example of
  recovering this from a display list.
- **The extra block some skies carry.** `06_Track`'s sky puts `0x1a8` bytes
  between the material array and the geometry: six `0x40`-byte records holding
  stale PSP main-RAM pointers (`0x080db6c0`). Stepped over, not decoded.
- **What separates the 1-, 5- and 6-material skies.** The obvious guess is face
  count - five being a cube with no floor - but triangle counts do not vary with
  it and no payload field distinguishes the groups. Comparing each group's
  texture content, or reading the geometry's face normals, would settle it.
- **The float at `+0x70`.** `Fog_FindVolume` picks the smallest when volumes
  nest, and the shipped values (1.6e9, 2.5e9, 7.8e9) are large enough to be a
  volume or a squared extent - but the units are not established.
- ~~The fog curve.~~ **Resolved.** The GE ramp is linear within a frame, and its
  parameters are re-interpolated every frame from where the camera sits in the fog
  volume. A linear ramp is correct and must not be "fixed" into a curve. See
  [`fog.md`](../ghidra/functions/psp-pulse-usa/fog.md).
- **`cloudCube` `0x3d8`, `cloudGroup` `0x3d9` are undecoded.** Present in the
  census - `05_Track` alone authors 5 clouds and 3 cloud groups.
- **`weatherPos` `0x3da`'s registration is found, and a live PPSSPP capture
  confirms its runtime constructor actually runs, loading a real race.**
  `WeatherPos_RegisterClass` (`0x0892c684`) registers class `0x3da` like every
  other environment class; the value it installs at the descriptor's `+4` is
  not a handler but a class-identity tag (the mechanism `FUN_08a71364`'s
  per-class node gather actually reads). `FUN_0892c404`, which allocates and
  tags a real `weatherPos` instance, has no static call site anywhere in the
  binary - and a breakpoint on it fires anyway, during Talon's Junction's
  race load, called through a generic per-class spawner
  (`FUN_08908f98`) resolving `weatherPos`'s own `+0x7c` `init` slot at
  runtime, the same layout `exhaust.md` documents for other classes. So the
  class is genuinely constructed, not dead code. What none of that says: which
  of `WO_RAIN`/`WO_SNOW`/`WO_MODESTO_STEAM_A`/`WO_BLUE_WELDER` a given
  instance carries, or whether a track's 1 to 21 zero-length-payload
  instances share one effect or several do - still open, detail in
  [`weatherpos.md`](../ghidra/functions/psp-pulse-usa/weatherpos.md).
- **PS2 parity.** Not checked. Pulse's class IDs do not carry to Pure, and the
  PS2 build's numbering is unconfirmed.

**PS2 rendering was broken until 2026-08-07: `build_sky` never received the
external texture set, so every PS2 sky drew white.** A PS2 `.vex` embeds no
texture block at all (see [`ps2-texture.md`](ps2-texture.md)), and
`oag_render::mesh::build_sky` hardcoded `external: None`, unlike
[`build_with_textures`] which the track model already resolved an external set
for. The `Skycube`'s materials name a texture by its ordinal among *all* of the
file's `Texture` nodes, the same ordinal space the track mesh and the speedup
pads use - see `ps2-texture.md`'s "How a model finds its texture set" - so the
fix is passing the *same* resolved set into `build_sky` (and `build_pads`,
which had the identical bug) rather than resolving a second one. Confirmed on
`03_Track` (Moa Therma, the reported case): white sky before, textured sky
after, `just view --sky` gives the identical result standalone. Also confirmed
on the two circuits with a texture set short by 1-2 slots
(`02_Track`, `12_Track`) - the missing ordinals are not among the sky's own, so
both skies draw fully textured despite the track's own art meshes reporting a
couple of undecoded slots.
