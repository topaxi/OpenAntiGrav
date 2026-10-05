//! The two hull overlays' drawables, one of each per craft: the hull redrawn
//! under `absorb_surface.mip` for the second after that craft absorbs a
//! pickup, and under `leachbeam_surface.mip` while that craft's own LeachBeam
//! pulses.
//!
//! The picture is `oag_fx::hull_overlay`'s - one draw routine,
//! `HullOverlay_Submit`, serves both, and only the texture and the pulse
//! differ; this is only the per-frame half - where they are written, when
//! they are drawn - kept out of `frame.rs` under the 1,000-line rule.

use super::*;

/// Which overlay a pass is over.
#[derive(Clone, Copy)]
enum Overlay {
    Absorb,
    LeachBeam,
}

impl Scene {
    /// The slots whose `which` overlay draws this frame, with its pulse.
    fn hull_overlays<'a>(
        &'a self,
        race: &'a Race,
        which: Overlay,
    ) -> impl Iterator<Item = (usize, &'a Drawable, f32)> + 'a {
        let drawn = usize::from(race.ship_count());
        let drawables = match which {
            Overlay::Absorb => &self.absorb_overlay[0],
            Overlay::LeachBeam => &self.absorb_overlay[1],
        };
        drawables
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, overlay)| {
                let overlay = overlay.as_ref()?;
                if !race.ship_active(slot) || (slot == 0 && !race.draws_own_ship()) {
                    return None;
                }
                let pulse = match which {
                    Overlay::Absorb => race.absorb_overlay_pulse(slot),
                    Overlay::LeachBeam => race.leach_overlay_pulse(slot),
                };
                Some((slot, overlay, pulse?))
            })
    }

    /// Writes every live overlay's pose, tint and scroll: the craft's own
    /// matrix, [`oag_fx::hull_overlay::tint`] of
    /// [`oag_fx::hull_overlay::alpha`], and
    /// the texture coordinates slid by [`oag_fx::hull_overlay::scroll`].
    pub(super) fn write_absorb_overlays(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        self.write_shine(race, queue, view_projection, prev_vp, prev, scratch);
        for which in [Overlay::Absorb, Overlay::LeachBeam] {
            for (slot, overlay, pulse) in self.hull_overlays(race, which) {
                overlay.write(
                    queue,
                    view_projection,
                    race.ship_model_matrix_of(slot),
                    prev_vp * prev.ship(slot, race),
                );
                let a = oag_fx::hull_overlay::alpha(pulse);
                overlay.write_overlay(
                    queue,
                    oag_fx::hull_overlay::tint(a),
                    oag_fx::hull_overlay::scroll(pulse),
                    scratch,
                );
            }
        }
    }

    /// Draws every live overlay, after the hulls so their depth is in the
    /// buffer the overlay tests against.
    pub(super) fn draw_absorb_overlays(
        &self,
        race: &Race,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        self.draw_shine(race, pass, stats);
        for which in [Overlay::Absorb, Overlay::LeachBeam] {
            for (_, overlay, _) in self.hull_overlays(race, which) {
                stats.add(overlay.draw_overlay(pass));
            }
        }
    }
}
