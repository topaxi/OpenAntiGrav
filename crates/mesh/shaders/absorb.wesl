// The absorb-shading override.

// **Wipeout HD's absorb shell program** (`hd_absorbinternal.rcsmaterial`), off
// by default and on only for a team's `AbsorbEffect` model - see
// `oag_fx::absorb_shell` and
// docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md. Its `0.5` and `5.0`
// are literals in the microcode, not material parameters, so this is a flag
// rather than a parameter set.
override absorb_shading: f32 = 0.0;
