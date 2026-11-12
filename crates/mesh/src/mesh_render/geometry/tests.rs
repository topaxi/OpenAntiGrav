use super::*;
use crate::mesh_render::pipeline_cache::tests::{device_with_ceiling, triangle_model};

/// `quads` quads of four vertices, one draw each.
pub(in crate::mesh_render) fn quads(quads: u32) -> Model {
    let mut model = triangle_model();
    let vertex = model.vertices[0];
    model.vertices = vec![vertex; quads as usize * 4];
    model.indices.clear();
    model.draws.clear();
    let template = triangle_model().draws.remove(0);
    for quad in 0..quads {
        let start = model.indices.len() as u32;
        model.indices.extend([0, 1, 2, 0, 2, 3].map(|i| quad * 4 + i));
        let mut draw = template.clone();
        draw.range = start..start + 6;
        model.draws.push(draw);
    }
    model
}

#[test]
fn a_device_with_a_small_buffer_ceiling_gets_several_buffers_each_under_it() {
    let ceiling = 8 * VERTEX_BYTES;
    let Some((device, queue)) = device_with_ceiling(ceiling) else {
        return;
    };
    assert_eq!(device.limits().max_buffer_size, ceiling);
    let model = quads(6);
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let geometry = Geometry::new(&device, &model, device.limits().max_buffer_size).unwrap();
    geometry.write_all(&queue, &model.vertices);
    geometry.write(&queue, 6, &model.vertices[..12]);
    assert!(pollster::block_on(scope.pop()).is_none());
    assert_eq!(geometry.buffer_count(), 3, "24 vertices under a cap of 8");
    assert!(geometry.largest_buffer() <= ceiling);
    assert!(!geometry.is_whole());
    let total: u64 = geometry.buffers().iter().map(wgpu::Buffer::size).sum();
    assert_eq!(total, 24 * VERTEX_BYTES);
}

#[test]
fn a_model_that_fits_keeps_its_one_buffer() {
    let Some((device, _queue)) = device_with_ceiling(1 << 20) else {
        return;
    };
    let model = quads(6);
    let geometry = Geometry::new(&device, &model, device.limits().max_buffer_size).unwrap();
    assert!(geometry.is_whole());
    assert_eq!(geometry.buffer_count(), 1);
    assert_eq!(geometry.sole().size(), 24 * VERTEX_BYTES);
}
