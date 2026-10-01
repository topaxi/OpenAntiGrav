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

    /// Writes the craft's scene uniform: every hull's own, and the shine
    /// passes' (a copy with the fog colour **black**).
    ///
    /// One function for both so the hull and its extra pass cannot drift: the
    /// pass was drawn with fog off until the two were read side by side in a
    /// recorded GE list. On a live Metropia grid the six hull batches drawn
    /// under `TEXMAPMODE` 2 carry fog **enabled** with the fog colour register
    /// at `0`, where the ordinary pass over the same batch carries the
    /// circuit's own colour; the fog's distance registers are the same on both
    /// (`FOG1` 1600, `FOG2` 1/1540 there). So the pass fades to nothing with
    /// distance rather than into the haze, and a rival far down a straight
    /// loses its sheen as it loses the rest of its colour. Inside the fog's
    /// near distance, where the chase camera sits, it changes nothing.
    pub(super) fn write_ship_scenes(&self, queue: &wgpu::Queue, ship_scene: &mesh_render::Scene) {
        for drawable in self.ships.iter() {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(ship_scene));
        }
        let shine = shine_scene(ship_scene);
        for pass in self.shine.iter().flatten() {
            queue.write_buffer(&pass.fog, 0, bytemuck::bytes_of(&shine));
        }
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

/// `ship_scene` with the fog colour black: see [`Scene::write_ship_scenes`].
fn shine_scene(ship_scene: &mesh_render::Scene) -> mesh_render::Scene {
    mesh_render::Scene {
        fog: mesh_render::Fog {
            colour: [0.0; 3],
            ..ship_scene.fog
        },
        ..*ship_scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pass fades to black with distance and keeps the rest of the hulls'
    /// fog: a pass bound to the scene's own fog colour would add fog colour to
    /// the hull wherever the sheen sits.
    #[test]
    fn the_shine_pass_fogs_to_black_and_keeps_the_hulls_fog_otherwise() {
        let mut ship = mesh_render::Scene::off();
        ship.fog.colour = [1.0, 0.95, 0.77];
        ship.fog.near = 450.0;
        ship.fog.far = 1850.0;
        ship.fog.enabled = 1.0;
        let shine = shine_scene(&ship);
        assert_eq!(shine.fog.colour, [0.0; 3]);
        assert_eq!(
            (shine.fog.near, shine.fog.far, shine.fog.enabled),
            (450.0, 1850.0, 1.0)
        );
    }
}
