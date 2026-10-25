//! The ray tier's view of a race: the circuit's and the hulls' acceleration structures,
//! built once at load, and the `--ray-debug` picture over them.
//!
//! Present only on a device that was created with the tier (`mesh_render::ray`); a race
//! on any other device holds nothing and builds nothing here. Render-side only: no
//! simulation state reads any of it.

use oag_mesh::mesh_render::ray;

use super::*;

/// A frame's TLAS inputs: animated node matrices, then each craft's slot and model matrix.
type Instances = (Vec<[f32; 16]>, Vec<(usize, [f32; 16])>);

/// The rays a `high` quality `traced` pixel casts across the sun's disc.
///
/// **Ours.** Eight is the fewest that keep the penumbra from banding into
/// visible steps at a canopy's distance; each is one more ray query per lit
/// pixel, so `high` costs that many times `medium`'s traversal.
const TRACED_SOFT_RAYS: f32 = 8.0;

/// What a race holds when the tier is on.
#[derive(Debug)]
pub(super) struct RayTier {
    scene: ray::Scene,
    debug: ray::Debug,
}

impl RayTier {
    /// Builds the circuit's BLASes from `track` and one hull BLAS per ship drawable.
    /// `None` when the circuit has nothing opaque to trace.
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tlas: Option<wgpu::Tlas>,
        track: &Drawable,
        ships: &[Drawable],
    ) -> Option<Self> {
        let tlas = tlas?;
        let circuit = ray::ModelAccel::build(
            device,
            queue,
            "circuit",
            track.model(),
            track.vertex_buffer(),
        )?;
        let hulls: ray::Hulls = ships
            .iter()
            .enumerate()
            .filter_map(|(slot, ship)| {
                ray::ModelAccel::build(
                    device,
                    queue,
                    &format!("hull {slot}"),
                    ship.model(),
                    ship.vertex_buffer(),
                )
            })
            .collect();
        if hulls.len() != ships.len() {
            log::warn!(
                "ray: {} of {} hulls had nothing to trace",
                ships.len() - hulls.len(),
                ships.len()
            );
        }
        let scene = ray::Scene::new(tlas, circuit, hulls, ships.len() as u32);
        log::info!("ray: circuit {}", scene.track_stats());
        Some(Self {
            scene,
            debug: ray::Debug::new(device),
        })
    }
}

impl Scene {
    /// What the TLAS holds this frame: the circuit's animated nodes at the frame's clock
    /// and each craft the raster draws, at the matrix it draws with.
    fn ray_instances(&self, race: &Race) -> Instances {
        let nodes = self.track.model().sample_anim_nodes(self.anim_clock.get());
        let craft = (0..usize::from(race.ship_count()))
            .filter(|&slot| !self.hull_skipped(race, slot))
            .map(|slot| (slot, race.ship_model_matrix_of(slot).to_cols_array()))
            .collect();
        (nodes, craft)
    }

    /// Rebuilds the TLAS for this frame into `encoder`, ahead of the scene pass. A no-op
    /// without the tier.
    ///
    /// **Every frame the tier is on, whatever the shadow setting**: every drawable's scene
    /// group names the TLAS, and wgpu refuses a draw whose bind group holds one that was
    /// never built.
    pub(super) fn update_ray_scene(&self, encoder: &mut wgpu::CommandEncoder, race: &Race) {
        let Some(tier) = &self.ray else {
            return;
        };
        let (nodes, craft) = self.ray_instances(race);
        tier.borrow_mut().scene.update(encoder, &nodes, &craft);
    }

    /// The shadow tier this frame draws: `shadows`, except that `traced` on a race without
    /// acceleration structures is `mapped`, said once. Never an error, and never silent: a
    /// tier that quietly becomes another is how a missing feature stops being noticed.
    pub(super) fn resolve_shadows(
        &self,
        shadows: oag_display::display::Shadows,
    ) -> oag_display::display::Shadows {
        use oag_display::display::Shadows;
        if shadows == Shadows::Traced && self.ray.is_none() {
            static SAID: std::sync::Once = std::sync::Once::new();
            SAID.call_once(|| {
                log::warn!(
                    "shadows traced: this device was not created with ray queries (the adapter \
                     lacks them, or the tier was not requested at launch: --ray, OAG_RAY=1 or \
                     --shadows traced); drawing mapped"
                );
            });
            return Shadows::Mapped;
        }
        shadows
    }

    /// How the `traced` tier softens from the next frame on. Called before every
    /// draw, as [`Scene::set_blur_resolution`] is, so the menu row applies live.
    pub fn set_traced_quality(&self, quality: oag_display::display::TracedQuality) {
        self.traced_quality.set(quality);
    }

    /// How many rays a `traced` pixel casts at the current quality: one for
    /// `medium`, [`TRACED_SOFT_RAYS`] for `high`.
    pub(super) fn traced_rays(&self) -> f32 {
        match self.traced_quality.get() {
            oag_display::display::TracedQuality::Medium => 1.0,
            oag_display::display::TracedQuality::High => TRACED_SOFT_RAYS,
        }
    }

    /// Whether this race has acceleration structures.
    #[must_use]
    pub fn has_ray_tier(&self) -> bool {
        self.ray.is_some()
    }

    /// The `--ray-debug` picture of `race` as the raster frames it: one primary ray per
    /// pixel from the camera `Scene::render` would use for a `width` by `height`
    /// viewport, RGBA8 with row 0 on top. `None` without the tier.
    #[must_use]
    pub fn ray_debug(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        race: &Race,
        fov: oag_display::display::Fov,
        (width, height): (u32, u32),
    ) -> Option<Vec<u8>> {
        let tier = self.ray.as_ref()?;
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let view_projection = race.projection(aspect, self.far, fov) * race.view();
        let (nodes, craft) = self.ray_instances(race);
        let mut tier = tier.borrow_mut();
        let RayTier { scene, debug } = &mut *tier;
        Some(debug.render(
            device,
            queue,
            scene,
            &ray::Frame {
                nodes: &nodes,
                craft: &craft,
            },
            view_projection.inverse().to_cols_array(),
            (width, height),
        ))
    }
}
