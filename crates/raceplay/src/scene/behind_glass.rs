//! Wipeout HD's behind-the-glass target: the picture Vineta K's tunnel glass
//! shows.
//!
//! **What the original draws, read off Vineta K's RPCS3 captures** (each draw
//! tied to its chunk by vertex offset; `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`
//! and `docs/formats/rcsmaterial.md`, confidence 88): before the
//! main view, a **640x360** target (VRAM `0xC1E50000`, surface clip
//! `0x280 x 0x168`) is cleared and drawn with the sky's six faces, then every
//! chunk whose render flag has `0x10`, under the main camera with a projection
//! **4/3 wider in tangent** (`c[256]`'s `x` and `y` rows are the main view's
//! times 0.75). Those chunks are culled with the main view's own planes, so a
//! chunk just outside the main view is absent from the target too. Each is
//! fogged with the circuit's alternate fog pair where its flag has `0x20` and
//! the primary pair where not, and lit as the main view lights it (the same
//! programs). Depth test `LEQUAL` with write on, blend off for the opaque
//! draws. The tunnel glass then samples the target through `refractProject`,
//! which is this target's matrix (`oag_mesh::mesh::rcs::refraction`).
//!
//! The chunks come from [`oag_mesh::mesh::rcs::build_scene_views`], two models
//! split by fog; the sky is a second drawable over the same sky model, because
//! a uniform buffer written twice in one frame holds only the second write
//! when the passes run.
//!
//! **Built only for a circuit with such chunks**, Vineta K in either
//! direction, so every other circuit draws exactly what it drew without it.
//!
//! # The planar reflection (a [`Mirror`])
//!
//! The original has one such screen-space target and two jobs that fill it:
//! `Scene_PrepareFrame` (`0x003aa888`) clears it and runs RenderTrackRefract
//! when the eye is at or below the circuit's `Water plane height` `h`, and
//! RenderTrackReflect (`TrackReflect_RenderJob`, `0x006cf358`) when above. The
//! reflect job draws the sky, then the chunks with
//! `oag_rcs::rcsmodel::RENDER_REFLECTED` (fogged as here), under the main
//! view-projection composed with the mirror `y' = h - y`, the main eye
//! unchanged, and with the sun colour and the prelit scale bound doubled
//! (`directionalLight0Colour` and `prelitScaleSpecular` read from copies
//! multiplied by 2.0). HD's sea (`reflectplane_dc_seawater`) samples it
//! through the main view-projection (`reflectProject`) at its own point pushed
//! `distortion * (N.x, 0, N.z)`. A 960x540 target, read live on Moa Therma
//! (`docs/ghidra/functions/ps3-hdfury-eu/track-reflect.md`,
//! `docs/rendering/water.md`, "The planar reflection, drawn").
//!
//! The mirror reverses the winding; this draws the target with the clip `x`
//! flipped as well, so the two flips cancel and every draw keeps the main
//! view's own cull, and the sea's lookup flips `u` with the same matrix
//! ([`Scene::target_projection`]). The original draws only the reflect half on
//! these circuits: what the refract job puts in the target when the eye drops
//! below `h` there is unread, so this clears it to black and draws nothing.

use super::*;

/// The original's target, in pixels: measured, and kept at that size whatever
/// the window's, so the glass's picture is as soft as the original's at its own
/// 1280x720 and no sharper at a larger one.
pub(super) const SIZE: (u32, u32) = (640, 360);

/// How much the target's projection scales the main view's clip `x` and `y`:
/// `c[256]`'s `sy` is `1.29923 = 0.75 * 1.7323`, the main view's, measured.
const NARROWING: f32 = 0.75;

/// The planar reflection target's size: measured on Moa Therma (surface clip `0x3c0 x
/// 0x21c` beside a 1920x1080 output), kept whatever the window's, as [`SIZE`] is.
pub(crate) const REFLECTION_SIZE: (u32, u32) = (960, 540);

