//! Draws casters into the shadow map atlas, reads it back, and asserts what
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
use oag_render::shadow::map::{
    ATLAS_HEIGHT, ATLAS_WIDTH, Caster, CraftCaster, DEPTH_SIZE, Fit, Map, RANK_SIDES,
};

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
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
        texcoord2: [0.0, 0.0],
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

/// The quad's vertex and index buffers, uploaded, and its index count.
fn quad_buffers(device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::Buffer, wgpu::Buffer, u32) {
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
    (vertex_buffer, index_buffer, indices.len() as u32)
}

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return None;
    };
    Some(
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device"),
    )
}

/// A craft's own square of the atlas in texels, `(x, y, side)`, read off the
/// record the receiver reads rather than off the constants, so a receiver and
/// a caster that disagreed about it would fail here too.
fn square_of(map: &Map, slot: usize) -> (u32, u32, u32) {
    let rect = map.craft_shadows()[slot].rect;
    (
        (rect[0] * ATLAS_WIDTH as f32).round() as u32,
        (rect[1] * ATLAS_HEIGHT as f32).round() as u32,
        (rect[2] * ATLAS_WIDTH as f32).round() as u32,
    )
}

#[test]
fn a_craft_covers_the_middle_of_its_square_and_leaves_the_rest_of_the_atlas_clear() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut map = Map::new(&device);
    let (vertex_buffer, index_buffer, index_count) = quad_buffers(&device, &queue);

    // Straight down at the origin, fitted to twice the caster's own half-width
    // so the quad covers the middle quarter of the craft's square and nothing
    // reaches its border.
    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 2.0,
        towards_light: Vec3::Y,
    };
    // A single range, built as a slice so clippy's `single_range_in_vec_init`
    // does not read it as an indexing mistake: a `Caster` takes the model's
    // own draw-call ranges, and this fixture has one draw call.
    let whole = 0..index_count;
    let ranges = std::slice::from_ref(&whole);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(
        &queue,
        &mut encoder,
        &[CraftCaster {
            slot: 3,
            rank: 0,
            fit,
            caster: Caster {
                vertices: &vertex_buffer,
                indices: &index_buffer,
                ranges,
                model: Mat4::IDENTITY,
            },
        }],
    );
    queue.submit([encoder.finish()]);
    let texels = map.read_back(&device, &queue);
    assert_eq!(map.casters(), 1);

    // The record is the craft's slot's, not the first index's, and holds the
    // rank's own square.
    let (x0, y0, side) = square_of(&map, 3);
    assert_eq!((x0, y0, side), (0, 0, RANK_SIDES[0]));
    assert_eq!(map.craft_shadows()[0].rect, [0.0; 4], "an unused slot");

    let at = |x: u32, y: u32| texels[(y * ATLAS_WIDTH + x) as usize];
    let middle = (x0 + side / 2, y0 + side / 2);
    assert_eq!(at(middle.0, middle.1), 0xff, "the caster's own centre");
    // A quarter of the way in from a corner is outside a caster that spans the
    // middle half, and the border is what the receiver's clamped read takes:
    // both have to be clear, or every surface outside the box draws shadowed.
    assert_eq!(at(x0, y0), 0, "the square's corner");
    assert_eq!(at(x0 + side - 1, y0 + side - 1), 0, "its far corner");
    assert_eq!(at(middle.0, y0 + 2), 0, "its top border");

    // And the coverage is the shape it was given: the quad's own edges at a
    // quarter and three quarters across, within a texel of the fit.
    let quarter = side / 4;
    assert_eq!(
        at(x0 + quarter + 2, middle.1),
        0xff,
        "just inside the left edge"
    );
    assert_eq!(at(x0 + quarter - 2, middle.1), 0, "just outside it");

    // Nothing leaks outside the square into the rest of the atlas.
    let covered = texels.iter().filter(|texel| **texel != 0).count();
    let inside = (side / 2) * (side / 2);
    assert!(
        covered.abs_diff(inside as usize) < (inside as usize) / 50,
        "{covered} texels covered, the quad's own area is {inside}"
    );

    // Rendering again with no casters clears it: last frame's coverage must
    // not drag behind a craft that stopped casting.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(&queue, &mut encoder, &[]);
    queue.submit([encoder.finish()]);
    let cleared = map.read_back(&device, &queue);
    assert!(cleared.iter().all(|texel| *texel == 0), "the atlas cleared");
    assert_eq!(map.casters(), 0);
    assert_eq!(map.craft_shadows()[3].rect, [0.0; 4], "and the records");

    // A second empty pass over a map that is already clear records nothing, and
    // the map is still clear: the clear above was the one that mattered.
    let before = map.passes_recorded();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(&queue, &mut encoder, &[]);
    queue.submit([encoder.finish()]);
    assert_eq!(
        map.passes_recorded(),
        before,
        "an empty pass over a clear atlas is skipped"
    );
    assert!(
        map.read_back(&device, &queue)
            .iter()
            .all(|texel| *texel == 0)
    );
}

