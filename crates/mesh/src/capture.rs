//! Headless capture: renders a [`Model`] offscreen and reads the frame back,
//! for the asset viewer's `--screenshot` and for tests that need a picture
//! without a window.
//!
//! Split out of `mesh_render` on 2026-08-17 so a ground-truth test could add
//! a pixel-returning entry point without pushing that module past its
//! frozen file-size ceiling - see `scripts/check-file-size.py`.

use anyhow::{Context, Result};
use std::path::Path;

use crate::mesh::Model;
use crate::mesh_render::{
    Anisotropy, Built, DEPTH_FORMAT, Depth, GlowMask, NodeAnims, ShadowMaps, TRANSPARENT_BLEND,
    TexAnims, UNIFORMS_SIZE, Velocity, build, write_uniforms,
};
use crate::orbit::Orbit;

/// Makes every pixel opaque, in place, before a frame is encoded as a PNG.
///
/// **The alpha channel of a rendered frame is not coverage; it is the bloom
/// mask.** `mesh_render::GlowMask::Protected` binds `ColorWrites::COLOR`, so
/// such geometry leaves the channel at the pass's clear - zero - while a
/// `Stamped` model (every `.vex` model of a Pulse PSP race) writes its batch's
/// stamp, `4` under an ordinary surface, and a `Written` one its own alpha.
/// `post::bloom`'s `fs_bright` reads exactly that (`texel.rgb * texel.a`). The
/// window never presents the channel, so nothing on screen depends on it.
///
/// A PNG does present it. Encoding a race frame straight from the readback
/// buffer therefore writes a **fully transparent image**: correct bytes, and
/// every viewer shows nothing. This is the one line that turns a frame into a
/// picture of a frame.
///
/// Takes `[r, g, b, a]` rows as `capture_pixels_from` returns them; a length
/// that is not a multiple of four leaves its tail alone rather than panicking,
/// because a truncated readback is already a reported error elsewhere.
pub fn make_opaque(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel[3] = 0xff;
    }
}

/// The environment variable that asks [`dump_glow_mask_if_asked`] for a PNG
/// of a frame's glow mask, and names the path.
pub const DUMP_GLOW_MASK_VAR: &str = "OAG_DUMP_GLOW_MASK";

/// Writes the frame's alpha channel - the bloom's glow mask - as a greyscale
/// PNG when [`DUMP_GLOW_MASK_VAR`] names a path, before [`make_opaque`]
/// erases it. The comparison `docs/rendering/glow-mask.md` makes against the
/// original's EDRAM needs exactly this channel.
///
/// # Errors
///
/// A path that cannot be written.
pub fn dump_glow_mask_if_asked(width: u32, height: u32, pixels: &[u8]) -> anyhow::Result<()> {
    let Some(path) = std::env::var_os(DUMP_GLOW_MASK_VAR) else {
        return Ok(());
    };
    let grey: Vec<u8> = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[3], p[3], p[3], 0xff])
        .collect();
    std::fs::write(&path, oag_texture::png::encode_rgba(width, height, &grey))?;
    println!(
        "wrote the glow mask to {}",
        std::path::Path::new(&path).display()
    );
    Ok(())
}

/// Renders one frame of `model` to a PNG from a given orbit angle.
///
/// `pitch` near zero looks along the ground; near `PI / 2` looks straight down,
/// which is what a track wants and a model does not.
///
/// `seconds` is where on their authored loops the model's texture-transform
/// tracks are sampled - `0.0` for the first frame of every one of them. It is
/// a parameter rather than a fixed zero because this is the **only** headless
/// way to see an animated surface move: two captures at two times, differenced.
#[allow(clippy::too_many_arguments)]
pub fn capture_from(
    model: &Model,
    path: &Path,
    width: u32,
    height: u32,
    yaw: f32,
    pitch: f32,
    anisotropy: Anisotropy,
    seconds: f32,
) -> Result<()> {
    let pixels = capture_pixels_from(model, width, height, yaw, pitch, anisotropy, seconds)?;
    std::fs::write(path, oag_texture::png::encode_rgba(width, height, &pixels))
        .with_context(|| format!("writing {}", path.display()))
}

