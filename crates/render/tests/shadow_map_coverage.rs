//! Draws one caster into the shadow map, reads the map back, and asserts what
//! landed where.
//!
//! **The pass has no other observable.** A shadow map is consumed by a sampler
//! inside another shader, so a caster pass that draws nothing, draws in the
//! wrong place, or leaves the previous frame's coverage behind all produce a
//! frame that still renders - which is how the two bugs in the `original`
//! tier's Pulse half got as far as they did. Reading the texels is the check
//! that cannot be fooled that way.
//!
//! Skips when there is no adapter, like `velocity_target.rs`.

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_render::shadow::map::{Caster, DEPTH_SIZE, Fit, Map, SIZE};

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 1.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

/// A quad in the `y = 0` plane, two units across, centred on the origin: a
/// caster the light looks straight down at.
fn quad() -> ([GpuVertex; 4], [u32; 6]) {
    (
        [
            vertex([-1.0, 0.0, -1.0]),
            vertex([1.0, 0.0, -1.0]),
            vertex([1.0, 0.0, 1.0]),
            vertex([-1.0, 0.0, 1.0]),
        ],
        [0, 1, 2, 0, 2, 3],
    )
}

/// Reads the map back as one byte per texel.
fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, map: &Map) -> Vec<u8> {
    let row = SIZE.div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow map readback"),
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        map.texture().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let view = slice.get_mapped_range().expect("mapping");
    let mut out = vec![0u8; (SIZE * SIZE) as usize];
    for y in 0..SIZE as usize {
        let start = y * row as usize;
        out[y * SIZE as usize..(y + 1) * SIZE as usize]
            .copy_from_slice(&view[start..start + SIZE as usize]);
    }
    drop(view);
    buffer.unmap();
    out
}

#[test]
fn a_caster_covers_the_middle_of_the_map_and_leaves_the_border_clear() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut map = Map::new(&device);
    let (vertices, indices) = quad();
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster vertices"),
        size: std::mem::size_of_val(&vertices) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster indices"),
        size: std::mem::size_of_val(&indices) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

    // Straight down at the origin, fitted to twice the caster's own half-width
    // so the quad covers the middle quarter of the map and nothing reaches the
    // border.
    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 2.0,
        towards_light: Vec3::Y,
    };
    // A single range, built as a slice so clippy's `single_range_in_vec_init`
    // does not read it as an indexing mistake: a `Caster` takes the model's
    // own draw-call ranges, and this fixture has one draw call.
    let whole = 0..indices.len() as u32;
    let ranges = std::slice::from_ref(&whole);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(
        &queue,
        &mut encoder,
        &fit,
        &[Caster {
            vertices: &vertex_buffer,
            indices: &index_buffer,
            ranges,
            model: Mat4::IDENTITY,
        }],
    );
    queue.submit([encoder.finish()]);
    let texels = read_back(&device, &queue, &map);

    let at = |x: u32, y: u32| texels[(y * SIZE + x) as usize];
    let middle = SIZE / 2;
    assert_eq!(at(middle, middle), 0xff, "the caster's own centre");
    assert_eq!(map.casters(), 1);
    // A quarter of the way in from a corner is outside a caster that spans the
    // middle half, and the border is what the receiver's clamped sampler
    // reads: both have to be clear, or every surface outside the map's frustum
    // draws shadowed.
    assert_eq!(at(0, 0), 0, "the corner");
    assert_eq!(at(SIZE - 1, SIZE - 1), 0, "the far corner");
    assert_eq!(at(middle, 2), 0, "the top border");

    // And the coverage is the shape it was given: the quad's own edges at a
    // quarter and three quarters across, within a texel of the fit.
    let quarter = SIZE / 4;
    assert_eq!(at(quarter + 2, middle), 0xff, "just inside the left edge");
    assert_eq!(at(quarter - 2, middle), 0, "just outside it");

    // Rendering again with no casters clears it: last frame's coverage must
    // not drag behind a craft that stopped casting.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(&queue, &mut encoder, &fit, &[]);
    queue.submit([encoder.finish()]);
    let cleared = read_back(&device, &queue, &map);
    assert!(cleared.iter().all(|texel| *texel == 0), "the map cleared");
    assert_eq!(map.casters(), 0);
}

