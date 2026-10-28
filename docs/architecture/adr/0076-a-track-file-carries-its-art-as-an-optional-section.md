# ADR-0076: A track file carries its art as an optional section

## Status

Proposed (2026-10-29). Extends [ADR-0063](0063-the-own-track-format-is-glb-plus-a-toml-manifest.md),
which is immutable: its decision 5 ("absent means absent", adding sky, fog, lights and art
as optional records) is what this carries out. The format is unpublished, so the change
stays inside major version 1 and the schema gains keys rather than a new kind.

## Context

A track file races on flat-shaded collision: blue floor, red walls, a black sky. Everything a
disc circuit shows beyond that lives in its `.vex` blob and is read by the race loader, not by
the simulation: art meshes with their materials and textures, the sky, fog volumes, lights.
An author of a custom track wants to draw with the same things Pulse and Pure draw with,
including textures that already ship in a Pulse circuit, without this project committing any
of them.

The renderer's inputs are the spec. For a Pulse or Pure circuit it takes an `oag_mesh::mesh::Model`:
vertices of position, normal, colour, texcoord and a `lit` flag; draw calls that name one
texture slot, a blend class, a culled flag and bounds; `Texture` nodes that are named and
embedded in the `.vex`; `oag_vex::fog::FogVolume`s and `oag_vex::lighting` lights. A track's
vertex colours are **baked lighting**, so for these two titles `COLOR_0` is the main light and
the light records mainly reach the ships. Pure's sun lens flare (wired 2026-10-26) is the one
authored-and-drawn item that is neither mesh nor light: a `lensflare` node of a direction and a
colour, with the drawing owned by the executable.

## Decision

1. **`Track` gains `art: Option<Art>`**, a field kept apart from every simulation field. The
   simulation never reads it, so a world hash cannot change with it, and a track with no art
   is what 1.0 wrote, byte for byte.
