//! Two readings of a material's own microcode that decide how a hull's light
//! is combined, split out of [`super::roles`] for its length.
//!
//! # `VertexColour1` is a factor on the light, on the programs that say so
//!
//! `diffuse_vcol` (HD's ship hull, block `@0x32e0`) ends on
//!
//! ```text
//! @0x0e  MUL H2.xyz, f[TC0], H0      ; VertexColour1 * (ambient + sun * N.L)
//! @0x11  TEX H1.xyz, f[TC3] unit0
//! @0x12  MAD R1.xyz, H1, H2, -fog    ; albedo * that
//! ```
//!
//! and its `SVC1` twin (`@0x7c90`) puts the SPU light *inside* the same
//! multiply (`H2 = f[TC0] * (f[TC1] + ambient + sun * N.L)`). The track's
//! `colorSet1` is the opposite shape - `ADD`ed into the light sum - and is what
//! `mesh.wesl` models as a baked light. [`colour_factor`] separates the two off
//! the program: the vertex block routes `VertexColour1` into `o[TCn]` and the
//! fragment block only ever `MOV`s or `MUL`s `f[TCn]`.
//!
//! # No `LG2`, no specular
//!
//! A program with no `log2` has no `pow(N.H, e)`. The shared stand-in exponent
//! stays for the track materials it was decided for; a hull material in this
//! class draws no specular at all ([`crate::mesh::NO_SPECULAR`] as its
//! exponent).

use oag_rcs::rcsmaterial::{self, fragment::Program, vertex};

use crate::mesh::rcs::Report;

/// `~crc32("VertexColour1")`.
const VERTEX_COLOUR_1: u32 = 0x7493_d450;

/// Whether this material's program takes `VertexColour1` as a factor on its
/// light - see the module doc. `false` for any material whose blocks do not
/// resolve, which is the direction that leaves the additive reading alone.
pub(super) fn colour_factor(
    variant: Option<rcsmaterial::Variant>,
    blob: Option<&[u8]>,
    program: Option<&Program>,
) -> bool {
    let (Some(variant), Some(blob), Some(program)) = (variant, blob, program) else {
        return false;
    };
    let Some(vertex) = vertex::Program::of(blob, variant.vertex) else {
        return false;
    };
    let fed = vertex.texcoords_fed_by(VERTEX_COLOUR_1);
    !fed.is_empty() && fed.iter().all(|&n| program.input_is_only_a_factor(n))
}

/// The exponent this material's specular term is drawn with: its own chain's
/// literal, [`crate::mesh::NO_SPECULAR`] for a [`colour_factor`] material
/// whose program has no chain, else the shared stand-in.
pub(super) fn specular_exponent(
    program: Option<&Program>,
    material: &oag_rcs::rcsmodel::Material,
    factor: bool,
    report: &mut Report,
) -> f32 {
    // **A resolved `0.0` is checked against the model's own patch table
    // before it is discarded.** `Program::specular_exponent`'s own doc
    // comment carries the disc-wide evidence that `0.0` is the strongest
    // candidate for `SpecularPower` patched at draw time rather than a
    // real value baked in the file - `pow(x, 0) = 1` is not a plausible
    // authored shininess. That is now checked rather than assumed:
    // `Program::patches` says whether the exponent's own code slot is one
    // `SpecularPower` overwrites, and where it is, the material's own
    // parameter table (the same table `Flame::from_material` reads) has
    // the value the engine actually puts there. Verified disc-wide before
    // being wired - `crates/render/examples/hd_specular_patch_census.rs` -
    // every one of 62 materials across 16 circuits whose `0.0` chain is
    // patched this way also authors a non-zero `SpecularPower`, at values
    // (30 to 100, non-round) the shared-literal population never carries.
    // A `0.0` that is *not* patched, or a material with no authored value
    // for it, still falls back to `DEFAULT_SPECULAR_EXPONENT` - every
    // other resolved value is wired exactly as read.
    let resolved = program.and_then(|program| {
        let value = program.specular_exponent()?;
        if value != 0.0 {
            return Some(value);
        }
        let slot = program.specular_exponent_slot()?;
        program
            .patches(rcsmaterial::SPECULAR_POWER)
            .any(|patched| patched == slot)
            .then(|| {
                material
                    .parameters
                    .iter()
                    .find(|p| p.hash == rcsmaterial::SPECULAR_POWER)
                    .map(|p| p.value[0])
            })
            .flatten()
            .filter(|value| *value != 0.0)
    });
    if let Some(value) = resolved {
        return value;
    }
    if factor && program.is_some_and(|p| !p.has_specular_chain()) {
        return crate::mesh::NO_SPECULAR;
    }
    report.specular_exponent_unresolved += 1;
    crate::mesh::DEFAULT_SPECULAR_EXPONENT
}
