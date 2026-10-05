//! Which of a PS4 model's draws are see-through, and how.
//!
//! Wipeout: Omega Collection authors Wipeout HD's own **state word** on every
//! material - the `u16` at header `+0x22`, see
//! [`oag_rcs::rcsmodel::psp2::material::Material::state`] - and the
//! executable reads it the way HD's reads its own: `FUN_015fa610`
//! (`/omega/eboot-ps4-omega-eu.bin`) turns it into a draw-order key, `8` for
//! bit 1 and bit 0 both clear, `0x10` for bit 1 (an alpha test), `0x18` for
//! bit 0 (a blend), less `(state >> 9) & 7`. Those three buckets are the
//! three lists [`Model`] already has: opaque, alpha-tested, blended.
//!
//! **What the file does not say is the equation.** HD authors a factor pair
//! beside the state word and neither Omega's header (zero on all 32,880
//! materials) nor 2048's carries one, and no material-pass call that programs a
//! per-material blend was found in either executable. So a blended draw takes
//! HD's own pair for a material of the **same name** where
//! [`oag_rcs::rcsmodel::psp2::lineage_blend`] holds one (inherited, not
//! measured here: `emissive_bloom`, `hd_enginetrail` and the light barriers are
//! `SRC_ALPHA`/`ONE` there), and alpha-over (`SRC_ALPHA`, `ONE_MINUS_SRC_ALPHA`)
//! for every other name, **chosen, not measured**. The alpha test's reference is
//! HD's `0.5` for the same reason (HD's `GL_GREATER`/`0.5` holds on every one of
//! its mode-2 materials).
//!
//! Only a **textured** draw moves: the alpha a blend or a test reads is the
//! texture's, and an unpainted draw has nothing but the white placeholder's `1.0`.

use oag_rcs::rcsmodel::psp2::{self, lineage_blend, material::Mode};

use super::Report;
use crate::mesh::Model;

/// The alpha test's reference: HD's, `GL_GREATER` against `0.5` - chosen for
/// Omega, not measured.
const ALPHA_REF: f32 = 0.5;

/// Moves each textured draw of a model whose materials carry a state word into
/// the alpha-tested or the blended list. Draws and submeshes must still be
/// one-to-one, which is what [`super::build_planned`] guarantees at this point.
pub(super) fn route(decoded: &psp2::Model, model: &mut Model, report: &mut Report) {
    if decoded.materials.iter().all(|m| m.state.is_none()) {
        return;
    }
    let draws = std::mem::take(&mut model.draws);
    for (mut draw, submesh) in draws.into_iter().zip(&decoded.submeshes) {
        let material = submesh.material.and_then(|i| decoded.materials.get(i));
        let mode = material.and_then(|m| m.mode());
        match (mode, draw.texture) {
            (Some(Mode::Blended), Some(_)) => {
                let inherited = material.and_then(|m| lineage_blend::inherited(&m.name));
                draw.blend_state = Some(match inherited {
                    Some((src, dst)) => super::super::blend_state(src, dst),
                    None => crate::mesh_render::TRANSPARENT_BLEND,
                });
                report.inherited_blend_draws += usize::from(inherited.is_some());
                report.blended_draws += 1;
                model.transparent_draws.push(draw);
            }
            (Some(Mode::AlphaTest), Some(_)) => {
                report.cutout_draws += 1;
                model.alpha_test_ref = Some(ALPHA_REF);
                model.alpha_tested_draws.push(draw);
            }
            _ => model.draws.push(draw),
        }
    }
}
