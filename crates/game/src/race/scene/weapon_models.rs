//! What the Rocket, the Mine and the Bomb share on the GPU side: one
//! [`Drawable`] list per kind, built the same way and written the same way
//! every frame.
//!
//! Split out under the 1,000-line rule the moment a second and third weapon
//! needed the Rocket's own pattern (`scripts/check-file-size.py`); `scene.rs`
//! was at 985 of 1,000 and `frame.rs` at 978, with no room to triple the
//! Rocket's inline blocks in place. A move plus a genuine narrowing: before
//! [`Race::rocket_model_matrices`] filtered `kind.is_some()` rather than
//! `kind == Some(Weapon::Rocket)`, so every live projectile of *every* kind
//! drew as a Rocket mesh whenever one was airborne alongside a loaded
//! [`ROCKET_MODEL_ENTRY`] - see that function's doc comment.

use super::super::*;
use super::motion::Snapshot;

/// One kind's own drawables, one per pool slot - the Rocket's, the Mine's or
/// the Bomb's, built identically.
///
/// Empty when `model` is `None` or decoded to no geometry, same terms
/// [`super::Scene::new`] used for the Rocket alone before this existed.
///
/// A mine's hull is opaque - a painted shell, not a glow - which is why this
/// takes the same transparent blend and protected glow mask the Rocket's
/// drawables do; nothing here is Rocket-specific.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: Option<Model>,
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    scene_depth: mesh_render::Depth,
    zone_art: Option<&std::sync::Arc<oag_render::mesh::ModelTexture>>,
    shadow_map: &oag_render::shadow::map::Map,
) -> Result<Vec<Drawable>> {
    let mut drawables = Vec::new();
    if let Some(model) = model.filter(|model| !model.indices.is_empty()) {
        for _ in 0..oag_gameplay::projectile::MAX_PROJECTILES {
            drawables.push(Drawable::new(
                device,
                queue,
                model.clone(),
                format,
                anisotropy,
                sample_count,
                scene_depth,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
                zone_art,
                Some(shadow_map.view()),
                Some(shadow_map.depth_view()),
                mesh_render::ShadowReceiver::Mapped,
            )?);
        }
    }
    Ok(drawables)
}

impl super::Scene {
    /// Writes this tick's matrices onto the Rocket's, the Mine's and the
    /// Bomb's drawables, and hands back all three matrix lists so the caller
    /// can bound its own draw loops the same way it already does for the
    /// ships - a drawable past the live count keeps last frame's uniforms and
    /// must not be drawn.
    ///
    /// One matrix per live projectile of that kind. `zip` bounds the write the
    /// same way for all three: nothing in the air writes nothing, and a
    /// drawable whose uniform buffer went unwritten this frame is exactly the
    /// one the caller must not draw.
    ///
    /// A projectile that just spawned has no previous pose; this frame's is
    /// zero object velocity, so it takes camera blur alone for one frame
    /// rather than a smear from another slot. Indexed previous matrices do
    /// drift for one frame when a mid-list projectile despawns - bounded by
    /// the pass's own reach cap and accepted, same as the Rocket's own
    /// [ADR-0030](../../../../../docs/architecture/adr/0030-a-bounded-drift-in-motion-vectors-is-accepted.md).
    pub(super) fn write_weapon_models(
        &self,
        race: &Race,
        prev: &Snapshot,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        prev_vp: Mat4,
    ) -> (Vec<Mat4>, Vec<Mat4>, Vec<Mat4>) {
        let rocket_matrices = race.rocket_model_matrices();
        let mine_matrices = race.mine_model_matrices();
        let bomb_matrices = race.bomb_model_matrices();
        write_one_kind(
            &self.rockets,
            &rocket_matrices,
            &prev.rockets,
            queue,
            view_projection,
            prev_vp,
        );
        write_one_kind(
            &self.mines,
            &mine_matrices,
            &prev.mines,
            queue,
            view_projection,
            prev_vp,
        );
        write_one_kind(
            &self.bombs,
            &bomb_matrices,
            &prev.bombs,
            queue,
            view_projection,
            prev_vp,
        );
        (rocket_matrices, mine_matrices, bomb_matrices)
    }
}

/// One kind's own write loop - the shared body of
/// [`super::Scene::write_weapon_models`]'s three calls.
fn write_one_kind(
    drawables: &[Drawable],
    matrices: &[Mat4],
    previous: &[Mat4],
    queue: &wgpu::Queue,
    view_projection: Mat4,
    prev_vp: Mat4,
) {
    for (index, (drawable, matrix)) in drawables.iter().zip(matrices).enumerate() {
        let previous = previous.get(index).copied().unwrap_or(*matrix);
        drawable.write(queue, view_projection, *matrix, prev_vp * previous);
    }
}
