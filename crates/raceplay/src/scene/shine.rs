//! The hull's extra pass, one [`Drawable`] per craft: the `0x2000` batches
//! redrawn after the hulls, under their material's second texture, with
//! texture coordinates generated from the craft's own rotation.
//!
//! The picture and its evidence are `oag_render::shine`'s; this is the
//! per-frame half, kept beside `absorb_overlay` and out of `frame.rs` under
//! the 1,000-line rule.

use super::*;
use crate::drawable::TrackCull;

/// A circuit's extra pass: its drawable and, for each draw of it, the circuit
/// draw it redraws - the key the circuit's own section mask, level-of-detail
/// child and frustum bound are looked up by, so the pass shows exactly what
/// the circuit shows.
#[derive(Debug)]
pub(super) struct TrackShine {
    drawable: Drawable,
    sources: Vec<usize>,
    /// This frame's coordinates, refilled rather than rebuilt - the drawable
    /// streams them alone (`mesh_render::Texcoords::Streamed`).
    texcoords: std::cell::RefCell<Vec<[f32; 2]>>,
    /// The vertices' turned normals, which no camera decides: computed once for the race
    /// when no node moves, else once per clock value, however many views draw at it.
    normals: std::cell::RefCell<oag_render::shine::TrackNormals>,
    /// For each draw, the vertices it reads ([`oag_render::shine::draw_vertices`]).
    vertices: Vec<std::ops::Range<usize>>,
    /// This view's share of [`Self::vertices`], refilled rather than rebuilt.
    shown: std::cell::RefCell<Vec<std::ops::Range<usize>>>,
}

impl TrackShine {
    /// The pass's drawable and, per draw, the circuit draw it redraws: what recording it reads.
    pub(super) fn parts(&self) -> (&Drawable, &[usize]) {
        (&self.drawable, &self.sources)
    }

    /// Moves the pass's matrices onto the shared buffer.
    pub(super) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        self.drawable.share_uniforms(device, blocks);
    }

    /// Moves the pass onto the shine block, written with the hulls' shine passes'.
    pub(super) fn share_scene(&mut self, device: &wgpu::Device, blocks: &mut SceneBlocks) {
        self.drawable.share_scene(device, blocks, SceneKind::Shine);
    }

    /// `None` when the circuit authors no drawable shine batch.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track: &Model,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        anisotropy: Anisotropy,
        sample_count: u32,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Option<Self>> {
        let Some(pass) = oag_render::shine::build_track(track) else {
            return Ok(None);
        };
        let vertices = oag_render::shine::draw_vertices(&pass.model);
        let drawable = Drawable::new_with(
            device,
            queue,
            pass.model,
            format,
            store,
            anisotropy,
            sample_count,
            mesh_render::Depth::Overlay,
            oag_render::shine::BLEND,
            mesh_render::GlowMask::Written,
            zone_art,
            shadow_maps,
            mesh_render::ShadowReceiver::Never,
            mesh_render::Texcoords::Streamed,
            false,
            false,
            mesh_render::Velocity::Write,
            true,
        )?;
        Ok(Some(Self {
            drawable,
            sources: pass.sources,
            texcoords: std::cell::RefCell::default(),
            normals: std::cell::RefCell::default(),
            vertices,
            shown: std::cell::RefCell::default(),
        }))
    }
}

