//! The button glyphs Pulse's text faces embed.
//!
//! `ε γ δ β Λ Ν` are in every one of `pulse_text.fnt`, `Pulse_14.fnt` and
//! `Pulse_20.fnt`, as a 12/17/22 px cell each. Read off the rendered cells of
//! the Pulse EU disc (`oag-tools --example prompt_glyph_probe`): `ε` is the
//! circled cross, `γ` the circled circle, `δ` the circled square, `β` the
//! circled triangle, `Λ` the `L` shoulder box and `Ν` the `R` one (both
//! twice the width of a face button). The strings carry them inline, for
//! example `FE_PRESS_TO_CONT` ("Press ε to continue").
//!
//! Checked against Pure: its strings and faces were not read for this.

use oag_title::prompts::{Prompt, Prompts, Scope};

/// Pulse's prompt table. Measured.
pub const PROMPTS: &Prompts = &Prompts {
    scope: Scope::AnyFace,
    glyphs: &[
        ('ε', Prompt::Cross),
        ('γ', Prompt::Circle),
        ('δ', Prompt::Square),
        ('β', Prompt::Triangle),
        ('Λ', Prompt::L),
        ('Ν', Prompt::R),
    ],
};
