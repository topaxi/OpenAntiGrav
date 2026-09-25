//! Building [`oag_render::weapon_quads::Pipeline`] for [`super::Scene`].
//!
//! The state and the maths - the bolt streak's two crossed quads, the
//! muzzle flash's rotated one - are all `oag_render::weapon_quads`'; see
//! that module's own doc and
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
//! evidence. This is only the seam: the placeholder-on-absence fallback and
//! the one construction call, split out for the same reason
//! [`super::clouds`] is - see `scripts/check-file-size.py`.

use super::*;

/// The Cannon's quad pipeline and what drawing a round needs beside it - see
/// [`crate::race::CannonDraw`].
#[derive(Debug)]
pub(super) struct Cannon {
    /// `RefCell` for the reason [`super::Scene`]'s `exhaust` gives.
    pub(super) pipeline: std::cell::RefCell<oag_render::weapon_quads::Pipeline>,
    pub(super) draw: crate::race::CannonDraw,
}

/// Builds the pipeline, falling back to [`FlareTexture::placeholder`] for
/// either texture that did not decode - the same terms `flare`/`noise`
/// already use in [`super::Scene::new`]. Wrapped in the `RefCell`
/// [`super::Scene`] wants, so the call at the one site this has stays on
/// one line.
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    assets: crate::race::CannonAssets,
    liveries: &[Livery],
    sample_count: u32,
) -> Cannon {
    let (bolt, flash, look) = assets;
    let bolt = bolt.unwrap_or_else(|| FlareTexture::placeholder(64));
    let flash = flash.unwrap_or_else(|| FlareTexture::placeholder(64));
    let draw = crate::race::CannonDraw {
        look,
        muzzles: liveries.iter().map(|livery| livery.cannon_flash).collect(),
    };
    let pipeline = std::cell::RefCell::new(oag_render::weapon_quads::Pipeline::new(
        device,
        queue,
        format,
        &bolt,
        &flash,
        sample_count,
        // The race pass carries the velocity attachment - see
        // `mesh_render::Velocity` and `oag_render::weapon_quads`'s own doc
        // comment for why this pipeline still masks its own write.
        mesh_render::Velocity::Write,
    ));
    Cannon { pipeline, draw }
}

impl Scene {
    /// This frame's Cannon quads into `bolt`/`flash` - see
    /// [`Race::cannon_quad_vertices`]. A method here rather than the call
    /// inline in `frame.rs` for the line budget that file has none of.
    pub(super) fn gather_cannon_quads(
        &self,
        race: &Race,
        right: Vec3,
        up: Vec3,
        bolt: &mut Vec<oag_render::mesh::GpuVertex>,
        flash: &mut Vec<oag_render::mesh::GpuVertex>,
    ) {
        race.cannon_quad_vertices(&self.weapon_quads.draw, right, up, bolt, flash);
    }

    /// Uploads what [`Self::gather_cannon_quads`] built.
    pub(super) fn upload_cannon_quads(
        &self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        bolt: &[oag_render::mesh::GpuVertex],
        flash: &[oag_render::mesh::GpuVertex],
    ) {
        self.weapon_quads
            .pipeline
            .borrow_mut()
            .upload(queue, view_projection, bolt, flash);
    }
}
