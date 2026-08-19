//! Drawing one frame of a race: [`Scene::render`] and the two attachments it
//! draws into.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/scene.rs`.

use super::super::*;
use super::Scene;

impl Scene {
    /// Draws one frame into `view`.
    ///
    /// One render pass with one clear: a second pass would either wipe the first's
    /// colour or need its own decision about the depth buffer.
    ///
    /// `cull` is `[graphics] frustum_culling` - off by default, see that
    /// setting's own doc comment for the measurement behind that default.
    ///
    /// Returns what the track's frustum culling did, for the performance
    /// overlay - see [`SceneStats`].
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        viewport: (f32, f32, f32, f32),
        fov: crate::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
    ) -> SceneStats {
        let aspect = viewport.2.max(1.0) / viewport.3.max(1.0);
        let view_projection = race.projection(aspect, self.far, fov) * race.view();
        let frustum = cull.then(|| Frustum::from_view_projection(view_projection));
        // Tier one, built once a frame. Both sections come from the authored
        // spline rather than from the section boxes: a control point's
        // `section_id` is what the artists wrote, while a point-in-box test is
        // something this project invented.
        let visible_set = self
            .visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .map(|visibility| {
                let (craft, camera) = race.visibility_sections();
                visibility.set(craft, camera)
            });
        // The animation clock, in seconds, from the tick and never the wall
        // clock - so a replay of the same tick draws the same frame. The
        // original passes the race clock to every world mesh's updater
        // (`docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The
        // values gap is closed"), and each authored track wraps on its own
        // period, so there is no global phase to keep in step.
        //
        // `anim_seconds` overrides it, and is `None` in a race. It exists for a
        // comparison harness that needs to pin the animation phase
        // independently of the tick - `--anim-seconds`, the same flag and the
        // same meaning `oag-view` already has.
        //
        // **This replaced a `[graphics] animated_textures` boolean**, which
        // dated from when the animation was a guess at which surfaces scroll
        // and how fast. What draws now is the disc's own keyframe data, for
        // both mechanisms, so a boolean could only switch the reproduction
        // *off* - and since the `Anim Transform` port its name had quietly come
        // to cover trackside objects *moving* as well as surfaces scrolling. A
        // still-frame comparison wants a chosen time, not a freeze; a stale
        // `false` in a settings file wants nothing at all.
        let seconds = anim_seconds.unwrap_or(race.world.tick as f32 / 60.0);
        // Fog, sampled where the eye is. `oag_formats::fog::sample` reimplements
        // `FogCube_Sample`: the camera is transformed into the volume's space,
        // rejected if outside, and all six parameters interpolated across the
        // box's local Z. Outside every volume - or on a track with none - this
        // is `None` and the drawables keep `Fog::off`.
        //
        // The sky is deliberately left unfogged. It rides on the camera at a
        // radius of 18 to 62 units while fog starts at 30 to 250, so fogging it
        // would drown it in fog colour; the original's sky geometry is authored
        // `_nolight` and stands in for infinity, which is behind the fog rather
        // than inside it.
        let eye = race.camera_position();
        // Outside every volume, Wipeout HD's authored distance fog binds
        // instead - static for the race, since its curve reads view depth
        // rather than anything sampled at the camera. `Fog::off` remains for
        // every title that authors neither.
        let mut eye_fog = self.authored_fog.unwrap_or_else(mesh_render::Fog::off);
        // The specular term reads the eye out of the fog block - it is the one
        // slot in bind group 2 that carries a position - so it is kept current
        // even when the fog itself is static or off.
        eye_fog.camera = eye.to_array();
        let fog = oag_formats::fog::sample(&self.fog_volumes, eye.to_array())
            .map_or(eye_fog, |p| mesh_render::Fog::new(&p, eye.to_array()));
        // The circuit's own light rig where it authors one - Wipeout HD does,
        // in `track.envsettings` - and `mesh.wgsl`'s stand-in where it does
        // not, which is every title whose rig has not been recovered.
        let scene = mesh_render::Scene {
            fog,
            light: self.light,
        };
        for drawable in [
            Some(&self.track),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        .chain(self.ships.iter())
        {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(&scene));
        }
        // The scenery: both animation mechanisms off the one clock.
        for drawable in [
            Some(&self.track),
            self.sky.as_ref(),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.write_anims(queue, seconds);
            // The scenery that *moves* rides the same clock as the scenery
            // that scrolls, so `--anim-seconds` aims both at once and a race
            // runs both off the tick.
            drawable.write_node_anims(queue, seconds);
        }
        // The craft always animate. Their blink lights are the one animation
        // on the disc confirmed against a frame-accurate capture of the
        // original, so there is nothing about them for that switch to test.
        for drawable in &self.ships {
            drawable.write_anims(queue, seconds);
        }
        // `self.boost` is deliberately left out of both lists, at `Fog::off`
        // from `mesh_render::build`. Whether the original fogs the plume is
        // unrecovered - it is scene geometry like the ship, but additive like
        // the flare, and the flare's own post-projection draw is not answered
        // by fog either way. Left unfogged rather than guessed.

        // The sky rides with the eye. Translating it to the camera is what makes
        // an authored cube tens of units across stand in for a horizon: the
        // camera sits permanently at its centre, so the faces never approach and
        // never need to enclose the track. `race.view()` is the full view
        // matrix, roll included, so the horizon rolls with the ship through a
        // barrel roll exactly as `camera::chase` describes the original's
        // external view doing.
        if let Some(sky) = &self.sky {
            sky.write(
                queue,
                view_projection,
                Mat4::from_translation(race.camera_position()),
            );
        }
        self.track.write(queue, view_projection, Mat4::IDENTITY);
        // How many slots this race fills, which bounds every per-craft loop from
        // here down: the scene always holds a full grid's worth of drawables and a
        // time trial fields one craft.
        let drawn = usize::from(race.ship_count());
        // Every craft in play, the player first. `zip` rather than an index so a
        // race with fewer craft than the scene has drawables writes only the
        // ones it has - the rest keep last frame's uniforms and are not drawn.
        let matrices = race.ship_model_matrices();
        for (drawable, matrix) in self.ships.iter().zip(&matrices) {
            drawable.write(queue, view_projection, *matrix);
        }
        // The flaps move in *model* space, before the ship's own matrix, so
        // this is a vertex write and not a second uniform - see
        // `Drawable::deflect_airbrakes`. Unconditional rather than
        // change-gated: two kilobytes a frame is cheaper than the state needed
        // to know they have not moved, and a gate would have to be invalidated
        // by anything that ever rebuilds the buffer.
        let [left, right] = race.airbrake_flaps();
        if let Some(player) = self.ships.first() {
            player.deflect_airbrakes(queue, left, right);
        }
        // One matrix per rocket in the air. `zip` bounds it the way the ships'
        // loop is bounded: nothing in the air writes nothing, and the drawables
        // past the live count keep last frame's uniforms and are not drawn.
        let rocket_matrices = race.rocket_model_matrices();
        for (drawable, matrix) in self.rockets.iter().zip(&rocket_matrices) {
            drawable.write(queue, view_projection, *matrix);
        }
        // Same model matrix as the ship: the original parents the plume to the
        // craft, not to the flare - see `Loaded::boost_model`. Skipped while
        // hidden rather than written and left undrawn, since there is nothing
        // for the stale buffer contents to affect either way.
        //
        // The UVs are the authored ones, scrolled: the plume draws under
        // `TEXMAPMODE` 0 (settled live - mesh-draw.md, "The plume is replayed
        // under `TEXMAPMODE` 0"), sampling its authored coordinates through
        // the keyframed `TEXSCALE`/`TEXOFFSET` transform the file itself
        // carries. This replaced `oag_render::texgen` here 2026-08-10;
        // texgen's environment mapping is real but belongs to the
        // transparent-pass bracket, which the plume is not inside.
        //
        // The clock is the plume's own life timer, not the race clock: the
        // original's updater receives `flare+0x88` verbatim (measured at its
        // entry, t == the timer on every hit), and that field resets to 0 at
        // each reveal. So every boost plays the 90-frame track exactly once -
        // bright first key at reveal, darkening as it fades, clamped at the
        // last key just as the plume hides (`PLUME_SECONDS` and the track
        // span are both 1.5 s, by authoring, not coincidence). A free-running
        // clock here is visibly wrong in both directions: a boost can start
        // mid-ramp already faded, and one that outlives the wrap re-brightens
        // as a second pulse. `Exhaust::plume_timer` carries exactly the
        // original's reset-at-reveal semantics.
        //
        // **Per craft, and each on its own timer.** Eight craft can be mid-boost
        // at once and their reveals are independent, so the sample time is that
        // craft's `plume_timer` rather than one shared clock. Indexed by slot -
        // not `zip`ped over `ship_model_matrices`, which filters inactive craft
        // and so does not keep slot alignment.
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            // `None` is a slot whose *team* ships no plume, which is a
            // reported absence rather than a hidden one - see `livery::plume`.
            let Some(boost) = boost else {
                continue;
            };
            if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            boost.write(queue, view_projection, race.ship_model_matrix_of(slot));
            let (scale, offset) = match self.boost_uv_transforms.get(slot).and_then(Option::as_ref)
            {
                Some(transform) => {
                    let t = race.exhaust_of(slot).plume_timer() * 60.0;
                    (transform.scale.sample(t), transform.offset.sample(t))
                }
                None => ((1.0, 1.0), (0.0, 0.0)),
            };
            boost.apply_uv_transform(queue, scale, offset);
        }
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY);
        }
        for pads in [self.pads.as_ref(), self.weapon_pads.as_ref()]
            .into_iter()
            .flatten()
        {
            pads.write(queue, view_projection, Mat4::IDENTITY);
        }
        // Ready-to-collect vs cooling down - see `Drawable::tint_weapon_pads`
        // and `oag_render::weapon_pad` for the recovered mechanism this
        // reproduces. This is gameplay state rather than scenery, which used
        // to matter because a setting could freeze the scenery clock without
        // freezing this one; both now run off `seconds` and there is nothing
        // left to keep apart.
        if let Some(weapon_pads) = &self.weapon_pads {
            let ready: Vec<bool> = race
                .weapon_pad_refresh_left()
                .iter()
                .map(|&left| left <= 0.0)
                .collect();
            weapon_pads.tint_weapon_pads(queue, seconds, &ready);
        }

        // The camera's own axes, read out of the view matrix: for a view matrix
        // `V`, world-space right and up are rows 0 and 1 of its rotation part.
        // Building the quad from these is what makes it face the viewer, and it is
        // the whole reason this is world-space rather than the original's
        // post-projection sprite.
        let camera = race.view();
        let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
        let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
        // One flare and one ribbon per craft, the player's first, all in the two
        // buffers `exhaust::Pipeline` owns - a flare is six vertices and a ribbon
        // is a fixed 648, so eight of each is one upload and one draw call apiece
        // rather than eight. The budgets are sized for exactly this: see
        // `exhaust::MAX_SPRITES`' table and `exhaust::MAX_TRAILS`.
        //
        // **The player's own flare is not skipped in the cockpit view**, and that
        // is deliberate rather than overlooked: the nozzle sits behind the eye, so
        // it falls outside the frustum on its own. See `Race::draws_own_ship`,
        // which skips the *hull* because a hull drawn around the camera really
        // does put polygons across the middle of the screen.
        let mut vertices = Vec::new();
        let mut trail = Vec::new();
        for slot in 0..drawn {
            // No locator, nothing drawn - rather than a flare at the origin.
            // `break` rather than `continue` because the whole field shares one
            // model, so a missing `Engine Flare` node is missing for every craft.
            let Some(nozzle) = race.nozzle_of(slot) else {
                break;
            };
            let exhaust = race.exhaust_of(slot);
            // The flare quad only where the source authors no effect for it
            // - see `Race::engine_flare_effect`. The ribbon always.
            if !race.engine_flare_effect() {
                vertices.extend(exhaust.vertices(nozzle, right, up));
            }
            trail.extend(exhaust.trail_vertices(right, up));
        }
        // Only the billboard *fallback* for a rocket whose model did not
        // load - the flare around one that did is an asset now, and goes
        // through the particle pipeline below with everything else.
        vertices.extend(race.projectile_sprites(right, up, !self.rockets.is_empty()));
        self.exhaust.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &vertices,
            &trail,
        );
        // The hull's collision sparks and the stage's rocket effects share
        // one pipeline and one pair of buffers: both are `.pob` particles in
        // the same two blend classes, so a second pipeline would buy nothing
        // but a second pass.
        let (mut additive, mut alpha) = race.spark_vertices(right, up);
        let (stage_additive, stage_alpha) = race.stage_vertices(right, up);
        additive.extend(stage_additive);
        alpha.extend(stage_alpha);
        self.sparks.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &additive,
            &alpha,
        );

        let depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        // Under the HD chain the whole scene draws into its linear float
        // target instead of the caller's view; the chain's own encode pass is
        // what reaches `view`, after the read bloom. See `Self::hd`.
        let target = self.hd.as_ref().map_or(view, |hd| hd.scene_view());
        // MSAA draws into its own multisampled attachment and resolves into
        // the target at the end of this one pass; everything else draws
        // straight into it, exactly as before this setting existed. See
        // `Self::msaa_color`.
        let msaa_view = self
            .msaa_color
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let (attachment_view, resolve_target) = match &msaa_view {
            Some(msaa_view) => (msaa_view, Some(target)),
            None => (target, None),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment_view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    // Black rather than the near-black blue this used to clear
                    // to. The clear covers the whole attachment and the scene is
                    // then drawn into a sub-rectangle, so this colour is what
                    // `Aspect`'s bars are made of - and a bar has to read as a
                    // bar. The old value was 0.03/0.04/0.06, dark enough that
                    // losing it costs nothing inside the viewport either.
                    // Black, and **alpha zero**: alpha is the bloom's glow mask
                    // and a frame starts with nothing glowing.
                    // `wgpu::Color::BLACK` has `a: 1.0`, which would mask the
                    // entire frame in and bloom everything.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
        // First, and that ordering is as load-bearing as the exhaust's being
        // last. The sky writes no depth and compares `Always`, so it paints the
        // whole viewport and every later draw covers it wherever the track has
        // geometry; drawn at any other point it would overwrite what is already
        // there. Neither culling tier is offered it: a skybox is never outside
        // the frustum and belongs to no visibility section, which is the
        // behaviour ADR-0011 already assumes for it.
        //
        // Its draw calls are deliberately **not** added to `stats`. Being exempt
        // from both tiers, folding them in would shift the denominator that
        // ADR-0011's and the roadmap's PVS effectiveness figures are quoted
        // against - a silently moved percentage nobody would think to question.
        if let Some(sky) = &self.sky {
            let _ = sky.draw(&mut pass, None, None, None);
        }
        let mut stats = self.track.draw(
            &mut pass,
            self.visibility.as_ref().map(|v| &v.sections),
            visible_set.as_ref(),
            frustum.as_ref(),
        );
        // After the track, so a pad sitting flush on the surface wins the depth
        // test rather than z-fighting whatever it was authored on top of.
        // Frustum culling applies; the PVS does not, because a pad carries no
        // `section` id to look up - the same exemption the sky takes, for a
        // different reason.
        if let Some(pads) = &self.pads {
            stats.add(pads.draw(&mut pass, None, None, frustum.as_ref()));
        }
        if let Some(pads) = &self.weapon_pads {
            stats.add(pads.draw(&mut pass, None, None, frustum.as_ref()));
        }
        // Skipped outright in the cockpit view rather than moved or scaled away:
        // the original sets one flag on the craft and draws no hull, and a draw
        // call not issued is the only version of that with no chance of a stray
        // polygon across the middle of the screen. See `Race::draws_own_ship`.
        // Only the *player's* hull is skipped in the cockpit view. The opponents
        // in front are exactly what a cockpit view is for.
        for (index, drawable) in self.ships.iter().take(drawn).enumerate() {
            if index == 0 && !race.draws_own_ship() {
                continue;
            }
            stats.add(drawable.draw(&mut pass, None, None, None));
        }
        // Rockets, with the hulls: an opaque painted model that occludes and is
        // occluded, not an effect. Bounded by how many matrices were written
        // this frame, for the same reason the plumes are - a drawable whose
        // uniform buffer went unwritten would draw at last frame's pose.
        for drawable in self.rockets.iter().take(rocket_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None));
        }
        // After the ships, so the hulls' depth is already in the buffer: a
        // plume's own blend pipeline writes no depth, the same reasoning as
        // the flares below. One draw per boosting craft, gated on the same
        // craft's plume the write above was gated on - a plume drawn from a
        // uniform buffer that was not written this frame would be last frame's
        // pose.
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            let Some(boost) = boost else {
                continue;
            };
            if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            stats.add(boost.draw(&mut pass, None, None, None));
        }
        if let Some(collision) = &self.collision {
            stats.add(collision.draw(&mut pass, None, None, None));
        }
        // Last, and that ordering is load-bearing: the flare tests depth but does
        // not write it, so the hull's depth has to already be in the buffer for the
        // flare to be occluded by it. Sparks are the same kind of blended,
        // depth-tested-not-written geometry, so they follow right after for the
        // same reason.
        self.exhaust.borrow().draw(&mut pass);
        self.sparks.borrow().draw(&mut pass);
        // The scene pass has to close before the bloom can sample what it drew,
        // so this ends the borrow rather than waiting for the scope to.
        drop(pass);

        // Wipeout HD's read post chain: gate, downsample, the two blurs, the
        // composite back over the linear scene, and the encode into the
        // caller's view. It replaces the PSP bloom outright - its gate
        // consumes the same glow-mask alpha as one of its two terms. See
        // `oag_render::post::hd_bloom`.
        if let Some(hd) = &self.hd {
            hd.run(encoder, view);
            return stats;
        }
        // The recovered post-process, reading the alpha channel the ribbon and
        // the flare stamped and adding a blurred copy of the masked colour back
        // over the frame. `view` is the resolved image in both the MSAA and the
        // single-sample case, which is why this runs on it rather than on
        // `attachment_view`. See `oag_render::post::bloom`.
        if let Some(bloom) = &self.bloom {
            bloom.render(device, encoder, view);
        }
        stats
    }
}

pub(super) fn depth_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// The multisampled colour attachment MSAA draws into, resolved into the
/// caller's own target at the end of [`Scene::render`]'s one pass.
///
/// `None` at `sample_count` 1: a single-sample scene draws straight into the
/// caller's view and there is nothing here to resolve.
pub(super) fn msaa_color_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    sample_count: u32,
) -> Option<wgpu::Texture> {
    (sample_count > 1).then(|| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("race msaa colour"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    })
}