/// What the reflect job multiplies the sun colour and the prelit scale by before it binds
/// them: the vector at `0x007b0da0`, `{2.0, 0, 0, 0}` splatted (`Scene_PrepareFrame`,
/// `0x003ab074`..`0x003ab118`).
const REFLECTED_LIGHT: f32 = 2.0;

/// A circuit's planar reflection: the mirror and the sea's sample offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mirror {
    /// `Water.Water plane height`, the `h` of the mirror `y' = h - y` and of the
    /// eye test.
    pub plane_height: f32,
    /// `Water.Water distortion`, the sea program's `distortion.z`.
    pub distortion: f32,
}

impl Mirror {
    /// The executable's defaults (`Environment_RegisterLightingSchema`, `0x003a83d8`:
    /// `+0x528` = -50.0, `+0x520` = 40.0), which every circuit with a planar sea authors.
    pub const DEFAULT: Self = Self {
        plane_height: -50.0,
        distortion: 40.0,
    };

    /// World to world: `y' = h - y`.
    fn matrix(self) -> Mat4 {
        Mat4::from_translation(Vec3::new(0.0, self.plane_height, 0.0))
            * Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
    }
}

/// The clip-space flip that keeps the mirrored target's winding the main view's.
fn flip_x() -> Mat4 {
    Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0))
}

/// `scene` as the reflect job binds it: the sun colour and the prelit scale doubled, nothing
/// else changed.
pub(super) fn reflected_scene(scene: &mesh_render::Scene) -> mesh_render::Scene {
    let mut out = *scene;
    out.light.sun = scene.light.sun.map(|c| c * REFLECTED_LIGHT);
    out.light.prelit_scale = scene.light.prelit_scale.map(|c| c * REFLECTED_LIGHT);
    out
}

/// What a load hands the scene for the target: the chunks, split by fog, and
/// the circuit's alternate fog pair.
#[derive(Debug)]
pub struct BehindGlassModels {
    /// The original's target size in pixels: HD's [`SIZE`], or the psp2 titles' own
    /// (`oag_mesh::mesh::rcs::psp2::refraction_only::target_size`).
    pub size: (u32, u32),
    /// The fraction of [`Self::size`] the target is drawn at first
    /// (`graphics.reflection_resolution`); the texture is always [`Self::size`].
    pub resolution: oag_display::display::ReflectionResolution,
    /// The two fog groups, both `None` on every circuit but Vineta K.
    pub groups: oag_mesh::mesh::rcs::BehindGlass,
    /// `Fog.Alternate Fog Color` / `Alternate Fog Density`, read only when a
    /// group exists.
    pub alternate_fog: Option<mesh_render::Fog>,
    /// `Some` where the groups are the planar reflection's chunks rather than the
    /// behind-the-glass ones.
    pub mirror: Option<Mirror>,
}

impl BehindGlassModels {
    /// The size the target is drawn at: [`Self::size`] at [`Self::resolution`].
    #[must_use]
    pub fn drawn_size(&self) -> (u32, u32) {
        self.resolution.size(self.size)
    }
}

impl Default for BehindGlassModels {
    fn default() -> Self {
        Self {
            size: SIZE,
            resolution: oag_display::display::ReflectionResolution::Original,
            groups: oag_mesh::mesh::rcs::BehindGlass::default(),
            alternate_fog: None,
            mirror: None,
        }
    }
}

/// The target and what draws into it.
///
/// **Allocated at the original's size and drawn into a rectangle of it** from the top-left
/// corner, the way dynamic resolution varies a viewport and not an allocation (ADR-0037): a
/// smaller `graphics.reflection_resolution`, a step of `auto`, or a photo's full size is a new
/// rectangle on the next frame, never a new texture or bind group. The readers scale their
/// lookup to the rectangle and clamp it half a texel inside ([`Scene::target_projection`],
/// [`Scene::sea_reflection`]).
#[derive(Debug)]
pub(super) struct Target {
    /// The original's size: the colour texture's and the attachments', and what a photo draws.
    original: (u32, u32),
    /// The rectangle drawn this frame, from the top-left corner, at most [`Self::original`].
    drawn: (u32, u32),
    texture: wgpu::Texture,
    colour: wgpu::TextureView,
    depth: wgpu::TextureView,
    velocity: wgpu::TextureView,
    sky: Option<Drawable>,
    /// The `0x30` chunks, fogged with [`Self::alternate_fog`].
    alternate: Option<Drawable>,
    /// The `0x10` chunks without `0x20`, fogged with the primary pair.
    primary: Option<Drawable>,
    alternate_fog: Option<mesh_render::Fog>,
    mirror: Option<Mirror>,
}