/// [`capture_from`], stopped short of the PNG encode: raw `[r, g, b, a]`
/// rows, row-major, top to bottom - what a test asserts on without a decoder
/// this crate does not otherwise need.
#[allow(clippy::too_many_arguments)]
pub fn capture_pixels_from(
    model: &Model,
    width: u32,
    height: u32,
    yaw: f32,
    pitch: f32,
    anisotropy: Anisotropy,
    seconds: f32,
) -> Result<Vec<u8>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .context("no GPU adapter available")?;
    let (device, queue) = pollster::block_on(adapter.request_device(
        &crate::mesh_render::device_descriptor("oag-view mesh", &adapter),
    ))
    .context("requesting the device")?;

    // **Raw, because the shader now writes gamma-space values and nothing may
    // encode them a second time.** See
    // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
    //
    // This was `Rgba8UnormSrgb`, and the reasoning that put it there is
    // superseded rather than wrong: given a texture upload that *linearised*,
    // `mesh.wesl` multiplied linear light by the light rig and only an
    // encode-on-write target matched the window. That premise is gone - the
    // upload below is raw now - so the multiply happens in gamma space, the
    // window no longer encodes either, and an encoding target here would be the
    // double-encode. The old measurement (mean ~2.4/255, up to 57/255 against
    // the window) is exactly the error a *partial* migration reintroduces,
    // which is why the format here and the format of `make`'s upload move
    // together or not at all.
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let Built {
        pipeline,
        alpha_test_pipeline,
        cutout_pipelines,
        blend_pipeline,
        additive_pipeline,
        unblended_pipeline,
        authored_pipelines,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
        fog_bind,
        fog_buffer: _,
        zone_vis_texture: _,
        zone_rebind: _,
        anim_bind,
        anim_buffer,
        node_anim_buffer,
        stamp_pipeline: _,
        prepass: _,
    } = build(
        &device,
        &queue,
        model,
        format,
        anisotropy,
        1,
        Depth::Scene,
        TRANSPARENT_BLEND,
        // The offscreen helper has no bloom behind it, so the mask is moot,
        // and it draws alone into one target, so there is no velocity buffer.
        GlowMask::Protected,
        Velocity::None,
        // No Zone stage: the offscreen helper draws one model, not a race.
        &crate::mesh_render::zone::StageArt::NONE,
        // And no shadow map, and nothing receiving one: this helper draws a
        // single model against nothing, which is what makes it a comparison.
        ShadowMaps::NONE,
        crate::mesh_render::ShadowReceiver::Never,
    )?;

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    write_uniforms(
        &queue,
        &uniform_buffer,
        model,
        width as f32 / height as f32,
        // The capture path frames the whole model from its centre; panning is
        // an interactive affordance and there is no cursor here.
        Orbit {
            yaw,
            pitch,
            ..Orbit::default()
        },
    );

    queue.write_buffer(
        &anim_buffer,
        0,
        bytemuck::bytes_of(&TexAnims::sample(model, seconds)),
    );
    // The same clock moves the scenery. A capture of a track at `seconds` has
    // to place its `Anim Transform` nodes at `seconds` too, or the still is of
    // a circuit that never existed.
    queue.write_buffer(
        &node_anim_buffer,
        0,
        bytemuck::bytes_of(&NodeAnims::sample(model, seconds)),
    );

    // The bind group must reference the buffer we just filled.
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = {
        let _ = bind_group;
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        })
    };

    let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("mesh"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mesh"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.06,
                        g: 0.07,
                        b: 0.09,
                        a: 1.0,
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
        // `Buffer::slice` panics on a zero-length buffer, so a model with no
        // geometry has to skip every command below rather than binding one. The
        // pass still runs, so the clear above lands and a valid frame comes out;
        // deciding whether an empty model is worth capturing at all belongs to
        // the caller, not here.
        if !model.vertices.is_empty() && !model.indices.is_empty() {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Group 2 never changes within a pass and survives `set_pipeline`, so it
            // is bound once here rather than per draw call. The capture path leaves
            // it at `Fog::off`.
            pass.set_bind_group(2, &fog_bind, &[]);
            pass.set_bind_group(3, &anim_bind, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // One draw per material run. Slot 0 is the white fallback, so a texture
            // index of n binds slot n + 1 - and an index past the end **falls back
            // to slot 0** rather than to the last bind.
            //
            // Clamping to `len - 1` is what this did until finding R2 of the
            // 2026-08-18 review, and it painted an inconsistent model with
            // whichever texture happened to be last instead of with the
            // documented, visible white. An arbitrary texture reads as a
            // deliberate one; white reads as "this slot is missing", which is
            // the honest answer and the one every other path here gives.
            let bind = |draw: &crate::mesh::DrawCall| {
                let slot = draw.texture.map_or(0, |t| t + 1);
                &texture_binds[if slot < texture_binds.len() { slot } else { 0 }]
            };

            for draw in &model.draws {
                pass.set_bind_group(1, bind(draw), &[]);
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
            }

            // Second pipeline group, same pass: `Model::alpha_tested_draws`
            // wants a cutout, not a hardcoded alpha of 1.0 - see
            // `fs_main_alpha_test`.
            //
            // **One `set_pipeline` per distinct reference, because the batch's
            // own flag bits pick it** - a `.vex` batch authors `0x10` or
            // `0x7f` and a circuit mixes them, exactly as the transparent
            // classes below do. Switched only when it changes, so a model with
            // one reference (or none, which is every PS3 model) pays for a
            // single call as before. See `mesh_render::cutout`.
            let cutouts = crate::mesh_render::CutoutPipelines {
                default: &alpha_test_pipeline,
                by_reference: &cutout_pipelines,
            };
            let mut current: Option<&wgpu::RenderPipeline> = None;
            for draw in &model.alpha_tested_draws {
                let pipeline = cutouts.select(draw);
                if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                    pass.set_pipeline(pipeline);
                    current = Some(pipeline);
                }
                pass.set_bind_group(1, bind(draw), &[]);
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
            }

            // Third pipeline group, same pass: `Model::transparent_draws`,
            // blended rather than replaced and drawn last so opaque and cutout
            // depth is already resolved. See `crate::exhaust::Pipeline` for the
            // precedent of pairing an opaque and a blended pipeline this way.
            //
            // **One `set_pipeline` per draw call, because the batch's own
            // `pass_mask` picks its blend equation** - the `0x0700` class
            // splits three ways and a single model mixes them. Grouping the
            // draws by class first would reorder them, and transparent draws
            // are order-dependent by definition.
            let pipelines = crate::mesh_render::TransparentPipelines {
                alpha_over: &blend_pipeline,
                additive: &additive_pipeline,
                unblended: &unblended_pipeline,
                authored: &authored_pipelines,
            };
            let mut current: Option<&wgpu::RenderPipeline> = None;
            for draw in &model.transparent_draws {
                let pipeline = pipelines.select(draw);
                if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                    pass.set_pipeline(pipeline);
                    current = Some(pipeline);
                }
                pass.set_bind_group(1, bind(draw), &[]);
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
            }
        }
    }
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;
    let mapped = slice.get_mapped_range().context("mapping readback")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();

    Ok(pixels)
}