/// The depth map keeps one clear on the frame it stops drawing, skips the empty
/// passes after it, and records again the moment a caster arrives.
#[test]
fn an_empty_depth_pass_over_a_clear_map_is_skipped_and_a_caster_still_draws() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut map = Map::new(&device);
    let (vertex_buffer, index_buffer, index_count) = quad_buffers(&device, &queue);
    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 2.0,
        towards_light: Vec3::Y,
    };
    let whole = 0..index_count;
    let ranges = std::slice::from_ref(&whole);
    let run = |map: &mut Map, casters: &[Caster<'_>]| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        map.render_depth(&queue, &mut encoder, &fit, casters);
        queue.submit([encoder.finish()]);
    };
    let caster = || Caster {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        ranges,
        model: Mat4::IDENTITY,
    };
    run(&mut map, &[caster()]);
    let drawn = map.passes_recorded();
    run(&mut map, &[]);
    assert_eq!(
        map.passes_recorded(),
        drawn + 1,
        "the transition clears once"
    );
    let depth = read_back_depth(&device, &queue, &map);
    assert!(
        depth.iter().all(|d| *d == 1.0),
        "and the map is at the far plane"
    );
    run(&mut map, &[]);
    run(&mut map, &[]);
    assert_eq!(
        map.passes_recorded(),
        drawn + 1,
        "empty passes after it are skipped"
    );
    run(&mut map, &[caster()]);
    assert_eq!(map.passes_recorded(), drawn + 2, "a caster records again");
    let depth = read_back_depth(&device, &queue, &map);
    assert!(depth.iter().any(|d| *d < 1.0), "and is drawn");
}

/// **The check that fails if the per-craft maps are dropped.** Two craft 500
/// units apart each get their own square at their own rank's side, and each
/// is crisp in it - the quad covers the same fraction of a 512 square and of a
/// 256 one. One map fitted to both would need a box 500 units across, where a
/// two-unit quad is under one texel: neither craft would show.
#[test]
fn craft_far_apart_each_draw_into_the_square_of_their_own_rank() {
    let Some((device, queue)) = device() else {
        return;
    };
    let mut map = Map::new(&device);
    let (vertex_buffer, index_buffer, index_count) = quad_buffers(&device, &queue);
    let whole = 0..index_count;
    let ranges = std::slice::from_ref(&whole);
    let craft = |slot: usize, rank: usize, at: Vec3| {
        let model = Mat4::from_translation(at);
        CraftCaster {
            slot,
            rank,
            fit: Fit {
                centre: at,
                radius: 2.0,
                towards_light: Vec3::Y,
            },
            caster: Caster {
                vertices: &vertex_buffer,
                indices: &index_buffer,
                ranges,
                model,
            },
        }
    };
    let near = Vec3::new(0.0, 0.0, 0.0);
    let far = Vec3::new(500.0, 0.0, -300.0);

    let render = |map: &mut Map, crafts: &[CraftCaster<'_>]| {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        map.render(&queue, &mut encoder, crafts);
        queue.submit([encoder.finish()]);
        map.read_back(&device, &queue)
    };

    // Slot 0 nearest (rank 0, 512), slot 1 the third nearest (rank 2, 256).
    let texels = render(&mut map, &[craft(0, 0, near), craft(1, 2, far)]);
    let (x0, y0, side0) = square_of(&map, 0);
    let (x1, y1, side1) = square_of(&map, 1);
    assert_eq!((side0, side1), (RANK_SIDES[0], RANK_SIDES[2]));
    assert_ne!((x0, y0), (x1, y1), "two craft, two squares");
    let covered_in = |x: u32, y: u32, side: u32| {
        (y..y + side)
            .flat_map(|row| (x..x + side).map(move |column| (column, row)))
            .filter(|(column, row)| texels[(row * ATLAS_WIDTH + column) as usize] == 0xff)
            .count()
    };
    for (x, y, side) in [(x0, y0, side0), (x1, y1, side1)] {
        let area = ((side / 2) * (side / 2)) as usize;
        let got = covered_in(x, y, side);
        assert!(
            got.abs_diff(area) < area / 50,
            "the square at ({x}, {y}) side {side} has {got} covered texels, its craft's area is {area}"
        );
    }
    let total = texels.iter().filter(|texel| **texel != 0).count();
    assert_eq!(
        total,
        covered_in(x0, y0, side0) + covered_in(x1, y1, side1),
        "nothing outside the two squares"
    );

    // The rank is the frame's: swap the two and the squares and sides swap.
    let _ = render(&mut map, &[craft(0, 2, near), craft(1, 0, far)]);
    assert_eq!(square_of(&map, 0), (x1, y1, side1));
    assert_eq!(square_of(&map, 1), (x0, y0, side0));
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
    let Some((device, queue)) = device() else {
        return;
    };
    let mut map = Map::new(&device);
    let (vertex_buffer, index_buffer, index_count) = quad_buffers(&device, &queue);

    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 2.0,
        towards_light: Vec3::Y,
    };
    let whole = 0..index_count;
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

