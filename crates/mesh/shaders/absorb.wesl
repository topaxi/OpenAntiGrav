// The absorb-shading override.

// **Wipeout HD's absorb shell program** (`hd_absorbinternal.rcsmaterial`), off
// by default and on only for a team's `AbsorbEffect` model - see
// `oag_fx::absorb_shell` and
// docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md. Its `0.5` and `5.0`
// are literals in the microcode, not material parameters, so this is a flag
// rather than a parameter set.
override absorb_shading: f32 = 0.0;

// A shield shell's draw is multiplied by its vertex colour, which carries the
// shield's `ShieldColour`; see `oag_mesh::mesh::Model::shield_shell`.
override shield_shading: f32 = 0.0;

// HD's shield shell under its own lit program (`oag_mesh::mesh::Model::shield_lit`):
// `ShieldColour * texture * (ambient + sun * sat(N.L))` on the output scale, which the
// fog then lerps as for every scene program
// (docs/ghidra/functions/ps3-hdfury-eu/shield.md, "The shell's draw state").
override shield_lit: f32 = 0.0;
