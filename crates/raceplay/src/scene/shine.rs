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
    /// This frame's coordinates, refilled rather than rebuilt - the drawable
    /// streams them alone (`mesh_render::Texcoords::Streamed`).
    texcoords: std::cell::RefCell<Vec<[f32; 2]>>,
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
        let drawable = Drawable::new_with(
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
            mesh_render::Texcoords::Streamed,
            false,
        )?;
        Ok(Some(Self {
            drawable,
            sources: pass.sources,
            texcoords: std::cell::RefCell::default(),
        }))
    }
}

impl Scene {
    /// Draws the circuit's solid lists, the sky, the circuit's blended list,
    /// then the circuit's extra pass over it: the pass is tested against the
    /// circuit's depth, so it follows the circuit and precedes everything
    /// drawn after it.
    ///
    /// **The sky goes behind the solid circuit rather than first.** It writes
    /// no depth, so it used to be drawn first comparing `Always`, filling the
    /// whole viewport for the circuit to cover - every covered sample shaded
    /// for nothing, the full frame's worth: 5.1 ms of HD's 47 ms race pass at
    /// 3200x1800 on an integrated GPU, and the largest single span in Pulse's.
    /// Drawn here with its depth range pinned to the far plane, it compares
    /// `LessEqual` against the 1.0 the pass cleared to, so it lands on exactly
    /// the samples nothing solid has reached - the ones it was visible on
    /// before. The blended list still draws over it, as it always did, and
    /// nothing solid ever read the colour underneath it.
    ///
    /// Neither culling tier is offered it: a skybox is never outside the
    /// frustum and belongs to no visibility section, which is the behaviour
    /// ADR-0011 already assumes for it. Its draw calls are deliberately **not**
    /// added to the stats either: being exempt from both tiers, folding them in
    /// would shift the denominator ADR-0011's and the roadmap's PVS
    /// effectiveness figures are quoted against.
    /// Sorts the circuit's opaque draws nearest first from `eye`, under HD's
    /// chain only, as the order its depth prepass lays depth down in - so the
    /// depth test rejects what is behind as early as it can. See
    /// `mesh_render::Prepass`.
    ///
    /// **Only the prepass reads it, and that is why it moves no pixel.** The
    /// depth `Less` leaves is the nearest surface whatever order it arrives
    /// in; the shading that follows keeps the model's own order. Sorting the
    /// shading itself was tried first, 2026-10-03: 44.1 -> 37.4 ms at
    /// 3200x1800 on an integrated GPU, at the cost of 1 to 419 pixels a
    /// capture where coplanar ties resolved to the nearer batch. The prepass
    /// took that to 32.4 ms with the original's picture back.
    ///
    /// The key is the distance from `eye` to the nearest point of each
    /// batch's bounding sphere; `sort_by` is stable, so equal keys keep the
    /// model's order.
    pub(super) fn sort_opaque(&self, eye: Vec3) {
        let mut order = self.opaque_order.borrow_mut();
        order.clear();
        if self.hd.is_none() {
            return;
        }
        order.extend(
            self.track
                .opaque_draws()
                .iter()
                .zip(0u32..)
                .map(|(draw, i)| {
                    let centre = Vec3::from_array(draw.bounds.centre);
                    (centre.distance(eye) - draw.bounds.radius, i)
                }),
        );
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    pub(super) fn draw_track(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        viewport: (f32, f32, f32, f32),
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        oag_gpu::perfprobe::marks::mark(pass, "track solid");
        let order = self.opaque_order.borrow();
        let order = (!order.is_empty()).then_some(&order[..]);
        let mut stats =
            self.track
                .draw_lists(pass, sections, set, chunks, frustum, Lists::Solid { order });
        oag_gpu::perfprobe::marks::mark(pass, "sky");
        if let Some(sky) = &self.sky {
            let (x, y, w, h) = viewport;
            pass.set_viewport(x, y, w, h, 1.0, 1.0);
            let _ = sky.draw(pass, None, None, None, None);
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
        }
        oag_gpu::perfprobe::marks::mark(pass, "track blended");
        stats.add(
            self.track
                .draw_lists(pass, sections, set, chunks, frustum, Lists::Blended),
        );
        oag_gpu::perfprobe::marks::mark(pass, "track shine");
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
        // **The Plasma bolt's head reads the eye out of this block** (`rim = 1 -
        // N.V`), and a drawable nothing writes holds `Scene::off`, whose camera
        // is the origin: the rim came out of the direction to the world's origin
        // and painted the whole disc white. Fog and light reach it as they
        // reach a hull; its program applies neither.
        //
        // **The LeachBall's `RIM_GLOW` reads the same eye** (`rim = saturate(1 -
        // N.V)`, brightest face-on): left on `Scene::off` its brightness and
        // falloff came from the direction to the world's origin.
        for drawable in self
            .plasma_blast
            .ball
            .iter()
            .chain(self.leach_ball.as_ref())
        {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(ship_scene));
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
        if let Some(track) = &self.track_shine {
            let view = race.view();
            let seconds = self.anim_clock.get();
            track
                .drawable
                .write(queue, view_projection, Mat4::IDENTITY, prev_vp);
            track.drawable.write_node_anims(queue, seconds);
            track
                .drawable
                .write_view_map(queue, view, seconds, &mut track.texcoords.borrow_mut());
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
