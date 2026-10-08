//! Wipeout HD's two rim-shaded weapon glows, matched by what their own
//! fragment programs compute and never by a model's name.
//!
//! Both programs are read instruction by instruction in
//! `docs/rendering/hd-unlit-programs.md`. They share one skeleton - a noise
//! tap at `(u, v + 0.0001 t)` whose **alpha** displaces a second tap at
//! `(u, v) + 0.2 noise + 0.4 t` on both axes, and a rim term
//! `saturate(1 - N.V)` from the eye vector and normal the vertex program
//! hands over - and differ in how the rim shapes the colour:
//!
//! - [`slots::RIM_GLOW`] (`hd_leachbeam_ball_glow.rcsmaterial`, the
//!   LeachBall): `a = (0.9 (1 - rim^5))^5`, colour `a * c / (1 - c)`, alpha
//!   `a`. Brightest face-on, gone at the silhouette.
//! - [`slots::RIM_EDGE`] (`plasmasphere_subtractive_glow.rcsmaterial`, the
//!   Plasma bolt's head): colour `1000 rim^5 * c`, alpha the material's own
//!   `0x7611a2d8`. Dark face-on, blown out at the silhouette.
//!
//! Neither declares an ambient, a sun or a lightmap, and neither reads the
//! vertex colour. `globalAlphaScaler` is the identity `(0, 1)` the engine
//! initialises and never changes for these draws, and `fogColour` is the
//! fog `shaders/fog.wesl`'s `fogged` already applies - so every number the shader
//! needs beyond the texture and the clock is a literal in the program itself,
//! which is what [`classify`] checks before it sets a bit.
//!
//! # The clock-scroll shapes
//!
//! [`slots::CLOCK_SCROLL_RING`] and [`slots::CLOCK_SCROLL_HALO`] are the
//! Plasma explosion's ring and halo: the same skeleton again (a noise alpha
//! tap displacing the colour tap, both scrolled) but over the declared
//! `UV_offset` - the model's own clock - instead of the engine's `time`. They
//! sit here because the classification is the same fingerprint; what they
//! change in the picture is only where the texture is sampled.
//!
//! # The fingerprint
//!
//! A mnemonic sequence, the file's own (unpatched) literals in order, the
//! declared parameter set and a single sampler - all four must match. The
//! literals are the load-bearing part: they are exactly the numbers
//! `mesh.wesl` hard-codes for each bit, so a program that computed the same
//! shape with a different constant is refused rather than drawn with this
//! file's. A mnemonic sequence alone would not be enough; the DATA02 copy of
//! the LeachBall's material moves its `0.9` into a model parameter and is
//! not matched (see the page for why that copy is not the one served).

use oag_rcs::rcsmaterial::{self, fragment};

use crate::mesh::slots;

/// `~crc32("fogColour")`.
const FOG_COLOUR: u32 = 0x3dc3_1258;
/// `~crc32("globalAlphaScaler")` - engine parameter slot 76.
const GLOBAL_ALPHA_SCALER: u32 = 0x4c13_d3af;
/// `~crc32("time")` - engine parameter slot 0.
const TIME: u32 = 0x906b_67ba;
/// `~crc32("UV_offset")`: the model's own animation clock, bound by pointer
/// to its first Anim-class node's `+0xc0` (`AnimNode_GetTime`).
const UV_OFFSET: u32 = 0x8f2f_e704;
/// The Plasma head's alpha, authored per material; no preimage.
pub(super) const RIM_EDGE_ALPHA: u32 = 0x7611_a2d8;

/// One program shape: what it must look like, and the bit it earns.
struct Shape {
    bit: u32,
    mnemonics: &'static [&'static str],
    literals: &'static [[f32; 4]],
    parameters: &'static [u32],
}

