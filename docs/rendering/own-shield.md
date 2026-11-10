# Our own HD/Fury-style shield for Pulse and Pure

An opt-in extra of this project's, **not the original's**: `graphics.own_shield` (GRAPHICS ADVANCED, `off`/`on`,
default `off`, takes effect the next race) draws a shield shell in the look of Wipeout HD's, in place of the shell
Pulse's or Pure's disc authors. It is a sanctioned exception to "never invent what the assets author": the shell is
ours, authored here as a shader and a handful of numbers, and reads **no byte from an HD, 2048 or Omega disc**. It
works with only a Pulse or Pure disc present. The row is dropped on HD, 2048 and Omega, which draw their own.

HD's own shell is the visual reference only (`weapon-absorb-pairs.md`, `shield.md` under
`docs/ghidra/functions/ps3-hdfury-eu/`): an icy-blue, faceted, hull-hugging glow that flashes amber on an absorbed hit.

## How it is built

- **Geometry** (`oag_livery::own_shield::shell`): the craft's own hull, first level of detail, cloned with its
  textures and materials dropped and every batch moved to the blended list, so the shell adds to the hull instead of
  replacing it. Each vertex is pushed `0.4` world units out along its normal in the vertex stage. Hull-shaped, as HD's
  shell is; the disc's own shell (`shipshield.vex`, or `Data\Weapons\shield.vex` on Pure) is not loaded for the slot.
  The cockpit sphere (`vr_shield_cockpit.vex`) keeps the disc's geometry and takes the same shader, at its authored
  size.
- **Shader** (`crates/mesh/shaders/own_shield.wesl`): selected by the `own_shield_shading` pipeline override, which
  `oag_mesh::mesh::Model::own_shield` sets. A rim term (`(1 - |n . v|)^2.2`), a hexagon lattice in the craft's own
  frame (triplanar, so it rides the hull and does not slide through it) with per-cell shimmer, and a bright band
  travelling nose to tail. The colour is the vertex colour the race writes each frame, exactly as for the disc's shell.
  With the setting off no model carries the override, so no other shader changes.
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
- **On reads like HD's.** Pulse with the setting on beside an HD shield frame (HD: ours rendering HD's own shell
  on Talon's Junction): an ice-blue faceted glow over the livery, which stays visible through it. No original-HD
  capture of a raised shield exists in the repo, so the HD frame is ours rendering HD's data, with the limit stated in
  `crates/livery/src/shield.rs` (it is brighter and flatter than the disc's material).
- **A hit**: `--force-hit 400:0` flashes the shell amber with the swell, and it settles back to blue.
- Tests: `crates/mesh/tests/own_shield_lit_path.rs` (the flag reaches the pipeline and reads no texture),
  `crates/raceplay/tests/own_shield_ground_truth.rs` (Pulse PSP and PS2, Pure PSP: off keeps the disc's shell, palette
  and sphere; on swaps all three; HD ignores it) and `menu_definition_settings::the_own_shield_row_is_offered_only_where_the_original_draws_another_look`.

## Open

- A live hit's colour flash is the palette's; a brighter hit flare in the shader is a possible follow-up.
- **The cockpit sphere was not seen.** In a headless `--camera-view internal` capture the frames with the setting on and
  off are byte-identical at ticks 345, so neither the disc's sphere nor ours showed anything there; the sphere's
  appearance from inside the hull is unjudged. From inside, the rim term is near zero by construction.
- A hull whose meshes do not map back to its file draws no shield with the setting on (reported), never the disc's shell
  under our palette. Pulse PSP, Pulse PS2 and Pure PSP all build for every grid slot (the test).
- The shell draws the hull's zero-alpha LOD children too and re-uploads a hull-sized vertex buffer per raised shield each
  frame; dropping the draws outside the first level of detail in `own_shield::shell` would cut the fragment cost.
- A frame at speed (364 km/h, ticks 380 and 430): no smear and the airbrake flaps do not poke through.
- **Cross-title check (CLAUDE.md "2048 and Omega")**: checked, differs. HD, 2048 and Omega draw the HD-style shield
  from their own `shipshield.rcsmodel`, so the setting does nothing there and the row is dropped. Not wired for them
  by design.