/// The receiver's lookup in `shadow.wesl` - `uv = (ndc.x * 0.5 + 0.5, 0.5 -
/// ndc.y * 0.5)`, placed in the craft's square - reads the texel the caster
/// pass actually wrote, in both maps.
///
/// **An off-centre caster, deliberately.** A centred caster cannot tell the
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
    let Some((device, queue)) = device() else {
        return;
    };
    let mut map = Map::new(&device);
    let (vertex_buffer, index_buffer, index_count) = quad_buffers(&device, &queue);
    // A tilted light, the shape of a circuit's own sun, and the caster
    // lifted above the fit's centre: in the light's view that is a shift
    // along the map's vertical axis, which is the axis the mirror flips.
    let fit = Fit {
        centre: Vec3::ZERO,
        radius: 4.0,
        towards_light: Vec3::new(0.3, 0.8, 0.2).normalize(),
    };
    let model = Mat4::from_translation(Vec3::new(2.5, 2.5, 0.0));
    let whole = 0..index_count;
    let ranges = std::slice::from_ref(&whole);
    let caster = Caster {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        ranges,
        model,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    map.render(
        &queue,
        &mut encoder,
        &[CraftCaster {
            slot: 0,
            rank: 1,
            fit,
            caster,
        }],
    );
    map.render_depth(&queue, &mut encoder, &fit, &[caster]);
    queue.submit([encoder.finish()]);

    // The same arithmetic as `craft_shadow_coverage` in `shadow.wesl`, for the
    // caster's own centre, against each map's own matrix and placement: the
    // atlas rectangle for the craft map, the whole texture for the depth map.
    let lookup = |matrix: Mat4, rect: [f32; 4], width: u32, height: u32| {
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
        let texel = |u: f32, v: f32| {
            (
                ((rect[0] + u * rect[2]) * width as f32) as u32,
                ((rect[1] + v * rect[3]) * height as f32) as u32,
            )
        };
        let (column, row) = texel(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        let (_, mirrored) = texel(ndc.x * 0.5 + 0.5, ndc.y * 0.5 + 0.5);
        let (mirrored_column, _) = texel(0.5 - ndc.x * 0.5, 0.5 - ndc.y * 0.5);
        (column, row, mirrored, ndc.z, mirrored_column)
    };

    let texels = map.read_back(&device, &queue);
    let record = map.craft_shadows()[0];
    let (column, row, mirrored, _, mirrored_column) = lookup(
        Mat4::from_cols_array_2d(&record.matrix),
        record.rect,
        ATLAS_WIDTH,
        ATLAS_HEIGHT,
    );
    let at = |x: u32, y: u32| texels[(y * ATLAS_WIDTH + x) as usize];
    assert_eq!(
        at(mirrored_column, row),
        0,
        "the column mirrored about the centre is clear"
    );
    assert_eq!(at(column, row), 0xff, "the receiver's row is covered");
    assert_eq!(at(column, mirrored), 0, "the mirrored row is clear");

    let depth = read_back_depth(&device, &queue, &map);
    let (column, row, mirrored, z, _) = lookup(
        map.depth_matrix(),
        [0.0, 0.0, 1.0, 1.0],
        DEPTH_SIZE,
        DEPTH_SIZE,
    );
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
    // `shadow.wesl` lights a pixel when `ndc.z - depth_bias <= nearest`, and a
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
