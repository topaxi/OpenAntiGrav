//! The cockpit hit static drawn into a small target: noise strong at the view's
//! side edges, gone before the middle, nothing without a run.
//!
//! Hand-laid noise, no disc: what is asserted is the pipeline (the repeat
//! sampler, the gradient from the vertex colours, the alpha-over blend, the
//! clip-space mapping), not the disc's own `noisematt.mip`, which
//! `crates/game/tests/cockpit_hit_ground_truth.rs` draws through a race.
//!
//! **Skips when there is no adapter.**

use oag_core::Rng;
use oag_fx::cockpit_hit::{self, Node, Pipeline};
use oag_fx::exhaust::FlareTexture;
use oag_gpu::formats::SceneStore;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity};

/// Four 32-texel rows make 128 bytes; 64 texels are the copy's 256-byte row.
const WIDTH: u32 = 64;
const HEIGHT: u32 = 16;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// What the target holds behind the static, per channel.
const BEHIND: u8 = 128;

fn noise() -> FlareTexture {
    let mut rng = Rng::new(7);
    let rgba = (0..cockpit_hit::TEXELS * cockpit_hit::TEXELS)
        .flat_map(|_| {
            let grey = rng.below(256) as u8;
            [grey, grey, grey, 255]
        })
        .collect();
    FlareTexture {
        width: cockpit_hit::TEXELS,
        height: cockpit_hit::TEXELS,
        rgba,
    }
}

/// The node `steps` steps into a run, so `p = steps / 36`.
fn node_at(steps: u32) -> Node {
    let mut node = Node::default();
    let mut rng = Rng::new(3);
    node.arm();
    for _ in 0..=steps {
        node.advance(1.0 / 60.0, &mut rng);
    }
    node
}

/// A target cleared to [`BEHIND`], the static drawn over it, and its texels
/// row by row, or `None` with no adapter.
fn render(node: &Node) -> Option<Vec<[u8; 4]>> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| panic!("wgpu validation: {e}")));
    let mut pipeline = Pipeline::new(&device, FORMAT, SceneStore::Computed, 1, Velocity::None);
    assert!(pipeline.sync_texture(&queue, &noise()));
    pipeline.upload(&queue, node.strips());
    let size = wgpu::Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    };
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cockpit hit"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = target(
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let grey = f64::from(BEHIND) / 255.0;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cockpit hit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: grey,
                        g: grey,
                        b: grey,
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
        pipeline.draw(&mut pass, pipeline.drawn());
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cockpit hit readback"),
        size: u64::from(WIDTH * 4 * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: None,
            },
        },
        size,
    );
    queue.submit([encoder.finish()]);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.expect("mapped"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("the readback completes");
    let bytes = readback
        .slice(..)
        .get_mapped_range()
        .expect("mapped")
        .to_vec();
    Some(bytes.as_chunks::<4>().0.to_vec())
}

/// How far column `x`'s red strays from the background: the mean of
/// `|texel - behind|` down its rows.
fn deviation(frame: &[[u8; 4]], x: u32) -> f32 {
    let total: u32 = (0..HEIGHT)
        .map(|y| u32::from(frame[(y * WIDTH + x) as usize][0].abs_diff(BEHIND)))
        .sum();
    total as f32 / HEIGHT as f32
}

/// Half way through a run each strip is 144 of the PSP's 480 pixels, a 0.3 share
/// of the view: noise at the edges, nothing in the middle.
#[test]
fn the_static_is_noise_at_the_edges_and_clear_in_the_middle() {
    let node = node_at(18);
    assert!((node.progress() - 0.5).abs() < 1e-3);
    let Some(frame) = render(&node) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (left, right) = (deviation(&frame, 0), deviation(&frame, WIDTH - 1));
    assert!(left > 15.0, "the left edge is not noisy: {left}");
    assert!(right > 15.0, "the right edge is not noisy: {right}");
    assert_eq!(deviation(&frame, WIDTH / 2), 0.0, "the middle is touched");
    assert_eq!(deviation(&frame, WIDTH / 2 - 8), 0.0);
    // A gradient: the noise is strongest at the screen's edge and fades inwards.
    let inward: Vec<f32> = [0, 4, 8, 12, 16].map(|x| deviation(&frame, x)).to_vec();
    assert!(
        inward.windows(2).all(|pair| pair[0] >= pair[1]) && inward[0] > 2.0 * inward[3],
        "the strip does not fade inwards: {inward:?}"
    );
}

/// The same pixels are what a run with nothing to show leaves untouched.
#[test]
fn no_run_draws_nothing() {
    let Some(frame) = render(&Node::default()) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        frame.iter().all(|texel| texel[..3] == [BEHIND; 3]),
        "a node that never ran drew"
    );
}
