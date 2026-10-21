//! The glow mask a material's own fragment program writes.
//!
//! Wipeout HD's scene target is `A8R8G8B8` and the bloom's gate multiplies the
//! frame's colour by its alpha, so every fragment program decides how much of
//! its surface blooms: `MOV H0.w, 1` on a neon sign, `MOV H0.w, 0` on lit
//! road, `MOV H0.w, H1` (a texture's alpha) on the emissive pads and lights.
//! [`oag_rcs::rcsmaterial::fragment::Program::output_alpha`] reads which, and
//! this turns it into the one float `GpuVertex::glow` carries under
//! `mesh_render::GlowMask::StampedByProgram`.

use oag_rcs::rcsmaterial::fragment::{Lane, Program};
use oag_rcs::rcsmodel;

use super::Report;

/// The value `GpuVertex::glow` carries for one material: `>= 0` a constant the
/// program writes, `< 0` the picture's alpha times `-value`.
///
/// `routed` is the `(unit, channel)` the material's coverage actually samples
/// (`slots`' alpha routing), because the shader's picture alpha is that
/// channel and no other. **Only a read the renderer can reproduce is acted
/// on**: a texel expression with a bias, or from a channel the coverage does
/// not carry, draws no glow and is counted in [`Report::glow_unread`] -
/// nothing is invented where the program was not followed.
pub(super) fn glow_of(
    program: Option<&Program>,
    material: &rcsmodel::Material,
    routed: (u8, u8),
    report: &mut Report,
) -> f32 {
    let Some(program) = program else {
        return 0.0;
    };
    let lane = program.output_alpha(|hash| {
        material
            .parameters
            .iter()
            .find(|p| p.hash == hash)
            .map(|p| p.value)
    });
    match lane {
        // The register's `w` was never written; the hardware leaves whatever the
        // register held, which is the cleared `0` of a fresh one.
        Lane::Unwritten => 0.0,
        Lane::Constant(v) => v.clamp(0.0, 1.0),
        Lane::Texel {
            unit,
            channel,
            scale,
            bias,
        } if bias == 0.0 && scale > 0.0 && (unit, channel) == routed => -scale,
        Lane::Texel { .. } | Lane::Other => {
            report.glow_unread += 1;
            0.0
        }
    }
}
