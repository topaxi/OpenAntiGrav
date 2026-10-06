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
use oag_weapons::projectile::repulser::POOL_SIZE;

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
    let mut drawables: Vec<Drawable> = Vec::new();
    if let Some(model) = model.filter(|model| !model.indices.is_empty()) {
        for _ in 0..oag_weapons::projectile::MAX_PROJECTILES {
            // Every slot but the first is another name for the first's GPU
            // resources - see `Drawable::instance` for what it shares.
            if let Some(slot) = drawables.first().map(|first| first.instance(device, queue)) {
                drawables.push(slot);
                continue;
            }
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
    /// Each of the four projectile pools' length, and whether every slot in it
    /// draws from the first slot's own vertex and index buffers.
    #[cfg(test)]
    pub(crate) fn weapon_pool_sharing(&self) -> [(usize, bool); 4] {
        [&self.rockets, &self.mines, &self.bombs, &self.cannon_rounds].map(|pool| {
            let shared = pool.iter().all(|slot| {
                pool.first()
                    .is_some_and(|first| first.shares_geometry_with(slot))
            });
            (pool.len(), shared)
        })
    }

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
        seconds: f32,
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
            None,
        );
        write_one_kind(
            &self.plasma_blast.ball,
            &plasma_ball_matrices,
            &prev.plasma_balls,
            queue,
            view_projection,
            prev_vp,
            None,
        );
        write_one_kind(
            &self.plasma_blast.shuriken,
            &race.shuriken_model_matrices(),
            &prev.shurikens,
            queue,
            view_projection,
            prev_vp,
            Some(seconds),
        );
        write_one_kind(
            &self.mines,
            &mine_matrices,
            &prev.mines,
            queue,
            view_projection,
            prev_vp,
            Some(seconds),
        );
        write_one_kind(
            &self.bombs,
            &bomb_matrices,
            &prev.bombs,
            queue,
            view_projection,
            prev_vp,
            Some(seconds),
        );
        write_one_kind(
            &self.cannon_rounds,
            &cannon_matrices,
            &prev.cannon_rounds,
            queue,
            view_projection,
            prev_vp,
            None,
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
    /// Draws the `live` Shuriken blades [`Self::write_weapon_models`] wrote.
    pub(super) fn draw_shurikens(
        &self,
        live: usize,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for drawable in self.plasma_blast.shuriken.iter().take(live) {
            stats.add(drawable.draw(pass, None, None, None, None));
        }
    }

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
                if draw.hd_scale.is_some() {
                    drawable.write_clocked(queue, view_projection, matrix, mvp, draw.hd_clock);
                } else {
                    drawable.write(queue, view_projection, matrix, mvp);
                }
                if draw.hd_scale.is_none() {
                    drawable.write_node_anims(queue, seconds);
                    // `Node_SetAnimTimeTree` hands the one scrubbed time to
                    // the `Anim Transform` and `Mesh` nodes of the tree alike
                    // (anim-transform.md), so the texture offsets ride the
                    // same `age * rate`. `hemisphere1` has no node animation
                    // at all: this is the only thing it writes.
                    drawable.write_anims(queue, seconds);
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
    /// **No per-tick node scrub, unlike the Plasma's.** `BombBlast_Update`
    /// scales the basis directly rather than scrubbing a baked node animation
    /// (see `bomb_blast`'s own module doc comment), so [`Drawable::write`]
    /// carries the geometry. The *texture* tracks play on the blast's own age
    /// (`Drawable::write_anims`): `BombBlast_Construct` and `Repulser_Init`
    /// seed each model with `Node_SetAnimTimeTree(0.0)` and `Mesh`'s per-frame
    /// update then adds the clock's delta, so the texture time is the object's
    /// age at rate 1 (measured live on PPSSPP, `scenery-animation.md`).
    ///
    /// The magstrip effect's pair rides here too, on the container's terms
    /// (`bomb_blast::BombBlastModels::mag_floor`). It does scrub: MagEffect2
    /// authors an `Anim Transform`, played on the one animation clock
    /// `seconds`.
    pub(super) fn write_bomb_blasts(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        seconds: f32,
    ) -> BlastsActive {
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
                drawable.write_anims(queue, draw.age);
                hemisphere_active[slot] = true;
            }
            if let (true, Some(drawable)) =
                (draw.shockwave_visible, self.bomb_blast.shockwave.get(slot))
            {
                let mvp = view_projection * draw.shockwave_matrix;
                drawable.write(queue, view_projection, draw.shockwave_matrix, mvp);
                drawable.write_anims(queue, draw.age);
                // The ambient light's alpha the blast's update eases (`bomb_blast`'s doc): the
                // vertex colours times white with that alpha. Per live shockwave, so a scratch of
                // its own rather than the scene's.
                drawable.tint(
                    queue,
                    [1.0, 1.0, 1.0, draw.shockwave_alpha],
                    &mut Vec::new(),
                );
                shockwave_active[slot] = true;
            }
        }
        let mut repulser_active = [false; POOL_SIZE];
        for (slot, draw) in race.repulser_field_draws().iter().enumerate() {
            let (Some((matrix, alpha, age)), Some(drawable)) =
                (draw, self.bomb_blast.repulser_field.get(slot))
            else {
                continue;
            };
            drawable.write(queue, view_projection, *matrix, view_projection * *matrix);
            drawable.write_anims(queue, *age);
            // `Image_SetVertexColours(model, alpha << 24 | 0xffffff)`, the
            // shockwave's own mechanism above.
            drawable.tint(queue, [1.0, 1.0, 1.0, *alpha], &mut Vec::new());
            repulser_active[slot] = true;
        }
        BlastsActive {
            hemisphere: hemisphere_active,
            shockwave: shockwave_active,
            repulser_field: repulser_active,
            mag_floor: self.write_mag_floor_fx(race, queue, view_projection, seconds),
        }
    }

    /// Draws every live Bomb blast's two models - the two arrays are
    /// [`Self::write_bomb_blasts`]'s own return.
    pub(super) fn draw_bomb_blasts(
        &self,
        active: &BlastsActive,
        pass: &mut wgpu::RenderPass<'_>,
        stats: &mut SceneStats,
    ) {
        for (slot, _) in active.mag_floor.iter().enumerate().filter(|(_, l)| **l) {
            for pool in &self.bomb_blast.mag_floor {
                if let Some(drawable) = pool.get(slot) {
                    stats.add(drawable.draw(pass, None, None, None, None));
                }
            }
        }
        for (slot, _) in active
            .repulser_field
            .iter()
            .enumerate()
            .filter(|(_, live)| **live)
        {
            if let Some(drawable) = self.bomb_blast.repulser_field.get(slot) {
                stats.add(drawable.draw(pass, None, None, None, None));
            }
        }
        for (slot, _) in active
            .hemisphere
            .iter()
            .enumerate()
            .filter(|(_, live)| **live)
        {
            if let Some(drawable) = self.bomb_blast.hemisphere.get(slot) {
                stats.add(drawable.draw(pass, None, None, None, None));
            }
        }
        for (slot, _) in active
            .shockwave
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

/// [`super::Scene::write_bomb_blasts`]'s return: which slots of each eased
/// view-side model are live and visible this frame.
pub(super) struct BlastsActive {
    hemisphere: [bool; bomb_blast::BOMB_BLAST_SLOTS],
    shockwave: [bool; bomb_blast::BOMB_BLAST_SLOTS],
    /// Per Repulser pool slot - see `repulser_field`.
    repulser_field: [bool; POOL_SIZE],
    /// Per ship slot - see `mag_floor_fx`.
    mag_floor: [bool; oag_weapons::MAX_SHIPS],
}

impl super::Scene {
    /// Writes each shown magstrip effect's two models and hands back which
    /// ship slots drew - see `mag_floor_fx` for the two matrices. No tint:
    /// the original stamps both with opaque white.
    ///
    /// Both play their authored animation on `seconds`: MagEffect2's `Anim
    /// Transform` (the halo's spin) through [`Drawable::write_node_anims`] and
    /// both meshes' texture-offset tracks (the lightning scrolls `v` about
    /// 1.2 per second) through [`Drawable::write_anims`]. Until 2026-10-05
    /// only the first was written, so the streaks held one texture phase.
    fn write_mag_floor_fx(
        &self,
        race: &Race,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        seconds: f32,
    ) -> [bool; oag_weapons::MAX_SHIPS] {
        let mut active = [false; oag_weapons::MAX_SHIPS];
        for (slot, draw) in race.mag_floor_fx_draws().iter().enumerate() {
            let Some(matrices) = draw else { continue };
            for (pool, matrix) in self.bomb_blast.mag_floor.iter().zip(matrices) {
                if let Some(drawable) = pool.get(slot) {
                    drawable.write(queue, view_projection, *matrix, view_projection * *matrix);
                    drawable.write_node_anims(queue, seconds);
                    drawable.write_anims(queue, seconds);
                    active[slot] = true;
                }
            }
        }
        active
    }
}

/// One kind's own write loop - the shared body of
/// [`super::Scene::write_weapon_models`]'s five calls.
///
/// `anim_seconds` is the one animation clock (`g_ingame->0x40`, see
/// `anim-transform.md`) for a model whose own `Anim Transform` nodes play, or
/// `None` to leave the identity table `mesh_render::build` initialised. The
/// Mine and the Bomb pass it: the Bomb's `orbit` ring tumbles about its own `X`
/// and its `bomb` body about `Y`; neither was ever written before, so both drew
/// at their time-zero pose. Their texture-offset tracks ride the same clock
/// (`Drawable::write_anims`, 2026-10-05).
fn write_one_kind(
    drawables: &[Drawable],
    matrices: &[Mat4],
    previous: &[Mat4],
    queue: &wgpu::Queue,
    view_projection: Mat4,
    prev_vp: Mat4,
    anim_seconds: Option<f32>,
) {
    for (index, (drawable, matrix)) in drawables.iter().zip(matrices).enumerate() {
        let previous = previous.get(index).copied().unwrap_or(*matrix);
        drawable.write(queue, view_projection, *matrix, prev_vp * previous);
        if let Some(seconds) = anim_seconds {
            drawable.write_node_anims(queue, seconds);
            drawable.write_anims(queue, seconds);
        }
    }
}
