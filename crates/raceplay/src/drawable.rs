//! One model as the renderer sees it - [`Drawable`] - and the uniform block the
//! mesh shader reads.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;
mod instance;
mod pad_glow;

/// One model on the GPU: its pipeline, its geometry and its own uniform buffer.
///
/// Two of these are drawn into one render pass. Separate pipelines rather than one
/// shared between the models, because each `mesh_render::build` creates its own bind
/// group layouts and pairing a bind group with another pipeline's layout is a
/// validation error waiting to happen.
pub(super) struct Drawable {
    model: std::sync::Arc<Model>,
    pipeline: wgpu::RenderPipeline,
    alpha_test_pipeline: wgpu::RenderPipeline,
    /// One per alpha-test reference this model's own batches ask for - see
    /// `oag_vex::vex::Batch::alpha_test_reference`. Empty on anything but a
    /// `.vex` model.
    cutout_pipelines: Vec<(f32, wgpu::RenderPipeline)>,
    blend_pipeline: [wgpu::RenderPipeline; 2],
    /// The other two recovered transparent blend classes. Selected per draw
    /// call, because a single model mixes them - see
    /// `oag_vex::vex::Batch::blend_class`.
    additive_pipeline: [wgpu::RenderPipeline; 2],
    unblended_pipeline: [wgpu::RenderPipeline; 2],
    /// One pair per equation this model's own file authors - see
    /// `mesh_render::Built::authored_pipelines`. Empty on a Pulse model.
    authored_pipelines: Vec<(wgpu::BlendState, [wgpu::RenderPipeline; 2])>,
    /// The depth prepass pair, for the one drawable built with it - Wipeout
    /// HD's circuit. See `mesh_render::Prepass`.
    prepass: Option<mesh_render::Prepass>,
    /// The index buffer with its triangles in reverse order, each triangle's
    /// own vertices untouched, for the prepass's shaded half - see
    /// `draw_lists`. `None` without a prepass.
    reversed_indices: Option<wgpu::Buffer>,
    /// The opaque draws one `draw_lists` call shows, refilled per call -
    /// see `draw_lists`.
    shown: std::cell::RefCell<draw::Shown>,
    /// The alpha-only glow-mask stamp for blended batches - see
    /// `mesh_render::Built::stamp_pipeline` and [`Self::draw_stamps`].
    stamp_pipeline: Option<mesh_render::Stamp>,
    vertices: wgpu::Buffer,
    /// The coordinates' own buffer, bound to slot 1, for a drawable built
    /// [`mesh_render::Texcoords::Streamed`] - `None` for every other. See
    /// [`Self::bind_vertices`].
    texcoords: Option<wgpu::Buffer>,
    indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    textures: Vec<wgpu::BindGroup>,
    /// Bind group 2: the fog this drawable is rendered with.
    fog_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame from the track's `fogCube`,
    /// or left at [`mesh_render::Fog::off`] for the sky and for a track that
    /// authors no fog.
    pub(super) fog: wgpu::Buffer,
    /// Bind group 3: this drawable's authored texture transforms, sampled for
    /// the current tick.
    anim_bind: wgpu::BindGroup,
    /// The buffer behind it. Rewritten each frame by [`Self::write_anims`], or
    /// left all-identity for a model that authors no track.
    anims: wgpu::Buffer,
    /// This drawable's `Anim Transform` node matrices, bound as binding 1 of
    /// the same group 3 as [`Self::anims`] - see
    /// `mesh_render::Built::node_anim_buffer` for why they share a group.
    ///
    /// The buffer behind it. Rewritten each frame by
    /// [`Self::write_node_anims`], or left all-identity for a model with no
    /// `Anim Transform` at all - which is every ship, every sky and every
    /// synthetic overlay.
    node_anims: wgpu::Buffer,
    /// The glow table's own buffer, binding 2 of the same group 3. Rewritten
    /// by [`Self::glow_weapon_pads`] only; every other drawable leaves what
    /// [`mesh_render::Emissives::of`] wrote at build time.
    emissive: wgpu::Buffer,
    /// The Zone visualiser's own lookup texture - bind group 2's binding 4.
    /// All-black until [`Self::write_zone_vis`] is called, which
    /// `race::Scene::render` does once a frame alongside [`Self::fog`].
    zone_vis: wgpu::Texture,
    /// What [`Self::fog_bind`] needs besides [`Self::fog`] and the showing
    /// stage's own four textures to be rebuilt - see
    /// [`mesh_render::zone::RebindResources`] and [`Self::rebind_zone`].
    zone_rebind: std::sync::Arc<mesh_render::zone::RebindResources>,
    /// This model's opaque index ranges, kept so the shadow caster pass can
    /// draw them without walking the draw list again every frame.
    ///
    /// **Opaque only**: a hull casts, and the transparent flare bolted to it
    /// does not - a shadow map of a craft's glow would darken the road under
    /// its own light.
    opaque_ranges: Vec<std::ops::Range<u32>>,
    /// This instance's per-frame `LodGroup` choice - see [`Self::select_lod`].
    /// Starts on every group's finest child, which a drawable nobody
    /// switches keeps.
    lod: oag_mesh::mesh::LodSwitch,
    /// The road spans a Quake ripples through this model - see
    /// [`oag_render::ripple`]. `None` on everything but a Pulse PSP circuit.
    ripple: std::cell::RefCell<Option<oag_render::ripple::Ripple>>,
    /// Each weapon pad's own position in HD's colour cycle - see
    /// [`Self::glow_weapon_pads`]. Empty on every drawable but HD's pads.
    pad_glow: std::cell::RefCell<pad_glow::PadGlow>,
    /// Draws left out this frame, by [`oag_render::gantry::panel::key`]:
    /// the start gantry's states off its panel ([`Self::set_hidden`]).
    /// Empty on everything else.
    hidden: std::cell::RefCell<Vec<u32>>,
}

