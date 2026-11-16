# Our own HD/Fury-style shield for Pulse and Pure

An opt-in extra of this project's, **not the original's**: `graphics.own_shield` (GRAPHICS ADVANCED, `off`/`on`,
default `off`, takes effect the next race) draws a shield shell in the look of Wipeout HD's, in place of the shell
Pulse's or Pure's disc authors. It is a sanctioned exception to "never invent what the assets author": the shell is
ours, authored here as a shader and a handful of numbers, and reads **no byte from an HD, 2048 or Omega disc**. It
works with only a Pulse or Pure disc present. The row is dropped on HD, 2048 and Omega, which draw their own.

HD's own shell is the visual reference only (`weapon-absorb-pairs.md`, `shield.md` under
`docs/ghidra/functions/ps3-hdfury-eu/`): an icy-blue, faceted, hull-hugging glow that flashes amber on an absorbed hit.

## How it is built

- **Geometry** (`oag_livery::own_shield::shell`, `own_shield/offset_surface.rs`): a closed surface standing `0.26`
  model units (`STAND_OFF`) off the craft's own hull, so the shell is hull-shaped, as HD's is, with a gap between the
  two. The hull's first level of detail (less any triangle an `Anim Transform` moves) is turned into a distance field
  on a grid of `0.11`-unit cubes, and the surface where the distance crosses `0.26` is extracted with one vertex per
  straddling cube and one quad per straddling grid edge (naive surface nets). Only the surface the outside can reach is
  kept, so a hollow inside the hull grows no second shell. The result is one two-sided batch on the blended list, drawn
  after the hull and depth-tested against it. The disc's own shell (`shipshield.vex`, or `Data\Weapons\shield.vex` on
  Pure) is not loaded for the slot. The cockpit sphere (`vr_shield_cockpit.vex`) keeps the disc's geometry and takes
  the same shader, at its authored size.
- **Shader** (`crates/mesh/shaders/own_shield.wesl`): selected by the `own_shield_shading` pipeline override, which
  `oag_mesh::mesh::Model::own_shield` sets; fragment stage only, the vertex stage is untouched. Four terms: facets
  (the flat normal of the triangle under the fragment, from screen-space derivatives), a hexagon lattice in the craft's
  own frame (triplanar, so it rides the hull) with per-cell shimmer, a bright band travelling nose to tail, and a
  bright line along the shell's silhouette. The line is `0.035` world units wide at any resolution: the smooth
  normal's `n . v` over its own screen-space rate of change is the distance to the silhouette in pixels, and a pixel's
  world size at that depth converts the width. A plain rim term cannot do this job: it lights a wing seen at a grazing
  angle whole and leaves the true silhouette a pixel wide. The colour is the vertex colour the race writes each frame,
  exactly as for the disc's shell. With the setting off no model carries the override, so no other shader changes.
- **Animation**: the shared `oag_render::shield::ShipShield` law (fade up, swell, hit bulge, fade-out with a swelling
  scale) under `OWN_PALETTE`: it fades up from deep blue, settles to an ice blue `(0.45, 0.80, 1.0)` and flashes
  amber `(1.0, 0.55, 0.10)` on an absorbed hit.
- **Gate**: `oag_title::Looks::own_shield` (a `Rule`, `Origin::Chosen`) is on for Pulse and Pure. `Options::own_shield`
  is the player's setting AND that rule (`oag_raceplay::load::roster::own_shield`).

## Evidence

Captures are `--race --mode single_race --give shield`, the shield raised at tick 330, `--size 960x544`, settings
file with `[graphics] own_shield = true` for "on".

- **Off is byte-identical.** Thirteen ticks (334 to 880: just raised, settled, fading, gone) on Pulse USA and Pure USA,
  captured with the binary before the change and the one after with the setting off: all 26 PNGs compare equal with
  `cmp`. After the shield has expired, on and off are also identical (no residue).
- **The shell stands off the hull.** At the chase camera, `960x544`, tick 400, measured from three captures of the
  same tick (nothing, hull alone, shell alone; the last two against the first give the two silhouettes):

  | | nose | wing tips | sides (median per row) |
  | --- | --- | --- | --- |
  | Pulse USA | 7 px | 17 px | 12 px |
  | Pure USA | 7 px | 12 px | 12 px |

  The hull-alone and shell-alone captures need the player's hull draw skipped, a local edit to
  `Scene`'s craft loop that is not in the tree.
- **A trap this page exists to record:** the first version pushed the hull's own vertices out along their normals in
  the vertex stage by a pipeline constant. `mesh_render::build` hands pipeline constants to the fragment stage only, so
  the push was always zero and the shell drew on the hull's surface as a tint, at any distance asked for. A per-vertex
  push is also the wrong tool on a hard-edged hull: faces split apart at every crease.
- **On reads like HD's.** Pulse and Pure with the setting on beside an HD shield frame (HD: ours rendering HD's own
  shell on Talon's Junction): an icy, faceted shell around the craft with a bright edge and the livery visible through
  it. No original-HD capture of a raised shield exists in the repo, so the HD frame is ours rendering HD's data, with
  the limit stated in `crates/livery/src/shield.rs` (it is brighter and flatter than the disc's material).
- **A hit**: `--force-hit 400:0` flashes the shell amber with the swell, and it settles back to blue. The amber wash over the whole view in that frame is Pulse's own hit burst: the frame with the setting off carries the same wash, and it is not the shell.
- Tests: `own_shield::offset_surface::tests` (the surface is closed, the asked distance off every triangle, outside
  only), `crates/mesh/tests/own_shield_lit_path.rs` (the flag reaches the pipeline and reads no texture),
  `crates/raceplay/tests/own_shield_ground_truth.rs` (Pulse PSP and PS2, Pure PSP: off keeps the disc's shell, palette
  and sphere; on swaps all three and the shell's box holds the hull's with room on every side; HD ignores it) and `menu_definition_settings::the_own_shield_row_is_offered_only_where_the_original_draws_another_look`.

## Open

- A live hit's colour flash is the palette's; a brighter hit flare in the shader is a possible follow-up.
- **The cockpit sphere was not seen.** In a headless `--camera-view internal` capture the frames with the setting on and
  off are byte-identical at ticks 345, so neither the disc's sphere nor ours showed anything there; the sphere's
  appearance from inside the hull is unjudged. From inside, the rim term is near zero by construction.
- A hull whose meshes do not map back to its file draws no shield with the setting on (reported), never the disc's shell
  under our palette. Pulse PSP, Pulse PS2 and Pure PSP all build for every grid slot (the test).
- The shell is 13,000 to 22,000 vertices a craft on Pulse, and its vertex buffer is rewritten each frame a shield is
  up (the tint). Larger cubes cut that at the cost of coarser facets; a per-draw colour uniform would remove the
  rewrite.
- The airbrake flaps are not part of the shell's shape (their triangles are authored in their node's space), and a
  deflected flap was not checked against the shell at speed.
- The silhouette line follows the shell's facets, so it wavers by a pixel or two along a long edge.
- **Cross-title check (CLAUDE.md "2048 and Omega")**: checked, differs. HD, 2048 and Omega draw the HD-style shield
  from their own `shipshield.rcsmodel`, so the setting does nothing there and the row is dropped. Not wired for them
  by design.
