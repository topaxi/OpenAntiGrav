//! The hull's extra pass, one [`Drawable`] per craft: the `0x2000` batches
//! redrawn after the hulls, under their material's second texture, with
//! texture coordinates generated from the craft's own rotation.
//!
//! The picture and its evidence are `oag_render::shine`'s; this is the
//! per-frame half, kept beside `absorb_overlay` and out of `frame.rs` under
//! the 1,000-line rule.

use super::*;

impl Scene {
    /// The slots whose shine pass draws this frame - the same craft the hull
    /// itself is drawn for.
    fn shines<'a>(&'a self, race: &'a Race) -> impl Iterator<Item = (usize, &'a Drawable)> + 'a {
        let drawn = usize::from(race.ship_count());
        self.shine
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, shine)| {
                if !race.ship_active(slot) || (slot == 0 && !race.draws_own_ship()) {
                    return None;
                }
                Some((slot, shine.as_ref()?))
            })
    }

    /// Writes every live craft's pose and its environment-mapped coordinates:
    /// [`oag_render::shine::write`] over the craft's model matrix, never the
    /// camera.
    pub(super) fn write_shine(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        for (slot, shine) in self.shines(race) {
            let ship = race.ship_model_matrix_of(slot);
            shine.write(
                queue,
                view_projection,
                ship,
                prev_vp * prev.ship(slot, race),
            );
            shine.write_environment_map(queue, ship, scratch);
        }
    }

    /// Draws every live craft's shine pass, after the hulls so their depth is
    /// in the buffer it tests against.
    pub(super) fn draw_shine(
        &self,
        race: &Race,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for (_, shine) in self.shines(race) {
            stats.add(shine.draw_overlay(pass));
        }
    }
}
