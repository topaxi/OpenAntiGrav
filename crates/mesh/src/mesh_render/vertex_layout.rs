//! Where [`super::build_with`]'s pipelines read their vertex attributes from:
//! all from the vertex, or `texcoord` from a buffer of its own. Split out of
//! `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`.

use crate::mesh::GpuVertex;

/// [`GpuVertex`]'s attributes, in location order with their real offsets.
pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 13] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
    4 => Float32, 5 => Uint32, 6 => Float32x2, 7 => Uint32, 8 => Float32,
    9 => Uint32, 10 => Float32, 11 => Float32, 12 => Float32x2
];

/// [`VERTEX_ATTRIBUTES`] without `texcoord`, for [`Texcoords::Streamed`].
/// Filtered rather than written as a second `vertex_attr_array!`, which lays
/// offsets out by position and would shift every later attribute eight bytes
/// - a mistake that draws, just wrongly.
pub(super) fn streamed_attributes() -> Vec<wgpu::VertexAttribute> {
    VERTEX_ATTRIBUTES
        .iter()
        .filter(|attribute| attribute.shader_location != 3)
        .copied()
        .collect()
}

/// Where a model's pipelines read each vertex's `texcoord` from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Texcoords {
    /// From the vertex itself, like every other attribute.
    #[default]
    Interleaved,
    /// From a second vertex buffer, slot 1, of one `[f32; 2]` per vertex -
    /// which the caller creates and binds. For a model that regenerates only
    /// its coordinates every frame: the circuit's environment-mapped shine,
    /// which otherwise rebuilt and uploaded every whole vertex to change eight
    /// bytes of each.
    Streamed,
}

/// The vertex buffer layouts for `texcoords`, `streamed` being
/// [`streamed_attributes`] - borrowed, so the caller owns it for as long as
/// the layouts live.
pub(super) fn buffers(
    texcoords: Texcoords,
    streamed: &[wgpu::VertexAttribute],
) -> Vec<Option<wgpu::VertexBufferLayout<'_>>> {
    let vertex = |attributes| {
        Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes,
        })
    };
    match texcoords {
        Texcoords::Interleaved => vec![vertex(&VERTEX_ATTRIBUTES[..])],
        Texcoords::Streamed => vec![
            vertex(streamed),
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![3 => Float32x2],
            }),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every attribute but `texcoord` keeps the offset it has interleaved, so
    /// one vertex buffer serves both layouts.
    #[test]
    fn streaming_texcoords_moves_no_other_attribute() {
        let streamed = streamed_attributes();
        assert_eq!(streamed.len(), VERTEX_ATTRIBUTES.len() - 1);
        for attribute in &streamed {
            let interleaved = VERTEX_ATTRIBUTES
                .iter()
                .find(|a| a.shader_location == attribute.shader_location)
                .unwrap();
            assert_eq!(attribute, interleaved);
        }
        assert!(streamed.iter().all(|a| a.shader_location != 3));
    }
}