/// The target's colour texture: created ahead of the drawables, because the
/// circuit's own drawable binds it at build. `None` when there is nothing to
/// draw into it.
fn colour_texture(
    device: &wgpu::Device,
    models: &BehindGlassModels,
    format: wgpu::TextureFormat,
) -> Option<wgpu::Texture> {
    if models.groups.is_empty() && models.mirror.is_none() {
        return None;
    }
    Some(colour_texture_of(device, models.size, format))
}

fn colour_texture_of(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("behind the glass"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
        texture_binding_view_dimension: None,
    })
}

/// The depth and velocity attachments the target's passes draw with, `size` wide.
fn attachments(device: &wgpu::Device, size: (u32, u32)) -> (wgpu::TextureView, wgpu::TextureView) {
    let target = |label: &str, format| {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent(size),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
                texture_binding_view_dimension: None,
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    (
        target("behind the glass depth", mesh_render::DEPTH_FORMAT),
        target(
            "behind the glass velocity",
            oag_gpu::formats::VELOCITY_FORMAT,
        ),
    )
}

fn extent(size: (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
    }
}

/// The target's matrix from the main view's: the original's `refractProject`
/// before its texture mapping.
pub(super) fn view_projection(main: Mat4) -> Mat4 {
    Mat4::from_scale(Vec3::new(NARROWING, NARROWING, 1.0)) * main
}

impl Target {
    /// The target for a circuit's `models`, or `None` when it has no chunk to
    /// draw into one. `sky` is the circuit's sky model, the one the main view
    /// draws.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: BehindGlassModels,
        sky: Option<&Model>,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        anisotropy: Anisotropy,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Option<Self>> {
        let Some(texture) = colour_texture(device, &models, format) else {
            return Ok(None);
        };
        let sky = sky.cloned();
        Self::new(
            device,
            queue,
            texture,
            models,
            sky,
            format,
            store,
            anisotropy,
            zone_art,
            shadow_maps,
        )
        .map(Some)
    }

    /// The colour texture the circuit's glass reads.
    pub(super) fn colour(&self) -> &wgpu::TextureView {
        &self.colour
    }

    /// The rectangle drawn at `resolution`: a function of the original's size alone.
    fn rect(&self, resolution: oag_display::display::ReflectionResolution) -> (u32, u32) {
        resolution.size(self.original)
    }

    /// Builds the drawables that fill `texture`.
    #[allow(clippy::too_many_arguments)]
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: wgpu::Texture,
        models: BehindGlassModels,
        sky: Option<Model>,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        anisotropy: Anisotropy,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Self> {
        let (depth, velocity) = attachments(device, models.size);
        // Single-sided, as every chunk draw into the original's target is
        // (`oag_mesh::mesh::rcs::build_scene_views`): its eye sits inside
        // shells authored to be seen from outside. The sky is two-sided, as
        // the original's six sky draws there are.
        let drawable = |model: Model, depth, receiver, cull_back| {
            Drawable::new_with(
                device,
                queue,
                model,
                format,
                store,
                anisotropy,
                1,
                depth,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
                zone_art,
                shadow_maps,
                receiver,
                mesh_render::Texcoords::Interleaved,
                false,
                cull_back,
                mesh_render::Velocity::Write,
                true,
            )
        };
        // The reflection's chunks keep each draw's own cull, the main view's: the mirror and
        // the clip flip cancel ([`Mirror`]).
        let cull_all = models.mirror.is_none();
        let scenery = |model| {
            drawable(
                model,
                mesh_render::Depth::Scene,
                mesh_render::ShadowReceiver::Both,
                cull_all,
            )
        };
        let drawn = models.drawn_size();
        let BehindGlassModels {
            size: original,
            resolution: _,
            groups,
            alternate_fog,
            mirror,
        } = models;
        Ok(Self {
            original,
            drawn,
            colour: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture,
            depth,
            velocity,
            sky: sky
                .filter(|model| !model.indices.is_empty())
                .map(|model| {
                    drawable(
                        model,
                        mesh_render::Depth::Sky,
                        mesh_render::ShadowReceiver::Never,
                        false,
                    )
                })
                .transpose()?,
            alternate: groups.alternate_fog.map(scenery).transpose()?,
            primary: groups.primary_fog.map(scenery).transpose()?,
            alternate_fog,
            mirror,
        })
    }

    /// Moves every drawable's matrices onto the shared buffer.
    pub(super) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        for drawable in [
            self.alternate.as_mut(),
            self.primary.as_mut(),
            self.sky.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.share_uniforms(device, blocks);
        }
    }

    /// Moves the two groups onto the race's scene blocks: the primary reads the circuit's own
    /// block, the alternate its own, which [`Self::write`] fills.
    pub(super) fn share_scene(&mut self, device: &wgpu::Device, blocks: &mut SceneBlocks) {
        if let Some(alternate) = &mut self.alternate {
            alternate.share_scene(device, blocks, SceneKind::GlassAlternate);
        }
        // The reflection's primary group reads its own block: the circuit's with the
        // reflect job's doubled lights ([`reflected_scene`]).
        let primary_kind = if self.mirror.is_some() {
            SceneKind::Reflection
        } else {
            SceneKind::Circuit
        };
        if let Some(primary) = &mut self.primary {
            primary.share_scene(device, blocks, primary_kind);
        }
    }

    /// Writes every drawable's uniforms for this frame: `matrix` is
    /// [`view_projection`] of the main view's (or the mirrored one), `eye` where the
    /// sky cube is centred, `scene` the circuit's own scene
    /// uniform (whose fog the alternate group swaps for its pair, in the view's
    /// alternate block), `seconds` the scenery clock. The primary group reads the circuit's
    /// block, which the caller wrote.
    pub(super) fn write(
        &self,
        queue: &Uploads<'_>,
        (blocks, block): (&SceneBlocks, ViewBlock),
        matrix: Mat4,
        eye: Vec3,
        scene: &mesh_render::Scene,
        seconds: f32,
    ) {
        if let Some(sky) = &self.sky {
            let at = Mat4::from_translation(eye);
            sky.write(queue, matrix, at, matrix * at);
        }
        let alternate = mesh_render::Scene {
            fog: self
                .alternate_fog
                .map_or(scene.fog, |pair| mesh_render::Fog {
                    colour: pair.colour,
                    density: pair.density,
                    curve: pair.curve,
                    enabled: pair.enabled,
                    ..scene.fog
                }),
            ..*scene
        };
        if self.mirror.is_some() {
            blocks.write(
                queue,
                block,
                SceneKind::GlassAlternate,
                &reflected_scene(&alternate),
            );
            blocks.write(queue, block, SceneKind::Reflection, &reflected_scene(scene));
        } else {
            blocks.write(queue, block, SceneKind::GlassAlternate, &alternate);
        }
        for drawable in [&self.alternate, &self.primary].into_iter().flatten() {
            drawable.write(queue, matrix, Mat4::IDENTITY, matrix);
            drawable.write_anims(queue, seconds);
            drawable.write_node_anims(queue, seconds);
        }
    }

    /// Draws the target: its chunks' solid lists, the sky, then their blended
    /// lists - the main view's own order - culled by the main view's own
    /// visibility, as the original culls them.
    pub(super) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        block: ViewBlock,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
        draw: bool,
    ) -> SceneStats {
        let stamp = oag_gpu::timing::stamp("glass");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("behind the glass"),
            color_attachments: &[
                // Cleared to opaque black, the clear value the capture holds;
                // the sky covers every pixel anyway.
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.colour,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.velocity,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: stamp.as_ref().map(oag_gpu::timing::Stamp::render_writes),
            occlusion_query_set: None,
            multiview_mask: None,
        });
        oag_gpu::perfprobe::marks::mark(&mut pass, "behind the glass");
        // The rectangle this frame draws; the clear above covers the whole texture, so a texel
        // outside it is black, and the readers never reach one (`Scene::sea_reflection`).
        let (width, height) = (self.drawn.0 as f32, self.drawn.1 as f32);
        pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, self.drawn.0, self.drawn.1);
        if !draw {
            oag_gpu::perfprobe::marks::close(&mut pass);
            return SceneStats::default();
        }
        let groups = || self.alternate.iter().chain(&self.primary);
        let mut stats = SceneStats::default();
        let solid = Lists::Solid { order: None };
        for group in groups() {
            stats.add(group.draw_lists(&mut pass, block, None, set, chunks, frustum, solid));
        }
        if let Some(sky) = &self.sky {
            pass.set_viewport(0.0, 0.0, width, height, 1.0, 1.0);
            let _ = sky.draw(&mut pass, block, None, None, None, None);
            pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
        }
        for group in groups() {
            stats.add(group.draw_lists(
                &mut pass,
                block,
                None,
                set,
                chunks,
                frustum,
                Lists::Blended {
                    depth_plane: None,
                    between: None,
                    foliage_depth: false,
                },
            ));
        }
        oag_gpu::perfprobe::marks::close(&mut pass);
        stats
    }
}

