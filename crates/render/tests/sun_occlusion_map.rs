//! Draws a track into one layer of the per-craft sun-occlusion array, reads
//! the layer back, and asserts the road's own mask landed where the craft
//! would sample it.
//!
//! **The pass has no other observable**, for the reason `shadow_map_coverage.rs`
//! gives: the map is consumed by a sampler inside the hull's shader, so a
//! pass that draws nothing, draws the albedo instead of the mask, or forgets
//! the cull all produce a frame that still renders. Reading the texels is the
//! check that cannot be fooled that way.
//!
//! Skips when there is no adapter, like `velocity_target.rs`.

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, slots};
use oag_mesh::mesh_render::OCCLUSION_LAYERS;
use oag_mesh::mesh_render::material_bind_group_layout;
use oag_render::shadow::map::Fit;
use oag_render::shadow::occlusion::{Maps, RADIUS, SIZE, Track};

fn vertex(position: [f32; 3], lightmap_texcoord: [f32; 2], sun_mask: f32, slots: u32) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 1.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord,
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask,
        slots,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

/// A quad in the `y = 0` plane, `half` units to each side of `centre`, whose
/// lightmap coordinate runs `u` 0 to 1 across it (`x`) and `v` 0 to 1 along
/// it (`z`).
fn quad(centre: [f32; 3], half: f32, sun_mask: f32, slots: u32) -> ([GpuVertex; 4], [u32; 6]) {
    let [cx, cy, cz] = centre;
    (
        [
            vertex([cx - half, cy, cz - half], [0.0, 0.0], sun_mask, slots),
            vertex([cx + half, cy, cz - half], [1.0, 0.0], sun_mask, slots),
            vertex([cx + half, cy, cz + half], [1.0, 1.0], sun_mask, slots),
            vertex([cx - half, cy, cz + half], [0.0, 1.0], sun_mask, slots),
        ],
        [0, 2, 1, 0, 3, 2],
    )
}

fn draw(range: std::ops::Range<u32>, centre: [f32; 3], radius: f32, moving: bool) -> DrawCall {
    DrawCall {
        range,
        texture: Some(0),
        bounds: Bounds { centre, radius },
        moving,
        culled: false,
        blend: None,
        blend_state: None,
        layer: 0,
        alpha_test_ref: None,
        node: None,
        chunk: None,
    }
}

/// Reads one layer back, through the map's own readback.
fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, maps: &Maps, layer: u32) -> Vec<u8> {
    maps.read_back(device, queue, layer)
}

/// A material bind group in `mesh_render`'s albedo layout: a white albedo
/// and a 2x1 lightmap whose left texel has alpha 0 and right texel alpha 255.
fn material(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::BindGroup {
    let make = |width: u32, rgba: &[u8], label: &str| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        texture.create_view(&Default::default())
    };
    let albedo = make(1, &[255, 255, 255, 255], "albedo");
    let lightmap = make(2, &[0, 0, 0, 0, 255, 255, 255, 255], "lightmap");
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material"),
        layout: &material_bind_group_layout(device),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&albedo),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&lightmap),
            },
            // The pad mask and the magstrip wave, never sampled by this pass.
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&lightmap),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&lightmap),
            },
        ],
    })
}

