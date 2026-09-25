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
    zone_art: &mesh_render::zone::StageArt,
    shadow_maps: mesh_render::ShadowMaps<'_>,
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
                shadow_maps,
                mesh_render::ShadowReceiver::Mapped,
            )?);
        }
    }
    Ok(drawables)
}

/// Builds every weapon's own drawable pool in one call - the Rocket's, the
/// Mine's, the Bomb's, the Cannon round's, the Plasma blast's three and the
/// Bomb blast's two - each with [`build`], on `Scene::new`'s own terms.
///
/// One function rather than one `let` per kind in `scene.rs`, for the same
/// line-budget reason [`super::super::load::weapon_models::load_bodies`]
/// exists on the loading side.
/// [`build_all`]'s own return: one drawable pool per kind, the Plasma's own
/// (the bolt's head included) grouped as [`blast_models::PlasmaBlastDrawables`]
/// already are, and the Bomb's own two grouped as
/// [`bomb_blast::BombBlastDrawables`].
type WeaponBodies = (
    Vec<Drawable>,
    Vec<Drawable>,
    Vec<Drawable>,
    Vec<Drawable>,
    blast_models::PlasmaBlastDrawables,
    bomb_blast::BombBlastDrawables,
);

/// [`Scene::write_weapon_models`]'s own return: one matrix list per kind, in
/// the same order [`WeaponBodies`] builds its drawable pools.
type WeaponMatrices = (Vec<Mat4>, Vec<Mat4>, Vec<Mat4>, Vec<Mat4>, Vec<Mat4>);

#[allow(clippy::too_many_arguments)]
pub(super) fn build_all(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rocket_model: Option<Model>,
    mine_model: Option<Model>,
    bomb_model: Option<Model>,
    cannon_model: Option<Model>,
    plasma_blast_models: blast_models::PlasmaBlastModels,
    bomb_blast_models: bomb_blast::BombBlastModels,
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    scene_depth: mesh_render::Depth,
    zone_art: &mesh_render::zone::StageArt,
    shadow_maps: mesh_render::ShadowMaps<'_>,
) -> Result<WeaponBodies> {
    let one = |model| {
        build(
            device,
            queue,
            model,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            zone_art,
            shadow_maps,
        )
    };
    Ok((
        one(rocket_model)?,
        one(mine_model)?,
        one(bomb_model)?,
        one(cannon_model)?,
        blast_models::PlasmaBlastDrawables::build(plasma_blast_models, one)?,
        bomb_blast::BombBlastDrawables::build(bomb_blast_models, one)?,
    ))
}

impl super::Scene {
    /// Writes this tick's matrices onto the Rocket's, the Plasma bolt's, the
    /// Mine's, the Bomb's and the Cannon round's drawables, and hands back
    /// every matrix list so the caller can bound its own draw loops the same
    /// way it already does for the ships - a drawable past the live count
    /// keeps last frame's uniforms and must not be drawn.
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
    ) -> WeaponMatrices {
        let rocket_matrices = race.rocket_model_matrices();
        let plasma_ball_matrices = race.plasma_ball_model_matrices();
        let mine_matrices = race.mine_model_matrices();
        let bomb_matrices = race.bomb_model_matrices();
        let cannon_matrices = race.cannon_model_matrices(&self.weapon_quads.draw);
        write_one_kind(
            &self.rockets,
            &rocket_matrices,
            &prev.rockets,
            queue,
            view_projection,
            prev_vp,
        );
        write_one_kind(
            &self.plasma_blast.ball,
            &plasma_ball_matrices,
            &prev.plasma_balls,
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
        write_one_kind(
            &self.cannon_rounds,
            &cannon_matrices,
            &prev.cannon_rounds,
            queue,
            view_projection,
            prev_vp,
        );
        (
            rocket_matrices,
            plasma_ball_matrices,
            mine_matrices,
            bomb_matrices,
            cannon_matrices,
        )
    }
}

