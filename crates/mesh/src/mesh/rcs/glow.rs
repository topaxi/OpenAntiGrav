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

/// Where a material's glow mask is read from, when it is a texture's channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    /// The coverage alpha the shader already resolves for this material.
    Coverage,
    /// The second texture's sample (the emissive picture, scrolled where the
    /// material adds it).
    Second,
    /// The first texture's sample.
    First,
    /// The pad's third binding, the `_ne` mask.
    PadMask,
}

/// The `GpuVertex::glow` value for a texel-sourced mask: negative, `scale`
/// plus `8 * (4 * source + channel)`. `mesh.wesl`'s `glow.wesl` decodes it.
pub(super) fn texel_glow(source: Source, channel: u8, scale: f32) -> f32 {
    let source = match source {
        Source::Coverage => 0.0,
        Source::Second => 1.0,
        Source::First => 2.0,
        Source::PadMask => 3.0,
    };
    -(scale + 8.0 * (4.0 * source + f32::from(channel & 3)))
}

/// The value `GpuVertex::glow` carries for one material: `>= 0` a constant the
/// program writes, `< 0` a texel's channel times a constant ([`texel_glow`]).
///
/// `routed` is the `(unit, channel)` the material's coverage actually samples
/// (`slots`' alpha routing); `first` and `second` are the texture units the
/// material's two bindings reach. **Only a read the renderer can reproduce is
/// acted on**: a texel expression with a bias or from a unit nothing is bound
/// to draws no glow and is counted in [`Report::glow_unread`] - nothing is
/// invented where the program was not followed.
pub(super) fn glow_of(
    program: Option<&Program>,
    material: &rcsmodel::Material,
    routed: (u8, u8),
    units: (u32, u32),
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
        Lane::Texel { scale, .. } if scale == 0.0 => 0.0,
        Lane::Texel {
            unit,
            channel,
            scale,
            bias,
        } if bias == 0.0 && scale > 0.0 && scale < 8.0 => {
            if (unit, channel) == routed {
                texel_glow(Source::Coverage, channel, scale)
            } else if u32::from(unit) == units.1 {
                texel_glow(Source::Second, channel, scale)
            } else if u32::from(unit) == units.0 {
                texel_glow(Source::First, channel, scale)
            } else {
                report.glow_unread += 1;
                0.0
            }
        }
        Lane::Texel { .. } | Lane::Other => {
            report.glow_unread += 1;
            0.0
        }
    }
}