2. **`Art` mirrors what the renderer takes, not glTF PBR.**
   - Meshes are glb nodes `oag/art/<n>`: `TRIANGLES`, `POSITION`, `NORMAL`, `TEXCOORD_0`,
     `COLOR_0` (the baked light, linear RGBA). One primitive per draw.
   - A primitive's material is extras on its node: `oag_texture` (an index into the manifest's
     texture list, absent for untextured), `oag_blend` (`opaque`, `cutout`, `alpha`, `additive`,
     `unblended`; the classes `oag_vex::vex::BlendClass` already names), `oag_culled`, `oag_lit`.
     A stock glTF material on the node is ignored and the node is counted in `Loaded::unread`.
   - Textures are `[[art.texture]]` rows in the manifest, each exactly one of:
     - `image = "name.png"`: the author's own picture, a file beside the manifest or a glTF
       image in the glb. This is the only form a committed fixture may use.
     - `host = { entry = "Data\\Tracks\\03_Track.vex", name = "..." }`: a `Texture` node of a
       circuit on the host title's disc, resolved when the race loads. The track file holds the
       **name**, never the pixels, so no disc content travels with it.
   - `[art.sky] from = "<circuit .vex entry>"` plays that circuit's authored `Skycube`
     through the loader's existing sky path. `[[art.fog]]` and `[[art.light]]` rows carry the
     numbers of `FogVolume` and the ambient, directional and point lights.
   - **`[art.sun]` is the track's one real light**: a unit `direction` (towards the sun), a linear
     `colour`, an `intensity`, an `angular_diameter` in degrees (0 is a point sun, a hard edge)
     and `shadows` (whether it may cast). The ships, the `mapped` shadow tier and the lens flare all
     read this one record, so they cannot disagree about where the sun is. The `ambient` light
     is its own `[art.ambient]` colour. Point lights stay `[[art.light]]` rows.
   - `[art.lensflare]` (Pure's sun flare, [`lens-flare.md`](../../rendering/lens-flare.md)) is a
     switch plus the atlas: the flare is drawn along the sun's direction in the sun's colour
     (`colour` overrides it), and the atlas is a host reference to a circuit's
     `LensflareComponents.tga` or the author's own 64 x 64 picture. The ten-ring layout, the fade
     by angle and the occlusion probe stay the renderer's law, so a track that names a flare gets
     Pure's flare exactly and one that names none gets none.
3. **A host reference that does not resolve draws nothing for the material and says so**,
   by entry and name, in the loader report. No stand-in texture and no neighbour's picture
   (CLAUDE.md, "Never invent what the assets already author"). A track with no art at all keeps
   today's flat-shaded collision and its report names which of the parts are absent.
4. **Art floats are decimal in the manifest or binary in the glb as each belongs.** Mesh data
   is binary accessors as before. Fog and light numbers are TOML floats: nothing simulates
   them, and Rust's shortest-round-trip `f32` text is exact.
5. **No trigonometry in `oag-trackfile`.** It is in `check-determinism`'s scan, so a generated
   dome or any curve sampling belongs in the Blender add-on or on the render side.
6. **Where it lives**: `oag_trackfile::art` (the types) and its own glb module (`glb.rs` is
   already 790 lines); `oag_raceplay::foreign::art` builds the `Model`, sky, fog volumes and
   lights from it, on the foreign path in `track_inputs::resolve`. `oag-mesh` and the
   render-side crates learn nothing of the format (rule 3 of `just check-deps`).
7. **Proof is an equality, not a screenshot.** The local disc exporter writes a circuit's art
   as an `Art`; the `Model` built from the track file must equal the `Model` built from the
   `.vex` in vertices, indices and draws, for every Pulse and Pure circuit, as one more check
   inside the existing `trackfile_ground_truth` walk.
8. **Titles.** Pulse and Pure first: one `.vex` model path. 2048 and Omega share the
   `.rcsmodel` material model and HD its shader table; those are *checked, applies, not wired*
   until a lane opens for them, and the track-file page says so.

What stays absent in 1.x after this (named in every loader report): clouds, scenery and its
animation, track particle emitters, sound emitters, visibility sets (PVS), shadow hulls and the Zone
variant. A track without PVS draws every art mesh every frame, which is a cost to measure, not a
reason to author visibility by hand.

## Consequences

- **A sun on our own tracks is what `docs/rendering/shadows.md` lists first among the three things
  ray-traced shadows are blocked on**: "a light to trace against", where Pulse has none and the
  direction would have to be invented. A track file authors one, with a size, so a `mapped`
  shadow can be soft today and a traced one needs no format change later. The other two blockers
  (ray queries in wgpu, a reason beyond contact shadows) are untouched, and this ADR adds no
  `raytraced` variant: that row waits for its infrastructure.
- An art mesh may be flagged a **caster** or a **receiver** (`oag_shadow`: `both`, `cast`,
  `receive`, `none`; default `both`), because a sky dome or a decal that casts is wrong and the
  author is the only one who knows. Collision geometry casts and receives nothing by itself.

- A custom track can look like a Pulse circuit by naming its textures and sky, with a disc
  required only at race time, as the host title's ship and physics already require.
- The format is wider than 1.0 and bound to what Pulse and Pure's renderer can draw. A later
  title's richer materials are a minor-version addition, not a reason to hold this one back.
- The Blender add-on grows an art export (untagged meshes, vertex colours, image textures,
  host-texture picks); that is the second landing and is tracked in the roadmap, not here.

## Alternatives considered

- **Embed disc textures in the glb.** Simpler to load, and it turns every exported circuit
  into a redistributable copy of the game's art. Rejected: the leakage rule.
- **Write a `.vex` blob.** The loader would need no new path, and the format would inherit a
  node tree nobody should author by hand. Rejected: ADR-0063's point was a form to be authored in.
- **glTF PBR materials.** Blender and stock viewers would show something, and the renderer
  would still need a mapping to blend classes and baked light it cannot infer. Rejected.