/// `hd_plasmaring_glow.rcsmaterial`, block `@0x1900` (`DATA02`): the
/// explosion ring. Not a rim program - it earns [`slots::CLOCK_SCROLL_RING`]
/// for the `UV_offset` clock it shares with the rest of this file's shapes.
const PLASMA_RING: Shape = Shape {
    bit: slots::CLOCK_SCROLL_RING,
    mnemonics: &[
        "MOV", "MOV", "MOV", "MAD", "TEX", "MAD", "MAD", "MUL", "MUL", "TEX", "ADD", "MUL", "ADD",
        "MAD", "EX2", "MAD", "MUL", "MAD",
    ],
    literals: &[
        [0.01, 0.0, 0.0, 0.0],
        [0.15, 0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER, UV_OFFSET],
};

/// `hd_plasmahalo_glow.rcsmaterial`, block `@0x1960` (`DATA02`): the
/// explosion halo. See [`PLASMA_RING`].
const PLASMA_HALO: Shape = Shape {
    bit: slots::CLOCK_SCROLL_HALO,
    mnemonics: &[
        "MOV", "MOV", "MAD", "MOV", "TEX", "MUL", "MOV", "MAD", "ADD", "MAD", "MUL", "MUL", "TEX",
        "ADD", "MUL", "ADD", "MAD", "EX2", "MUL", "MOV", "MAD", "MAD",
    ],
    literals: &[
        [0.4, 0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0, 0.0],
        [0.04, 0.0, 0.0, 0.0],
        [0.1, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.333_251_95],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER, UV_OFFSET],
};

/// `AlphaAnim`, bound by pointer to the model's own clock (`node + 0xc0`).
const ALPHA_ANIM: u32 = 0x1d1b_5e79;
/// `ColourAnim`, bound by pointer to a float of the blast object itself.
const COLOUR_ANIM: u32 = 0x2eb8_d703;
/// `Speed`, the halo's vertex-side `v` offset (`vp @0x1860`: `ADD o[TC1].w, v[2].y, c[208].x`).
pub(super) const HALO_V_OFFSET: u32 = 0x3118_2e0d;
/// `Shockwave_scalar`, bound by pointer to the model's own clock.
const SHOCKWAVE_SCALAR: u32 = 0x3b5f_3bd7;

/// `hd_bombfire_glow.rcsmaterial`, block `@0x1d90` (`DATA02`): the Bomb's
/// fireball and its white core. Earns [`slots::BOMB_FIRE`].
const BOMB_FIRE: Shape = Shape {
    bit: slots::BOMB_FIRE,
    mnemonics: &[
        "MOV", "MOV", "MOV", "MOV", "DP3", "MAD", "MOV", "MAD", "DP3", "MOV", "TEX", "ADD", "DP3",
        "MUL", "MUL", "MUL", "DIVSQ", "MOV", "MUL", "ADD", "TEX", "LG2", "ADD", "RCP", "MUL",
        "EX2", "ADD", "RCP", "MAD", "LG2", "ADD", "RCP", "MUL", "EX2", "MUL", "MOV", "TEX", "ADD",
        "MAD", "ADD", "MUL", "ADD", "ADD", "RCP", "MUL", "EX2", "MAD", "MAD", "DIV", "MAD",
    ],
    literals: &[
        [0.04, 0.0, 0.0, 0.0],
        [0.3, 0.0, 0.0, 0.0],
        [6.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [5.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.86, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [50.0, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.5, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 100.0, 0.0, 0.0],
        [0.5, 0.0, 0.0, 0.0],
    ],
    parameters: &[
        ALPHA_ANIM,
        COLOUR_ANIM,
        FOG_COLOUR,
        GLOBAL_ALPHA_SCALER,
        TIME,
    ],
};

/// `hd_bomb_halo.rcsmaterial`, block `@0x19c0` (`DATA02`): the laid Bomb's
/// pulsing shell, a Fresnel rim over a 4x16 ramp. Earns [`slots::BOMB_HALO`].
const BOMB_HALO: Shape = Shape {
    bit: slots::BOMB_HALO,
    mnemonics: &[
        "MOV", "DP3", "MOV", "DP3", "DP3", "MUL", "MUL", "MOV", "DIVSQ", "MOV", "MUL", "ADD",
        "TEX", "MAD", "LG2", "MUL", "MUL", "EX2", "ADD", "MUL", "MUL", "EX2", "MUL", "MOV", "MUL",
        "MAD", "MAD",
    ],
    literals: &[
        [1.0, 0.0, 0.0, 0.0],
        [10.0, 0.0, 0.0, 0.0],
        [50.0, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.099_975_586],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER],
};

/// `hd_bombfire_shockwaves_glow.rcsmaterial`, block `@0x1950` (`DATA02`): the
/// Bomb's shockwave rings. Earns [`slots::BOMB_SHOCK`].
const BOMB_SHOCK: Shape = Shape {
    bit: slots::BOMB_SHOCK,
    mnemonics: &[
        "MOV", "MOV", "MOV", "MAD", "MUL", "TEX", "MUL", "MOV", "ADD", "MAD", "MOV", "MOV", "TEX",
        "ADD", "ADD", "TEX", "MUL", "MUL", "MUL", "MAD", "EX2", "MAD", "MAD",
    ],
    literals: &[
        [0.01, 0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0, 0.0],
        [0.45, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
    ],
    parameters: &[SHOCKWAVE_SCALAR, FOG_COLOUR, GLOBAL_ALPHA_SCALER],
};

/// `hd_missile_explosion_core_glow.rcsmaterial`, block `@0x17f0` (`DATA02`):
/// the Missile blast's sphere and bloom. Earns [`slots::MISSILE_CORE`].
const MISSILE_CORE: Shape = Shape {
    bit: slots::MISSILE_CORE,
    mnemonics: &[
        "MOV", "TEX", "MUL", "MUL", "MUL", "MUL", "MAD", "MAD", "EX2", "MAD",
    ],
    literals: &[[1.442_694_9, 0.0, 0.0, 0.0]],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER],
};

/// `hd_missile_explosion_lightrays_glow.rcsmaterial`, block `@0x17f0`
/// (`DATA02`): the Missile blast's rays. Earns [`slots::MISSILE_RAYS`].
const MISSILE_RAYS: Shape = Shape {
    bit: slots::MISSILE_RAYS,
    mnemonics: &[
        "MOV", "DP3", "MOV", "DP3", "DP3", "MUL", "MUL", "DIVSQ", "ADD", "MOV", "LG2", "MUL",
        "MUL", "EX2", "MAD", "LG2", "MUL", "MOV", "EX2", "MUL", "TEX", "MAD", "EX2", "MAD", "MAD",
    ],
    literals: &[
        [1.0, 0.0, 0.0, 0.0],
        [5.0, 0.0, 0.0, 0.0],
        [0.9, 0.0, 0.0, 0.0],
        [5.0, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER],
};

/// `hd_missile_explosion_shockwaves_glow.rcsmaterial`, block `@0x19b0`
/// (`DATA02`): the Missile blast's rings. Earns [`slots::MISSILE_SHOCK`].
const MISSILE_SHOCK: Shape = Shape {
    bit: slots::MISSILE_SHOCK,
    mnemonics: &[
        "MOV", "MOV", "MOV", "MAD", "TEX", "MUL", "MOV", "ADD", "MUL", "MUL", "MAD", "MOV", "TEX",
        "ADD", "MOV", "ADD", "TEX", "MUL", "MUL", "MAD", "MAD", "EX2", "MAD", "MAD",
    ],
    literals: &[
        [0.01, 0.0, 0.0, 0.0],
        [0.05, 0.0, 0.0, 0.0],
        [0.45, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
    ],
    parameters: &[SHOCKWAVE_SCALAR, FOG_COLOUR, GLOBAL_ALPHA_SCALER],
};

/// `hd_leachbeam_ball_glow.rcsmaterial`, block `@0x19b0` (`DATA00`).
const LEACH_BALL: Shape = Shape {
    bit: slots::RIM_GLOW,
    mnemonics: &[
        "MOV", "MOV", "MOV", "MOV", "MAD", "MOV", "TEX", "MOV", "DP3", "MUL", "MAD", "DP3", "MUL",
        "DP3", "DIVSQ", "ADD", "MAD", "TEX", "ADD", "LG2", "MUL", "MUL", "EX2", "ADD", "RCP",
        "MAD", "LG2", "ADD", "RCP", "MUL", "EX2", "MUL", "MUL", "RCP", "MAD", "EX2", "MAD", "MAD",
    ],
    literals: &[
        [0.0001, 0.0, 0.0, 0.0],
        [0.2, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.4, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [5.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.9, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [5.0, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER, TIME],
};

/// `plasmasphere_subtractive_glow.rcsmaterial`, block `@0x18d0` (`DATA02`).
const PLASMA_HEAD: Shape = Shape {
    bit: slots::RIM_EDGE,
    mnemonics: &[
        "MOV", "MOV", "DP3", "DP3", "MOV", "DP3", "MUL", "DIVSQ", "MOV", "MAD", "ADD", "TEX",
        "MOV", "MAD", "MUL", "MAD", "MOV", "TEX", "LG2", "MUL", "MUL", "MUL", "EX2", "MUL", "MAD",
        "EX2", "MAD", "MAD",
    ],
    literals: &[
        [0.0001, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.2, 0.0, 0.0, 0.0],
        [0.4, 0.0, 0.0, 0.0],
        [1.442_694_9, 0.0, 0.0, 0.0],
        [0.0, 5.0, 0.0, 0.0],
        [1000.0, 0.0, 0.0, 0.0],
    ],
    parameters: &[FOG_COLOUR, GLOBAL_ALPHA_SCALER, RIM_EDGE_ALPHA, TIME],
};

/// The [`slots`] bit a material's resolved program earns, or `0`.
///
/// `authored_alpha` is the material's own value for [`RIM_EDGE_ALPHA`], which
/// the Plasma head's program moves straight to its output alpha. `mesh.wesl`
/// carries no per-material alpha for this path, so the bit is only set where
/// that value is exactly `1.0` - every material on the disc that matches the
/// shape, measured by `crates/render/examples/hd_unlit_census.rs` - and a
/// future one authoring anything else is refused rather than drawn opaque.
#[must_use]
pub(super) fn classify(
    declared: &rcsmaterial::Declared,
    program: &fragment::Program,
    authored_alpha: Option<f32>,
) -> u32 {
    if declared.samplers.len() != 1 {
        return 0;
    }
    let mut parameters = declared.parameters.clone();
    parameters.sort_unstable();
    for shape in [
        &LEACH_BALL,
        &PLASMA_HEAD,
        &PLASMA_RING,
        &PLASMA_HALO,
        &BOMB_FIRE,
        &BOMB_SHOCK,
        &BOMB_HALO,
        &MISSILE_CORE,
        &MISSILE_RAYS,
        &MISSILE_SHOCK,
    ] {
        let mut wanted = shape.parameters.to_vec();
        wanted.sort_unstable();
        if parameters != wanted || !mnemonics_match(program, shape.mnemonics) {
            continue;
        }
        if literals(declared, program) != shape.literals {
            continue;
        }
        if shape.bit == slots::RIM_EDGE && authored_alpha != Some(1.0) {
            continue;
        }
        return shape.bit;
    }
    0
}

fn mnemonics_match(program: &fragment::Program, want: &[&str]) -> bool {
    program.instructions.len() == want.len()
        && program
            .instructions
            .iter()
            .zip(want)
            .all(|(i, w)| i.name() == Some(*w))
}

/// Every inline constant the file itself authors, in program order - the
/// ones no declared parameter patches over at draw time.
pub(super) fn literals(
    declared: &rcsmaterial::Declared,
    program: &fragment::Program,
) -> Vec<[f32; 4]> {
    let patched: Vec<u16> = declared
        .parameters
        .iter()
        .flat_map(|&hash| program.patches(hash).collect::<Vec<_>>())
        .collect();
    program
        .instructions
        .iter()
        .filter(|i| i.const_slot.is_some_and(|slot| !patched.contains(&slot)))
        .filter_map(|i| i.constant)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_shapes_earn_different_bits_and_neither_is_a_default_role() {
        assert_ne!(LEACH_BALL.bit, PLASMA_HEAD.bit);
        for bit in [LEACH_BALL.bit, PLASMA_HEAD.bit] {
            assert_eq!(bit & slots::ROLE_MASK, bit, "a role bit, below the index");
            assert_eq!(bit & slots::DEFAULT, 0);
            assert_eq!(bit & slots::EMISSIVE, 0);
        }
    }

    #[test]
    fn an_empty_program_matches_nothing() {
        let declared = rcsmaterial::Declared {
            parameters: vec![FOG_COLOUR, GLOBAL_ALPHA_SCALER, TIME],
            samplers: vec![(0x3bdc_0403, 0)],
            parameter_patches: Vec::new(),
        };
        let program = fragment::Program::default();
        assert_eq!(classify(&declared, &program, Some(1.0)), 0);
    }
}
