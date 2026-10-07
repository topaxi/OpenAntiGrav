//! Wipeout HD's behind-the-glass target: the picture Vineta K's tunnel glass
//! shows.
//!
//! **What the original draws, read off Vineta K's RPCS3 captures** (each draw
//! tied to its chunk by vertex offset; `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`
//! and `docs/formats/rcsmaterial.md`, 2026-10-07, confidence 88): before the
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

use super::*;

/// The original's target, in pixels: measured, and kept at that size whatever
/// the window's, so the glass's picture is as soft as the original's at its own
/// 1280x720 and no sharper at a larger one.
pub(super) const SIZE: (u32, u32) = (640, 360);

/// How much the target's projection scales the main view's clip `x` and `y`:
/// `c[256]`'s `sy` is `1.29923 = 0.75 * 1.7323`, the main view's, measured.
const NARROWING: f32 = 0.75;

/// What a load hands the scene for the target: the chunks, split by fog, and
/// the circuit's alternate fog pair.
#[derive(Debug, Default)]
pub struct BehindGlassModels {
    /// The two fog groups, both `None` on every circuit but Vineta K.
    pub groups: oag_mesh::mesh::rcs::BehindGlass,
    /// `Fog.Alternate Fog Color` / `Alternate Fog Density`, read only when a
    /// group exists.
    pub alternate_fog: Option<mesh_render::Fog>,
}

/// The target and what draws into it.
#[derive(Debug)]
pub(super) struct Target {
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
}

/// The target's colour texture: created ahead of the drawables, because the
/// circuit's own drawable binds it at build. `None` when there is nothing to
/// draw into it.
fn colour_texture(
    device: &wgpu::Device,
    models: &BehindGlassModels,
    format: wgpu::TextureFormat,
) -> Option<wgpu::Texture> {
    if models.groups.is_empty() {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("behind the glass"),
        size: extent(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    Some(texture)
}

fn extent() -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: SIZE.0,
        height: SIZE.1,
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

    /// Builds the drawables that fill `texture`.
    #[allow(clippy::too_many_arguments)]
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: wgpu::Texture,
        models: BehindGlassModels,
        sky: Option<Model>,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        zone_art: &mesh_render::zone::StageArt,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Self> {
        let target = |label: &str, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: extent(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let depth = target("behind the glass depth", mesh_render::DEPTH_FORMAT);
        let velocity = target(
            "behind the glass velocity",
            oag_gpu::formats::VELOCITY_FORMAT,
        );
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
            )
        };
        let scenery = |model| {
            drawable(
                model,
                mesh_render::Depth::Scene,
                mesh_render::ShadowReceiver::Both,
                true,
            )
        };
        let BehindGlassModels {
            groups,
            alternate_fog,
        } = models;
        Ok(Self {
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
        })
    }

    /// Writes every drawable's uniforms for this frame: `matrix` is
    /// [`view_projection`] of the main view's, `scene` the circuit's own scene
    /// uniform (whose fog the alternate group swaps for its pair), `seconds`
    /// the scenery clock.
    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
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
        for (drawable, scene) in [(&self.alternate, &alternate), (&self.primary, scene)] {
            let Some(drawable) = drawable else {
                continue;
            };
            drawable.write(queue, matrix, Mat4::IDENTITY, matrix);
            drawable.write_anims(queue, seconds);
            drawable.write_node_anims(queue, seconds);
            queue.write_buffer(&drawable.fog, 0, bytemuck::bytes_of(scene));
        }
    }

    /// Draws the target: its chunks' solid lists, the sky, then their blended
    /// lists - the main view's own order - culled by the main view's own
    /// visibility, as the original culls them.
    pub(super) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
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
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let groups = || self.alternate.iter().chain(&self.primary);
        let mut stats = SceneStats::default();
        let solid = Lists::Solid { order: None };
        for group in groups() {
            stats.add(group.draw_lists(&mut pass, None, set, chunks, frustum, solid));
        }
        if let Some(sky) = &self.sky {
            let (width, height) = (SIZE.0 as f32, SIZE.1 as f32);
            pass.set_viewport(0.0, 0.0, width, height, 1.0, 1.0);
            let _ = sky.draw(&mut pass, None, None, None, None);
            pass.set_viewport(0.0, 0.0, width, height, 0.0, 1.0);
        }
        for group in groups() {
            stats.add(group.draw_lists(&mut pass, None, set, chunks, frustum, Lists::Blended));
        }
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
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        scene: &mesh_render::Scene,
        (matrix, eye, seconds): (Mat4, Vec3, f32),
        set: Option<&VisibleSet>,
        chunks: Option<&ChunkSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let Some(target) = &self.behind_glass else {
            return SceneStats::default();
        };
        target.write(queue, view_projection(matrix), eye, scene, seconds);
        target.render(encoder, set, chunks, frustum)
    }
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
        let texel = target.texture.format().block_copy_size(None).unwrap_or(4);
        let row = (SIZE.0 * texel).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("behind the glass readback"),
            size: u64::from(row * SIZE.1),
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
                    rows_per_image: Some(SIZE.1),
                },
            },
            extent(),
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let mapped = slice.get_mapped_range()?;
        let mut rgba = Vec::with_capacity((SIZE.0 * SIZE.1 * 4) as usize);
        for line in mapped.chunks(row as usize).take(SIZE.1 as usize) {
            for pixel in line[..(SIZE.0 * texel) as usize].chunks(texel as usize) {
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
        std::fs::write(&dump, oag_texture::png::encode_rgba(SIZE.0, SIZE.1, &rgba))
            .map_err(|error| anyhow::anyhow!("writing {dump}: {error}"))?;
        println!("wrote {dump} (behind the glass, {}x{})", SIZE.0, SIZE.1);
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
