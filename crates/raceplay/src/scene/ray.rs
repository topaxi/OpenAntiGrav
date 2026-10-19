//! The ray tier's view of a race: the circuit's and the hulls' acceleration structures,
//! built once at load, and the `--ray-debug` picture over them.
//!
//! Present only on a device that was created with the tier (`mesh_render::ray`); a race
//! on any other device holds nothing and builds nothing here. Render-side only: no
//! simulation state reads any of it.

use oag_mesh::mesh_render::ray;

use super::*;

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
        track: &Drawable,
        ships: &[Drawable],
    ) -> Option<Self> {
        if !ray::enabled(device) {
            return None;
        }
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
            log::warn!("ray: {} of {} hulls had nothing to trace", ships.len() - hulls.len(), ships.len());
        }
        let scene = ray::Scene::new(device, circuit, hulls, ships.len() as u32);
        log::info!("ray: circuit {}", scene.track_stats());
        Some(Self {
            scene,
            debug: ray::Debug::new(device),
        })
    }
}

impl Scene {
    /// Whether this race has acceleration structures.
    #[must_use]
    pub fn has_ray_tier(&self) -> bool {
        self.ray.is_some()
    }

    /// The `--ray-debug` picture of `race` as the raster frames it: one primary ray per
    /// pixel from the camera `Scene::render` would use for a `width` by `height`
    /// viewport, RGBA8 with row 0 on top. The craft are the ones the raster draws, at the
    /// matrices it draws them with. `None` without the tier.
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
        let nodes = self.track.model().sample_anim_nodes(self.anim_clock.get());
        let craft: Vec<(usize, [f32; 16])> = (0..usize::from(race.ship_count()))
            .filter(|&slot| !self.hull_skipped(race, slot))
            .map(|slot| (slot, race.ship_model_matrix_of(slot).to_cols_array()))
            .collect();
        let mut tier = tier.borrow_mut();
        let RayTier { scene, debug } = &mut *tier;
        Some(debug.render(
            device,
            queue,
            scene,
            &nodes,
            &craft,
            view_projection.inverse().to_cols_array(),
            (width, height),
        ))
    }
}