impl Scene {
    /// Writes and draws the behind-the-glass target ahead of the main view,
    /// when the circuit has one: `matrix` is the main view's unjittered
    /// view-projection, the culling is the main view's. Returns what it drew.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_behind_glass(
        &self,
        queue: &Uploads<'_>,
        encoder: &mut wgpu::CommandEncoder,
        block: ViewBlock,
        scene: &mesh_render::Scene,
        (matrix, eye, seconds): (Mat4, Vec3, f32),
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let Some(target) = &self.behind_glass else {
            return SceneStats::default();
        };
        if oag_gpu::skip::skipped("glass") {
            return SceneStats::default();
        }
        let Some(mirror) = target.mirror else {
            target.write(
                queue,
                (&self.scene_blocks, block),
                view_projection(matrix),
                eye,
                scene,
                seconds,
            );
            return target.render(encoder, block, set, chunks, frustum, true);
        };
        // The reflect job runs only while the eye is above the plane height
        // (`Scene_PrepareFrame`, `0x003ac014`..`0x003ac030`); below it the target is cleared.
        let above = eye.y > mirror.plane_height;
        // The sky cube is placed at the mirrored eye, so the mirror carries it back round the
        // camera: the sky's rotation with its `y` row negated, as the target's sky draws read.
        target.write(
            queue,
            (&self.scene_blocks, block),
            flip_x() * matrix * mirror.matrix(),
            mirror.matrix().transform_point3(eye),
            scene,
            seconds,
        );
        // The main view's frustum would drop what is off screen but seen in the sea (a
        // tower's top); the visible set stays. How the original culls its reflect list
        // is unread, so this is chosen, not measured.
        target.render(encoder, block, set, None, None, above)
    }

    /// Draws the circuit's glass and sea target at `resolution` of the original's size from the
    /// next frame on: a rectangle of the one texture, so nothing is reallocated or rebound. A
    /// circuit with no target does nothing. Called every frame with the frame's step (a fixed
    /// choice, the step `auto` is at, or a photo's `original`); `auto` itself draws the
    /// original's size.
    pub fn set_reflection_resolution(
        &mut self,
        resolution: oag_display::display::ReflectionResolution,
    ) {
        if let Some(target) = self.behind_glass.as_mut() {
            target.drawn = target.rect(resolution);
        }
    }

    /// The glass and sea target's drawn rectangle in texels now, or `None` on a circuit with no
    /// target.
    #[must_use]
    pub fn reflection_target_size(&self) -> Option<(u32, u32)> {
        self.behind_glass.as_ref().map(|target| target.drawn)
    }

    /// The matrix the circuit's screen-space target is read through (`scene.refraction`):
    /// the glass's `refractProject` ([`view_projection`]), or, for the planar reflection, the
    /// main view-projection the sea's `reflectProject` is, flipped as the target is drawn;
    /// either then mapped onto the drawn rectangle ([`rect_mapping`]).
    pub(super) fn target_projection(&self, main: Mat4) -> Mat4 {
        let Some(target) = self.behind_glass.as_ref() else {
            return view_projection(main);
        };
        let projection = match target.mirror {
            Some(_) => flip_x() * main,
            None => view_projection(main),
        };
        rect_mapping(target.drawn, target.original) * projection
    }

    /// `scene.sea_reflection`: `x` the sea's `distortion` and `y` the flag that the planar target
    /// is bound (both zero where the circuit has no mirror), `zw` the largest texture coordinate
    /// a reader of the target may sample: half a texel inside the drawn rectangle, so neither a
    /// lookup past it nor the filter's footprint reaches a texel outside it. `1.0` with no target.
    pub(super) fn sea_reflection(&self) -> [f32; 4] {
        let Some(target) = self.behind_glass.as_ref() else {
            return [0.0, 0.0, 1.0, 1.0];
        };
        let (distortion, bound) = target
            .mirror
            .map_or((0.0, 0.0), |mirror| (mirror.distortion, 1.0));
        let [u, v] = uv_limit(target.drawn, target.original);
        [distortion, bound, u, v]
    }
}

