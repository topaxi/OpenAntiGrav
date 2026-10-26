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
    ///
    /// `sections` is the circuit's PVS placement (`TrackVisibility::sections`), so each
    /// section's geometry is a BLAS of its own and a frame instances only the sections its
    /// raster shows ([`Scene::ray_visible`]). `None` for a circuit with no section
    /// partition: one BLAS for its static geometry, in every frame.
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tlas: Option<wgpu::Tlas>,
        track: &Drawable,
        sections: Option<&DrawSections>,
        ships: &[Drawable],
    ) -> Option<Self> {
        let tlas = tlas?;
        let circuit = ray::ModelAccel::build(
            device,
            queue,
            "circuit",
            track.model(),
            sections.map(|s| ray::Sections {
                opaque: &s.opaque,
                alpha_tested: &s.alpha_tested,
            }),
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
                    None,
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
    pub(super) fn update_ray_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        pvs_cull: bool,
    ) {
        let Some(tier) = &self.ray else {
            return;
        };
        let (nodes, craft) = self.ray_instances(race);
        let visible = self.ray_visible(race, pvs_cull);
        let mut tier = tier.borrow_mut();
        tier.scene.update(encoder, &nodes, &craft, visible);
    }

    /// The sections the TLAS instances this frame: what the raster may draw in any direction
    /// from every view of the frame (`Race::ray_visible_mask`), or everything where the
    /// raster hides nothing - `pvs_culling` off, or a circuit with no section partition
    /// (Pure, and every PS3, Vita and PS4 circuit, whose chunk partition is not applied here).
    ///
    /// **Why it follows the raster.** A Pulse circuit authors alternative copies of one road
    /// in different sections (a far-LOD copy a few hundredths of a unit above or below the
    /// detailed one, by turns), and its visibility masks never name both. A structure
    /// holding every section let each copy shadow the other in triangle-shaped patches.
    fn ray_visible(&self, race: &Race, pvs_cull: bool) -> u64 {
        self.visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .filter(|visibility| visibility.has_sections())
            .map_or(ray::ALWAYS, |visibility| race.ray_visible_mask(visibility))
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
    /// viewport, RGBA8 with row 0 on top, over the sections the frame's shadows trace
    /// against (`pvs_cull` as the frame was drawn with). `None` without the tier.
    #[must_use]
    pub fn ray_debug(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        race: &Race,
        fov: oag_display::display::Fov,
        pvs_cull: bool,
        (width, height): (u32, u32),
    ) -> Option<Vec<u8>> {
        let tier = self.ray.as_ref()?;
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let view_projection = race.projection(aspect, self.far, fov) * race.view();
        let (nodes, craft) = self.ray_instances(race);
        let visible = self.ray_visible(race, pvs_cull);
        let mut tier = tier.borrow_mut();
        let RayTier { scene, debug } = &mut *tier;
        Some(debug.render(
            device,
            queue,
            scene,
            &ray::Frame {
                nodes: &nodes,
                craft: &craft,
                visible,
            },
            view_projection.inverse().to_cols_array(),
            (width, height),
        ))
    }
}
