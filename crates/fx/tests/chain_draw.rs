//! An effect texture drawn down its own mip chain at a title's sampler bias, read back.
//!
//! Wipeout 2048's loader gives every texture trilinear filtering and a bias of -1.5
//! (`docs/ghidra/functions/vita-2048-eu-v104/gxt-loader.md`). Each level here is one flat
//! colour, so the pixel says which level the draw read: 64 texels over 8 pixels is level 3
//! on the sampler's own selection and level 1.5 biased, half red and half green.
//!
//! **Skips when there is no adapter.**

use oag_core::math::camera::perspective;
use oag_fx::psys::Pipeline as Particles;
use oag_fx::psys::sprite::{Sheet, Sprite};
use oag_fx::weapon_quads::{FlareTexture, Pipeline as Quads};
use oag_gpu::formats::SceneStore;
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

const SIZE: u32 = 8;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIDE: u32 = 64;

/// The base white, then red, green, blue, red, green, blue.
fn levels() -> (Vec<u8>, Vec<Vec<u8>>) {
    let flat =
        |side: u32, rgb: [u8; 3]| [rgb[0], rgb[1], rgb[2], 255].repeat((side * side) as usize);
    let colours = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];
    let lower = (1..=6)
        .map(|level| flat(SIDE >> level, colours[(level - 1) % 3]))
        .collect();
    (flat(SIDE, [255, 255, 255]), lower)
}

fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = oag_gpu::test_adapter::request(&instance)?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    Some((device, queue))
}

/// A quad over the whole screen carrying `uv` as `[u0, v0, u1, v1]`, `w` away.
fn quad(uv: [f32; 4]) -> Vec<GpuVertex> {
    let w = 10.0;
    let corner = |x: f32, y: f32| GpuVertex {
        position: [x * w, y * w, -w],
        normal: [1.0, 0.0, 0.0],
        colour: [1.0; 4],
        texcoord: [
            if x < 0.0 { uv[0] } else { uv[2] },
            if y > 0.0 { uv[1] } else { uv[3] },
        ],
        ..bytemuck::Zeroable::zeroed()
    };
    vec![
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(-1.0, 1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ]
}

fn projection() -> [[f32; 4]; 4] {
    perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0).to_cols_array_2d()
}

/// The centre pixel after `draw` ran over black.
fn centre(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    draw: impl FnOnce(&mut wgpu::RenderPass<'_>),
) -> [u8; 3] {
    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let texture = |label: &str, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
            texture_binding_view_dimension: None,
        })
    };
    let target = texture(
        "target",
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        "depth",
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let view = target.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("chain draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        draw(&mut pass);
    }
    let row = 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = buffer.slice(..).get_mapped_range().expect("range").to_vec();
    let at = (SIZE / 2 * row + SIZE / 2 * 4) as usize;
    [bytes[at], bytes[at + 1], bytes[at + 2]]
}

fn assert_levels_one_and_two(pixel: [u8; 3], what: &str) {
    let [red, green, blue] = pixel;
    assert!(
        (100..=156).contains(&red) && (100..=156).contains(&green) && blue < 20,
        "{what}: level 1.5 is half red, half green, read {pixel:?}"
    );
}

#[test]
fn a_biased_particle_sheet_reads_the_sprites_own_levels() {
    let Some((device, queue)) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (base, lower) = levels();
    let sprite = Sprite {
        width: SIDE as u16,
        height: SIDE as u16,
        rgba: base.into(),
        chain: Some(lower.into_iter().map(Into::into).collect()),
    };
    let draw = |bias: f32| {
        let mut pipeline = Particles::with_lod_bias(
            &device,
            FORMAT,
            SceneStore::Computed,
            1,
            Velocity::None,
            bias,
        );
        let mut sheet = Sheet::default();
        let rect = sheet.place(&sprite).expect("the sprite fits");
        pipeline.sync_sheet(&queue, &sheet);
        pipeline.upload(&queue, &projection(), 1.0, &[], &quad(rect));
        centre(&device, &queue, |pass| {
            pipeline.draw(pass, pipeline.drawn())
        })
    };
    assert_levels_one_and_two(draw(-1.5), "particles");
    assert_eq!(draw(0.0), [255; 3], "no bias, one level: the base");
}

#[test]
fn a_biased_effect_texture_reads_its_own_levels() {
    let Some((device, queue)) = gpu() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (base, lower) = levels();
    let draw = |lod_bias: f32| {
        let texture = FlareTexture {
            width: SIDE,
            height: SIDE,
            rgba: base.clone(),
            mips: lower.clone(),
            lod_bias,
        };
        let mut pipeline = Quads::new(
            &device,
            &queue,
            FORMAT,
            SceneStore::Computed,
            &texture,
            &texture,
            1,
            Velocity::None,
        );
        pipeline.upload(&queue, &projection(), &quad([0.0, 0.0, 1.0, 1.0]), &[]);
        centre(&device, &queue, |pass| {
            pipeline.draw(pass, pipeline.drawn())
        })
    };
    assert_levels_one_and_two(draw(-1.5), "weapon quads");
    assert_eq!(draw(0.0), [255; 3], "no bias: the base alone is bound");
}