/// Reads the depth map back as one `f32` per texel.
fn read_back_depth(device: &wgpu::Device, queue: &wgpu::Queue, map: &Map) -> Vec<f32> {
    let row = (DEPTH_SIZE * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow depth readback"),
        size: u64::from(row * DEPTH_SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        map.depth_texture().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(DEPTH_SIZE),
            },
        },
        wgpu::Extent3d {
            width: DEPTH_SIZE,
            height: DEPTH_SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let view = slice.get_mapped_range().expect("mapping");
    let mut out = vec![0f32; (DEPTH_SIZE * DEPTH_SIZE) as usize];
    for y in 0..DEPTH_SIZE as usize {
        let start = y * row as usize;
        for x in 0..DEPTH_SIZE as usize {
            let at = start + x * 4;
            out[y * DEPTH_SIZE as usize + x] =
                f32::from_le_bytes([view[at], view[at + 1], view[at + 2], view[at + 3]]);
        }
    }
    drop(view);
    buffer.unmap();
    out
}

/// The `mapped` tier's depth pass records how far the caster is, and clears to
/// the far plane everywhere else.
///
/// The same reasoning as the coverage test above: a depth map is read by a
/// comparison inside another shader, so a pass that writes nothing produces a
/// frame with no shadows and no error.
#[test]
fn the_depth_pass_records_the_caster_and_clears_the_rest() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut map = Map::new(&device);
    let (vertices, indices) = quad();
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster vertices"),
        size: std::mem::size_of_val(&vertices) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster indices"),
        size: std::mem::size_of_val(&indices) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));

    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 2.0,
        towards_light: Vec3::Y,
    };
    let whole = 0..indices.len() as u32;
    let ranges = std::slice::from_ref(&whole);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render_depth(
        &queue,
        &mut encoder,
        &fit,
        &[Caster {
            vertices: &vertex_buffer,
            indices: &index_buffer,
            ranges,
            model: Mat4::IDENTITY,
        }],
    );
    queue.submit([encoder.finish()]);
    let depth = read_back_depth(&device, &queue, &map);

    let at = |x: u32, y: u32| depth[(y * DEPTH_SIZE + x) as usize];
    let middle = DEPTH_SIZE / 2;
    assert_eq!(map.depth_casters(), 1);
    // The caster sits half way down the box, so its depth is neither the near
    // plane nor the far one.
    let centre = at(middle, middle);
    assert!(
        (0.05..0.95).contains(&centre),
        "the caster's own depth is {centre}"
    );
    // And everything it does not cover is the far plane, which is what "no
    // shadow" means to the receiver.
    assert_eq!(at(0, 0), 1.0, "the corner");
    assert_eq!(at(middle, 4), 1.0, "the top border");
}

