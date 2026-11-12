//! A model whose vertex data outgrows the device's `max_buffer_size` draws the same
//! pixels as one buffer does, through every path that writes or draws vertices
//! (`oag_mesh::mesh_render::Geometry`).
//!
//! The ceiling is lowered on the test device rather than the model raised: the code under
//! test reads `Device::limits`, so a device that is small is the same case as a model that
//! is large, at a size a test can afford. Omega's Rockway Stadium and Unity Square, the two
//! circuits that overran 256 MiB, are
//! `oag-mesh`'s `omega_vertex_chunks_ground_truth`.

use super::bindless::{drawable_of, pixels, pixels_with, strips};
use super::scene_build::{test_gpu, test_gpu_with_buffer_ceiling};
use super::*;

/// Above every uniform and table buffer a drawable makes, below its vertex data.
const CEILING: u64 = 64 * 1024;

fn vertices_per_buffer() -> u32 {
    (CEILING / std::mem::size_of::<mesh::GpuVertex>() as u64) as u32
}

/// `strips(half)` with `gap` vertices no draw uses between one strip's and the next, so the
/// strips sit in different buffers once the ceiling is low and the model is cut between them.
fn spread(half: usize, gap: u32) -> Model {
    let mut model = strips(half);
    let blank = model.vertices[0];
    let mut vertices = Vec::new();
    let mut moved = Vec::new();
    for quad in model.vertices.chunks(4) {
        moved.extend((0..quad.len() as u32).map(|i| vertices.len() as u32 + i));
        vertices.extend_from_slice(quad);
        vertices.extend((0..gap).map(|_| blank));
    }
    for index in &mut model.indices {
        *index = moved[*index as usize];
    }
    model.vertices = vertices;
    model
}

/// A drawable of the spread strips, each strip more than half a buffer from its neighbour.
fn spread_drawable(device: &wgpu::Device, queue: &wgpu::Queue, half: usize) -> Drawable {
    drawable_of(device, queue, spread(half, vertices_per_buffer() / 2), false)
}

fn pair() -> Option<((wgpu::Device, wgpu::Queue), (wgpu::Device, wgpu::Queue))> {
    Some((test_gpu()?, test_gpu_with_buffer_ceiling(CEILING)?))
}

/// **Every draw lands in its own buffer with its own base vertex.** Drawn on the device
/// that holds the strips in one buffer and on the one that cuts them in four: every pixel
/// equal, and the strips distinct, so a draw read from another chunk's vertices (a base
/// vertex missing, a chunk left bound from the draw before) changes a strip.
#[test]
fn a_model_cut_into_several_buffers_draws_the_pixels_one_buffer_does() {
    let Some(((whole_device, whole_queue), (cut_device, cut_queue))) = pair() else {
        return;
    };
    let whole = [0, 1].map(|half| spread_drawable(&whole_device, &whole_queue, half));
    let cut = [0, 1].map(|half| spread_drawable(&cut_device, &cut_queue, half));
    assert!(whole.iter().all(|d| d.vertex_buffer_count() == 1));
    assert!(
        cut.iter().all(|d| d.vertex_buffer_count() >= 4),
        "eight strips each over half a buffer from the next: {:?}",
        cut.iter().map(Drawable::vertex_buffer_count).collect::<Vec<_>>()
    );
    let expected = pixels(&whole_device, &whole_queue, &whole);
    let got = pixels(&cut_device, &cut_queue, &cut);
    let distinct: std::collections::HashSet<_> = expected.chunks(4).collect();
    assert!(distinct.len() > 8, "the strips differ: {}", distinct.len());
    assert!(got == expected, "every pixel as one buffer draws it");
}

/// **A write that spans buffers is split, not dropped or wrapped.** `tint` rewrites every
/// vertex's colour in one call; on the cut model that is four writes, and the picture is the
/// one-buffer picture tinted.
#[test]
fn a_whole_model_write_reaches_every_buffer_it_spans() {
    let Some(((whole_device, whole_queue), (cut_device, cut_queue))) = pair() else {
        return;
    };
    let tinted = |device: &wgpu::Device, queue: &wgpu::Queue| {
        let drawables = [0, 1].map(|half| spread_drawable(device, queue, half));
        let mut scratch = Vec::new();
        for drawable in &drawables {
            drawable.tint(queue, [0.25, 0.5, 1.0, 1.0], &mut scratch);
        }
        pixels(device, queue, &drawables)
    };
    let untinted = pixels(
        &cut_device,
        &cut_queue,
        &[0, 1].map(|half| spread_drawable(&cut_device, &cut_queue, half)),
    );
    let expected = tinted(&whole_device, &whole_queue);
    let got = tinted(&cut_device, &cut_queue);
    assert!(expected != untinted, "the tint changes the picture");
    assert!(got == expected, "every pixel as one buffer draws it");
}

/// **The overlay list walks the chunks too.**
#[test]
fn the_overlay_list_draws_the_same_pixels_from_several_buffers() {
    let Some(((whole_device, whole_queue), (cut_device, cut_queue))) = pair() else {
        return;
    };
    let overlay = |drawable: &Drawable, pass: &mut wgpu::RenderPass<'_>| {
        drawable.draw_overlay(pass, ViewBlock::NONE);
    };
    let render = |device: &wgpu::Device, queue: &wgpu::Queue| {
        let drawables = [0, 1].map(|half| spread_drawable(device, queue, half));
        pixels_with(device, queue, &drawables, overlay)
    };
    assert!(render(&cut_device, &cut_queue) == render(&whole_device, &whole_queue));
}

/// A model whose draw reaches across every cut that fits is refused by name, not drawn wrong.
#[test]
fn a_draw_wider_than_the_ceiling_is_a_load_error() {
    let Some((device, queue)) = test_gpu_with_buffer_ceiling(CEILING) else {
        return;
    };
    let mut model = spread(0, vertices_per_buffer());
    model.draws[0].range = 0..model.indices.len() as u32;
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Drawable::new_with(
            &device,
            &queue,
            model,
            wgpu::TextureFormat::Rgba8Unorm,
            oag_gpu::formats::SceneStore::Computed,
            Anisotropy::Off,
            1,
            mesh_render::Depth::Scene,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
            &mesh_render::zone::StageArt::NONE,
            mesh_render::ShadowMaps::NONE,
            mesh_render::ShadowReceiver::Never,
            mesh_render::Texcoords::Interleaved,
            false,
            false,
            mesh_render::Velocity::None,
            false,
        )
    }));
    let error = built.expect("an error, not a panic").expect_err("refused");
    assert!(error.to_string().contains("straddle"), "{error}");
}
