//! The absorb hull overlay's drawables: one per craft, the hull redrawn under
//! `absorb_surface.mip` for the second after that craft absorbs a pickup.
//!
//! The picture is `oag_render::hull_overlay`'s; this is only the per-frame
//! half - where it is written, when it is drawn - kept out of `frame.rs`
//! under the 1,000-line rule.

use super::*;

impl Scene {
    /// The slots whose overlay draws this frame, with its pulse.
    fn absorb_overlays<'a>(
        &'a self,
        race: &'a Race,
    ) -> impl Iterator<Item = (usize, &'a Drawable, f32)> + 'a {
        let drawn = usize::from(race.ship_count());
        self.absorb_overlay
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, overlay)| {
                let overlay = overlay.as_ref()?;
                if !race.ship_active(slot) || (slot == 0 && !race.draws_own_ship()) {
                    return None;
                }
                Some((slot, overlay, race.absorb_overlay_pulse(slot)?))
            })
    }

    /// Writes every live overlay's pose, tint and scroll: the craft's own
    /// matrix, `(a, a, a, a)` from [`oag_render::hull_overlay::alpha`], and
    /// the texture coordinates slid by [`oag_render::hull_overlay::scroll`].
    pub(super) fn write_absorb_overlays(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        for (slot, overlay, pulse) in self.absorb_overlays(race) {
            overlay.write(
                queue,
                view_projection,
                race.ship_model_matrix_of(slot),
                prev_vp * prev.ship(slot, race),
            );
            let a = oag_render::hull_overlay::alpha(pulse);
            overlay.write_overlay(
                queue,
                [a; 4],
                oag_render::hull_overlay::scroll(pulse),
                scratch,
            );
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
        for (_, overlay, _) in self.absorb_overlays(race) {
            stats.add(overlay.draw_additive(pass));
        }
    }
}
