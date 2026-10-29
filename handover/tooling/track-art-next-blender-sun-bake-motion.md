---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-10-29
summary: "Track art: format, hull rig (2a) and Blender art export, sun, cover light (2b) landed; next a Bake operator, motion, HD sun"
size: L (60)
difficulty: medium (50)
categories: [tooling, rendering]
---

# Track art, landing 2: Blender authoring of art, the sun, baked light and motion

Landing 1 (ADR-0076, branch `track-art`, merged 2026-10-29) put the art section in the track
file, loaded it on the foreign path and proved it against all 96 Pulse and Pure circuits
(`trackfile_ground_truth::every_*_circuit_exports_and_reads_back_the_same`, per-draw equality;
`docs/formats/track-file.md` lists what is not bit for bit and why). The sibling thread
`an-own-track-format-with-importers-into-it.md` holds the landing-1 open items (rcsmodel art for
HD, 2048 and Omega, the alpha-test reference, pad plates). This thread is the **design that was
agreed in conversation and not yet built**, so nothing is re-derived. The decisions are in
`docs/architecture/adr/0076-a-track-file-carries-its-art-as-an-optional-section.md`
(**Proposed**: the maintainer accepts it, or edits it, before landing 2 starts).

## Agreed, not built

1. **The Blender add-on writes art** (`scripts/blender/oag_track`): untagged meshes with their
   vertex colours (`COLOR_0` is the baked light), image textures (the author's own PNG, a generated
   checker for a committed fixture), **host picks** (a circuit entry plus a texture name from the
   disc, so a track can reuse Pulse's textures without carrying them), a sky picked by circuit, and
   per-mesh `oag_shadow` (`both`, `cast`, `receive`, `none`). The importer reads it back. Then
   `just blender-fixtures --update`, `crates/trackfile/tests/blender.rs`, `docs/tools/blender.md`.
2. **The sun is a Blender Sun lamp** and becomes `[art.sun]` (direction, colour, intensity,
   `angular_diameter` in degrees, `shadows`). One record feeds the `mapped` shadows, the lens flare
   and the lit ships, so they cannot disagree. Ambient is `[art.ambient]`. A real authored sun is the
   first of the three blockers `docs/rendering/shadows.md` lists for traced shadows ("a light to
   trace against"); the other two (wgpu ray queries, a reason beyond contact shadows) are untouched
   and **no `raytraced` variant is added**.
3. **A *Bake lighting* operator** (Cycles, headless-capable) writes sun, sky and bounce light with
   scenery's cast shadows into each art mesh's vertex colours, the way Pulse and Pure light their own
   circuits. The format stores the result, never the method. It does not double-count: `mapped` casts
   only what moves (`shadows.md`, since 2026-10-25). Report vertices per square unit; a vertex bake
   is only as sharp as the mesh is dense. A lightmap atlas (HD's `Model::lightmaps` draws one) is a
   later minor version: second UV set plus a texture.
4. **Motion is keyframed in Blender and carried as glTF animation**: translation, rotation and scale
   channels on art meshes and on the sun, played against the race clock in ticks (`oag_loop`:
   loop or clamp), never the wall clock; `--anim-seconds` pins it. The renderer already plays
   authored node motion (`Model::anim_nodes`, `Motion::sample`, texture-transform tracks), so a moving
   mesh is a mapping and a moving sun is one more channel. **Visual only in 1.x**: collision, pads
   and the spline never move.
5. **A moving sun does not relight baked colour.** Vertex colours are baked for the sun at time zero;
   a moving sun relights what is lit live (ships, the flare, `mapped` shadows) and the operator warns.
   Several bakes crossfaded over time (a day cycle) is a later minor version.
6. **No trigger events in 1.x** (maintainer, 2026-10-29): nothing starts an animation but the clock.
   Behaviour that reacts to the race is a mod behaviour on the WebAssembly rules host decided
   2026-10-12 (the custom race modes and campaigns thread); a track grows no runtime of its own.
7. **Moving collision** (gates, platforms) is simulation state: world hash, replays, ghosts. Its own
   decision and ADR; do not smuggle it in through animation.

## Open

- **Own sky landed** (branch `track-host`, ADR-0076 decision 14): `[art.skybox]` faces and/or a
  zenith/horizon gradient on HD's sky cube; add-on Own sky panel; `blender_loop` carries one and
  races on HD natively. Open: images inside the glb (no texture form for them in 1.x), and a
  Blender world Sky Texture or Environment Texture is not mapped (the panel's colours and images are).
- **What a track borrows pins the disc** (branch `track-host`, 2026-10-29, ADR-0076 decision 13):
  `[track] host` required exactly when the art borrows (host texture or sky), optional `titles`
  filter (checked against the craft's title). A pinned track on another title auto-remixes (track
  from a host disc, craft as asked; `oag_game::custom_tracks::route_to_host`), refused by name with
  no host disc. Hillside's data copy and builder name Pulse. The example campaign's Hillside copy
  needs the same `host` line (another session's work; the lead has the hand-over).
- **Landing 2b is in** (branch `track-art-2b`, 2026-10-29): the add-on writes and reads the art
  section (`scripts/blender/oag_track/scene_art.py`, `artfile.py`; mapping table in
  `docs/tools/blender.md`, "Art"): untagged meshes per material slot, linear `.color`, host and
  own-image textures, alpha clip from a Greater Than Math node (Blender 5.2 turns
  `blend_method = 'CLIP'` into Hashed), the Sun lamp (`oag_intensity`, not strength), further Sun
  lamps as `[[art.light]]`, world colour as ambient, sky, flare, `oag_shadow`/`oag_layer`/`oag_lit`,
  and a *Cover light* that derives `_OAG_LIGHT_SCALE` from art over the line. Fixture
  `blender_loop` (+ `blender_checker.png`, generated) and its re-export, read by
  `crates/trackfile/tests/blender.rs`; the lone-craft ground truth asserts its art resolves on Pulse.
- **Hillside** lives in the public mod-examples repository (`hillside/`, mod `oag.hillside`,
  mounted at `assets/mods/examples`), with its builder `hillside/hillside.py` and an in-game
  screenshot in that repository's README. Since 2026-10-29 it carries a lake, a mountain range,
  airships (static until motion lands), a start gantry with painted grid slots, 4 speed and 5
  weapon pads and `[track] host = "wipeout-pulse"`. The example campaign races it as
  `oag.hillside:tracks/hillside.track.toml`; nothing of it is in this repository.
- The bake is a builder-script step, not an operator. Recipe in `docs/tools/blender.md`.

- **Landing 2a is in** (branch `track-art-2`, 2026-10-29): linear `COLOR_0` (sRGB curve, rounded to
  the byte, in `oag_raceplay::foreign::art::colour`), `oag_alpha_ref` and `oag_layer` per art node,
  `[[art.light]]` directional rows, `[art.lensflare] direction`, `shadows` absent meaning `true` (a
  disc export writes `false`), and the PSP hull rig from `[art.ambient]` plus the sun plus the
  directional rows, proven equal to `HullLights::from_track` on all 96 circuits and wired on the
  foreign path. `docs/formats/track-file.md` has the census.
- **`light_scale` was already in 1.0** on the spline (`_OAG_LIGHT_SCALE`), filled by the exporter
  and read by the renderer on a foreign track. ADR-0076 decision 12 says so, with `shadows = false` on
  every disc export and the flare's own `direction` (lead approved 2026-10-29, relayed to the
  maintainer, who may override).
- **HD-family hosts** do not feed the sun to `Light::authored` yet: it needs the authored rig turned
  on for a foreign track on an HD host and a check that it does not relight anything else. Open.
- A lit art mesh's colour is a gamma float, not a byte (the `.vex` path's invented grey for an
  uncoloured vertex is not a byte). The add-on should export `oag_lit` meshes knowing that.
- ADR-0076 is still **Proposed**; the maintainer accepts it now that landing 2a is in.

## State of the sibling work

- **Hillside**, the loop built headless with the add-on (12 control points, two rises, one speedup and
  one weapon pad), is wired into the example campaign as the tier-2 "Hillside Sprint" event on branch
  `custom-campaigns` (uncommitted when this was written; the campaigns session owns it). A race launched
  from the campaign on it was **not verified**. Records and ghosts key as `foreign:hillside`; the
  manifest's `[track]` `id` and `title` keys are relied on and must not change meaning.
- Its builder script lived at `~/.cache/oag-scratch/hillside.py` (not committed). It drives
  `bpy.ops.oag.build_ribbon`, `add_start` and `add_pad` headless; `scripts/blender/roundtrip.py loop`
  is the committed pattern to copy.
- **2048**: the base `.pkg` is still refused in place (`oag-disc`: PFS-encrypted, key derived in
  hardware); `just play 2048` reads `data/extracted/vita/PCSF00007` and an `altima` export works from
  that folder. Art for it is *checked, applies, not wired* (one lane with Omega, which shares its
  `.rcsmodel` material model).

## Next Steps

1. ~~Blender art export and import~~ (2b). ~~Sun lamp to `[art.sun]`~~ (2b).
2. The *Bake lighting* operator (item 3): lift the builder's bake into the add-on (swap to albedo
   materials, hide collision, `DIFFUSE` direct + indirect to `VERTEX_COLORS`, report vertices per
   area, warn on an animated sun). Half a day.
3. Keyframed motion (items 4, 5): glTF animation read and write, the sampler against the race clock.
   Half a day to a day.
4. HD-family host: the sun into `Light::authored` (about half a day with a screenshot check).
5. ~~Re-skin Hillside~~ (2b). A launch of Hillside from the campaign menu is still unverified (the
   lane raced it with `--race --track-file`).
