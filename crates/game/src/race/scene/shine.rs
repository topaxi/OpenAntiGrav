//! The hull's extra pass, one [`Drawable`] per craft: the `0x2000` batches
//! redrawn after the hulls, under their material's second texture, with
//! texture coordinates generated from the craft's own rotation.
//!
//! The picture and its evidence are `oag_render::shine`'s; this is the
//! per-frame half, kept beside `absorb_overlay` and out of `frame.rs` under
//! the 1,000-line rule.

use super::*;

/// A circuit's extra pass: its drawable and, for each draw of it, the circuit
/// draw it redraws - the key the circuit's own section mask, level-of-detail
/// child and frustum bound are looked up by, so the pass shows exactly what
/// the circuit shows.
#[derive(Debug)]
pub(super) struct TrackShine {
    drawable: Drawable,
    sources: Vec<usize>,
}

impl TrackShine {
    /// `None` when the circuit authors no drawable shine batch.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track: &Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Option<Self>> {
        let Some(pass) = oag_render::shine::build_track(track) else {
            return Ok(None);
        };
        let drawable = Drawable::new(
            device,
            queue,
            pass.model,
            format,
            anisotropy,
            sample_count,
            mesh_render::Depth::Overlay,
            oag_render::shine::BLEND,
            mesh_render::GlowMask::Written,
            zone_art,
            shadow_maps,
            mesh_render::ShadowReceiver::Never,
        )?;
        Ok(Some(Self {
            drawable,
            sources: pass.sources,
        }))
    }
}

impl Scene {
    /// Draws the circuit, then its extra pass over it: the pass is tested
    /// against the circuit's depth, so it follows the circuit and precedes
    /// everything drawn after it.
    pub(super) fn draw_track(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = self.track.draw(pass, sections, set, chunks, frustum);
        oag_render::perfprobe::marks::mark(pass, "track shine");
        if let Some(shine) = &self.track_shine {
            stats.add(shine.drawable.draw_track_shine(
                pass,
                &self.track,
                &shine.sources,
                sections,
                set,
                chunks,
                frustum,
            ));
        }
        stats
    }

    /// The slots whose shine pass draws this frame - the same craft the hull
    /// itself is drawn for.
    fn shines<'a>(&'a self, race: &'a Race) -> impl Iterator<Item = (usize, &'a Drawable)> + 'a {
        let drawn = usize::from(race.ship_count());
        self.shine
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, shine)| {
                if !race.ship_active(slot) || self.hull_skipped(race, slot) {
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
        let track_shine = self.track_shine.iter().map(|t| &t.drawable);
        for pass in self.shine.iter().flatten().chain(track_shine) {
            queue.write_buffer(&pass.fog, 0, bytemuck::bytes_of(&shine));
        }
        self.write_wreck_scenes(queue, ship_scene);
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
        if let Some(track) = &self.track_shine {
            let view = race.view();
            let seconds = self.anim_clock.get();
            track
                .drawable
                .write(queue, view_projection, Mat4::IDENTITY, prev_vp);
            track.drawable.write_node_anims(queue, seconds);
            track.drawable.write_view_map(queue, view, seconds, scratch);
        }
        // The flaps the hull's base draw swings: the player's alone
        // (`deflect_airbrakes` runs for `ships.first()`), so a rival's pass
        // stays with its own stowed hull.
        let player_flaps = race.airbrake_flaps();
        for (slot, shine) in self.shines(race) {
            let flaps = if slot == 0 { player_flaps } else { [0.0; 2] };
            let ship = race.ship_model_matrix_of(slot);
            shine.write(
                queue,
                view_projection,
                ship,
                prev_vp * prev.ship(slot, race),
            );
            shine.write_environment_map(queue, ship, flaps, scratch);
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
