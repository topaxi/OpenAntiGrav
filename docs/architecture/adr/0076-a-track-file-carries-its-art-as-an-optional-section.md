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
     `COLOR_0` (the baked light, linear RGB plus coverage alpha). One primitive per draw.
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
     (`direction` and `colour` override them, each written only where it differs), and the atlas is a host reference to a circuit's
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

9. **Static shadows are baked in Blender, into `COLOR_0`.** The add-on's sun lamp is `[art.sun]`,
   and a *Bake lighting* operator (Cycles, headless-capable) writes sun, sky and bounce light with
   its cast shadows into each art mesh's vertex colours, the way Pulse and Pure light their own
   circuits. The format stores the result and never the method, so the engine needs no baker. It
   does not double-count: the `mapped` tier casts only what moves (`shadows.md`, since
   2026-10-25), so a baked scenery shadow and a live ship shadow land on the road once each.
   Vertex-colour bake is limited by mesh density (the operator reports the vertices per square
   unit it baked at); a lightmap atlas, which HD's `Model::lightmaps` already draws, is a later
   minor version that adds a second UV set and a texture.

10. **Motion is keyframed in Blender and carried as glTF animation.** An art mesh node, the sun
    and the lens flare's host may each have translation, rotation and scale channels (glTF
    `animations`, which Blender's Dope Sheet writes), played against the race clock. The
    renderer already plays authored node motion (`Model::anim_nodes`, `Motion::sample`, the
    texture-transform tracks) and the `mapped` tier already casts what animates, so a moving
    mesh is a mapping onto that and a moving sun is one more channel of the same clock.
    - **Visual only in 1.x.** Collision, pads and the spline never move: a moving collider is
      simulation state, belongs in the world hash and in replays, and is its own decision.
    - **A baked light does not follow a moving sun.** Vertex colours are baked for the sun's
      pose at time zero. A sun that moves relights what is lit live (ships, the flare, `mapped`
      shadows, the sky if it reads the sun) and leaves the baked track as it is; the operator
      says so when it bakes a track whose sun is animated. Several baked sets crossfaded over
      time (a day cycle) is a later minor version.
    - Playback is the race clock in ticks, looping or clamped per animation (`oag_loop`), never
      the wall clock; `--anim-seconds` pins it as it does for a disc circuit.

11. **No trigger events in 1.x** (maintainer, 2026-10-29). Animations are time-driven against the
    race clock and nothing else starts them: "after lap one, move X" is not expressible, and the
    format carries no trigger rows. Behaviour that reacts to the race is a **mod behaviour**, the
    WebAssembly rules host decided on 2026-10-12
    (maintainer; tracked in the custom race modes and campaigns handover thread); a track does not grow a runtime of
    its own, and a later trigger table would be a minor version that adds optional rows.

12. **Settled after landing 1** (maintainer, 2026-10-29):
    - **`COLOR_0` stays linear**, as glTF reads it, and the loader converts it once to the gamma
      bytes the renderer draws (PSP: byte / 255). The disc exporter converts the other way, and
      the equality of decision 7 holds through the round trip. Blender writes linear, so its
      export needs no conversion.
    - **`oag_alpha_ref`**, an optional per-primitive extra in `0..=255`, is the alpha-test
      reference for `cutout`. Absent means the renderer's default; a disc export writes the
      disc's own (Pure's speedup pad's is 0). It is per draw, as on the disc: the renderer
      already submits `DrawCall::alpha_test_ref` per draw (`oag_mesh::mesh_render::cutout`, one
      pipeline per distinct value), so no renderer change is needed. Blender maps it from the
      material's alpha clip.
    - **`oag_layer`**, an optional integer per art node, default 0. The loader draws by layer,
      then by file order (a stable sort), so file order alone is the default case. A disc export
      writes the `.vex` layer, so its buffers match the disc build, not only its draws.
    - **`[art.sun] shadows`** is `false` on every disc export (neither Pulse's nor Pure's light
      nodes are measured as their shadow light; each title keeps its own shadows) and `true` when
      absent, so a Blender-authored track's sun is the shadow light by definition (lead, 2026-10-29,
      relayed to the maintainer).
    - **Ships read the sun through the host title's own hull law.** Pulse and Pure on the PSP build
      their GE light rig (`oag_mesh`'s `HullLights`) from `[art.ambient]` and `[art.sun]` as
      directional 0, colour times intensity, clipped to a byte as the PSP does. An HD-family host
      feeds the same sun to its own authored-light equation. No project-owned light model.
    - **A `light_scale` per spline point** (four bytes, the disc's `SplinePoint::light_scale`
      order: ambient, directional, then the two point-light bytes) is how a ship darkens in a
      tunnel. **Format 1.0 already carries it** on the spline, as the required `_OAG_LIGHT_SCALE`
      attribute of `oag/spline/<i>`: a disc export fills it with the circuit's own bytes, and a
      foreign track's renderer reads it through `oag_race::Course::light_scale_at` as a disc
      circuit's does. No world or state hash reads it (only `Course`'s render-side queries do), so
      it gains no second copy in `Art` (found in landing 2a; lead, 2026-10-29). The Bake operator
      may write it from the baked light under the centre line.
    - **A disc export carries every directional the circuit authors**: the first is `[art.sun]`,
      the rest (Pulse authors up to four) are `[[art.light]]` rows of `kind = "directional"`, so
      the hull rig built from the file equals the one built from the `.vex` (all 96 Pulse and Pure
      circuits, 2026-10-29). A colour is written as the linear value of the byte the PSP packs,
      and the loader rounds back to that byte.
    - **Pure's flare is not its first directional light**: on every Pure circuit that authors a
      flare (7 of the 8 on each of the EU and USA images, 14 of 14 flares), the flare node's
      direction differs from the first `DirectionalLight`'s. So `[art.sun]` is the first
      directional, as for Pulse, and `[art.lensflare]` carries its own `direction` (lead,
      2026-10-29, relayed to the maintainer).