/// Clip space onto the drawn rectangle's: the readers map clip `x` and `y` to a texture coordinate
/// `u = (1 + x) / 2` and `v = (1 - y) / 2`, so scaling `u` and `v` by the rectangle's share of the
/// texture is `x' = f x + (f - 1) w` and `y' = f y + (1 - f) w`, a linear map folded into the
/// projection the readers already take. The identity at the original's size.
pub(super) fn rect_mapping(drawn: (u32, u32), original: (u32, u32)) -> Mat4 {
    let fx = drawn.0 as f32 / original.0.max(1) as f32;
    let fy = drawn.1 as f32 / original.1.max(1) as f32;
    Mat4::from_cols_array(&[
        fx,
        0.0,
        0.0,
        0.0, //
        0.0,
        fy,
        0.0,
        0.0, //
        0.0,
        0.0,
        1.0,
        0.0, //
        fx - 1.0,
        1.0 - fy,
        0.0,
        1.0,
    ])
}

/// The largest texture coordinate inside the drawn rectangle: its far edge less half a texel,
/// where a bilinear lookup reads that edge's texels alone. `1.0` at the original's size (the
/// sampler's clamp already holds there).
pub(super) fn uv_limit(drawn: (u32, u32), original: (u32, u32)) -> [f32; 2] {
    let axis = |drawn: u32, original: u32| {
        if drawn >= original {
            1.0
        } else {
            (drawn as f32 - 0.5) / original.max(1) as f32
        }
    };
    [axis(drawn.0, original.0), axis(drawn.1, original.1)]
}

