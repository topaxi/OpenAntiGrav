//! Drawing one frame of a race: [`Scene::render`] and the two attachments it
//! draws into.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/scene.rs`.

use super::super::*;
use super::Scene;
use super::motion::MotionState;

mod craft;
use craft::{cockpit_shield_visible, hd_flame_transform, shell_visible};

impl Scene {
    /// Draws one frame into `view`. One render pass with one clear: a
    /// second pass would either wipe the first's colour or need its own
    /// decision about the depth buffer.
    ///
    /// `cull` is `[graphics] frustum_culling` - **on** by default, see that
    /// setting's own doc comment for the measurement behind that default and
    /// for the cost the test itself carries.
    ///
    /// Returns what the track's frustum culling did, for the performance
    /// overlay - see [`SceneStats`]. `camera_jitter` is the jitter
    /// sequence's length, or `None` for no jitter at all. **Not a bool,
    /// because the length is a property of what is resolving the frames** -
    /// a temporal upscaler magnifying by two wants four times the phases it
    /// wants at native. `--camera-jitter` on its own, with nothing
    /// reconstructing from it, is a worse picture and passes
    /// [`oag_post::jitter::DEFAULT_PHASES`]. See [`Scene::jittered`].
    /// `timestamps`, `blur_timestamps` and `hd_bloom_timestamps` bracket this
    /// pass, the motion-blur chain and the HD/Fury bloom chain, `Some` only on
    /// the window's own frame loop - see [`oag_gpu::timing::PassTimer`].
    /// **These three and no others**, because they are the passes whose cost
    /// falls with the render extent: the upscaler and the composite draw at
    /// presentation size whatever the scale is, so folding them in would put a
    /// fixed cost into a budget that exists to be divided by a moving one. See
    /// [dynamic-resolution.md](../../../../../docs/rendering/dynamic-resolution.md).
    /// `zone_spectrum` is a live audio spectrum, each band `0.0..=1.0` -
    /// `oag_audio::Output::spectrum`'s own snapshot, read by the caller once
    /// a frame. Empty outside a Zone race or with nothing to draw it into is
    /// fine: [`oag_mesh::mesh_render::zone::write_vis`] is a no-op on an
    /// empty slice. See [`crate::zone_grade::ZoneGrade`] for the stage tint this is coloured by.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        viewport: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        motion_blur: oag_display::display::MotionBlur,
        shadows: oag_display::display::Shadows,
        camera_jitter: Option<u32>,
        zone_spectrum: &[f32],
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
        blur_timestamps: Option<oag_post::motion_blur::ChainTimestamps<'_>>,
        hd_bloom_timestamps: Option<oag_post::hd_bloom::ChainTimestamps<'_>>,
    ) -> SceneStats {
        let aspect = viewport.2.max(1.0) / viewport.3.max(1.0);
        let projection = race.projection(aspect, self.far, fov);
        let view_projection = projection * race.view();
        // The previous tick's camera and model matrices, promoted from the
        // last frame that rendered a different tick - what every drawable's
        // `prev_mvp` velocity is measured against. See [`MotionState`].
        //
        // Shaken views, so the buffer stays true screen motion for the temporal
        // upscaler; the blur subtracts the shake itself, `blur_shake` below.
        let prev = MotionState::advance(
            &self.motion,
            race,
            &self.weapon_quads.draw,
            view_projection,
            usize::from(race.ship_count()),
        );
        let prev_vp = prev.view_projection;
        // The shake's own screen motion, for the blur - see `Race::shake_screen_motion`.
        let blur_shake = race.shake_screen_motion(projection, prev.shake);
        let frustum = cull.then(|| Frustum::from_view_projection(view_projection));
        // The same unjittered matrix, for the section boxes' view test.
        let section_view = view_projection;
        // Recorded before the offset is applied - see `Scene::record_frame`.
        self.record_frame(
            camera_jitter,
            race.projection(aspect, self.far, fov),
            race.camera_view(),
            race.camera_cuts(),
        );
        // Above this line the camera is unjittered, below it is not, and the
        // frustum and the snapshot are above deliberately - see the call.
        let (view_projection, prev_vp) =
            self.jittered(camera_jitter, viewport, view_projection, prev_vp);
        // Tier one, built once a frame. Both sections come from the authored
        // spline rather than from the section boxes: a control point's
        // `section_id` is what the artists wrote, while a point-in-box test is
        // something this project invented.
        let visible_set = self
            .visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .filter(|visibility| visibility.has_sections())
            .map(|visibility| race.visible_set(visibility, &section_view));
        // Tier one on a PS3 circuit, which partitions by chunk rather than by
        // section - see `oag_render::pvs::ChunkSet`. Located from world
        // positions rather than from spline sample ids because `track.pvs`
        // carries its own cell positions and is not indexed by the `.vex`
        // spline at all.
        let chunk_set = self
            .visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .and_then(|visibility| visibility.chunks())
            .and_then(|pvs| {
                ChunkSet::around(
                    pvs,
                    race.ship().physics.body.position,
                    race.camera_position(),
                )
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
        let seconds = anim_seconds.unwrap_or(race.sim.world.tick as f32 / 60.0);
        self.anim_clock.set(seconds);
        // Fog, sampled where the eye is. `oag_vex::fog::sample` reimplements
        // `FogCube_Sample`: the camera is transformed into the volume's space,
        // rejected if outside, and all six parameters interpolated across the
        // box's local Z. Outside every volume - or on a track with none - this
        // is `None` and the drawables keep `Fog::off`.
        //
        // The sky is deliberately left unfogged. It rides on the camera at a
        // radius of 18 to 62 units while fog starts at 30 to 250, so fogging it
        // would drown it in fog colour; the original's sky geometry is authored
        // `_nolight` and stands in for infinity, which is behind the fog rather than inside it.
        let eye = race.camera_position();
        // Outside every volume, Wipeout HD's authored distance fog binds
        // instead - static for the race, since its curve reads view depth
        // rather than anything sampled at the camera. `Fog::off` remains for
        // every title that authors neither.
        //
        // **The Zone stage grade lays over both of these, where a race has
        // one**: `.effectSettings` authors a full palette per speed class, and
        // the stage showing recolours the circuit's own fog and rig rather
        // than replacing them - see `crate::zone_grade`. Applied per
        // frame rather than folded in at load because the stage is a *runtime*
        // selection in the original, cross-faded against the stage before it
        // over the hundred frames after a step (`ZoneGrade::follow`).
        let authored_fog = self
            .zone_grade
            .as_ref()
            .map_or(self.authored_fog, |grade| grade.fog(self.authored_fog));
        let light = self
            .zone_grade
            .as_ref()
            .map_or(self.light, |grade| grade.light(self.light));
        let mut eye_fog = authored_fog.unwrap_or_else(mesh_render::Fog::off);
        // The specular term reads the eye out of the fog block - it is the one
        // slot in bind group 2 that carries a position - so it is kept current
        // even when the fog itself is static or off.
        eye_fog.camera = eye.to_array();
        let fog = oag_vex::fog::sample(&self.fog_volumes, eye.to_array())
            .map_or(eye_fog, |p| mesh_render::Fog::new(&p, eye.to_array()));
        // The circuit's own light rig where it authors one - Wipeout HD does,
        // in `track.envsettings` - and `mesh.wgsl`'s stand-in where it does
        // not, which is every title whose rig has not been recovered.
        // **Before the scene uniform is composed, not just before the scene
        // pass.** `shadow_uniform` reports a strength only once the map has
        // casters in it, so a pass encoded after this block would leave the
        // first frame - which is the only frame a `--screenshot` capture
        // draws - reading an empty map at zero strength.
        self.render_shadow_map(queue, encoder, race, shadows);
        self.render_sun_occlusion(queue, encoder, race, shadows);
        let scene = mesh_render::Scene {
            fog: fog.with_texture_detail(race.texture_detail()),
            // Pulse's hull lights reach the craft alone - see `ship_scene`.
            light: light.without_hull(),
            // The flame surface's scroll clock - `time`, engine parameter slot
            // 0 on the original. The same `seconds` the texture and node
            // animation ride, so `--anim-seconds` pins all three at once and a
            // race runs all three off the tick.
            time: [seconds; 4],
            // The Zone stage's own shader parameters, or all-zero (and so the
            // identity on every albedo) outside a Zone race. Per frame for the
            // same reason the fog and rig above are: the stage is a runtime
            // selection in the original - and the transition sphere inside
            // it moves with the craft every frame.
            zone: self
                .zone_grade
                .as_ref()
                .map_or_else(Default::default, |grade| grade.zone_uniform()),
            // The shadow map's own projection and strength, or `off` where
            // nothing casts - see `Scene::shadow_uniform`.
            shadow: self.shadow_uniform(shadows),
            spu_lights: mesh_render::SpuLights::from_slice(&race.hd_spu_lights()),
            sun_occlusion: self.sun_occlusion_matrices(),
            refraction: super::behind_glass::view_projection(section_view).to_cols_array_2d(),
        };
        // The visualiser's own tint - the showing stage's `EQ colour tint`,
        // or `None` where the file authors none (every 2048 table, and any
        // HD stage this project has not measured past `Start`/`Sub Venom`).
        // Not substituted with white: a missing input draws nothing, the
        // same rule `zone_uniform` above already keeps for its own inputs.
        let zone_tint = self.zone_grade.as_ref().and_then(|grade| grade.eq_tint());
        // Cleared, not rebuilt: see `Scene::scratch`.
        let mut scratch = self.scratch.borrow_mut();
        scratch.clear();
        let super::Scratch {
            vertices,
            trail,
            additive,
            alpha,
            cannon_bolt,
            cannon_flash,
            pads_ready,
            recoloured,
        } = &mut *scratch;

        for drawable in [
            Some(&self.track),
            self.collision.as_ref(),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(&scene));
            // Written for every drawable on this list regardless of whether a
            // Zone race is running: it blanks the lookup with no spectrum or
            // no tint, and `scene.zone.enabled` already gates the glow off
            // outside a Zone race, so this is cheap and never wrong to call.
            drawable.write_zone_vis(queue, zone_spectrum, zone_tint);
        }
        // **The craft get the same scene with the Zone half switched off**,
        // because the disc says they are not in it. The twelve
        // `data/materials/ships/*.rcsmaterial` in `DATA03.PSARC` carry no Zone
        // variant at all, and neither do the 39 under
        // `data/weapons/materials/` - so the original leaves hulls and rockets
        // ordinarily shaded while the circuit around them goes monochrome.
        //
        // This matters now in a way it did not before: the Zone term used to
        // *add* to the albedo and was a no-op on any surface with no pure-black
        // texels, so binding it to a ship cost nothing. It now **replaces** the
        // albedo, so binding it to a ship would blank the ship. Fog and the
        // light rig still reach them unchanged - only `zone` is dropped.
        oag_gpu::perfprobe::mark("fog+zonevis");
        // The SPU lights stay - at ride height the hull is the only receiver
        // in range, and the original selects `SVC1` for the hull (read live);
        // see `race::engine_light`, "Who receives it".
        let ship_scene = mesh_render::Scene {
            zone: mesh_render::Zone::default(),
            light,
            ..scene
        };
        self.write_ship_scenes(queue, &ship_scene);
        let weapon_scene = Self::weapon_scene(&scene);
        self.write_scenery_anims(queue, seconds);
        // The gantry's own clock: started so the title's `GO` edge lands on the
        // release and held on `GO` (`race::gantry::Clock`); it clamps itself.
        if let Some(gantry) = &self.gantry {
            queue.write_buffer(gantry.fog(), 0, bytemuck::bytes_of(&scene));
            let clock = anim_seconds.unwrap_or_else(|| gantry.clock_seconds(race));
            gantry.write(queue, view_projection, prev_vp, clock);
        }
        crate::adverts::render(&self.adverts, queue, encoder, seconds, race, anim_seconds);
        let (set, chunks) = (visible_set.as_ref(), chunk_set.as_ref());
        let glass = (section_view, eye, seconds);
        let behind_glass =
            self.render_behind_glass(queue, encoder, &scene, glass, set, chunks, frustum.as_ref());
        oag_gpu::perfprobe::mark("scenery-anims");
        // The craft always animate. Their blink lights are the one animation
        // on the disc confirmed against a frame-accurate capture of the
        // original, so there is nothing about them for that switch to test.
        for drawable in &self.ships {
            drawable.write_anims(queue, seconds);
            // A hull whose hinges are `Anim Transform` nodes (HD's, 2048's
            // `Airbrake_L`/`Airbrake_R`) bakes the flap's vertices in the
            // hinge's own space and carries its slot. The table starts as
            // all-identity, so without this the flap draws at the model's
            // origin - the middle of the hull - instead of on its hinge,
            // stowed or deflected. A no-op on a hull with none (every PSP
            // and PS2 one).
            drawable.write_node_anims(queue, seconds);
        }
        // `self.boost` is deliberately left out of both lists, at `Fog::off`
        // from `mesh_render::build`. Whether the original fogs the plume is
        // unrecovered - it is scene geometry like the ship, but additive like
        // the flare, and the flare's own post-projection draw is not answered
        // by fog either way. Left unfogged rather than guessed. Its animation
        // tables are written below instead, off its own reveal timer rather
        // than the race clock these lists ride.
        // **The flame surfaces need the clock, and nothing else from `scene`.**
        // Both a craft's flare and its boost plume draw through `mesh.wgsl`'s
        // `flame_shading` path, which replaces the lit result outright and
        // applies no fog - so binding `Scene::off()` plus the clock leaves
        // their appearance exactly where `mesh_render::build` put it while
        // giving `flame_speed * scene.time.x` something to advance. Writing
        // the *scene* here instead would fog and light them, which is the
        // question the paragraph above says is unrecovered.
        oag_gpu::perfprobe::mark("ship-anims");
        let flame_scene = mesh_render::Scene {
            time: [seconds; 4],
            ..mesh_render::Scene::off()
        };
        for drawable in self.flares.iter().chain(&self.boost).flatten() {
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(&flame_scene));
        }

        // The sky rides with the eye. Translating it to the camera is what makes
        // an authored cube tens of units across stand in for a horizon: the
        // camera sits permanently at its centre, so the faces never approach and
        // never need to enclose the track. `race.view()` is the full view
        // matrix, roll included, so the horizon rolls with the ship through a
        // barrel roll exactly as `camera::chase` describes the original's
        // external view doing.
        oag_gpu::perfprobe::mark("flame-scene");
        if let Some(sky) = &self.sky {
            sky.write(
                queue,
                view_projection,
                Mat4::from_translation(race.camera_position()),
                // The previous camera's *translation* cancels against the
                // previous view-projection exactly as this frame's does
                // against this one, leaving the sky with rotation-only
                // velocity - a horizon pans, it does not approach.
                prev_vp * prev.camera_translation,
            );
        }
        self.track
            .write(queue, view_projection, Mat4::IDENTITY, prev_vp);
        // How many slots this race fills, which bounds every per-craft loop from
        // here down: the scene always holds a full grid's worth of drawables and a
        // time trial fields one craft.
        oag_gpu::perfprobe::mark("sky+track-write");
        let drawn = usize::from(race.ship_count());
        self.write_hull_uniforms(queue, race, view_projection, prev_vp, &prev);
        let lod_eye = race.lod_eye(aspect, fov);
        self.select_lod(race, lod_eye);
        // The flaps move in *model* space, before the ship's own matrix, so
        // this is a vertex write and not a second uniform - see
        // `Drawable::deflect_airbrakes`. Unconditional rather than
        // change-gated: two kilobytes a frame is cheaper than the state needed
        // to know they have not moved, and a gate would have to be invalidated
        // by anything that ever rebuilds the buffer.
        let [left, right] = race.airbrake_flaps();
        if let Some(player) = self.ships.first() {
            player.deflect_airbrakes(queue, left, right, recoloured);
        }
        // The shield shell: the craft's own matrix with a uniform swell on top,
        // and its colour written into the vertex buffer.
        //
        // **The scale is applied on the right**, after the craft's rotation and
        // translation, so it grows the shell about the hull's own origin rather
        // than sliding it along the world axes. The original composes the same
        // way - its update builds a scale matrix from the identity basis and
        // installs it under the craft's node.
        //
        // Skipped while invisible rather than written and left undrawn, the same
        // as the plume: there is nothing for stale buffer contents to affect,
        // and a shell that is not up is the common case for every craft in every
        // race.
        for (slot, shell) in self.shield.iter().enumerate().take(drawn) {
            // `None` is a slot whose source ships no shell under either name -
            // a reported absence, not a hidden one. See `livery::shield::shell`.
            let Some(shell) = shell else {
                continue;
            };
            if !race.ship_active(slot) || !shell_visible(race, slot) {
                continue;
            }
            let state = race.shield_of(slot);
            // The previous pose composed with *this* frame's swell: the
            // shell's velocity is its craft's, and folding the swell delta
            // in would smear a stationary shell for growing.
            shell.write(
                queue,
                view_projection,
                race.ship_model_matrix_of(slot) * Mat4::from_scale(Vec3::splat(state.scale())),
                prev_vp * prev.ship(slot, race) * Mat4::from_scale(Vec3::splat(state.scale())),
            );
            shell.tint(queue, state.shell_colour(), recoloured);
            // The authored `u` scroll, on the global clock (shield-pickup.md).
            shell.write_anims(queue, seconds);
        }
        // The cockpit sphere replaces the player's shell (`ShipShield_Update` draws one
        // *or* the other), at `cockpit_scale`, the recovered `1.8` times the shell's own,
        // so it encloses a camera sitting inside the hull.
        if let Some(sphere) = &self.shield_cockpit
            && cockpit_shield_visible(race)
        {
            let state = race.shield_of(0);
            sphere.write(
                queue,
                view_projection,
                race.ship_model_matrix_of(0) * Mat4::from_scale(Vec3::splat(state.cockpit_scale())),
                prev_vp * prev.ship(0, race) * Mat4::from_scale(Vec3::splat(state.cockpit_scale())),
            );
            sphere.tint(queue, state.colour(), recoloured);
        }
        self.write_absorb_overlays(race, queue, view_projection, prev_vp, &prev, recoloured);
        self.write_absorb_shells(race, queue, view_projection, prev_vp, &prev, recoloured);
        self.write_ghost(race, queue, view_projection, prev_vp);
        oag_gpu::perfprobe::mark("ship+shield-write");
        let (rocket_matrices, ball_matrices, mine_matrices, bomb_matrices, cannon_matrices) = self
            .write_weapon_models(
                race,
                &prev,
                queue,
                view_projection,
                prev_vp,
                seconds,
                &weapon_scene,
            );
        let plasma_blast_active =
            self.write_plasma_blasts(race, queue, view_projection, &weapon_scene);
        let blasts_active =
            self.write_bomb_blasts(race, queue, view_projection, seconds, &weapon_scene);
        let leach_ball_active = self.write_leach_ball(race, queue, view_projection, &weapon_scene);
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
        // The always-on flame, on the craft's own matrix and nothing else: its
        // vertices were already moved to the `Engine Flare` locator at load, so
        // it rides the hull exactly as the plume does. See
        // `oag_livery::flare::per_team` for why the placement is baked.
        for (slot, flare) in self.flares.iter().enumerate().take(drawn) {
            let Some(flare) = flare else {
                continue;
            };
            if !race.ship_active(slot) {
                continue;
            }
            // HD breathes the flame with the craft: `EngineFlare_PlaceShapes`
            // scales `EF_Main` by the smoothed throttle plus the boost blend,
            // about the nozzle the model is baked at. Identity when the HD
            // exhaust is not active - the flame then draws as authored, and
            // the load report already carries what is missing.
            let flame = hd_flame_transform(race, slot, false);
            let model = race.ship_model_matrix_of(slot) * flame;
            // This frame's flame scale on the previous pose, like the shell's
            // swell: the flame's velocity is its craft's.
            flare.write(
                queue,
                view_projection,
                model,
                prev_vp * prev.ship(slot, race) * flame,
            );
        }
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            // `None` is a slot whose *team* ships no plume, which is a
            // reported absence rather than a hidden one - see `livery::plume`.
            let Some(boost) = boost else {
                continue;
            };
            // An inactive slot has nothing current to sample - its craft is
            // out of the race, not merely hidden. See `Race::ship_active`.
            if !race.ship_active(slot) {
                continue;
            }
            // Two reveals, per source. HD's plume has **no** visibility gate:
            // `EngineFlare_PlaceShapes` scales `EF_Boost`'s length by
            // `blend * 2.0` unconditionally, so it grows out of the nozzle
            // while the boost timer holds the blend at 1.0 and collapses back
            // over ~10 ticks after - `hd::Flame` is that state machine. The
            // PSP/PS2 plume keeps the recovered `plume_visible` reveal.
            if race.hd_trail_active() {
                if !race.hd_flame_of(slot).boost_visible() {
                    continue;
                }
            } else if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            let flame = hd_flame_transform(race, slot, true);
            let model = race.ship_model_matrix_of(slot) * flame;
            boost.write(
                queue,
                view_projection,
                model,
                prev_vp * prev.ship(slot, race) * flame,
            );
            let (scale, offset) = match self.boost_uv_transforms.get(slot).and_then(Option::as_ref)
            {
                Some(transform) => {
                    let t = race.exhaust_of(slot).plume_timer() * 60.0;
                    (transform.scale.sample(t), transform.offset.sample(t))
                }
                None => ((1.0, 1.0), (0.0, 0.0)),
            };
            boost.apply_uv_transform(queue, scale, offset);
            // **The PS2 plume is two nozzle flares on moving anchors; the PSP
            // one is loose geometry with none.** Both meshes of every PS2
            // `shipboost.vex`/`Zoneboost.vex` sit under an `Anim Transform`,
            // so they are baked in that node's space (`mesh::anim_node`) and
            // this table is what puts them back at the nozzles - all twelve
            // teams, at roughly `(+-1.0, -0.2, -6.5)`. Leaving it identity, as
            // this loop used to, drew both fins on top of each other at the
            // craft's own origin, several units forward of the engines. A PSP
            // plume carries no `Anim Transform` at all, so this early-returns
            // there and the PSP picture is untouched.
            //
            // **The clock is the shared race clock, not the plume's own
            // reveal timer** - recovered, confidence 90, not the earlier
            // approximate fit. `AnimTransform_Update` (`0x001d0350` in
            // `ps2-pulse-eu`) is the per-frame driver for every `Anim
            // Transform` node on the disc, plume anchors included: it reads
            // one shared session clock (`g_ingame`, with a fallback global,
            // neither per-object), advances it, and wraps it at the node's
            // own `LoopEnd` through `fmodf`. See
            // `docs/ghidra/functions/ps2-pulse-eu/anim-transform.md`. This
            // is the same clock every other `Anim Transform` node in this
            // frame already rides - `seconds`, below - so the plume anchors
            // were the one outlier still wired to a per-craft timer. Unlike
            // the UV scroll above, there is no independent evidence the
            // node clock ever resets per reveal; the disc drives it exactly
            // like scenery.
            //
            // Seconds, not frames. `vex::AnimTransform::sample` takes seconds
            // and wraps at the track's own `loop_seconds`, where the UV track
            // above takes 60 Hz frames and clamps.
            boost.write_node_anims(queue, seconds);
        }
        oag_gpu::perfprobe::mark("rockets+plumes");
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY, prev_vp);
        }
        // The Quake's ripple, then the pads' uniforms and the weapon pads'
        // tint, in that order - see `Scene::write_road`.
        self.write_road(
            queue,
            race,
            seconds,
            (view_projection, prev_vp),
            pads_ready,
            recoloured,
        );
        // **A speed pad is not recoloured at all, on any title.** Its class'
        // own `update` slot, `Pad_UpdateRefreshTimer` (`0x089265f0`,
        // `docs/ghidra/functions/psp-pulse-usa/pads.md`), decrements a timer
        // and writes no colour, so the original never overrides what the
        // artists painted - and what they painted is the whole picture on
        // every title measured: Pulse's `flicker1nonalpha_GLOW.tga` is the
        // gold chevron, HD's `ds_speedup_cs.gtf` the blue one. See
        // `docs/rendering/pads.md`.
        oag_gpu::perfprobe::mark("pads+tint");
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
        oag_gpu::perfprobe::mark("pre-exhaust");
        for slot in 0..drawn {
            // Per craft, unlike the nozzle check below - an inactive slot is
            // out of the race regardless of what its model authors. `continue`
            // rather than `break`: the slots after it can still be racing.
            // See `Race::ship_active`.
            if !race.ship_active(slot) {
                continue;
            }
            // No locator, nothing drawn - rather than a flare at the origin.
            // `break` rather than `continue` because the whole field shares one
            // model, so a missing `Engine Flare` node is missing for every craft.
            let Some(nozzle) = race.nozzle_of(slot) else {
                break;
            };
            let exhaust = race.exhaust_of(slot);
            // The flare quad only where the source authors no effect for it
            // - see `Race::engine_flare_effect` - and not on a wreck, which
            // shows none on a running original (`scene::wreck`). The ribbon always.
            if !race.engine_flare_effect() && !self.is_wrecked(race, slot) {
                vertices.extend(exhaust.vertices(nozzle, right, up));
            }
            // HD's sprite flare - Engine_Flare_Rich.gtf, enabled by the
            // disc's own tuning file - is the bright core the exhaust's
            // "solidity" comes from; the flare texture slot holds it on HD.
            vertices.extend(race.hd_sprite_quad(slot, right, up));
            // Wipeout HD draws its own measured tube - three fins over a
            // 54-sample ring, extruded in the craft's frame rather than the
            // camera's - and every other source the PSP preset's ribbon.
            if race.hd_trail_active() {
                race.extend_hd_trail_vertices(trail, slot);
            } else {
                exhaust.extend_trail_vertices(trail, right, up);
            }
        }
        // Only the billboard *fallback*, per kind: a body whose own model did
        // not load, or a Missile or Plasma, which has none at all.
        vertices.extend(race.projectile_sprites(right, up, |kind| match kind {
            oag_tables::weapons::Weapon::Rocket => !self.rockets.is_empty(),
            oag_tables::weapons::Weapon::Mine => !self.mines.is_empty(),
            oag_tables::weapons::Weapon::Bomb => !self.bombs.is_empty(),
            oag_tables::weapons::Weapon::Cannon => !self.cannon_rounds.is_empty(),
            oag_tables::weapons::Weapon::Plasma => !self.plasma_blast.ball.is_empty(),
            oag_tables::weapons::Weapon::Shuriken => !self.plasma_blast.shuriken.is_empty(),
            _ => false,
        }));
        self.gather_cannon_quads(race, right, up, cannon_bolt, cannon_flash);
        oag_gpu::perfprobe::mark("exhaust-gather");
        // Shared by every upload below - `to_cols_array_2d` is otherwise
        // recomputed once per pipeline for the same one matrix.
        let vp = view_projection.to_cols_array_2d();
        self.exhaust
            .borrow_mut()
            .upload(queue, &vp, race.camera_position(), vertices, trail);
        self.clouds
            .borrow_mut()
            .upload(queue, &vp, race.sim.world.tick, &race.camera_frame());
        oag_gpu::perfprobe::mark("exhaust-upload");
        self.upload_particles(race, queue, &vp, right, up, additive, alpha);
        self.upload_mist(race, queue, 1.0 / projection.y_axis.y);
        self.upload_cannon_quads(queue, &vp, cannon_bolt, cannon_flash);
        self.upload_beam(race, queue, &vp);
        self.upload_ribbons(race, queue, &vp);

        // Both shadow tiers' geometry - its own file, see `frame/shadow.rs`.
        let (quads, hull_vertices) = self.shadow_geometry(race, shadows);
        self.shadow
            .borrow_mut()
            .upload(queue, &vp, &quads, &hull_vertices);
        oag_gpu::perfprobe::mark("shadow-gather");

        oag_gpu::perfprobe::mark("psys-gather");
        let depth_view = &self.attachment_views.depth;
        // Under the HD chain the whole scene draws into its linear float
        // target instead of the caller's view; the chain's own encode pass is
        // what reaches `view`, after the read bloom. See `Self::hd`.
        let target = self.linear_scene_view().unwrap_or(view);
        // MSAA draws into its own multisampled attachment and resolves into
        // the target at the end of this one pass; everything else draws
        // straight into it, exactly as before this setting existed. See
        // `Self::msaa_color`.
        let msaa_view = self.attachment_views.msaa.as_ref();
        let (attachment_view, resolve_target) = match msaa_view {
            Some(msaa_view) => (msaa_view, Some(target)),
            None => (target, None),
        };
        let velocity_view = &self.attachment_views.velocity;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
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
                }),
                // The velocity target, cleared to zero - nothing moved where
                // nothing drew - and never resolved: under MSAA the blur's
                // prepare stage reads sample 0 instead, because averaging
                // velocity across a silhouette edge produces a vector that
                // describes neither surface. See `Self::velocity`.
                Some(wgpu::RenderPassColorAttachment {
                    view: velocity_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    // `Store`, not `Discard`: the motion blur's reconstruction
                    // weighs its taps by depth ordering after this pass
                    // closes, and a discarded attachment is undefined memory
                    // by then. This is also the depth half of
                    // `docs/overview/modern-features.md`'s FSR 3.1
                    // prerequisite table.
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            // The one pass whose cost falls with the render extent, and so the
            // one a resolution controller will be driven by - see
            // `oag_gpu::timing::PassTimer` and this function's own doc.
            // `None` on every path that is not the window's frame loop: a
            // capture measures nothing, because a capture has to be
            // reproducible rather than fast.
            timestamp_writes: timestamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        oag_gpu::perfprobe::mark("pass-open");
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
        // The sky draws inside this, between the circuit's solid and blended
        // lists - see `draw_track` for why there.
        self.sort_opaque(race.camera_position());
        let mut stats = behind_glass;
        stats.add(self.draw_track(
            &mut pass,
            viewport,
            self.visibility.as_ref().map(|v| &v.sections),
            visible_set.as_ref(),
            chunk_set.as_ref(),
            frustum.as_ref(),
        ));
        // After the track, so a pad sitting flush on the surface wins the depth
        // test rather than z-fighting whatever it was authored on top of.
        // Frustum culling applies; the PVS does not, because a pad carries no
        // `section` id to look up - the same exemption the sky takes, for a
        // different reason.
        oag_gpu::perfprobe::marks::mark(&mut pass, "pads, gantry, shadow");
        if let Some(pads) = &self.pads {
            stats.add(pads.draw(&mut pass, None, None, None, frustum.as_ref()));
        }
        if let Some(pads) = &self.weapon_pads {
            stats.add(pads.draw(&mut pass, None, None, None, frustum.as_ref()));
        }
        // With the track and before the hulls: scenery a craft passes under.
        if let Some(gantry) = &self.gantry {
            stats.add(gantry.draw(&mut pass, frustum.as_ref()));
        }
        // After the track and the pads, before the hulls: the road's depth has
        // to be in the buffer for the quad to be occluded by the geometry in
        // front of it, and the hulls are opaque and write depth, so a craft
        // drawn afterwards covers its own shadow where the two overlap. Draws
        // nothing at all while `graphics.shadows` is `off` - the upload above
        // gave it no placements. **Where in the frame this sits is ours**: the
        // original's own draw-order key has no layer for a blob, because no
        // title in the lineage draws one. See `docs/rendering/draw-order.md`.
        self.shadow.borrow().draw(&mut pass);
        // Skipped outright in the cockpit view rather than moved or scaled away:
        // the original sets one flag on the craft and draws no hull, and a draw
        // call not issued is the only version of that with no chance of a stray
        // polygon across the middle of the screen. See `Race::draws_own_ship`.
        // Only the *player's* hull is skipped in the cockpit view. The opponents
        // in front are exactly what a cockpit view is for.
        oag_gpu::perfprobe::marks::mark(&mut pass, "craft, weapons, shields");
        for (index, drawable) in self.ships.iter().take(drawn).enumerate() {
            if self.hull_skipped(race, index) {
                continue;
            }
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        self.draw_wrecks(race, &mut pass, &mut stats);
        // Every weapon's own body: opaque painted models, bounded by how many
        // matrices were written this frame, same as the plumes below.
        for drawable in self.rockets.iter().take(rocket_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        for drawable in self.mines.iter().take(mine_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        for drawable in self.bombs.iter().take(bomb_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        for drawable in self.cannon_rounds.iter().take(cannon_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        for drawable in self.plasma_blast.ball.iter().take(ball_matrices.len()) {
            stats.add(drawable.draw(&mut pass, None, None, None, None));
        }
        self.draw_shurikens(race.shuriken_model_matrices().len(), &mut pass, &mut stats);
        self.draw_plasma_blasts(&plasma_blast_active, &mut pass, &mut stats);
        self.draw_bomb_blasts(&blasts_active, &mut pass, &mut stats);
        self.draw_leach_ball(leach_ball_active, &mut pass, &mut stats);
        // After the ships, so the hulls' depth is already in the buffer: a
        // plume's own blend pipeline writes no depth, the same reasoning as
        // the flares below. One draw per boosting craft, gated on the same
        // craft's plume the write above was gated on - a plume drawn from a
        // uniform buffer that was not written this frame would be last frame's
        // pose.
        // With the plumes and for the same reason: the hulls' depth is in the
        // buffer, and this model's own material blend writes none of its own.
        //
        // **Gated on the craft being in the race and nothing else.** The flame
        // is authored as a permanent part of the craft - `EF_Main` carries no
        // reveal condition and the file's every transform is the identity - so
        // a throttle or speed gate here would be this project's invention
        // rather than the asset's. What the original *does* animate on it is
        // unread: `EF_Main` carries one attribute, `AnimEnd = 1.0`, and
        // nothing here plays it. See docs/rendering/trail-ribbon.md.
        for (slot, flare) in self.flares.iter().enumerate().take(drawn) {
            let Some(flare) = flare else {
                continue;
            };
            if !race.ship_active(slot) {
                continue;
            }
            // `draw`, not `draw_additive`: every batch of this model already
            // carries its material's own factor pair, so the ordinary path
            // routes it through the authored pipeline. The override the plume
            // needs is for a model that authors none.
            stats.add(flare.draw(&mut pass, None, None, None, None));
        }
        for (slot, boost) in self.boost.iter().enumerate().take(drawn) {
            let Some(boost) = boost else {
                continue;
            };
            if !race.ship_active(slot) {
                continue;
            }
            if !race.exhaust_of(slot).plume_visible() {
                continue;
            }
            // Additive for every list, not just the transparent one - see
            // `Drawable::draw_additive`, which carries the reference frame
            // that settled it and the PS2 dispatch that has not been followed.
            stats.add(boost.draw_additive(&mut pass));
        }
        // With the plumes, and after the hulls for the same reason: the shell
        // wraps a craft whose depth has to already be in the buffer, or the far
        // side of the shell draws over the near side of the ship inside it.
        //
        // Gated on exactly what the write above was gated on, for the reason the
        // plume's loop gives - a shell drawn from a uniform buffer that was not
        // written this frame would sit at last frame's pose *and* at last
        // frame's swell.
        for (slot, shell) in self.shield.iter().enumerate().take(drawn) {
            let Some(shell) = shell else {
                continue;
            };
            if !race.ship_active(slot) || !shell_visible(race, slot) {
                continue;
            }
            stats.add(shell.draw(&mut pass, None, None, None, None));
        }
        if let Some(sphere) = &self.shield_cockpit
            && cockpit_shield_visible(race)
        {
            stats.add(sphere.draw(&mut pass, None, None, None, None));
        }
        self.draw_absorb_overlays(race, &mut pass, &mut stats);
        self.draw_absorb_shells(race, &mut pass, &mut stats);
        if let Some(collision) = &self.collision {
            stats.add(collision.draw(&mut pass, None, None, None, None));
        }
        // Last, and that ordering is load-bearing: the flare tests depth but does
        // not write it, so the hull's depth has to already be in the buffer for the
        // flare to be occluded by it. Sparks are the same kind of blended,
        // depth-tested-not-written geometry, so they follow right after for the
        // same reason.
        self.draw_effects(race, lod_eye, &mut pass, &mut stats);
        // The scene pass has to close before the bloom can sample what it drew,
        // so this ends the borrow rather than waiting for the scope to.
        drop(pass);
        oag_gpu::perfprobe::mark("scene-pass");

        // Wipeout HD's read post chain (`oag_post::hd_bloom`): gate,
        // downsample, the two blurs, the composite back over the linear
        // scene, and the encode into the caller's view. Replaces the PSP
        // bloom outright - its gate consumes the same glow-mask alpha.
        if let Some(hd) = &self.hd {
            // The zoom-streak ring's pulses (stepped once a tick); `None` draws none.
            hd.set_zoom_frame(race.hd_zoom_frame());
            hd.run(
                queue,
                encoder,
                view,
                (viewport.0, viewport.1),
                (viewport.2 as u32, viewport.3 as u32),
                hd_bloom_timestamps,
            );
            stats.hd_bloom_encoded = true;
        } else if let Some(omega) = &self.omega {
            // Omega's tone map: the executable's exposure and curve over the
            // linear scene, encoded into `view`. See `Self::omega`.
            self.run_omega(omega, device, queue, encoder, view, viewport);
        } else if self.bloom.is_some() || self.ps2_bloom.is_some() {
            // The recovered post-process, reading the alpha channel the scene
            // stamped (`GlowMask::Stamped`, or the PS2's `StampedByTexel`) and
            // adding a blurred copy of the masked colour back over the frame.
            // `view` is the resolved image in both the MSAA and the
            // single-sample case, which is why this runs on it rather than on
            // `attachment_view`. See `oag_post::bloom` - its own
            // `Frame` fields document `size`/`origin`/`viewport` in full - and
            // `oag_post::ps2_bloom` for the PS2's own chain.
            let size = self.depth.size();
            let frame = oag_post::bloom::Frame {
                scene: view,
                size: (size.width, size.height),
                origin: (viewport.0, viewport.1),
                viewport: (viewport.2 as u32, viewport.3 as u32),
            };
            // The composite waits for the HUD: the caller runs
            // `Scene::composite_bloom` after drawing it, as Pulse PSP's queue
            // order does (`bloom.md`, "The bloom draws over the HUD"). The PS2
            // inherits that order, unmeasured on its own disc.
            if let Some(bloom) = &self.bloom {
                bloom.prepare(device, queue, encoder, frame);
            } else if let Some(bloom) = &self.ps2_bloom {
                bloom.prepare(device, queue, encoder, frame);
            }
            self.bloom_pending.set(true);
        }
        // Motion blur, last: it smears the finished frame - but not the
        // Pulse glow, whose composite comes after the HUD - and it
        // runs before the caller composites the HUD over `view`, so the
        // readouts stay sharp however the world moves. `motion_blur` is the
        // strength read fresh off the settings this frame, so the row
        // applies live; at `off` the pass encodes nothing. The velocity
        // buffer it reads was written by the scene pass above either way -
        // see `Scene::velocity`.
        stats.blur_encoded = self.encode_motion_blur(
            device,
            queue,
            encoder,
            [view, velocity_view, depth_view],
            viewport,
            motion_blur.shutter(),
            blur_shake,
            blur_timestamps,
        );
        oag_gpu::perfprobe::mark("post-chain");
        stats
    }
}

mod anims;
mod attachments;
mod beam;
mod lod;
mod particles;
mod road;
mod shadow;
pub(super) use attachments::{depth_texture, msaa_color_texture};