impl Scene {
    /// The slots whose shine pass this view draws - the same craft the hull itself is drawn
    /// for, frustum included ([`Scene::hull_seen`]), so a craft behind the camera costs its
    /// pass neither a map nor a draw.
    fn shines<'a: 'f, 'f>(
        &'a self,
        race: &'a Race,
        frustum: Option<&'f Frustum>,
    ) -> impl Iterator<Item = (usize, &'a Drawable)> + 'f {
        let drawn = usize::from(race.ship_count());
        self.shine
            .iter()
            .enumerate()
            .take(drawn)
            .filter_map(move |(slot, shine)| {
                if !race.ship_active(slot)
                    || self.hull_skipped(race, slot)
                    || !self.hull_seen(race, slot, frustum)
                {
                    return None;
                }
                Some((slot, shine.as_ref()?))
            })
    }

    /// Writes every drawn craft's pose and the environment-mapped coordinates:
    /// [`oag_render::shine::write`] over the craft's model matrix, never the camera.
    ///
    /// **The coordinates are the frame's, not the view's**: a craft's pose, its flaps and the
    /// set of live craft (`Scene::shines` reads the slot, never the viewpoint) are the same
    /// from every camera, so only the first view of a split uploads them
    /// ([`Scene::does_frame_work`]) and a later view draws what it left in the vertex
    /// buffers. That first view maps every craft, not only those it sees: a later view's
    /// write is copied into its log and again into the queue, which costs a split more than
    /// mapping a craft no view shows. A race of one view maps what it draws. The pose uniform
    /// carries the view-projection and stays per view.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_shine(
        &self,
        race: &Race,
        queue: &Uploads<'_>,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &motion::Snapshot,
        block: ViewBlock,
        cull: TrackCull<'_>,
        scratch: &mut Vec<mesh::GpuVertex>,
    ) {
        if let Some(track) = &self.track_shine {
            let view = race.view();
            let seconds = self.anim_clock.get();
            track
                .drawable
                .write(queue, view_projection, Mat4::IDENTITY, prev_vp);
            if self.does_frame_work() {
                track
                    .drawable
                    .write_node_anims_started(queue, seconds, &|start| race.channel_started(start));
            }
            let mut shown = track.shown.borrow_mut();
            shown.clear();
            shown.extend(
                track
                    .drawable
                    .track_shine_shown(block, &self.track, &track.sources, cull)
                    .map(|index| track.vertices[index].clone()),
            );
            oag_render::shine::merge_spans(&mut shown);
            track.drawable.write_view_map(
                queue,
                view,
                seconds,
                &mut track.normals.borrow_mut(),
                &mut track.texcoords.borrow_mut(),
                &shown,
            );
        }
        // The flaps the hull's base draw swings: a person's own (`deflect_airbrakes` runs for
        // every craft a person flies), so a rival's pass stays with its own stowed hull.
        for (slot, shine) in self.shines(race, cull.frustum) {
            shine.write(
                queue,
                view_projection,
                race.ship_model_matrix_of(slot),
                prev_vp * prev.ship(slot, race),
            );
        }
        if self.does_frame_work() {
            let mapped = cull
                .frustum
                .filter(|_| self.layer.get() == super::viewports::Layer::Whole);
            for (slot, shine) in self.shines(race, mapped) {
                let ship = race.ship_model_matrix_of(slot);
                let flaps = race.airbrake_flaps_of(slot);
                shine.write_environment_map(queue, ship, flaps, scratch);
            }
            self.frame_work.update(|w| super::FrameWork {
                shine_maps: w.shine_maps + 1,
                ..w
            });
        }
    }

    /// Draws the shine pass of every craft this view shows, after the hulls so their depth is
    /// in the buffer it tests against.
    pub(super) fn draw_shine<'a>(
        &'a self,
        race: &'a Race,
        frustum: Option<&Frustum>,
        list: &mut super::record::DrawList<'a>,
    ) {
        for (_, shine) in self.shines(race, frustum) {
            list.overlay(shine);
        }
    }
}

/// The shine passes' scene block: `ship_scene` with the fog colour **black**.
///
/// Written with the hull's own, one function's worth, so the hull and its extra pass cannot
/// drift: the pass was drawn with fog off until the two were read side by side in a recorded GE
/// list. On a live Metropia grid the six hull batches drawn under `TEXMAPMODE` 2 carry fog
/// **enabled** with the fog colour register at `0`, where the ordinary pass over the same batch
/// carries the circuit's own colour; the fog's distance registers are the same on both (`FOG1`
/// 1600, `FOG2` 1/1540 there). So the pass fades to nothing with distance rather than into the
/// haze, and a rival far down a straight loses its sheen as it loses the rest of its colour.
/// Inside the fog's near distance, where the chase camera sits, it changes nothing.
pub(super) fn shine_scene(ship_scene: &mesh_render::Scene) -> mesh_render::Scene {
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