/// `OAG_DUMP_BEHIND_GLASS=<path.png>`: writes the target after a capture.
const DUMP_VAR: &str = "OAG_DUMP_BEHIND_GLASS";

impl Scene {
    /// Writes the behind-the-glass target to `OAG_DUMP_BEHIND_GLASS`'s path,
    /// when set and the circuit has one: linear light, clamped and sRGB
    /// encoded, before the HD chain's exposure. The target's only other
    /// observable is the glass's colour, so this is how the picture behind it
    /// is told apart from the glass's own weight - the same reason
    /// the sun-occlusion dump beside it in `dump_offscreen_if_asked` exists.
    ///
    /// # Errors
    ///
    /// A failed write.
    pub(super) fn dump_behind_glass_if_asked(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<()> {
        let (Ok(dump), Some(target)) = (std::env::var(DUMP_VAR), &self.behind_glass) else {
            return Ok(());
        };
        let size = target.drawn;
        let texel = target.texture.format().block_copy_size(None).unwrap_or(4);
        let row = (size.0 * texel).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("behind the glass readback"),
            size: u64::from(row * size.1),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            target.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(size.1),
                },
            },
            extent(size),
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let mapped = slice.get_mapped_range()?;
        let mut rgba = Vec::with_capacity((size.0 * size.1 * 4) as usize);
        for line in mapped.chunks(row as usize).take(size.1 as usize) {
            for pixel in line[..(size.0 * texel) as usize].chunks(texel as usize) {
                if texel == 8 {
                    let channel = |i: usize| {
                        half_to_f32(u16::from_le_bytes([pixel[2 * i], pixel[2 * i + 1]]))
                    };
                    rgba.extend((0..3).map(|i| srgb_byte(channel(i))));
                    rgba.push(255);
                } else {
                    rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
        }
        drop(mapped);
        buffer.unmap();
        std::fs::write(&dump, oag_texture::png::encode_rgba(size.0, size.1, &rgba))
            .map_err(|error| anyhow::anyhow!("writing {dump}: {error}"))?;
        println!("wrote {dump} (behind the glass, {}x{})", size.0, size.1);
        Ok(())
    }
}

/// An IEEE half's value.
fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x3ff);
    sign * match exponent {
        0 => mantissa * 2f32.powi(-24),
        31 => f32::INFINITY,
        e => (1.0 + mantissa / 1024.0) * 2f32.powi(e - 15),
    }
}