impl super::Scene {
    /// Writes this tick's transform and anim-time scrub onto every live
    /// Plasma blast's three drawables, and hands back which slots are live
    /// so the caller knows which to draw.
    ///
    /// **Sparse, unlike [`Self::write_weapon_models`]'s four pools.** A
    /// projectile pool swap-removes to keep its live entries packed from
    /// slot 0, so those four are bounded by a simple live *count*; a blast
    /// frees its own slot the moment [`blast_models::PLASMA_BLAST_LIFETIME_SECONDS`]
    /// runs out, independent of every other slot, so this hands back which
    /// slots are live rather than how many.
    ///
    /// **One `matrix` writes all three drawables** - see
    /// [`blast_models::PlasmaBlastDraw`]'s own doc comment for why - so only
    /// the anim-time scrub passed to [`Drawable::write_node_anims`] differs
    /// per model.
    ///
    /// **No previous-frame matrix is tracked for this pool**, so a blast
    /// draws with no motion blur of its own: `prev_mvp` is passed as this
    /// frame's own `view_projection * matrix`, which zeroes the shader's
    /// velocity term rather than smearing from a frame this pool never
    /// recorded. **Chosen, not measured** - a short-lived effect the camera
    /// is rarely tracking directly, against the cost of a second matrix
    /// array only this pool would need.
    pub(super) fn write_plasma_blasts(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
    ) -> [bool; blast_models::PLASMA_BLAST_SLOTS] {
        let draws = race.plasma_blast_draws(race.camera_position());
        let mut active = [false; blast_models::PLASMA_BLAST_SLOTS];
        for (slot, draw) in draws.iter().enumerate() {
            let Some(draw) = draw else { continue };
            active[slot] = true;
            for (index, (drawables, seconds)) in [
                (&self.plasma_blast.halo, draw.halo_seconds),
                (&self.plasma_blast.hemisphere2, draw.hemisphere2_seconds),
                (&self.plasma_blast.hemisphere1, draw.hemisphere1_seconds),
            ]
            .into_iter()
            .enumerate()
            {
                let Some(drawable) = drawables.get(slot) else {
                    continue;
                };
                // **HD scales each model on top of the shared billboard
                // basis; Pulse scrubs a baked anim-time track on the basis
                // alone.** See `blast_models`'s own module doc comment for
                // why these do not fold into one branch.
                let matrix = match draw.hd_scale {
                    Some(scale) => draw.matrix * Mat4::from_scale(Vec3::splat(scale[index])),
                    None => draw.matrix,
                };
                let mvp = view_projection * matrix;
                drawable.write(queue, view_projection, matrix, mvp);
                if draw.hd_scale.is_none() {
                    drawable.write_node_anims(queue, seconds);
                }
            }
        }
        active
    }

    /// Draws every live blast's three models - `active` is
    /// [`Self::write_plasma_blasts`]'s own return, sparse rather than a
    /// leading count the way the four kinds above are bounded.
    pub(super) fn draw_plasma_blasts(
        &self,
        active: &[bool; blast_models::PLASMA_BLAST_SLOTS],
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        let pools = [
            &self.plasma_blast.halo,
            &self.plasma_blast.hemisphere2,
            &self.plasma_blast.hemisphere1,
        ];
        for (slot, _) in active.iter().enumerate().filter(|(_, live)| **live) {
            for pool in pools {
                if let Some(drawable) = pool.get(slot) {
                    stats.add(drawable.draw(pass, None, None, None, None));
                }
            }
        }
    }
}

impl super::Scene {
    /// Writes this tick's transform onto every live Bomb blast's two
    /// drawables, and hands back which slots are live *and visible*, per
    /// model - the same sparse shape [`Self::write_plasma_blasts`] takes,
    /// two arrays rather than one because the hemisphere and the shockwave
    /// hide on two different schedules (`bomb_blast::HEMISPHERE_HIDE_AT_SECONDS`
    /// against the whole object's own retire).
    ///
    /// **No per-tick anim-time scrub, unlike the Plasma's.**
    /// `BombBlast_Update` scales the basis directly rather than scrubbing a
    /// baked node animation - see `bomb_blast`'s own module doc comment - so
    /// a plain [`Drawable::write`] carries the whole picture.
    pub(super) fn write_bomb_blasts(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
    ) -> (
        [bool; bomb_blast::BOMB_BLAST_SLOTS],
        [bool; bomb_blast::BOMB_BLAST_SLOTS],
    ) {
        let draws = race.bomb_blast_draws();
        let mut hemisphere_active = [false; bomb_blast::BOMB_BLAST_SLOTS];
        let mut shockwave_active = [false; bomb_blast::BOMB_BLAST_SLOTS];
        for (slot, draw) in draws.iter().enumerate() {
            let Some(draw) = draw else { continue };
            if let (true, Some(drawable)) = (
                draw.hemisphere_visible,
                self.bomb_blast.hemisphere.get(slot),
            ) {
                let mvp = view_projection * draw.hemisphere_matrix;
                drawable.write(queue, view_projection, draw.hemisphere_matrix, mvp);
                hemisphere_active[slot] = true;
            }
            if let (true, Some(drawable)) =
                (draw.shockwave_visible, self.bomb_blast.shockwave.get(slot))
            {
                let mvp = view_projection * draw.shockwave_matrix;
                drawable.write(queue, view_projection, draw.shockwave_matrix, mvp);
                shockwave_active[slot] = true;
            }
        }
        (hemisphere_active, shockwave_active)
    }

    /// Draws every live Bomb blast's two models - the two arrays are
    /// [`Self::write_bomb_blasts`]'s own return.
    pub(super) fn draw_bomb_blasts(
        &self,
        hemisphere_active: &[bool; bomb_blast::BOMB_BLAST_SLOTS],
        shockwave_active: &[bool; bomb_blast::BOMB_BLAST_SLOTS],
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for (slot, _) in hemisphere_active
            .iter()
            .enumerate()
            .filter(|(_, live)| **live)
        {
            if let Some(drawable) = self.bomb_blast.hemisphere.get(slot) {
                stats.add(drawable.draw(pass, None, None, None, None));
            }
        }
        for (slot, _) in shockwave_active
            .iter()
            .enumerate()
            .filter(|(_, live)| **live)
        {
            if let Some(drawable) = self.bomb_blast.shockwave.get(slot) {
                stats.add(drawable.draw(pass, None, None, None, None));
            }
        }
    }
}

/// One kind's own write loop - the shared body of
/// [`super::Scene::write_weapon_models`]'s four calls.
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