#[test]
fn the_road_under_the_craft_writes_its_own_mask_and_the_far_road_is_culled() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let mut maps = Maps::new(&device, &material_bind_group_layout(&device));

    // Three quads in one buffer: the lightmapped road under the craft, a
    // vertex-masked road just beside it at half sun, and a lightmapped road
    // far down the circuit that the cull must leave out.
    let (road, road_indices) = quad([0.0, 0.0, 0.0], 2.0, 1.0, slots::SECOND_IS_LIGHTMAP);
    let (beside, beside_indices) = quad([0.0, 0.0, 6.0], 2.0, 0.5, slots::DEFAULT);
    let (far, far_indices) = quad([0.0, 0.0, 60.0], 2.0, 1.0, slots::SECOND_IS_LIGHTMAP);
    let vertices: Vec<GpuVertex> = [road, beside, far].concat();
    let indices: Vec<u32> = road_indices
        .iter()
        .copied()
        .chain(beside_indices.iter().map(|i| i + 4))
        .chain(far_indices.iter().map(|i| i + 8))
        .collect();
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("track vertices"),
        size: std::mem::size_of_val(&vertices[..]) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("track indices"),
        size: std::mem::size_of_val(&indices[..]) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
    let draws = [
        draw(0..6, [0.0, 0.0, 0.0], 3.0, false),
        draw(6..12, [0.0, 0.0, 6.0], 3.0, false),
        draw(12..18, [0.0, 0.0, 60.0], 3.0, false),
    ];
    // `texture: Some(0)` indexes `materials[1]`, as the main pass does; slot 0
    // is the untextured placeholder's and is never reached here.
    let materials = [material(&device, &queue), material(&device, &queue)];
    let track = Track {
        vertices: &vertex_buffer,
        indices: &index_buffer,
        draws: &draws,
        cutouts: &[],
        transparent: &[],
        materials: &materials,
        model: Mat4::IDENTITY,
    };

    // The craft: four units over the origin, its box eight units across, so
    // the road under it spans the middle half of the map and the road beside
    // it lands in the map's lower quarter.
    let fit = Fit {
        centre: Vec3::new(0.0, 4.0, 0.0),
        radius: 8.0,
        towards_light: Vec3::Y,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let drawn = maps.render(&queue, &mut encoder, 3, &fit, &track);
    queue.submit([encoder.finish()]);
    assert_eq!(drawn, 2, "the road and the road beside it, not the far one");
    assert_eq!(maps.drawn(3), 2);
    const {
        assert!(
            60.0 - 3.0 > RADIUS,
            "the fixture's far road is beyond the cull"
        )
    };

    let texels = read_back(&device, &queue, &maps, 3);
    let at = |x: u32, y: u32| texels[(y * SIZE + x) as usize];
    let middle = SIZE / 2;
    // The lightmapped road: one half samples the atlas texel with alpha 0
    // and the other the one with alpha 255. Which way `+x` lands is the
    // fit's own basis - the receiver projects through the same matrix, so
    // the map's handedness is not this test's business, only that the mask
    // and not the albedo (white on both halves) is what was written.
    let halves = [
        at(middle - SIZE / 16, middle),
        at(middle + SIZE / 16, middle),
    ];
    assert!(
        halves == [0, 0xff] || halves == [0xff, 0],
        "the road's two halves carry the lightmap's alpha, got {halves:?}"
    );
    // Nothing is drawn where nothing is: the map's corners, and between the
    // two roads, stay at the black clear - what the hull reads as no sun.
    assert_eq!(at(0, 0), 0, "the corner");
    assert_eq!(at(SIZE - 1, SIZE - 1), 0, "the far corner");
    // The vertex-masked road beside it, centred at `z = 6` in a box of
    // radius 8: seven eighths of the way across, in whichever row direction
    // `z` maps to. Half sun, from `sun_mask` alone.
    let beside_row = if at(middle, SIZE * 7 / 8) != 0 {
        SIZE * 7 / 8
    } else {
        SIZE / 8
    };
    let half = at(middle, beside_row);
    assert!(
        (0x7f..=0x81).contains(&half),
        "the vertex-masked road reads half sun, got {half:#x}"
    );

    // Clearing the layer empties it and forgets the count.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    maps.clear(&mut encoder, 3);
    queue.submit([encoder.finish()]);
    let cleared = read_back(&device, &queue, &maps, 3);
    assert!(cleared.iter().all(|texel| *texel == 0), "the layer cleared");
    assert_eq!(maps.drawn(3), 0);
    assert_eq!(maps.matrix(3), Mat4::IDENTITY);

    // And the other layers were never touched: layer 0 reads black.
    let untouched = read_back(&device, &queue, &maps, 0);
    assert!(
        untouched.iter().all(|texel| *texel == 0),
        "layer 0 untouched"
    );
    const { assert!(OCCLUSION_LAYERS > 3) };
}