/// Linear light to an sRGB byte, clamped.
fn srgb_byte(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let encoded = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `y' = h - y`: Moa Therma's `h` -50 mirrors about y = -25, the plane read live.
    #[test]
    fn the_mirror_reflects_about_half_the_plane_height() {
        let mirror = Mirror::DEFAULT;
        assert_eq!((mirror.plane_height, mirror.distortion), (-50.0, 40.0));
        let m = mirror.matrix();
        assert_eq!(
            m.transform_point3(Vec3::new(3.0, 10.0, -7.0)),
            Vec3::new(3.0, -60.0, -7.0)
        );
        assert_eq!(
            m.transform_point3(Vec3::new(1.0, -25.0, 2.0)),
            Vec3::new(1.0, -25.0, 2.0)
        );
        // An involution, so the sky cube placed at the mirrored eye lands back on the eye.
        let eye = Vec3::new(211.9, 37.7, -368.0);
        assert!((m.transform_point3(m.transform_point3(eye)) - eye).length() < 1e-4);
    }

    /// The mirror alone reverses the winding; with the clip flip the determinant is the main
    /// view's again, which is what lets every draw keep its own cull.
    #[test]
    fn the_clip_flip_cancels_the_mirrors_winding() {
        let mirror = Mirror::DEFAULT.matrix();
        assert!(mirror.determinant() < 0.0);
        assert!((flip_x() * mirror).determinant() > 0.0);
    }

    /// A bilinear lookup with the sampler's clamp-to-edge, as `shadow_sampler` reads the target.
    fn bilinear(image: &[f32], size: (u32, u32), uv: [f32; 2]) -> f32 {
        let texel = |x: i64, y: i64| {
            let x = x.clamp(0, i64::from(size.0) - 1) as usize;
            let y = y.clamp(0, i64::from(size.1) - 1) as usize;
            image[y * size.0 as usize + x]
        };
        let u = uv[0].clamp(0.0, 1.0) * size.0 as f32 - 0.5;
        let v = uv[1].clamp(0.0, 1.0) * size.1 as f32 - 0.5;
        let (x0, y0) = (u.floor(), v.floor());
        let (fx, fy) = (u - x0, v - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = texel(x0, y0) * (1.0 - fx) + texel(x0 + 1, y0) * fx;
        let bottom = texel(x0, y0 + 1) * (1.0 - fx) + texel(x0 + 1, y0 + 1) * fx;
        top * (1.0 - fy) + bottom * fy
    }

    /// What `refraction.wesl` and `shade.wesl` do with a clip-space point and the target.
    fn shader_uv(clip: oag_core::math::Vec4, limit: [f32; 2]) -> [f32; 2] {
        let (x, y) = (clip.x / clip.w.max(1e-6), clip.y / clip.w.max(1e-6));
        [(0.5 + 0.5 * x).min(limit[0]), (0.5 - 0.5 * y).min(limit[1])]
    }

    /// The sub-viewport reads the same texels a quarter-size allocation would: the rectangle's
    /// mapping lands a lookup on the same texel centres, and the clamp half a texel inside it
    /// stands where the smaller texture's own edge clamp stood, so the junk outside the rectangle
    /// (a frame drawn larger before, or the clear) is never reached, whatever the point.
    #[test]
    fn a_quarter_rectangle_reads_what_a_quarter_size_target_holds() {
        use oag_display::display::ReflectionResolution;
        for original in [SIZE, REFLECTION_SIZE, (480, 272)] {
            for step in ReflectionResolution::STEPS {
                let small = step.size(original);
                let pattern = |x: u32, y: u32| ((x * 7919 + y * 104_729) % 251) as f32 / 251.0;
                let own: Vec<f32> = (0..small.1)
                    .flat_map(|y| (0..small.0).map(move |x| pattern(x, y)))
                    .collect();
                let full: Vec<f32> = (0..original.1)
                    .flat_map(|y| {
                        (0..original.0).map(move |x| {
                            if x < small.0 && y < small.1 {
                                pattern(x, y)
                            } else {
                                1000.0
                            }
                        })
                    })
                    .collect();
                let mapping = rect_mapping(small, original);
                let limit = uv_limit(small, original);
                for i in 0..2000u32 {
                    // Points across and well past the target's view, at several depths.
                    let x = (i % 50) as f32 / 49.0 * 3.2 - 1.6;
                    let y = (i / 50) as f32 / 39.0 * 3.2 - 1.6;
                    let w = 0.5 + (i % 7) as f32;
                    let clip = oag_core::math::Vec4::new(x * w, y * w, 0.3, w);
                    let want = bilinear(&own, small, shader_uv(clip, [1.0, 1.0]));
                    let got = bilinear(&full, original, shader_uv(mapping * clip, limit));
                    assert!(
                        (want - got).abs() < 1e-3,
                        "{step} of {original:?} at clip ({x}, {y}): {got} against {want}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_rectangle_mapping_is_the_identity_at_the_originals_size() {
        assert_eq!(rect_mapping(SIZE, SIZE), Mat4::IDENTITY);
        assert_eq!(uv_limit(SIZE, SIZE), [1.0, 1.0]);
    }

    /// The reflect job doubles the sun colour and the prelit scale and nothing else.
    #[test]
    fn the_reflected_scene_doubles_the_sun_and_the_prelit_scale_only() {
        let mut scene = mesh_render::Scene::off();
        scene.light.sun = [4.0, 3.843_137, 3.309_804];
        scene.light.prelit_scale = [1.0, 0.5, 0.25];
        scene.light.ambient = [0.8, 0.827_451, 0.831_373];
        scene.light.specular_scale = 2.0;
        scene.light.output_scale = 0.5;
        let out = reflected_scene(&scene);
        assert_eq!(out.light.sun, [8.0, 7.686_274, 6.619_608]);
        assert_eq!(out.light.prelit_scale, [2.0, 1.0, 0.5]);
        assert_eq!(out.light.ambient, scene.light.ambient);
        assert_eq!(out.light.specular_scale, 2.0);
        assert_eq!(out.light.output_scale, 0.5);
    }
}
