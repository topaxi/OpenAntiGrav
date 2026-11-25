//! A chunk's authored tangent stream.

use super::{Error, Mesh, NORMAL_OFFSET, Result, SubMesh, unpack_normal};
use oag_bytes::ByteOrder;

/// Where an inline (undeclared) chunk keeps its tangent when it has one: right
/// after the packed normal. Read off the live RSX arrays of HD's Detonator mine
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The lit draw, read live"):
/// position `S32K x3` at `+0`, normal `CMP` at `+6`, tangent `UB x4` at `+10`, `Uv1` `SF x2` at `+14`.
const INLINE_OFFSET: usize = NORMAL_OFFSET + 4;

/// How far a decoded inline tangent may be from unit length, and from perpendicular
/// to its own normal, for the four bytes to count as one. The bytes quantise each
/// component to `2 / 255`.
const INLINE_TOLERANCE: f32 = 0.06;

impl Mesh {
    /// The `tangent` attribute's four raw bytes per vertex, or `None` where the
    /// chunk declares none.
    ///
    /// Raw, because the vertex program decodes them itself: `xyz` as `2b - 1`
    /// (a unit vector, perpendicular to the normal) and `w` as the factor on
    /// the binormal `cross(N, T) * w`. `docs/rendering/hd-ship-materials.md`,
    /// "Hull normal maps".
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] as [`Self::positions`].
    pub fn tangents(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Option<Vec<[u8; 4]>>> {
        let Some(attribute) = self.decl.as_ref().and_then(|decl| decl.tangent()) else {
            return Ok(None);
        };
        self.four_bytes(data, submesh, stride, usize::from(attribute.offset))
            .map(Some)
    }

    /// An inline (undeclared) chunk's tangent: the four bytes after the normal,
    /// raw as [`Self::tangents`], or `None` where the chunk declares a layout or
    /// those bytes are not one.
    ///
    /// An inline chunk states nothing, so the bytes count as a tangent only when
    /// every vertex there decodes to a unit vector perpendicular to that vertex's
    /// normal, which a coordinate pair or a colour does not. Ask for it only where
    /// the material's vertex program is known to read a tangent there (HD's
    /// Detonator mine).
    ///
    /// # Errors
    ///
    /// [`Error::OutOfBounds`] as [`Self::positions`].
    pub fn inline_tangents(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
    ) -> Result<Option<Vec<[u8; 4]>>> {
        if self.decl.is_some() || stride < INLINE_OFFSET + 4 {
            return Ok(None);
        }
        let bytes = self.four_bytes(data, submesh, stride, INLINE_OFFSET)?;
        Ok(inline_tangents_hold(data, submesh, stride, &bytes).then_some(bytes))
    }

    fn four_bytes(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
        offset: usize,
    ) -> Result<Vec<[u8; 4]>> {
        let end =
            submesh.vertex_offset + stride * submesh.vertex_count.saturating_sub(1) + offset + 4;
        if submesh.vertex_count > 0 && end > data.len() {
            return Err(Error::OutOfBounds {
                what: "a vertex buffer's tangents",
                end,
                len: data.len(),
            });
        }
        Ok((0..submesh.vertex_count)
            .map(|k| {
                let at = submesh.vertex_offset + k * stride + offset;
                [data[at], data[at + 1], data[at + 2], data[at + 3]]
            })
            .collect())
    }
}

/// Whether every inline vertex's four bytes after the normal read as a unit
/// tangent perpendicular to its own normal.
fn inline_tangents_hold(data: &[u8], submesh: &SubMesh, stride: usize, bytes: &[[u8; 4]]) -> bool {
    !bytes.is_empty()
        && bytes.iter().enumerate().all(|(k, b)| {
            let t = [b[0], b[1], b[2]].map(|v| f32::from(v) / 255.0 * 2.0 - 1.0);
            let at = submesh.vertex_offset + k * stride + NORMAL_OFFSET;
            let n = unpack_normal(ByteOrder::Big.u32(data, at));
            let length = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
            let along = t[0] * n[0] + t[1] * n[1] + t[2] * n[2];
            (length - 1.0).abs() < INLINE_TOLERANCE && along.abs() < INLINE_TOLERANCE
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rcsmodel::{Layout, Space};

    fn inline_mesh() -> Mesh {
        Mesh {
            hash: 0,
            bias: [0.0; 3],
            scale: [1.0 / 128.0; 3],
            material: 0,
            layout: Layout::Inline,
            submeshes: Vec::new(),
            decl: None,
            space: Space::World,
            render_flags: 0,
            extra_surfaces: Vec::new(),
        }
    }

    fn submesh(vertex_count: usize) -> SubMesh {
        SubMesh {
            format: [0x83, 0x08, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00],
            vertex_count,
            vertex_offset: 0,
            index_count: 0,
            index_offset: 0,
        }
    }

    /// One stride-18 vertex: position, normal `+x` (`0x000003ff`), four bytes, a half pair.
    fn vertex(four: [u8; 4]) -> Vec<u8> {
        let mut v = vec![0u8; 6];
        v.extend_from_slice(&0x0000_03ffu32.to_be_bytes());
        v.extend_from_slice(&four);
        v.extend_from_slice(&[0x38, 0x00, 0x38, 0x00]);
        v
    }

    #[test]
    fn an_inline_unit_tangent_perpendicular_to_the_normal_is_read() {
        let data = [vertex([128, 255, 128, 255]), vertex([128, 0, 128, 255])].concat();
        let got = inline_mesh()
            .inline_tangents(&data, &submesh(2), 18)
            .expect("in bounds");
        assert_eq!(got, Some(vec![[128, 255, 128, 255], [128, 0, 128, 255]]));
    }

    #[test]
    fn inline_bytes_that_are_not_a_tangent_are_refused() {
        // Along the normal, then a colour: neither is a tangent of `+x`.
        for four in [[255, 128, 128, 255], [255, 255, 255, 204]] {
            let got = inline_mesh()
                .inline_tangents(&vertex(four), &submesh(1), 18)
                .expect("in bounds");
            assert_eq!(got, None, "{four:?}");
        }
    }
}
