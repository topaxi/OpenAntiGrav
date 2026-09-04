# The shadow setting, `blob` and Pulse's `original` tier are all still unbuilt

2026-09-03. Split off
[shadows-are-planned-and-pulses-occluder-payload-closes.md](shadows-are-planned-and-pulses-occluder-payload-closes.md)
once that thread's RE half (steps 1 and 4 of
[`docs/rendering/shadows.md`](../docs/rendering/shadows.md)'s plan) closed -
per this project's own rule, an RE thread that unblocks implementation gets
a separate thread for the implementation rather than bundling both kinds of
work into one sitting. **Nothing in this file's scope is built.** The design
page exists so the enum and the menu row are not written twice; nothing has
written them yet.

## What's decoded and ready to build on

Everything step 5 (`original` on Pulse) needs is done, at confidence 84
(Probable - decompilation-only, not runtime-verified):

- The `Dynamic Shadow Occluder` (`0x3c3`) payload format:
  `crates/formats/tests/shadow_occluder_ground_truth.rs`. `n` 32-byte face
  records (unit plane normal, vertex count, indices) and `m` 16-byte vertex
  records, closing at `0x50 + 32n + 16m`. 129 nodes on Pulse, 119 of them
  local-space and named (a craft's or weapon's own shadow hull), 10
  world-space and unnamed (a track feature, still not confirmed which one -
  see Open).
- The runtime reader/renderer,
  [`Shadow_RenderOccluderVolume`](../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md)
  (`0x089038c8`): derives its own projection direction from the occluder's
  local axis transformed by its own world matrix (never a light - Pulse has
  none to read), extrudes a silhouette-edge stencil shadow volume from the
  `n`/`m` records, and is the same function that draws the craft's own drop
  shadow.
- The static class dispatch:
  `DynamicShadowOccluder_RegisterClass` (`0x0890446c`) registers `0x3c3`
  and installs the method table that names the reader, byte for byte.

None of this is a rendering implementation yet - it is what the original
game does, read from its own binary and its own disc data. `oag-render` has
no shadow pass of any kind today.

## The plan, and what's actually next

[`shadows.md`](../docs/rendering/shadows.md)'s six-step plan (step numbers
match that page exactly):

1. ~~Make the census durable~~ - done, `shadow_occluder_ground_truth.rs`.
2. **The setting, with only `off` and `blob` live** -
   `crates/game/src/display/shadows.rs`, its own file per the pattern
   `reconstruction.rs`/`motion_blur.rs` already set, tests in
   `display/tests.rs`, a `graphics.shadows` key, a menu row on the GRAPHICS
   page. `original` and `mapped` are **not** offered until they exist.
   **This is the actual next step** - nothing past it can exist without it.
3. **`blob`**: a quad under each craft, sampling HD's own `ambient_shadow.gtf`
   where the title has one and a generated falloff where it does not,
   ray-cast onto the track ribbon for height and orientation. Plumbs into
   [`draw-order.md`](../docs/rendering/draw-order.md)'s queue - the step
   that costs more than it looks, per the design page's own estimate (two
   days).
4. ~~Decode the occluder's two record arrays, and read the runtime handler~~ -
   done, both halves, see above.
5. **`original` on Pulse: draw the 119 local-space hulls.** Geometry and
   projection are both decoded now. HD's and 2048's own `original` tiers are
   separate work (HD's shadow-map job pipeline, 2048's
   `track_proximity_shadow` pair) and not in this thread's scope - Pulse
   only, since that is what's unblocked.
6. ~~Measure 2048~~ - done, and it moved the design; see the RE thread.
7. `mapped` - opt-in cascaded shadow map, this project's own enhancement,
   not attempted here. Wants `wgpu::Features::DEPTH_CLIP_CONTROL` for its
   caster pass (probed and intersected the way `TEXTURE_COMPRESSION_BC`/
   `TIMESTAMP_QUERY` already are in `crates/render::mesh_render::optional_features`;
   supported on all three backends this project requests, so the probe is a
   formality here) - not needed before step 7 starts, which is not this
   thread's scope either.

So the actual order of work in this thread is **2, then 3, then 5** - the
setting has to exist before either drawn tier can be selected, and 3 and 5
don't depend on each other.

## Open

- **The `m` face-to-vertex index mapping's exact byte layout.** `Shadow_RenderOccluderVolume`
  reads the `m` vertex records positionally when building its silhouette
  edge list, but the precise index/winding relationship between a face
  record's vertex count (`+0x0c`) and which of the `m` vertex slots it
  claims was not worked out in the RE pass - see
  [`shadow-occluder.md`](../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md)'s
  own Open section. Worth resolving before step 5's draw code needs it,
  not after - drawing a plausible-looking but wrong winding is exactly the
  kind of invented stand-in this project's own rule (`CLAUDE.md`, "never
  invent what the assets already author") warns against.
- **Whether the 10 unnamed/world-space occluders are a track feature at all**,
  or authored-and-inert the way `DirectionalLight` turned out to be. Their
  bbox extents run to 882 units, so they are not craft-scale, but nothing
  confirms they are ever drawn rather than dead data. If step 5 only draws
  the 119 named/local-space population and leaves the 10 world-space ones
  unrendered pending this answer, say so in the implementation rather than
  silently dropping them.
- **Whether `original` should be the default once it exists**, on a title
  where the tier is not obviously better-looking than `blob`. A design
  question, not a technical one - needs eyes on both side by side once both
  are built.
- **The padded-bbox field's exact selection rule is still not fully
  explained** (see the RE thread's own Open list) - `Shadow_RenderOccluderVolume`
  reads it regardless, so this does not block drawing, but a future pass
  that wants to reproduce the original's behaviour exactly rather than
  approximately will want it resolved.

## Next Steps

- **Step 2**: `crates/game/src/display/shadows.rs`, the `graphics.shadows`
  setting (`off`/`blob` only to start), a menu row on GRAPHICS, tests in
  `display/tests.rs`. This is the one everything else waits on.
- **Step 3**: `blob` - a quad per craft, `ambient_shadow.gtf` where present
  else a generated falloff, ray-cast onto the track ribbon, plumbed into
  `draw-order.md`'s queue.
- **Step 5**: `original` on Pulse - draw the 119 local-space
  `Dynamic Shadow Occluder` hulls using `Shadow_RenderOccluderVolume`'s own
  projection math (own local axis, not a light; extrude a stencil volume
  from the `n`/`m` records). Resolve the `m` face/vertex indexing gap above
  first if the draw code needs the winding to be right rather than merely
  plausible.