impl std::fmt::Debug for Drawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawable")
            .field("model", &self.model.label)
            .field("triangles", &(self.model.indices.len() / 3))
            .finish_non_exhaustive()
    }
}

impl Drawable {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        depth: mesh_render::Depth,
        blend: wgpu::BlendState,
        glow: mesh_render::GlowMask,
        zone: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
        receives_shadow: mesh_render::ShadowReceiver,
    ) -> Result<Self> {
        Self::new_with(
            device,
            queue,
            model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
            glow,
            zone,
            shadow_maps,
            receives_shadow,
            mesh_render::Texcoords::Interleaved,
            false,
            false,
            mesh_render::Velocity::Write,
        )
    }

    /// [`Self::new`] for a model that rides a craft (plume, flare, shield and
    /// absorb shells, shine): its blended draws write the craft's own motion
    /// weighted by alpha, see [`mesh_render::Velocity::Attached`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new_attached(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        depth: mesh_render::Depth,
        blend: wgpu::BlendState,
        glow: mesh_render::GlowMask,
        zone: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
        receives_shadow: mesh_render::ShadowReceiver,
    ) -> Result<Self> {
        Self::new_with(
            device,
            queue,
            model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
            glow,
            zone,
            shadow_maps,
            receives_shadow,
            mesh_render::Texcoords::Interleaved,
            false,
            false,
            mesh_render::Velocity::Attached,
        )
    }

    /// [`Self::new`], with the coordinates' source chosen - see
    /// [`mesh_render::Texcoords`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new_with(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        depth: mesh_render::Depth,
        blend: wgpu::BlendState,
        glow: mesh_render::GlowMask,
        zone: &mesh_render::zone::StageArt,
        // The frame's shadow maps and whether this model's surfaces read them -
        // the track's do and a craft's do not, which is Wipeout HD's own split.
        // See `mesh_render::build`.
        shadow_maps: mesh_render::ShadowMaps<'_>,
        receives_shadow: mesh_render::ShadowReceiver,
        texcoords: mesh_render::Texcoords,
        // Whether the opaque list draws through a depth prepass - see
        // `mesh_render::Prepass` and `draw_lists`.
        prepass: bool,
        // Whether the opaque and cutout lists cull back faces - see
        // `mesh_render::build_with`.
        cull_back: bool,
        velocity: mesh_render::Velocity,
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            prepass,
            alpha_test_pipeline,
            cutout_pipelines,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            authored_pipelines,
            stamp_pipeline,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
            fog_bind,
            fog_buffer,
            zone_vis_texture,
            zone_rebind,
            anim_bind,
            anim_buffer,
            node_anim_buffer,
            emissive_buffer,
        } = mesh_render::build_with(
            device,
            queue,
            &model,
            format,
            anisotropy,
            sample_count,
            depth,
            blend,
            glow,
            // Everything a race draws writes the velocity buffer (or, for
            // its blended pipelines, carries the masked second target) -
            // the buffer is always on in the game path, whatever the
            // motion blur setting says. `Attached` models write their own
            // blended motion too. See `mesh_render::Velocity` and
            // `race::Scene::velocity`.
            velocity,
            zone,
            shadow_maps,
            receives_shadow,
            texcoords,
            prepass,
            cull_back,
            true,
        )?;
        let reversed_indices = prepass.is_some().then(|| {
            let reversed: Vec<u32> = model
                .indices
                .as_chunks::<3>()
                .0
                .iter()
                .rev()
                .flatten()
                .copied()
                .collect();
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("reversed indices"),
                size: std::mem::size_of_val(&reversed[..]) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            oag_gpu::deferred_upload::write_buffer(queue, &buffer, reversed);
            buffer
        });
        let texcoords = (texcoords == mesh_render::Texcoords::Streamed).then(|| {
            let coordinates: Vec<[f32; 2]> = model.vertices.iter().map(|v| v.texcoord).collect();
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("streamed texcoords"),
                size: std::mem::size_of_val(&coordinates[..]) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            oag_gpu::deferred_upload::write_buffer(queue, &buffer, coordinates);
            buffer
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("race uniforms"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        // **Both opaque lists**, and the second is not an optimisation: a
        // Pulse hull's batches are largely *alpha-tested* rather than plain
        // opaque, so a caster built from `draws` alone contributes nothing at
        // all for a craft on that title - which is exactly how the `mapped`
        // tier first drew shadows on the scenery and none under the ship.
        // The cutout itself is ignored in the caster pass: a chain-link fence
        // casts a solid shadow, which is a divergence and a cheap one.
        let opaque_ranges = model
            .draws
            .iter()
            .chain(model.alpha_tested_draws.iter())
            // The finest `LodGroup` tier only: the caster is built once, and
            // a switch per shadow pass is not what it is for - chosen, not
            // measured.
            .filter(|draw| model.lod_groups.shows_nearest(draw))
            .map(|draw| draw.range.clone())
            .collect();
        // The pictures are on the GPU now; the model keeps its slots and drops
        // the texels it decoded them from.
        let mut model = model;
        model.release_texels();
        let model = std::sync::Arc::new(model);
        if oag_gpu::deferred_upload::active() {
            mesh_render::deferred::defer_geometry(&vertices, &indices, &model);
        }
        Ok(Self {
            opaque_ranges,
            lod: oag_mesh::mesh::LodSwitch::new(&model.lod_groups),
            model,
            pipeline,
            alpha_test_pipeline,
            cutout_pipelines,
            blend_pipeline,
            additive_pipeline,
            unblended_pipeline,
            authored_pipelines,
            stamp_pipeline,
            prepass,
            shown: std::cell::RefCell::default(),
            hidden: std::cell::RefCell::default(),
            reversed_indices,
            vertices,
            texcoords,
            indices,
            uniforms,
            uniform_bind,
            textures,
            fog_bind,
            fog: fog_buffer,
            anim_bind,
            anims: anim_buffer,
            node_anims: node_anim_buffer,
            emissive: emissive_buffer,
            zone_vis: zone_vis_texture,
            zone_rebind: std::sync::Arc::new(zone_rebind),
            ripple: std::cell::RefCell::new(None),
            pad_glow: std::cell::RefCell::default(),
        })
    }

    /// Replaces the albedo of texture slot `texture_slot` with `view`: the
    /// billboard placeholders' own picture, drawn each frame by
    /// [`crate::adverts`]. A slot this model has no material for is left alone.
    pub(super) fn set_albedo(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_slot: usize,
        view: &wgpu::TextureView,
    ) {
        // Slot 0 is the white fallback, so a texture slot n binds group n + 1.
        let Some(bind) = self.textures.get_mut(texture_slot + 1) else {
            return;
        };
        *bind = mesh_render::albedo_bind_group(
            device,
            queue,
            &self.pipeline.get_bind_group_layout(1),
            view,
            "advert albedo",
        );
    }

    /// Binds this drawable's vertex buffer - and its streamed coordinates,
    /// when it has them - for the pipelines it built.
    pub(super) fn bind_vertices(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        if let Some(texcoords) = &self.texcoords {
            pass.set_vertex_buffer(1, texcoords.slice(..));
        }
    }

    /// Rebuilds bind group 2 with `stage`'s four textures - see
    /// `oag_mesh::mesh_render::zone::rebind`, which this forwards to.
    /// Everything else this drawable draws with (the pipeline, the
    /// geometry, the shadow map) is untouched.
    ///
    /// **Call this on the stage-change edge alone**, when
    /// `ZoneGrade::follow`/`commit` reports a new `(current, previous)`
    /// pair, not every frame. See `mesh_render::zone::rebind`'s own doc
    /// comment for why a per-frame call would still draw correctly and why
    /// it is still the wrong thing to do.
    pub(super) fn rebind_zone(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stage: &mesh_render::zone::StageArt,
    ) {
        self.fog_bind =
            mesh_render::zone::rebind(device, queue, &self.fog, &self.zone_rebind, stage);
    }

    /// This model's own bounding radius, in model space.
    ///
    /// What the shadow map's light view is fitted to - a box sized to the
    /// craft's *centres* alone clips the craft themselves, which draws a
    /// shadow the size of the overlap rather than of the hull.
    pub(super) fn radius(&self) -> f32 {
        self.model.radius
    }

    /// This drawable as a shadow caster: its own buffers, its opaque ranges
    /// and the matrix that places it.
    ///
    /// **Borrowed, not copied.** The caster pass draws the geometry already on
    /// the GPU with a second pipeline rather than re-uploading it - a full grid
    /// is some twelve thousand triangles, which would be more traffic per frame
    /// than the whole rest of the pass.
    /// Whether this model is shaded by a shipped PS3 program: some material
    /// resolved to an `.rcsmaterial` variant (`Model::material_variants`).
    /// Only `mesh::rcs::build` fills that, so a Pulse or Pure body, whose
    /// materials are the GE's fixed pipeline, answers `false`.
    pub(super) fn is_ps3_shaded(&self) -> bool {
        self.model.material_variants.iter().any(Option::is_some)
    }

    pub(super) fn caster(&self, model: Mat4) -> oag_render::shadow::map::Caster<'_> {
        oag_render::shadow::map::Caster {
            vertices: &self.vertices,
            indices: &self.indices,
            ranges: &self.opaque_ranges,
            model,
        }
    }

    /// Rewrites the Zone visualiser's lookup from `bands` levels, tinted by
    /// `tint` - or blanks it when `tint` is `None`, which is what a stage
    /// with no authored `EQ colour tint` gets. See
    /// `oag_mesh::mesh_render::zone::write_vis`, which this forwards to.
    pub(super) fn write_zone_vis(&self, queue: &wgpu::Queue, bands: &[f32], tint: Option<[u8; 3]>) {
        mesh_render::zone::write_vis(queue, &self.zone_vis, bands, tint);
    }

    /// `prev_mvp` is the previous simulation tick's `view_projection * model`,
    /// premultiplied - what the velocity target measures this drawable's
    /// screen motion against. A drawable with no previous pose (a scene's
    /// first frame, a rocket that just spawned) passes this frame's product,
    /// which is zero object velocity rather than a smear from stale state.
    /// See `race::scene::frame::MotionState`.
    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
    ) {
        self.write_hull(queue, view_projection, model, prev_mvp, None);
    }

    /// [`Self::write`] for a craft: `sun_occlusion_layer` names which layer
    /// of the frame's per-craft sun-occlusion array this hull samples - a
    /// craft under the `original` tier on Wipeout HD - or `None`, which is
    /// what every other drawable writes. See `oag_render::shadow::occlusion`.
    pub(super) fn write_hull(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
        sun_occlusion_layer: Option<usize>,
    ) {
        self.write_uniforms(
            queue,
            view_projection,
            model,
            prev_mvp,
            sun_occlusion_layer,
            [0.0, 0.0],
        );
    }

    /// [`Self::write`] for a model whose materials read their own animation
    /// clock - `clock` seconds, what an HD material's `UV_offset` is bound
    /// to. See `oag_mesh::mesh::slots::CLOCK_SCROLL_RING`.
    pub(super) fn write_clocked(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
        clock: f32,
    ) {
        self.write_uniforms(queue, view_projection, model, prev_mvp, None, [clock, 0.0]);
    }

    /// [`Self::write_clocked`] for the Bomb's fireball, whose program also
    /// reads `ColourAnim` - a float the blast object owns. See
    /// `oag_mesh::mesh::slots::BOMB_FIRE`.
    pub(super) fn write_blast(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
        clock: f32,
        colour: f32,
    ) {
        self.write_uniforms(
            queue,
            view_projection,
            model,
            prev_mvp,
            None,
            [clock, colour],
        );
    }

    fn write_uniforms(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        model: Mat4,
        prev_mvp: Mat4,
        sun_occlusion_layer: Option<usize>,
        [clock, colour]: [f32; 2],
    ) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            sun_occlusion_layer: sun_occlusion_layer.map_or(0.0, |layer| layer as f32 + 1.0),
            model_clock: clock,
            model_colour: colour,
            _pad2: 0.0,
            prev_mvp: prev_mvp.to_cols_array_2d(),
        };
        oag_gpu::perfprobe::write_buffer(queue, &self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Samples every authored texture-transform track this model carries at
    /// `seconds` and uploads the table the vertex shader indexes.
    ///
    /// One buffer write per drawable per frame, whatever the geometry: the
    /// curve is evaluated once per *track*, not per vertex or per draw call.
    ///
    /// **Not called for the boost plume**, whose *texture-transform* table
    /// stays all-identity. The plume needs its own clock - its flare's life
    /// timer, reset at every reveal, rather than the race's - and already
    /// gets it through [`Self::apply_uv_transform`], which rewrites its
    /// vertices directly. Driving it from here as well would apply the
    /// transform twice.
    ///
    /// That says nothing about [`Self::write_node_anims`], which is a
    /// different buffer: the plume's *node* table is written, off the same
    /// reveal timer, because a PS2 plume's two meshes hang off `Anim
    /// Transform` anchors and draw at the craft's origin without it.
    pub(super) fn write_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_tracks.is_empty() {
            return;
        }
        let anims = mesh_render::TexAnims::sample(&self.model, seconds);
        oag_gpu::perfprobe::write_buffer(queue, &self.anims, 0, bytemuck::bytes_of(&anims));
    }

    /// Samples every `Anim Transform` this model carries at `seconds` and
    /// uploads the matrix table the vertex shader indexes.
    ///
    /// The sibling of [`Self::write_anims`], and the same trade: one buffer
    /// write per drawable per frame, with each node's chain resolved once
    /// rather than per vertex. A model with no `Anim Transform` - every ship,
    /// the sky, both pad models, the collision overlay, and every **PSP**
    /// boost plume - writes nothing and keeps the identity table
    /// `mesh_render::build` initialised it with. A **PS2** plume carries two
    /// and is placed entirely by them.
    pub(super) fn write_node_anims(&self, queue: &wgpu::Queue, seconds: f32) {
        if self.model.anim_nodes.is_empty() {
            return;
        }
        let anims = mesh_render::NodeAnims::sample(&self.model, seconds);
        oag_gpu::perfprobe::write_buffer(queue, &self.node_anims, 0, bytemuck::bytes_of(&anims));
    }

    /// Applies a texture transform to this model's **authored** UVs and
    /// uploads them: `uv' = uv * scale + offset`, the GE's own
    /// `TEXSCALE`/`TEXOFFSET` arithmetic.
    ///
    /// Called only for the plume, and only on the frames it is visible. This
    /// replaced environment-mapped generation (`oag_render::texgen`) on
    /// 2026-08-10: the plume's compiled list is replayed under `TEXMAPMODE` 0
    /// (settled live - mesh-draw.md, "The plume is replayed under
    /// `TEXMAPMODE` 0"), so the original reads the authored coordinates,
    /// through the keyframed transform `Loaded::boost_uv_transform` carries.
    ///
    /// From `self.model.vertices` every time rather than from the last
    /// frame's buffer, for the same reason [`Self::deflect_airbrakes`] does:
    /// accumulating offsets would drift.
    pub(super) fn apply_uv_transform(
        &self,
        queue: &wgpu::Queue,
        scale: (f32, f32),
        offset: (f32, f32),
    ) {
        let transformed: Vec<_> = self
            .model
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                v.texcoord = [
                    v.texcoord[0] * scale.0 + offset.0,
                    v.texcoord[1] * scale.1 + offset.1,
                ];
                v
            })
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&transformed));
    }

    /// Swings this model's airbrake flaps to `left` and `right` radians.
    ///
    /// Rewrites the two flaps' own vertices in place rather than giving them a
    /// transform of their own. The alternative - pulling each flap into its own
    /// `Drawable`, the way the boost plume is one - would duplicate the ship's
    /// whole eight-texture set for two meshes of forty vertices, and a
    /// per-draw-call matrix would need a fourth bind group set on every draw
    /// call of every track. This costs two `write_buffer`s of about a kilobyte
    /// and no GPU state at all, and `exhaust.rs` already rewrites a vertex
    /// buffer every frame, so a mutable one is not a new idea here.
    ///
    /// A no-op on a model with no flaps, which is every track, the sky, the
    /// collision overlay and any ship whose file does not carry them.
    ///
    /// `moved` is scratch the caller owns and this refills - see
    /// `Scene::scratch`. It runs on every frame of every race, so building the
    /// span into a fresh `Vec` was two allocations a frame that never survived
    /// the upload.
    pub(super) fn deflect_airbrakes(
        &self,
        queue: &wgpu::Queue,
        left: f32,
        right: f32,
        moved: &mut Vec<mesh::GpuVertex>,
    ) {
        for (flap, angle) in self.model.airbrakes.iter().zip([left, right]) {
            let Some(flap) = flap else { continue };
            // From the model's own vertices every time, never from the last
            // frame's: accumulating rotations would drift, and worse, would
            // make the rest position depend on how the ship got there. The
            // swing is `Flap::swung`, which the extra pass shares.
            let Some(span) = flap.swung(&self.model.vertices, angle, moved) else {
                continue;
            };
            let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
            queue.write_buffer(
                &self.vertices,
                span.start as u64 * stride,
                bytemuck::cast_slice(moved),
            );
        }
    }

    /// Multiplies every vertex colour by `rgba`, from the model's own authored
    /// values.
    ///
    /// What the shield shell needs and the only thing it needs: the original
    /// hands its model a packed colour once a frame (`set_model_colour` in
    /// `ShipShield_Update`) and this engine's mesh pipeline has no per-draw
    /// colour uniform to put one in. Rewriting the vertex colours is the same
    /// trade [`Self::deflect_airbrakes`] makes for the flaps - one
    /// `write_buffer` of a small mesh against a shader change and a fourth bind
    /// group - and it is exactly a multiply, so the authored colour still
    /// decides the look and this only scales it.
    ///
    /// **From `self.model.vertices` every time, never from the last frame's**,
    /// for the reason the flaps give: compounding a per-frame multiply would
    /// darken the shell toward black over a few seconds and make the picture
    /// depend on how long the shield had been up.
    ///
    /// One write for the whole buffer rather than one per node: the shell is a
    /// single mesh, and a per-range loop would be machinery for a case that does
    /// not exist.
    /// `tinted` is scratch the caller owns and this refills - see
    /// `Scene::scratch`. A whole shell's vertices, once per craft with its
    /// shield up and once more for the cockpit sphere: the allocation this
    /// avoids is larger than the pads' and rarer.
    pub(super) fn tint(
        &self,
        queue: &wgpu::Queue,
        rgba: [f32; 4],
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        tinted.clear();
        tinted.extend(self.model.vertices.iter().map(|v| {
            let mut out = *v;
            for (channel, scale) in rgba.iter().enumerate() {
                out.colour[channel] = v.colour[channel] * scale;
            }
            out
        }));
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(tinted));
    }

    /// Recolours each weapon pad's own geometry by whether it currently hands
    /// out a pickup - see [`oag_render::weapon_pad`] for the recovered
    /// mechanism this reproduces. `ready[i]` pairs with this drawable's
    /// `i`-th [`mesh::Model::node_vertex_ranges`] entry - positional, not by
    /// name, for the reason that field's own doc comment gives.
    ///
    /// A no-op on a model with no weapon pads - every track and the sky -
    /// since `ready` is empty there.
    ///
    /// **Also a no-op on a Wipeout HD model**, and that gate is the point of
    /// [`mesh::Model::vertex_colour_is_light`] being read here rather than a
    /// title enum. Two independent reasons, either sufficient:
    ///
    /// 1. The keyframe table is *Pulse's*, read out of the PSP executable at
    ///    `0x08ac00c8` (see [`oag_render::weapon_pad`]). HD's own `Weapon Pad`
    ///    does cycle, but through its own table at `0x008c26a0` and into the
    ///    light bars' constant rather than the mesh - see
    ///    [`Self::glow_weapon_pads`]. `ds_weaponup_cs.gtf` is a grey plate with
    ///    a **red** cross on it, where Pulse's `weapon_under.tga` is neutral and
    ///    takes its colour entirely from `pad+0x6c`.
    /// 2. On an HD model `colour` is not a tint at all. It is the baked
    ///    per-vertex light the fragment program **adds** inside its authored
    ///    lighting sum - see `oag_mesh::mesh::rcs::emit`. Writing a palette
    ///    entry there is not "the wrong colour", it is a different quantity.
    ///
    /// What HD does instead is unrecovered - see `docs/rendering/pads.md`. An
    /// HD pad therefore draws its authored texture under the circuit's own
    /// light rig, unarmed and armed alike, which is a visible absence rather
    /// than an invented state.
    pub(super) fn tint_weapon_pads(
        &self,
        queue: &wgpu::Queue,
        seconds: f32,
        ready: &[bool],
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        if self.model.vertex_colour_is_light {
            return;
        }
        self.recolour_nodes(
            queue,
            ready.iter().map(|&is_ready| {
                if is_ready {
                    oag_render::weapon_pad::ready_colour(seconds)
                } else {
                    oag_render::weapon_pad::COOLDOWN_COLOUR
                }
            }),
            tinted,
        );
    }

    /// Rewrites each of this drawable's [`mesh::Model::node_vertex_ranges`]
    /// entries to its own flat `colour`, one per iterator item, positional
    /// and zipped the way [`Self::tint_weapon_pads`]' own doc comment
    /// describes.
    ///
    /// Rewrites the *replacement* colour, not a multiply against the pad's
    /// authored one - `pad+0x6c` overwrites the mesh's own GE colour command
    /// every tick in the original (see `oag_render::weapon_pad`), and this
    /// mirrors that shape for the speed pad's own, unrecovered override.
    /// Always from `self.model.vertices`, never from the last frame's
    /// buffer, for the same accumulation reason [`Self::deflect_airbrakes`]
    /// gives.
    ///
    /// `tinted` is scratch the caller owns and this refills - see
    /// `Scene::scratch`.
    fn recolour_nodes(
        &self,
        queue: &wgpu::Queue,
        colours: impl Iterator<Item = [f32; 3]>,
        tinted: &mut Vec<mesh::GpuVertex>,
    ) {
        let stride = std::mem::size_of::<mesh::GpuVertex>() as u64;
        // A Quake's bump rolling over a pad has to survive the pad's own
        // rewrite from the authored vertices - see `Ripple::displace`.
        let ripple = self.ripple.borrow();
        for (range, colour) in self.model.node_vertex_ranges.iter().zip(colours) {
            let span = range.start as usize..range.end as usize;
            let Some(base) = self.model.vertices.get(span) else {
                continue;
            };
            // Refilled rather than rebuilt - one pad's worth of vertices, and
            // there is a pad's worth of them on every lap of every circuit
            // every frame. See `Scene::scratch`.
            tinted.clear();
            tinted.extend(base.iter().zip(range.start..).map(|(v, index)| {
                let mut out = *v;
                out.colour = [colour[0], colour[1], colour[2], out.colour[3]];
                if let Some(ripple) = ripple.as_ref() {
                    ripple.displace(index, &mut out);
                }
                out
            }));
            queue.write_buffer(
                &self.vertices,
                u64::from(range.start) * stride,
                bytemuck::cast_slice(tinted),
            );
        }
    }

    /// Chooses this frame's child of each authored `LodGroup` for the
    /// instance at `model` seen from `eye` - `LodGroup_SelectChild`'s rule,
    /// see `oag_mesh::mesh::LodGroups::child_at`. Free on a model with no
    /// group, which is every drawable but a circuit and a hull.
    pub(super) fn select_lod(&self, model: Mat4, eye: oag_mesh::mesh::LodEye) {
        self.lod.select(&self.model.lod_groups, model, eye);
    }

    /// The opaque list, in the model's own order - what an `order` handed to
    /// [`Self::draw_lists`] indexes.
    pub(crate) fn opaque_draws(&self) -> &[DrawCall] {
        &self.model.draws
    }

    /// Whether `draw` is in the `LodGroup` child this frame enables, and not
    /// left out by [`Self::set_hidden`].
    fn lod_shows(&self, draw: &DrawCall) -> bool {
        self.lod.shows(&self.model.lod_groups, draw) && {
            let hidden = self.hidden.borrow();
            hidden.is_empty() || !hidden.contains(&oag_render::gantry::panel::key(draw))
        }
    }

    /// Leaves the draws keyed in `keys` out of every list until the next call.
    pub(super) fn set_hidden(&self, keys: &[u32]) {
        let mut hidden = self.hidden.borrow_mut();
        hidden.clear();
        hidden.extend_from_slice(keys);
    }

    /// This hull's geometry and materials, for the ghost ship's pipeline to
    /// draw through - every list, since the original's ghost state overrides
    /// every batch's own. See `oag_render::ghost`.
    ///
    /// The `LodGroup` child is the ghost's own, from `model` and `eye`, not
    /// the grid slot's whose hull this borrows: the ghost is its own craft
    /// at its own distance.
    pub(super) fn ghost_hull(
        &self,
        at: Mat4,
        eye: oag_mesh::mesh::LodEye,
    ) -> oag_render::ghost::Hull<'_> {
        let model = &self.model;
        oag_render::ghost::Hull {
            vertices: &self.vertices,
            indices: &self.indices,
            textures: &self.textures,
            draws: (model.draws.iter())
                .chain(&model.alpha_tested_draws)
                .chain(&model.transparent_draws)
                .filter(|draw| model.lod_groups.shows_at(draw, at, eye))
                .map(|draw| (draw.range.clone(), draw.texture))
                .collect(),
        }
    }
}