13. **What a track borrows from a disc pins that disc; `titles` is the author's filter**
    (maintainer, 2026-10-29). Found when Hillside, whose art borrows Pulse 03_Track's textures and
    sky by name, raced on HD and Omega: every host texture missed, one own-PNG mesh kept the
    flat-collision fallback off, and the track drew black. The maintainer first said "The track
    shall name the title it runs on, it only loads in another title via remix", then settled the
    rule as it stands here: a host is needed only where the art borrows, and a track with only its
    own art runs on any title.
    - **`[track] host`** names the title whose disc the art borrows from, spelled as
      `oag_source::layers::slug` spells its name (`wipeout-pulse`, `wipeout-pure`, `wipeout-hd`,
      `wipeout-omega-collection`, `wipeout-2048`), the vocabulary a mod's `titles` already uses.
      **Required exactly when the art borrows** (a `host` texture or a `sky`;
      `oag_trackfile::art::Art::borrows_from_a_disc`), refused by name when missing then or empty;
      optional otherwise. The disc exporter writes its source's title. The loader refuses a value
      naming no title it knows.
    - **Recorded once per file, in `[track]`, not on each host reference**: a race opens exactly
      one track source, so every host texture and the sky must resolve from the same disc; a
      per-reference title could name two discs no race can hold at once, and would repeat one
      value on every row. `[track]` rather than `[art]` because the launch reads it to pick the
      track source before anything draws, beside the other per-track facts (`id`, `titles`).
    - **A pinned track loads only with its host's disc as the track source.** On another title it
      runs as a Race Remix: `Options::source` (collision, environment, host textures, sky) is a
      disc of the host title, never a guest standing in for it (2048 for HD), and `craft_source`
      is the title asked for. The game arranges this itself
      (`oag_game::custom_tracks::route_to_host`, run where every launch passes: the command line's
      `Pending::race_options` and the session's `launch_race`, so RACE, a campaign event, RACE
      AGAIN and `--track-file` alike), and RACE REMIX's CUSTOM takes its track source from the file's
      host directly. With no disc of the host on the machine the launch is left as asked and the
      loader (`oag_raceplay::foreign::check_host`) refuses it by name, naming both titles.
    - **`[track] titles`**, optional, lists the titles the author offers the track on; absent is
      every title. The title it is checked against is the **craft's** (the game the player is in),
      chosen, not measured; `oag_raceplay::foreign::check_titles` refuses the rest by name, and
      RACE REMIX's CUSTOM lists such a file dimmed, "offered on <titles> only".
    - **`Track::host` is `None`** for a track that borrows nothing, which includes every BallisticNG
      Workshop track mapped from its `.wtrk`.

14. **A track may carry its own sky** (maintainer, 2026-10-29), so a track with only its own art
    is complete on every title. `[art.skybox]`, never together with the host form `[art] sky`:
    - `faces`, six texture rows in the GL cubemap order HD's `sky.gtf` uses (`+X, -X, +Y` zenith,
      `-Y` nadir, `+Z, -Z`), `-1` for a face drawn as the gradient. Rows are ordinary
      `[[art.texture]]` rows, so an own face is a PNG beside the manifest; a host row would pin
      the disc like any host texture. Images inside the glb are not a texture form in 1.x.
    - `zenith` and `horizon`, linear RGB, both or neither: the colour overhead fading to the
      horizon's at eye level, by elevation (the sine of the view's angle above the horizon), and
      the horizon's below it. Chosen, not measured.
    - At least one of the two forms; a face with no texture needs the gradient. The reader refuses
      the rest by name.
    - Drawn on the camera-centred cube HD's own sky draws on (`oag_mesh::mesh::sky_cube::own`, the
      same builder as `build` for a `.gtf`), unlit, no rotation and no glow mask. A gradient face
      is an 8x8 grid so the per-vertex colour shows no crease at the cube's corners (chosen).
    - The Blender add-on writes it from the Art panel's **Own sky**: a gradient's two colours and
      one image per face. The committed `blender_loop` fixture carries one (gradient, its
      generated checker overhead), borrows nothing, and loads natively on HD with its art.

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
