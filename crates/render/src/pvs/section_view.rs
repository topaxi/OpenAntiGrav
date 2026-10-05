//! The original's second visibility tier: a section's own authored box against
//! the view.
//!
//! # What the original does
//!
//! `FUN_0892b638`, the visibility predicate that twelve node classes share
//! through their vtables, answers "may this node draw?" in two steps. First the
//! section mask: the governing section's bit against the visible set
//! (`FUN_0897bc08` and `FUN_0891e908`, a 64-bit AND). **Then, if that passes,
//! the section's own authored box is tested against the view** - the copy of the
//! view-projection `FUN_08902a58` takes, and `FUN_08902a98` on the box struct at
//! the section object's `+0x5c`. The box is in world space (no node matrix is
//! multiplied in), it is the box the `.vex` `section` node carries at payload
//! `+0x10` and `+0x20`, and `oag_vex::pvs` reads it as [`oag_vex::pvs::Aabb`].
//!
//! The test is `FUN_08902594`: the eight corners go through clip space, each
//! gets a five-bit outcode (`FUN_08902894`) of `x < -w`, `y < -w`, `z < 0`,
//! `x > w`, `y > w`, and the box is rejected when **the AND of all eight is not
//! zero** - every corner outside the same plane. There is no far plane. In the
//! GE's clip space `z < 0` is not the near plane but the depth half way between
//! near and far, `d = -m32 / m22`: measured off the projection registers of 25
//! PPSSPP dumps it is `2.42 ..= 2.52` units (it moves with the fov) against a
//! near plane of about `1.24`, so [`NEAR_REJECT_DEPTH`] stands in for it.
//!
//! The same test runs for every `Mesh` node against its own box (`FUN_0890cc80`,
//! `+0x80`/`+0x90`) and this crate does not port that one: the original also
//! draws most static geometry through merged batch sets that never reach it,
//! so applying it per draw would hide batches the original submits. The section
//! test is the one the data supports - `docs/ghidra/functions/psp-pulse-usa/
//! section-view-cull.md`, `docs/rendering/frame-audit.md` section 3.
//!
//! # Why it matters here
//!
//! A moving draw has no bound this crate can trust (see [`DrawCall::moving`]),
//! and its authored *section* is static. Testing the section's box gives the
//! moving draw the cull the original gives it, from the asset's own bound, and
//! gives a static draw the same second tier its sphere test only approximates.
//!
//! **Chosen, not measured**: the test is made against *this camera's*
//! view-projection, where the original hard-codes the 480x272 aspect into the
//! planes it builds (`Camera_SubmitScene`). On a wider picture the original's
//! test would cut the edges of the screen; this one cuts exactly what the picture
//! cannot show, so it is never narrower than the picture and agrees with the
//! original at 480x272.
//!
//! [`DrawCall::moving`]: oag_mesh::mesh::DrawCall::moving

use oag_core::math::{Mat4, Vec3, Vec4};
use oag_vex::pvs::{MAX_SECTIONS, TrackPvs};

use super::VisibleSet;

/// The view depth below which a corner trips the `z < 0` outcode.
///
/// `2.482` is the rest-fov value; the range over 25 dumps was `2.42 ..= 2.52`.
/// Only a section box lying entirely within about two and a half units of the
/// camera plane, or behind it, is affected.
pub const NEAR_REJECT_DEPTH: f32 = 2.482;

/// The five outcode bits of one clip-space point.
fn outcode(clip: Vec4) -> u8 {
    let w = clip.w;
    let mut code = 0;
    if clip.x < -w {
        code |= 1;
    }
    if clip.y < -w {
        code |= 2;
    }
    if w < NEAR_REJECT_DEPTH {
        code |= 4;
    }
    if clip.x > w {
        code |= 8;
    }
    if clip.y > w {
        code |= 16;
    }
    code
}

/// Whether the original's corner test rejects the world-space box
/// `min..=max` under `view_projection`.
#[must_use]
pub fn box_outside_view(view_projection: &Mat4, min: [f32; 3], max: [f32; 3]) -> bool {
    let mut all = u8::MAX;
    for k in 0..8 {
        let corner = Vec3::new(
            if k & 1 == 0 { min[0] } else { max[0] },
            if k & 2 == 0 { min[1] } else { max[1] },
            if k & 4 == 0 { min[2] } else { max[2] },
        );
        all &= outcode(*view_projection * corner.extend(1.0));
        if all == 0 {
            return false;
        }
    }
    true
}

/// The sections whose authored box is not rejected by `view_projection`, as a
/// section mask.
///
/// A section that authors no box, or an id this track does not declare, stays
/// set: with nothing to test against the error has to point towards drawing.
#[must_use]
pub fn sections_in_view(pvs: &TrackPvs, view_projection: &Mat4) -> u64 {
    let mut mask = u64::MAX;
    for id in pvs.ids() {
        if usize::from(id) >= MAX_SECTIONS {
            continue;
        }
        if let Some(bound) = pvs.bounds_of(id)
            && box_outside_view(view_projection, bound.min, bound.max)
        {
            mask &= !(1u64 << id);
        }
    }
    mask
}

impl VisibleSet {
    /// This set, narrowed to the sections whose box the view reaches.
    ///
    /// The original's second tier (see the module docs), applied after the
    /// section mask so that an authored exclusion is never undone: it can only
    /// remove sections.
    #[must_use]
    pub fn within_view(self, pvs: &TrackPvs, view_projection: &Mat4) -> Self {
        let narrowed = self.mask & sections_in_view(pvs, view_projection);
        // Nothing in view at all would hide an unplaced draw too, whose mask
        // is every bit: not a state the original reaches, since the craft's own
        // section holds the craft. Keep the wider set rather than draw nothing.
        if narrowed == 0 {
            return self;
        }
        Self { mask: narrowed }
    }
}

#[cfg(test)]
mod tests;