/// Uniforms shared with `oag-render`'s `mesh.wgsl`.
///
/// Declared here rather than reused because `oag_mesh::mesh_render` only exposes
/// a writer that computes an *orbit* camera from the model's bounding sphere, which
/// is what a viewer wants and is not something a chase camera can use. The layout is
/// the shader's own, and a test below asserts it against
/// `mesh_render::UNIFORMS_SIZE`, so a change on that side is a failing test rather
/// than a silently wrong picture.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    /// Which layer of the per-craft sun-occlusion array this model samples,
    /// plus one; `0.0` for none - see `mesh_render`'s own mirror of this
    /// layout and `oag_render::shadow::occlusion`.
    sun_occlusion_layer: f32,
    /// See `mesh_render`'s own mirror: the model's own animation clock.
    model_clock: f32,
    /// See `mesh_render`'s own mirror: `ColourAnim` on the Bomb's fireball.
    model_colour: f32,
    _pad2: f32,
    /// The previous tick's `view_projection * model`, premultiplied - one
    /// matrix rather than a second pair, per `mesh.wgsl`'s own mirror: fog
    /// and lighting need world position, velocity needs only clip position.
    prev_mvp: [[f32; 4]; 4],
}

/// A craft's model matrix, which is the only correct way to place its mesh.
///
/// The barrel roll turns this about the craft's own nose: `FUN_08841f88`
/// composes its rotation with the rigid body's own matrix, about the
/// forward axis that matrix's own third row carries, so it is folded into
/// `body.orientation` here rather than applied as a separate factor -
/// `orientation * roll` is a local-space rotation about `-Z`
/// ([`oag_physics::ship::Body::forward`]'s own axis), and composing it there
/// keeps it independent of whatever `vmmul.q`'s unresolved operand order
/// would otherwise carry through a following scale. See `oag_render::roll`.
pub(super) fn model_matrix_of(ship: &Ship) -> Mat4 {
    let body = &ship.physics.body;
    let roll = oag_render::roll::rotation(Vec3::NEG_Z, ship.physics.roll_phase);
    Mat4::from_rotation_translation(body.orientation * roll, body.position)
        * Mat4::from_rotation_y(MODEL_YAW)
        * Mat4::from_scale(Vec3::splat(oag_fx::exhaust::CRAFT_ROW_SCALE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ship at the origin, upright and facing the body's own `-Z` - the
    /// axis [`model_matrix_of`] rolls about. `roll_phase` starts at `0.0`
    /// from [`ShipState::default`], so a level ship draws exactly what it
    /// drew before the roll landed.
    fn level_ship() -> Ship {
        Ship::default()
    }

    #[test]
    fn a_level_roll_phase_leaves_the_matrix_unrolled() {
        let ship = level_ship();
        assert_eq!(ship.physics.roll_phase, 0.0);
        let rolled = model_matrix_of(&ship);
        let expected = Mat4::from_rotation_translation(
            ship.physics.body.orientation,
            ship.physics.body.position,
        ) * Mat4::from_rotation_y(MODEL_YAW)
            * Mat4::from_scale(Vec3::splat(oag_fx::exhaust::CRAFT_ROW_SCALE));
        assert_eq!(rolled, expected);
    }

    /// A non-zero roll phase visibly changes the matrix - the point of this
    /// whole thread - and it does so about the body's own forward axis,
    /// which a rotation about that axis leaves fixed.
    #[test]
    fn a_rolling_ship_turns_about_its_own_forward_axis() {
        let mut ship = level_ship();
        ship.physics.roll_phase = 1.0;
        let rolled = model_matrix_of(&ship);
        let level = {
            let mut level = ship;
            level.physics.roll_phase = 0.0;
            model_matrix_of(&level)
        };
        assert_ne!(rolled, level, "a barrel roll must change the drawn matrix");

        let forward = ship.physics.body.forward();
        let a = rolled.transform_vector3(forward);
        let b = level.transform_vector3(forward);
        assert!(
            (a - b).length() < 1e-4,
            "the forward axis must not move: {a} vs {b}"
        );
    }

    /// [`oag_render::roll::ROLL_DIRECTION`] against the original's own
    /// display matrix, captured live 2026-09-08 - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`'s roll
    /// section for the full method and numbers. `entity+0x880` (the eased phase the original's display
    /// matrix reads) was written to `+0.2` at a breakpoint and the resulting
    /// display matrix's own row 0 - where the physics-local right axis
    /// lands in world space - was read back, alongside the body basis at
    /// the same instant. Reproducing that same body basis and the same
    /// angle through this crate's own `orientation * roll` composition
    /// and comparing the world height (`dot(row0, Vec3::Y)`) of the right
    /// axis is a sign check that needs no hand algebra: whichever
    /// `ROLL_DIRECTION` reproduces the original's sign is correct, and this
    /// pins it without assuming anything about axis handedness on either
    /// side.
    #[test]
    fn roll_direction_matches_the_original_captured_display_matrix() {
        // Body basis (right/up/forward rows) and the resulting display
        // matrix's own row 0, both read from `entity+0x794`'s rigid body and
        // `entity+0x8b0`'s display node while `entity+0x880` was pinned at
        // `+0.2` (`entity+0x854`, the steering lean, pinned at `0.0`).
        let right = Vec3::new(0.867_575_1, 0.002_168_793_2, 0.497_301_55);
        let up = Vec3::new(-0.045_852_03, 0.996_079_86, 0.075_647_85);
        let forward = Vec3::new(-0.495_188, -0.088_432_48, 0.864_273_5);
        let original_row0 = Vec3::new(0.168_767_69, 0.710_852_6, 0.169_427_96);

        let orientation = Quat::from_mat3(&oag_core::math::Mat3::from_cols(right, up, forward));
        // `entity+0x880` is the *eased* phase the display matrix reads
        // directly (`angle = entity[0x880] * 6.28`, no second `ease()`
        // pass) - not `roll_phase` itself, so the angle is built straight
        // from the same `0.2 * FULL_TURN` the capture used rather than
        // routed back through `oag_render::roll::ease`.
        let angle = 0.2 * oag_render::roll::FULL_TURN;
        let roll = Quat::from_axis_angle(Vec3::NEG_Z, angle * oag_render::roll::ROLL_DIRECTION);
        let ours_row0 = orientation * roll * Vec3::X;

        assert_eq!(
            ours_row0.dot(Vec3::Y).signum(),
            original_row0.dot(Vec3::Y).signum(),
            "our right axis's world height must rise/fall the same way the \
             original's did for the same body and the same positive phase: \
             ours={ours_row0} original={original_row0}"
        );
    }
}

pub(super) mod draw;
mod occlusion;
mod overlay;
mod ripple;
mod stamp;