/// The receiver's lookup in `mesh.wgsl` - `uv = (ndc.x * 0.5 + 0.5, 0.5 -
/// ndc.y * 0.5)` - reads the texel the caster pass actually wrote, in both
/// maps.
///
/// **An off-centre caster, deliberately.** The two tests above put the
/// caster at the map's own centre, and a centred caster cannot tell the
/// right lookup from one mirrored about the map's horizontal axis: both read
/// the middle. That mirror shipped on 2026-09-04 on the strength of a claim
/// that glam's `rh::proj::directx` projections are Y-down (they are Y-up,
/// with a `0..1` depth - glam's own `camera/rh/proj.rs` says so), and it
/// put every Wipeout HD craft's shadow on the wrong side of the craft, with
/// the silhouette turning the wrong way as the craft turned relative to the
/// sun. So this caster sits above the fit's centre under a tilted light,
/// which lands it well into the upper half of the map, and the assertion
/// is that the receiver's formula picks a covered texel there while the
/// mirrored row is clear. The caster is offset sideways as well, so the
/// column mapping is pinned the same way - a horizontal mirror would put a
/// craft's shadow on the wrong side of it just as surely.
#[test]
fn the_receivers_lookup_reads_the_row_the_caster_landed_in() {
    let Some(adapter) = pollster::block_on(
        wgpu::Instance::default().request_adapter(&wgpu::RequestAdapterOptions::default()),
    )
    .ok() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let mut map = Map::new(&device);
    let (vertices, indices) = quad();
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster vertices"),
        size: std::mem::size_of_val(&vertices) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("caster indices"),
        size: std::mem::size_of_val(&indices) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
    // A tilted light, the shape of a circuit's own sun, and the caster
    // lifted above the fit's centre: in the light's view that is a shift
    // along the map's vertical axis, which is the axis the mirror flips.
    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 4.0,
        towards_light: Vec3::new(0.3, 0.8, 0.2).normalize(),
    };
    let model = Mat4::from_translation(Vec3::new(2.5, 2.5, 0.0));
    let whole = 0..indices.len() as u32;
    let ranges = std::slice::from_ref(&whole);
    let caster = Caster {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        ranges,
        model,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(&queue, &mut encoder, &fit, &[caster]);
    map.render_depth(&queue, &mut encoder, &fit, &[caster]);
    queue.submit([encoder.finish()]);

    // The same arithmetic as `shadow_coverage` in `mesh.wgsl`, for the
    // caster's own centre, against each map's own matrix and size.
    let lookup = |matrix: Mat4, size: u32| {
        let clip = matrix * model.transform_point3(Vec3::ZERO).extend(1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(
            ndc.y.abs() > 0.2,
            "the fixture has to land well off the centre line to test anything, got ndc.y {}",
            ndc.y
        );
        assert!(
            ndc.x.abs() > 0.2,
            "the fixture has to land well off the centre column too, got ndc.x {}",
            ndc.x
        );
        let column = ((ndc.x * 0.5 + 0.5) * size as f32) as u32;
        let row = ((0.5 - ndc.y * 0.5) * size as f32) as u32;
        let mirrored = ((ndc.y * 0.5 + 0.5) * size as f32) as u32;
        let mirrored_column = ((0.5 - ndc.x * 0.5) * size as f32) as u32;
        (column, row, mirrored, ndc.z, mirrored_column)
    };

    let texels = read_back(&device, &queue, &map);
    let (column, row, mirrored, _, mirrored_column) = lookup(map.matrix(), SIZE);
    let at = |x: u32, y: u32| texels[(y * SIZE + x) as usize];
    assert_eq!(
        at(mirrored_column, row),
        0,
        "the column mirrored about the centre is clear"
    );
    assert_eq!(at(column, row), 0xff, "the receiver's row is covered");
    assert_eq!(at(column, mirrored), 0, "the mirrored row is clear");

    let depth = read_back_depth(&device, &queue, &map);
    let (column, row, mirrored, z, _) = lookup(map.depth_matrix(), DEPTH_SIZE);
    let at = |x: u32, y: u32| depth[(y * DEPTH_SIZE + x) as usize];
    assert!(
        (at(column, row) - z).abs() < 1e-3,
        "the receiver's row holds the caster's own depth: {} against {z}",
        at(column, row)
    );
    assert_eq!(
        at(column, mirrored),
        1.0,
        "the mirrored row is at the far plane"
    );
    // The receiver's own comparison for a surface that cast this very depth:
    // `mesh.wgsl` lights a pixel when `ndc.z - depth_bias <= nearest`, and a
    // caster has to come out lit against its own texel or the whole `mapped`
    // tier is acne. The rasterizer's slope-scaled bias in `Map::new` is what
    // pushes the written depth past `z`; this checks that it pushed it the
    // right way, for the smallest shader-side bias the tier could use.
    assert!(
        z <= at(column, row),
        "a caster's own surface reads shadowed against its own texel: z {z} against {}",
        at(column, row)
    );
}
